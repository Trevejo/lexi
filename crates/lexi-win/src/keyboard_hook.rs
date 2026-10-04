use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
#[cfg(windows)]
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use lexi_core::config::{AppConfig, DismissMode};

static HOOK_HANDLE: Mutex<Option<usize>> = Mutex::new(None);
static IS_OVERLAY_VISIBLE: AtomicBool = AtomicBool::new(false);
static DISMISS_MODE: AtomicU32 = AtomicU32::new(0); // 0: ReleaseKey, 1: Timeout, 2: Hotkey, 3: AnyKey, 4: Hybrid
static DISMISS_REQUESTED: AtomicBool = AtomicBool::new(false);

pub struct KeyboardHookManager;

impl KeyboardHookManager {
    pub fn init(config: &AppConfig) {
        let mode_u32 = match config.dismiss.mode {
            DismissMode::ReleaseKey => 0,
            DismissMode::Timeout => 1,
            DismissMode::Hotkey => 2,
            DismissMode::AnyKey => 3,
            DismissMode::Hybrid => 4,
        };
        DISMISS_MODE.store(mode_u32, Ordering::SeqCst);

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
                log::info!("Low-level keyboard hook installed for dismiss detection");
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
        log::info!("KeyboardHookManager updated dismiss mode to {:?}", config.dismiss.mode);
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

#[cfg(windows)]
unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 && IS_OVERLAY_VISIBLE.load(Ordering::SeqCst) {
        let msg = w_param.0 as u32;
        let kbd = *(l_param.0 as *const KBDLLHOOKSTRUCT);
        let vk_code = kbd.vkCode;
        let mode = DISMISS_MODE.load(Ordering::SeqCst);

        // Escape (0x1B) always closes the overlay
        if (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) && vk_code == 0x1B {
            DISMISS_REQUESTED.store(true, Ordering::SeqCst);
        }

        // Mode 0 (ReleaseKey) & Mode 4 (Hybrid):
        // Detects when the user lifts finger off key (WM_KEYUP or WM_SYSKEYUP)
        if (mode == 0 || mode == 4) && (msg == WM_KEYUP || msg == WM_SYSKEYUP) {
            DISMISS_REQUESTED.store(true, Ordering::SeqCst);
        }

        // Mode 3 (AnyKey):
        // Any key press dismisses the overlay immediately
        if mode == 3 && (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) {
            DISMISS_REQUESTED.store(true, Ordering::SeqCst);
        }
    }

    // Always forward to game/system so inputs are never dropped
    CallNextHookEx(HHOOK(std::ptr::null_mut()), n_code, w_param, l_param)
}
