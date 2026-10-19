use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
#[cfg(windows)]
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, PostMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use lexi_core::config::{AppConfig, DismissMode};
use lexi_core::hotkey::HotkeyBinding;

static HOOK_HANDLE: Mutex<Option<usize>> = Mutex::new(None);
static IS_OVERLAY_VISIBLE: AtomicBool = AtomicBool::new(false);
static DISMISS_MODE: AtomicU32 = AtomicU32::new(0); // 0: ReleaseKey, 1: Timeout, 2: Hotkey, 3: AnyKey, 4: Hybrid
static DISMISS_REQUESTED: AtomicBool = AtomicBool::new(false);

static HOTKEY_BINDING: Mutex<HotkeyBinding> = Mutex::new(HotkeyBinding {
    vk: 0x78, // Default VK_F9
    ctrl: false,
    alt: false,
    shift: false,
    win: false,
});
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

        let binding = lexi_core::parse_hotkey(&config.stt.hotkey);
        if let Ok(mut lock) = HOTKEY_BINDING.lock() {
            *lock = binding;
        }

        let act = if config.stt.activation_mode == "toggle" { 1 } else { 0 };
        ACTIVATION_MODE.store(act, Ordering::SeqCst);

        log::info!("KeyboardHookManager updated config: hotkey='{}' ({}), act_mode={}, dismiss_mode={:?}",
            config.stt.hotkey, binding.to_display_string(), act, config.dismiss.mode);
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
unsafe fn are_modifiers_active(binding: &HotkeyBinding) -> bool {
    if binding.ctrl {
        let ctrl_state = GetAsyncKeyState(VK_CONTROL.0 as i32);
        if (ctrl_state as u16 & 0x8000) == 0 {
            return false;
        }
    }
    if binding.alt {
        let alt_state = GetAsyncKeyState(VK_MENU.0 as i32);
        if (alt_state as u16 & 0x8000) == 0 {
            return false;
        }
    }
    if binding.shift {
        let shift_state = GetAsyncKeyState(VK_SHIFT.0 as i32);
        if (shift_state as u16 & 0x8000) == 0 {
            return false;
        }
    }
    if binding.win {
        let lwin_state = GetAsyncKeyState(0x5B); // VK_LWIN
        let rwin_state = GetAsyncKeyState(0x5C); // VK_RWIN
        if (lwin_state as u16 & 0x8000) == 0 && (rwin_state as u16 & 0x8000) == 0 {
            return false;
        }
    }
    true
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

        let binding = if let Ok(lock) = HOTKEY_BINDING.lock() {
            *lock
        } else {
            HotkeyBinding { vk: 0x78, ctrl: false, alt: false, shift: false, win: false }
        };
        let act_mode = ACTIVATION_MODE.load(Ordering::SeqCst);
        let hwnd_val = DAEMON_HWND.load(Ordering::SeqCst);

        // 1. Audio Recording Trigger (Push-to-Talk or Toggle)
        if hwnd_val != 0 {
            if act_mode == 0 {
                // Push-to-Talk Mode:
                // Primary key press with all required modifiers active starts recording
                if (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) && vk_code == binding.vk {
                    if are_modifiers_active(&binding) {
                        if !IS_RECORDING_VOICE.swap(true, Ordering::SeqCst) {
                            let _ = PostMessageW(HWND(hwnd_val as *mut _), WM_APP_START_RECORDING, WPARAM(0), LPARAM(0));
                        }
                    }
                } else if (msg == WM_KEYUP || msg == WM_SYSKEYUP) && IS_RECORDING_VOICE.load(Ordering::SeqCst) {
                    // Stop recording if the primary key OR any of its active modifiers is released!
                    if vk_code == binding.vk || binding.is_modifier_released(vk_code) {
                        if IS_RECORDING_VOICE.swap(false, Ordering::SeqCst) {
                            let _ = PostMessageW(HWND(hwnd_val as *mut _), WM_APP_STOP_RECORDING, WPARAM(0), LPARAM(0));
                        }
                    }
                }
            } else {
                // Toggle Mode:
                // Keydown with matching modifiers toggles recording state on/off
                if (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) && vk_code == binding.vk {
                    if are_modifiers_active(&binding) {
                        let was_recording = IS_RECORDING_VOICE.fetch_xor(true, Ordering::SeqCst);
                        if was_recording {
                            let _ = PostMessageW(HWND(hwnd_val as *mut _), WM_APP_STOP_RECORDING, WPARAM(0), LPARAM(0));
                        } else {
                            let _ = PostMessageW(HWND(hwnd_val as *mut _), WM_APP_START_RECORDING, WPARAM(0), LPARAM(0));
                        }
                    }
                }
            }
        }

        // 2. Overlay Dismiss Logic
        if IS_OVERLAY_VISIBLE.load(Ordering::SeqCst) {
            let mode = DISMISS_MODE.load(Ordering::SeqCst);

            // Escape (0x1B) always closes the overlay immediately
            if (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) && vk_code == 0x1B {
                DISMISS_REQUESTED.store(true, Ordering::SeqCst);
            }

            // Mode 0 (ReleaseKey) & Mode 4 (Hybrid):
            // Dismiss if any key other than the trigger key is released
            if (mode == 0 || mode == 4) && (msg == WM_KEYUP || msg == WM_SYSKEYUP) {
                if vk_code != binding.vk && !binding.is_modifier_released(vk_code) {
                    DISMISS_REQUESTED.store(true, Ordering::SeqCst);
                }
            }

            // Mode 3 (AnyKey):
            if mode == 3 && (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) && vk_code != binding.vk {
                DISMISS_REQUESTED.store(true, Ordering::SeqCst);
            }
        }
    }

    // Always forward to game/system so game inputs are never consumed or dropped
    CallNextHookEx(HHOOK(std::ptr::null_mut()), n_code, w_param, l_param)
}
