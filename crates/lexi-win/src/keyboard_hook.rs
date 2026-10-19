use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
#[cfg(windows)]
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, PostMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use lexi_core::config::{AppConfig, DismissMode};

static HOOK_HANDLE: Mutex<Option<usize>> = Mutex::new(None);
static IS_OVERLAY_VISIBLE: AtomicBool = AtomicBool::new(false);
static DISMISS_MODE: AtomicU32 = AtomicU32::new(0); // 0: ReleaseKey, 1: Timeout, 2: Hotkey, 3: AnyKey, 4: Hybrid
static DISMISS_REQUESTED: AtomicBool = AtomicBool::new(false);

static TRIGGER_VK: AtomicU32 = AtomicU32::new(0x78); // Default VK_F9
static ACTIVATION_MODE: AtomicU32 = AtomicU32::new(0); // 0: PushToTalk, 1: Toggle
static IS_RECORDING_VOICE: AtomicBool = AtomicBool::new(false);
static DAEMON_HWND: AtomicUsize = AtomicUsize::new(0);

pub const WM_APP: u32 = 0x8000;
pub const WM_APP_START_RECORDING: u32 = WM_APP + 20;
pub const WM_APP_STOP_RECORDING: u32 = WM_APP + 21;

pub struct KeyboardHookManager;

impl KeyboardHookManager {
    pub fn init(config: &AppConfig, daemon_hwnd: usize) {
        DAEMON_HWND.store(daemon_hwnd, Ordering::SeqCst);
        Self::update_config(config);

        #[cfg(windows)]
        unsafe {
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                HINSTANCE(std::ptr::null_mut()),
                0,
            );
            if let Ok(h) = hook {
                let mut lock = HOOK_HANDLE.lock().unwrap();
                *lock = Some(h.0 as usize);
                log::info!("Low-level keyboard hook installed successfully (hotkey={}, mode={})",
                    config.stt.hotkey, config.stt.activation_mode);
            } else {
                log::warn!("Failed to install low-level keyboard hook");
            }
        }
    }

    pub fn update_config(config: &AppConfig) {
        let mode_u32 = match config.dismiss.mode {
            DismissMode::ReleaseKey => 0,
            DismissMode::Timeout => 1,
            DismissMode::Hotkey => 2,
            DismissMode::AnyKey => 3,
            DismissMode::Hybrid => 4,
        };
        DISMISS_MODE.store(mode_u32, Ordering::SeqCst);

        let vk = parse_hotkey_vk(&config.stt.hotkey);
        TRIGGER_VK.store(vk, Ordering::SeqCst);

        let act = if config.stt.activation_mode == "toggle" { 1 } else { 0 };
        ACTIVATION_MODE.store(act, Ordering::SeqCst);

        log::info!("KeyboardHookManager updated config: trigger_vk={:#X}, act_mode={}, dismiss_mode={:?}",
            vk, act, config.dismiss.mode);
    }

    pub fn set_overlay_visible(visible: bool) {
        IS_OVERLAY_VISIBLE.store(visible, Ordering::SeqCst);
    }

    pub fn check_and_reset_dismiss_requested() -> bool {
        DISMISS_REQUESTED.swap(false, Ordering::SeqCst)
    }

    pub fn cleanup() {
        #[cfg(windows)]
        unsafe {
            let mut lock = HOOK_HANDLE.lock().unwrap();
            if let Some(h) = lock.take() {
                let hhook = HHOOK(h as *mut _);
                let _ = UnhookWindowsHookEx(hhook);
                log::info!("Low-level keyboard hook uninstalled");
            }
        }
    }
}

pub fn parse_hotkey_vk(hotkey: &str) -> u32 {
    let lower = hotkey.trim().to_lowercase();
    if lower.contains("caps") {
        0x14 // VK_CAPITAL
    } else if lower.contains("f13") {
        0x7C // VK_F13
    } else if lower.contains("f10") {
        0x79 // VK_F10
    } else if lower.contains("f11") {
        0x7A // VK_F11
    } else if lower.contains("f12") {
        0x7B // VK_F12
    } else if lower.contains("f1") {
        0x70 // VK_F1
    } else if lower.contains("f2") {
        0x71
    } else if lower.contains("f3") {
        0x72
    } else if lower.contains("f4") {
        0x73
    } else if lower.contains("f5") {
        0x74
    } else if lower.contains("f6") {
        0x75
    } else if lower.contains("f7") {
        0x76
    } else if lower.contains("f8") {
        0x77
    } else if lower.contains("f9") {
        0x78 // VK_F9
    } else if lower.contains("scroll") {
        0x91 // VK_SCROLL
    } else if lower.contains("space") {
        0x20 // VK_SPACE
    } else {
        0x78 // Default VK_F9
    }
}

#[cfg(windows)]
unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let msg = w_param.0 as u32;
        let kbd = *(l_param.0 as *const KBDLLHOOKSTRUCT);
        let vk_code = kbd.vkCode;

        let trigger_vk = TRIGGER_VK.load(Ordering::SeqCst);
        let act_mode = ACTIVATION_MODE.load(Ordering::SeqCst);
        let hwnd_val = DAEMON_HWND.load(Ordering::SeqCst);

        // 1. Audio Recording Trigger (Push-to-Talk or Toggle)
        if vk_code == trigger_vk && hwnd_val != 0 {
            if act_mode == 0 {
                // Push-to-Talk: Keydown starts recording, Keyup stops recording
                if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                    if !IS_RECORDING_VOICE.swap(true, Ordering::SeqCst) {
                        let _ = PostMessageW(HWND(hwnd_val as *mut _), WM_APP_START_RECORDING, WPARAM(0), LPARAM(0));
                    }
                } else if msg == WM_KEYUP || msg == WM_SYSKEYUP {
                    if IS_RECORDING_VOICE.swap(false, Ordering::SeqCst) {
                        let _ = PostMessageW(HWND(hwnd_val as *mut _), WM_APP_STOP_RECORDING, WPARAM(0), LPARAM(0));
                    }
                }
            } else {
                // Toggle Mode: Keydown toggles recording on/off
                if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                    let was_recording = IS_RECORDING_VOICE.fetch_xor(true, Ordering::SeqCst);
                    if was_recording {
                        let _ = PostMessageW(HWND(hwnd_val as *mut _), WM_APP_STOP_RECORDING, WPARAM(0), LPARAM(0));
                    } else {
                        let _ = PostMessageW(HWND(hwnd_val as *mut _), WM_APP_START_RECORDING, WPARAM(0), LPARAM(0));
                    }
                }
            }
        }

        // 2. Overlay Dismiss Logic
        if IS_OVERLAY_VISIBLE.load(Ordering::SeqCst) {
            let mode = DISMISS_MODE.load(Ordering::SeqCst);

            // Escape (0x1B) always closes the overlay
            if (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) && vk_code == 0x1B {
                DISMISS_REQUESTED.store(true, Ordering::SeqCst);
            }

            // Mode 0 (ReleaseKey) & Mode 4 (Hybrid):
            if (mode == 0 || mode == 4) && (msg == WM_KEYUP || msg == WM_SYSKEYUP) && vk_code != trigger_vk {
                DISMISS_REQUESTED.store(true, Ordering::SeqCst);
            }

            // Mode 3 (AnyKey):
            if mode == 3 && (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) && vk_code != trigger_vk {
                DISMISS_REQUESTED.store(true, Ordering::SeqCst);
            }
        }
    }

    // Always forward to game/system so inputs are never dropped
    CallNextHookEx(HHOOK(std::ptr::null_mut()), n_code, w_param, l_param)
}
