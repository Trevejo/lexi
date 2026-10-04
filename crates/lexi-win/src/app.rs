#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::ptr::{null, null_mut};
#[cfg(windows)]
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
#[cfg(windows)]
use std::thread;

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, KillTimer,
    PostMessageW, PostQuitMessage, RegisterClassW, SetTimer, TranslateMessage,
    HMENU, MSG, WM_COMMAND, WM_DESTROY, WM_RBUTTONUP, WM_TIMER, WNDCLASSW,
    WS_OVERLAPPEDWINDOW,
};

use lexi_core::config::AppConfig;
use lexi_core::models::LookupResult;
#[cfg(windows)]
use lexi_core::HandyBridgeServer;
use lexi_core::LookupService;

use crate::keyboard_hook::KeyboardHookManager;
use crate::overlay::OverlayManager;
#[cfg(windows)]
use crate::tray::{
    show_tray_context_menu, TrayIcon, IDM_TRAY_CONFIG, IDM_TRAY_EDIT_FILE, IDM_TRAY_EXIT,
    IDM_TRAY_LOG, IDM_TRAY_RELOAD, IDM_TRAY_TEST, WM_TRAY_ICON,
};
use crate::tts::TtsManager;

const WM_APP: u32 = 0x8000;
pub const WM_APP_TRIGGER_LOOKUP: u32 = WM_APP + 10;
pub const WM_APP_RESULT_READY: u32 = WM_APP + 11;
pub const TIMER_DISMISS_ID: usize = 101;
pub const TIMER_POLL_HOOK_ID: usize = 102;

struct AppState {
    config: std::sync::RwLock<AppConfig>,
    overlay: OverlayManager,
    tts: TtsManager,
    lookup_service: Arc<LookupService>,
    pending_query: Mutex<Option<String>>,
    pending_result: Mutex<Option<LookupResult>>,
    hwnd: std::sync::atomic::AtomicIsize,
}

impl AppState {
    #[allow(dead_code)]
    pub fn reload_config(&self, new_config: AppConfig) {
        log::info!("Live reloading configuration in AppState...");

        // 1. Updates state.config
        if let Ok(mut cfg) = self.config.write() {
            *cfg = new_config.clone();
        }

        // 2. Calls overlay.update_config(new_config.overlay.clone())
        self.overlay.update_config(new_config.overlay.clone());
        self.overlay.update_dismiss_mode(new_config.dismiss.mode.clone());

        // 3. Calls tts.update_config(new_config.audio.clone())
        self.tts.update_config(new_config.audio.clone());

        // 4. Calls KeyboardHookManager::update_config(&new_config)
        KeyboardHookManager::update_config(&new_config);

        // 5. Shows confirmation toast/overlay: "Configurações recarregadas com sucesso!"
        self.overlay.show_toast("Configurações recarregadas com sucesso!");
        KeyboardHookManager::set_overlay_visible(true);

        #[cfg(windows)]
        unsafe {
            let hwnd_val = self.hwnd.load(std::sync::atomic::Ordering::SeqCst);
            if hwnd_val != 0 {
                SetTimer(HWND(hwnd_val as *mut _), TIMER_DISMISS_ID, 3000, None);
            }
        }
    }
}

#[allow(dead_code)]
pub fn reload_global_config(new_config: AppConfig) {
    unsafe {
        if let Some(ref state) = GLOBAL_STATE {
            state.reload_config(new_config);
        }
    }
}

#[allow(dead_code)]
pub fn get_current_config() -> AppConfig {
    unsafe {
        if let Some(ref state) = GLOBAL_STATE {
            if let Ok(cfg) = state.config.read() {
                return cfg.clone();
            }
        }
    }
    AppConfig::default()
}

static mut GLOBAL_STATE: Option<Arc<AppState>> = None;

pub struct Application {
    config: AppConfig,
}

impl Application {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let overlay = OverlayManager::new(self.config.clone())?;
        let tts = TtsManager::new(self.config.audio.clone());
        let lookup_service = Arc::new(LookupService::new(self.config.clone()));

