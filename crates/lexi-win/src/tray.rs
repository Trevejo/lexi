#[cfg(windows)]
use std::mem::size_of;
#[cfg(windows)]
use std::ptr::null;
#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{HWND, POINT};
#[cfg(windows)]
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW, SetForegroundWindow,
    TrackPopupMenu, AppendMenuW, IDI_APPLICATION, MF_STRING, MF_SEPARATOR,
    TPM_RIGHTBUTTON, TPM_BOTTOMALIGN,
};

pub const WM_TRAY_ICON: u32 = 0x8001; // WM_APP + 1
pub const IDM_TRAY_TEST: usize = 1001;
pub const IDM_TRAY_CONFIG: usize = 1002;
pub const IDM_TRAY_RELOAD: usize = 1003;
pub const IDM_TRAY_LOG: usize = 1004;
pub const IDM_TRAY_EXIT: usize = 1005;
pub const IDM_TRAY_EDIT_FILE: usize = 1006;

pub struct TrayIcon {
    #[cfg(windows)]
    nid: NOTIFYICONDATAW,
}

impl TrayIcon {
    pub fn new(_hwnd: isize) -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(windows)]
        unsafe {
            let h_wnd = HWND(_hwnd as *mut _);
            let h_icon = LoadIconW(None, IDI_APPLICATION)?;

            let mut tip: [u16; 128] = [0; 128];
            let tip_str = "Lexi - Tradutor Gamer";
            for (i, c) in tip_str.encode_utf16().enumerate() {
                if i < 127 {
                    tip[i] = c;
                }
            }

            let mut nid = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: h_wnd,
                uID: 1,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                uCallbackMessage: WM_TRAY_ICON,
                hIcon: h_icon,
                szTip: tip,
                ..Default::default()
            };

            let _ = Shell_NotifyIconW(NIM_ADD, &mut nid);
            Ok(Self { nid })
        }

        #[cfg(not(windows))]
        {
            Ok(Self {})
        }
    }
}

#[cfg(windows)]
pub fn show_tray_context_menu(hwnd: HWND) {
    unsafe {
        let h_menu = match CreatePopupMenu() {
            Ok(m) => m,
            Err(_) => return,
        };

        let title = to_wide_str("Lexi: Tradutor Gamer (Ativo)");
        let settings = to_wide_str("Configurações...");
        let test = to_wide_str("Testar Overlay ('flank')");
        let reload = to_wide_str("Recarregar Configurações");
        let edit_file = to_wide_str("Abrir config.toml no Bloco de Notas");
        let log_item = to_wide_str("Ver Log de Execução (lexi.log)");
        let exit = to_wide_str("Sair");

        let _ = AppendMenuW(h_menu, MF_STRING, 0, PCWSTR(title.as_ptr()));
        let _ = AppendMenuW(h_menu, MF_SEPARATOR, 0, PCWSTR(null()));
        let _ = AppendMenuW(h_menu, MF_STRING, IDM_TRAY_CONFIG, PCWSTR(settings.as_ptr()));
        let _ = AppendMenuW(h_menu, MF_STRING, IDM_TRAY_TEST, PCWSTR(test.as_ptr()));
        let _ = AppendMenuW(h_menu, MF_STRING, IDM_TRAY_RELOAD, PCWSTR(reload.as_ptr()));
        let _ = AppendMenuW(h_menu, MF_SEPARATOR, 0, PCWSTR(null()));
        let _ = AppendMenuW(h_menu, MF_STRING, IDM_TRAY_EDIT_FILE, PCWSTR(edit_file.as_ptr()));
        let _ = AppendMenuW(h_menu, MF_STRING, IDM_TRAY_LOG, PCWSTR(log_item.as_ptr()));
        let _ = AppendMenuW(h_menu, MF_SEPARATOR, 0, PCWSTR(null()));
        let _ = AppendMenuW(h_menu, MF_STRING, IDM_TRAY_EXIT, PCWSTR(exit.as_ptr()));

        let mut pt = POINT { x: 0, y: 0 };
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(h_menu, TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, pt.x, pt.y, 0, hwnd, None);
        let _ = DestroyMenu(h_menu);
    }
}

#[cfg(not(windows))]
pub fn show_tray_context_menu(_hwnd: isize) {}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &mut self.nid);
        }
    }
}

#[cfg(windows)]
fn to_wide_str(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