        KeyboardHookManager::init(&self.config);

        let app_state = Arc::new(AppState {
            config: std::sync::RwLock::new(self.config.clone()),
            overlay,
            tts,
            lookup_service,
            pending_query: Mutex::new(None),
            pending_result: Mutex::new(None),
            hwnd: std::sync::atomic::AtomicIsize::new(0),
        });

        unsafe {
            GLOBAL_STATE = Some(Arc::clone(&app_state));
        }

        #[cfg(windows)]
        unsafe {
            let class_name: Vec<u16> = OsStr::new("LexiDaemonMessageWindowClass")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(daemon_wnd_proc),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };

            let _ = RegisterClassW(&wnd_class);

            let hwnd = CreateWindowExW(
                Default::default(),
                PCWSTR(class_name.as_ptr()),
                PCWSTR(null()),
                WS_OVERLAPPEDWINDOW,
                0, 0, 0, 0,
                HWND(null_mut()),
                HMENU(null_mut()),
                None,
                None,
            )?;

            app_state.hwnd.store(hwnd.0 as isize, std::sync::atomic::Ordering::SeqCst);

            let _tray = TrayIcon::new(hwnd.0 as isize)?;
            log::info!("Lexi system tray icon registered successfully");

            // Start the Handy Bridge HTTP server
            let (query_tx, query_rx): (Sender<String>, Receiver<String>) = channel();
            let bridge = HandyBridgeServer::new(self.config.bridge.clone());
            bridge.start(query_tx)?;

            // Background worker thread to receive query and trigger Win32 window message
            let hwnd_copy = hwnd.0 as usize;
            let state_worker = Arc::clone(&app_state);
            thread::spawn(move || {
                while let Ok(query) = query_rx.recv() {
                    log::info!("App worker received query: '{}'", query);

                    // Perform lookup in worker thread (instant <5ms)
                    let res = state_worker.lookup_service.lookup(&query);
                    log::info!("Lookup for '{}' completed: Normalized='{}', Trans='{}', Source={:?}",
                        query, res.normalized, res.translation, res.source);
                    {
                        let mut lock = state_worker.pending_result.lock().unwrap();
                        *lock = Some(res);
                    }
                    let _ = PostMessageW(HWND(hwnd_copy as *mut _), WM_APP_RESULT_READY, WPARAM(0), LPARAM(0));
                }
            });

            // Message Loop
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, HWND(null_mut()), 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            drop(_tray);
            KeyboardHookManager::cleanup();
        }

        #[cfg(not(windows))]
        {
            log::info!("Lexi background loop running (non-Windows platform)...");
        }

        Ok(())
    }
}

#[cfg(windows)]
unsafe extern "system" fn daemon_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let state = match GLOBAL_STATE {
            Some(ref s) => Arc::clone(s),
            None => return DefWindowProcW(hwnd, msg, wparam, lparam),
        };

        match msg {
            WM_APP_TRIGGER_LOOKUP => {
                let query = {
                    state.pending_query.lock().unwrap().clone()
                };
                if let Some(q) = query {
                    state.overlay.show_loading(&q);
                    KeyboardHookManager::set_overlay_visible(true);
                    SetTimer(hwnd, TIMER_POLL_HOOK_ID, 16, None);
                }
                LRESULT(0)
            }
            WM_APP_RESULT_READY => {
                let result = {
                    state.pending_result.lock().unwrap().take()
                };
                if let Some(res) = result {
                    state.overlay.show_result(&res);
                    state.tts.speak_result(&res);
                    KeyboardHookManager::set_overlay_visible(true);
                    SetTimer(hwnd, TIMER_POLL_HOOK_ID, 16, None);

                    let timeout_sec = state.config.read().map(|c| c.dismiss.timeout_seconds).unwrap_or(8.0);
                    let timeout_ms = (timeout_sec * 1000.0) as u32;
                    SetTimer(hwnd, TIMER_DISMISS_ID, timeout_ms, None);
                }
                LRESULT(0)
            }
            WM_TIMER => {
                let timer_id = wparam.0;
                if timer_id == TIMER_POLL_HOOK_ID {
                    if KeyboardHookManager::check_and_reset_dismiss_requested() {
                        state.overlay.hide();
                        KeyboardHookManager::set_overlay_visible(false);
                        let _ = KillTimer(hwnd, TIMER_DISMISS_ID);
                        let _ = KillTimer(hwnd, TIMER_POLL_HOOK_ID);
                    }
                } else if timer_id == TIMER_DISMISS_ID {
                    state.overlay.hide();
                    KeyboardHookManager::set_overlay_visible(false);
                    let _ = KillTimer(hwnd, TIMER_DISMISS_ID);
                    let _ = KillTimer(hwnd, TIMER_POLL_HOOK_ID);
                }
                LRESULT(0)
            }
            WM_TRAY_ICON => {
                let event = lparam.0 as u32;
                if event == WM_RBUTTONUP {
                    show_tray_context_menu(hwnd);
                }
                LRESULT(0)
            }
            WM_COMMAND => {
                let cmd_id = (wparam.0 & 0xFFFF) as usize;
                match cmd_id {
                    IDM_TRAY_TEST => {
                        log::info!("Simulating test lookup for 'flank'");
                        let test_query = "flank".to_string();
                        let res = state.lookup_service.lookup(&test_query);
                        {
                            let mut lock = state.pending_result.lock().unwrap();
                            *lock = Some(res);
                        }
                        let _ = PostMessageW(hwnd, WM_APP_RESULT_READY, WPARAM(0), LPARAM(0));
                    }
                    IDM_TRAY_CONFIG => {
                        log::info!("Opening Settings GUI window from tray menu");
                        crate::settings::open_settings_window();
                    }
                    IDM_TRAY_EDIT_FILE => {
                        let app_dir = crate::get_local_app_dir();
                        let local_p = app_dir.join("config.toml");
                        let cfg_path = if local_p.exists() {
                            local_p
                        } else if let Ok(exe) = std::env::current_exe() {
                            if let Some(exe_dir) = exe.parent() {
                                let p = exe_dir.join("config.toml");
                                if p.exists() { p } else { std::path::PathBuf::from("config.toml") }
                            } else {
                                std::path::PathBuf::from("config.toml")
                            }
                        } else {
                            std::path::PathBuf::from("config.toml")
                        };
                        let _ = std::process::Command::new("notepad.exe")
                            .arg(cfg_path)
                            .spawn();
                    }
                    IDM_TRAY_LOG => {
                        let app_dir = crate::get_local_app_dir();
                        let local_p = app_dir.join("lexi.log");
                        let log_p = if local_p.exists() {
                            local_p
                        } else if let Ok(exe) = std::env::current_exe() {
                            if let Some(exe_dir) = exe.parent() {
                                let p = exe_dir.join("lexi.log");
                                if p.exists() { p } else { std::path::PathBuf::from("lexi.log") }
                            } else {
                                std::path::PathBuf::from("lexi.log")
                            }
                        } else {
                            std::path::PathBuf::from("lexi.log")
                        };
                        let _ = std::process::Command::new("notepad.exe")
                            .arg(log_p)
                            .spawn();
                    }
                    IDM_TRAY_RELOAD => {
                        log::info!("Configuration reload requested from tray menu");
                        let app_dir = crate::get_local_app_dir();
                        let (disk_cfg, loaded_path) = crate::load_best_config(&app_dir);
                        log::info!("Configuration reloaded from {:?}", loaded_path);
                        state.reload_config(disk_cfg);
                    }
                    IDM_TRAY_EXIT => {
                        PostQuitMessage(0);
                    }
                    _ => {}
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }));

    match result {
        Ok(lres) => lres,
        Err(e) => {
            log::error!("Panic caught in daemon_wnd_proc: {:?}", e);
            LRESULT(0)
        }
    }
}
