#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
#[cfg(windows)]
use std::ptr::null_mut;
use std::sync::Mutex;
use std::thread;

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    CreateFontW, GetSysColorBrush, SetBkMode,
    COLOR_BTNFACE, DEFAULT_CHARSET, HDC, TRANSPARENT,
};
#[cfg(windows)]
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, ICC_BAR_CLASSES, ICC_STANDARD_CLASSES, INITCOMMONCONTROLSEX,
    TBM_SETPOS, TBM_SETRANGE,
};

#[cfg(windows)]
const TBM_GETPOS: u32 = 1024;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
    IsWindow, PostQuitMessage, RegisterClassW, SendMessageW, SetForegroundWindow,
    SetWindowLongPtrW, SetWindowTextW, ShowWindow,
    BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON, BS_GROUPBOX, BS_PUSHBUTTON,
    CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, CBS_DROPDOWNLIST, ES_AUTOHSCROLL,
    GWLP_USERDATA, HMENU, MSG, SM_CXSCREEN, SM_CYSCREEN, SW_RESTORE, SW_SHOW,
    WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLORDLG, WM_CTLCOLORSTATIC, WM_DESTROY,
    WM_HSCROLL, WM_SETFONT, WNDCLASSW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_MINIMIZEBOX,
    WS_OVERLAPPED, WS_SYSMENU, WS_VISIBLE, WS_VSCROLL,
};

use lexi_core::config::{AppConfig, DismissMode, OverlayPosition};

#[allow(dead_code)]
static SETTINGS_HWND: Mutex<Option<usize>> = Mutex::new(None);

#[allow(dead_code)]
const ID_BTN_SAVE: usize = 2001;
#[allow(dead_code)]
const ID_BTN_CANCEL: usize = 2002;
#[allow(dead_code)]
const ID_BTN_OPEN_FILE: usize = 2003;

#[allow(dead_code)]
const POSITION_CHOICES: &[(&str, OverlayPosition)] = &[
    ("Topo Central (top_center)", OverlayPosition::TopCenter),
    ("Centro da Tela (center)", OverlayPosition::Center),
    ("Inferior Central (bottom_center)", OverlayPosition::BottomCenter),
    ("Topo Esquerda (top_left)", OverlayPosition::TopLeft),
    ("Topo Direita (top_right)", OverlayPosition::TopRight),
    ("Meio Esquerda (middle_left)", OverlayPosition::MiddleLeft),
    ("Meio Direita (middle_right)", OverlayPosition::MiddleRight),
];

#[allow(dead_code)]
const DISMISS_CHOICES: &[(&str, DismissMode)] = &[
    ("Híbrido (Tempo limite + Soltar tecla)", DismissMode::Hybrid),
    ("Soltar Tecla de Trigger (Release Key)", DismissMode::ReleaseKey),
    ("Tempo Limite Apenas (Timeout)", DismissMode::Timeout),
    ("Atalho de Fechar (Hotkey - Esc)", DismissMode::Hotkey),
    ("Qualquer Tecla (Any Key)", DismissMode::AnyKey),
];

#[allow(dead_code)]
#[derive(Default)]
struct SettingsControls {
    h_combo_pos: usize,
    h_track_font: usize,
    h_lbl_font_val: usize,
    h_track_opacity: usize,
    h_lbl_opacity_val: usize,
    h_chk_tts: usize,
    h_track_volume: usize,
    h_lbl_volume_val: usize,
    h_combo_dismiss: usize,
    h_edit_timeout: usize,
    h_chk_llm: usize,
    h_edit_api_key: usize,
    initial_config: AppConfig,
}

pub fn open_settings_window() {
    #[cfg(windows)]
    {
        if let Ok(mut lock) = SETTINGS_HWND.lock() {
            if let Some(hwnd_val) = *lock {
                unsafe {
                    let hwnd = HWND(hwnd_val as *mut _);
                    if IsWindow(hwnd).as_bool() {
                        let _ = ShowWindow(hwnd, SW_RESTORE);
                        let _ = SetForegroundWindow(hwnd);
                        return;
                    }
                }
                *lock = None;
            }
        }

        thread::spawn(|| {
            run_settings_window();
        });
    }

    #[cfg(not(windows))]
    {
        log::info!("Settings window requested (non-Windows platform mock)");
    }
}

#[cfg(windows)]
fn to_wide_str(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn run_settings_window() {
    unsafe {
        let icce = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_BAR_CLASSES | ICC_STANDARD_CLASSES,
        };
        let _ = InitCommonControlsEx(&icce);

        let class_name = to_wide_str("LexiSettingsDialogClass");
        let wnd_class = WNDCLASSW {
            lpfnWndProc: Some(settings_wnd_proc),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
            ..Default::default()
        };

        let _ = RegisterClassW(&wnd_class);

        let win_w = 540;
        let win_h = 635;
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let pos_x = (screen_w - win_w) / 2;
        let pos_y = (screen_h - win_h) / 2;

        let title = to_wide_str("Configurações - Lexi (Tradutor Gamer)");
        let hwnd = match CreateWindowExW(
            Default::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            pos_x,
            pos_y,
            win_w,
            win_h,
            HWND(null_mut()),
            HMENU(null_mut()),
            None,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                log::error!("Failed to create settings window: {e}");
                return;
            }
        };

        if let Ok(mut lock) = SETTINGS_HWND.lock() {
            *lock = Some(hwnd.0 as usize);
        }

        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(null_mut()), 0, 0).as_bool() {
            let _ = windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if let Ok(mut lock) = SETTINGS_HWND.lock() {
            *lock = None;
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        windows::Win32::UI::WindowsAndMessaging::WM_CREATE => {
            let config = crate::app::get_current_config();

            // Font for modern clean Segoe UI look
            let font_name = to_wide_str("Segoe UI");
            let hfont = CreateFontW(
                -13, 0, 0, 0, 400, 0, 0, 0,
                DEFAULT_CHARSET.0 as u32, 0, 0, 0, 0,
                PCWSTR(font_name.as_ptr()),
            );
            let hfont_bold = CreateFontW(
                -13, 0, 0, 0, 700, 0, 0, 0,
                DEFAULT_CHARSET.0 as u32, 0, 0, 0, 0,
                PCWSTR(font_name.as_ptr()),
            );

            let static_class = to_wide_str("STATIC");
            let button_class = to_wide_str("BUTTON");
            let combo_class = to_wide_str("COMBOBOX");
            let edit_class = to_wide_str("EDIT");
            let track_class = to_wide_str("msctls_trackbar32");

            let make_ctrl = |class: &[u16], text: &str, style: u32, x: i32, y: i32, w: i32, h: i32, id: usize| -> HWND {
                let t_wide = to_wide_str(text);
                let h_ctl = CreateWindowExW(
                    Default::default(),
                    PCWSTR(class.as_ptr()),
                    PCWSTR(t_wide.as_ptr()),
                    windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(style),
                    x, y, w, h,
                    hwnd,
                    HMENU(id as *mut _),
                    None,
                    None,
                ).unwrap_or(HWND(null_mut()));
                SendMessageW(h_ctl, WM_SETFONT, WPARAM(hfont.0 as usize), LPARAM(1));
                h_ctl
            };

            // ==========================================
            // GROUP 1: Interface & Overlay
            // ==========================================
            let g1 = make_ctrl(&button_class, " Interface & Overlay ", WS_CHILD.0 | WS_VISIBLE.0 | BS_GROUPBOX as u32, 15, 12, 495, 142, 0);
            SendMessageW(g1, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));

            // Position
            make_ctrl(&static_class, "Posição na Tela:", WS_CHILD.0 | WS_VISIBLE.0, 30, 36, 130, 20, 0);
            let h_combo_pos = make_ctrl(&combo_class, "", WS_CHILD.0 | WS_VISIBLE.0 | WS_VSCROLL.0 | CBS_DROPDOWNLIST as u32, 165, 33, 330, 220, 100);
            let mut selected_pos_idx = 0;
            for (idx, (label, pos)) in POSITION_CHOICES.iter().enumerate() {
                let l_wide = to_wide_str(label);
                SendMessageW(h_combo_pos, CB_ADDSTRING, WPARAM(0), LPARAM(l_wide.as_ptr() as isize));
                if *pos == config.overlay.position {
                    selected_pos_idx = idx;
                }
            }
            SendMessageW(h_combo_pos, CB_SETCURSEL, WPARAM(selected_pos_idx), LPARAM(0));

            // Font size
            make_ctrl(&static_class, "Tamanho da Fonte:", WS_CHILD.0 | WS_VISIBLE.0, 30, 72, 130, 20, 0);
            let h_track_font = make_ctrl(&track_class, "", WS_CHILD.0 | WS_VISIBLE.0 | 0x0001, 165, 68, 260, 28, 101);
            let h_lbl_font_val = make_ctrl(&static_class, &format!("{} px", config.overlay.font_size), WS_CHILD.0 | WS_VISIBLE.0, 435, 72, 60, 20, 0);
            // Range: 12 to 36
            let range_font = ((12u32 & 0xFFFF) | ((36u32 & 0xFFFF) << 16)) as isize;
            SendMessageW(h_track_font, TBM_SETRANGE, WPARAM(1), LPARAM(range_font));
            SendMessageW(h_track_font, TBM_SETPOS, WPARAM(1), LPARAM(config.overlay.font_size as isize));

            // Opacity
            make_ctrl(&static_class, "Opacidade do Card:", WS_CHILD.0 | WS_VISIBLE.0, 30, 110, 130, 20, 0);
            let h_track_opacity = make_ctrl(&track_class, "", WS_CHILD.0 | WS_VISIBLE.0 | 0x0001, 165, 106, 260, 28, 102);
            let op_pct = ((config.overlay.opacity * 100.0).round() as i32).clamp(50, 100);
            let h_lbl_opacity_val = make_ctrl(&static_class, &format!("{} %", op_pct), WS_CHILD.0 | WS_VISIBLE.0, 435, 110, 60, 20, 0);
            // Range: 50 to 100
            let range_op = ((50u32 & 0xFFFF) | ((100u32 & 0xFFFF) << 16)) as isize;
            SendMessageW(h_track_opacity, TBM_SETRANGE, WPARAM(1), LPARAM(range_op));
            SendMessageW(h_track_opacity, TBM_SETPOS, WPARAM(1), LPARAM(op_pct as isize));

            // ==========================================
            // GROUP 2: Áudio & Leitura (TTS)
            // ==========================================
            let g2 = make_ctrl(&button_class, " Áudio & Leitura por Voz (TTS) ", WS_CHILD.0 | WS_VISIBLE.0 | BS_GROUPBOX as u32, 15, 164, 495, 105, 0);
            SendMessageW(g2, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));

            let h_chk_tts = make_ctrl(&button_class, "Ativar leitura por voz dos termos traduzidos", WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTOCHECKBOX as u32, 30, 190, 460, 24, 110);
            SendMessageW(h_chk_tts, BM_SETCHECK, WPARAM(if config.audio.enabled { 1 } else { 0 }), LPARAM(0));

            make_ctrl(&static_class, "Volume da Voz:", WS_CHILD.0 | WS_VISIBLE.0, 30, 226, 130, 20, 0);
            let h_track_volume = make_ctrl(&track_class, "", WS_CHILD.0 | WS_VISIBLE.0 | 0x0001, 165, 222, 260, 28, 111);
            let vol_val = config.audio.volume.min(100);
            let h_lbl_volume_val = make_ctrl(&static_class, &format!("{} %", vol_val), WS_CHILD.0 | WS_VISIBLE.0, 435, 226, 60, 20, 0);
            let range_vol = ((0u32 & 0xFFFF) | ((100u32 & 0xFFFF) << 16)) as isize;
            SendMessageW(h_track_volume, TBM_SETRANGE, WPARAM(1), LPARAM(range_vol));
            SendMessageW(h_track_volume, TBM_SETPOS, WPARAM(1), LPARAM(vol_val as isize));

            // ==========================================
            // GROUP 3: Fechamento do Card (Dismiss)
            // ==========================================
            let g3 = make_ctrl(&button_class, " Fechamento do Card (Dismiss) ", WS_CHILD.0 | WS_VISIBLE.0 | BS_GROUPBOX as u32, 15, 280, 495, 105, 0);
            SendMessageW(g3, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));

            make_ctrl(&static_class, "Modo de Fechamento:", WS_CHILD.0 | WS_VISIBLE.0, 30, 304, 130, 20, 0);
            let h_combo_dismiss = make_ctrl(&combo_class, "", WS_CHILD.0 | WS_VISIBLE.0 | WS_VSCROLL.0 | CBS_DROPDOWNLIST as u32, 165, 301, 330, 200, 120);
            let mut selected_dismiss_idx = 0;
            for (idx, (label, mode)) in DISMISS_CHOICES.iter().enumerate() {
                let l_wide = to_wide_str(label);
                SendMessageW(h_combo_dismiss, CB_ADDSTRING, WPARAM(0), LPARAM(l_wide.as_ptr() as isize));
                if *mode == config.dismiss.mode {
                    selected_dismiss_idx = idx;
                }
            }
            SendMessageW(h_combo_dismiss, CB_SETCURSEL, WPARAM(selected_dismiss_idx), LPARAM(0));

            make_ctrl(&static_class, "Tempo Limite (s):", WS_CHILD.0 | WS_VISIBLE.0, 30, 344, 130, 20, 0);
            let h_edit_timeout = make_ctrl(&edit_class, &format!("{:.1}", config.dismiss.timeout_seconds), WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32, 165, 341, 70, 24, 121);
            make_ctrl(&static_class, "segundos para fechar automaticamente", WS_CHILD.0 | WS_VISIBLE.0, 245, 344, 250, 20, 0);

            // ==========================================
            // GROUP 4: Inteligência Artificial (LLM)
            // ==========================================
            let g4 = make_ctrl(&button_class, " Inteligência Artificial & LLM ", WS_CHILD.0 | WS_VISIBLE.0 | BS_GROUPBOX as u32, 15, 396, 495, 110, 0);
            SendMessageW(g4, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));

            let h_chk_llm = make_ctrl(&button_class, "Ativar tradução contextual por IA (LLM)", WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTOCHECKBOX as u32, 30, 422, 460, 24, 130);
            SendMessageW(h_chk_llm, BM_SETCHECK, WPARAM(if config.llm.enabled { 1 } else { 0 }), LPARAM(0));

            make_ctrl(&static_class, "Chave de API (LLM):", WS_CHILD.0 | WS_VISIBLE.0, 30, 460, 130, 20, 0);
            let h_edit_api_key = make_ctrl(&edit_class, &config.llm.api_key, WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32, 165, 457, 330, 24, 131);

            // ==========================================
            // BOTTOM BUTTONS
            // ==========================================
            let h_btn_save = make_ctrl(&button_class, "Salvar e Aplicar", WS_CHILD.0 | WS_VISIBLE.0 | BS_DEFPUSHBUTTON as u32, 15, 524, 145, 34, ID_BTN_SAVE);
            SendMessageW(h_btn_save, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));

            make_ctrl(&button_class, "Cancelar", WS_CHILD.0 | WS_VISIBLE.0 | BS_PUSHBUTTON as u32, 170, 524, 100, 34, ID_BTN_CANCEL);
            make_ctrl(&button_class, "Abrir config.toml", WS_CHILD.0 | WS_VISIBLE.0 | BS_PUSHBUTTON as u32, 280, 524, 230, 34, ID_BTN_OPEN_FILE);

            let state = Box::new(SettingsControls {
                h_combo_pos: h_combo_pos.0 as usize,
                h_track_font: h_track_font.0 as usize,
                h_lbl_font_val: h_lbl_font_val.0 as usize,
                h_track_opacity: h_track_opacity.0 as usize,
                h_lbl_opacity_val: h_lbl_opacity_val.0 as usize,
                h_chk_tts: h_chk_tts.0 as usize,
                h_track_volume: h_track_volume.0 as usize,
                h_lbl_volume_val: h_lbl_volume_val.0 as usize,
                h_combo_dismiss: h_combo_dismiss.0 as usize,
                h_edit_timeout: h_edit_timeout.0 as usize,
                h_chk_llm: h_chk_llm.0 as usize,
                h_edit_api_key: h_edit_api_key.0 as usize,
                initial_config: config,
            });

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
            LRESULT(0)
        }

        WM_HSCROLL => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let state = &*(ptr as *const SettingsControls);
                let ctl_hwnd_val = lparam.0 as usize;

                if ctl_hwnd_val == state.h_track_font {
                    let pos = SendMessageW(HWND(ctl_hwnd_val as *mut _), TBM_GETPOS, WPARAM(0), LPARAM(0)).0;
                    let txt = to_wide_str(&format!("{} px", pos));
                    let _ = SetWindowTextW(HWND(state.h_lbl_font_val as *mut _), PCWSTR(txt.as_ptr()));
                } else if ctl_hwnd_val == state.h_track_opacity {
                    let pos = SendMessageW(HWND(ctl_hwnd_val as *mut _), TBM_GETPOS, WPARAM(0), LPARAM(0)).0;
                    let txt = to_wide_str(&format!("{} %", pos));
                    let _ = SetWindowTextW(HWND(state.h_lbl_opacity_val as *mut _), PCWSTR(txt.as_ptr()));
                } else if ctl_hwnd_val == state.h_track_volume {
                    let pos = SendMessageW(HWND(ctl_hwnd_val as *mut _), TBM_GETPOS, WPARAM(0), LPARAM(0)).0;
                    let txt = to_wide_str(&format!("{} %", pos));
                    let _ = SetWindowTextW(HWND(state.h_lbl_volume_val as *mut _), PCWSTR(txt.as_ptr()));
                }
            }
            LRESULT(0)
        }

        WM_COMMAND => {
            let cmd_id = (wparam.0 & 0xFFFF) as usize;
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);

            match cmd_id {
                ID_BTN_SAVE => {
                    if ptr != 0 {
                        let state = &*(ptr as *const SettingsControls);

                        // 1. Position
                        let pos_idx = SendMessageW(HWND(state.h_combo_pos as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        let selected_pos = if pos_idx < POSITION_CHOICES.len() {
                            POSITION_CHOICES[pos_idx].1.clone()
                        } else {
                            OverlayPosition::TopCenter
                        };

                        // 2. Font Size
                        let font_size = SendMessageW(HWND(state.h_track_font as *mut _), TBM_GETPOS, WPARAM(0), LPARAM(0)).0 as i32;

                        // 3. Opacity
                        let op_pct = SendMessageW(HWND(state.h_track_opacity as *mut _), TBM_GETPOS, WPARAM(0), LPARAM(0)).0 as f32;
                        let opacity = (op_pct / 100.0).clamp(0.5, 1.0);

                        // 4. TTS
                        let tts_checked = SendMessageW(HWND(state.h_chk_tts as *mut _), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1;
                        let volume = SendMessageW(HWND(state.h_track_volume as *mut _), TBM_GETPOS, WPARAM(0), LPARAM(0)).0 as u32;

                        // 5. Dismiss
                        let dis_idx = SendMessageW(HWND(state.h_combo_dismiss as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        let selected_dismiss = if dis_idx < DISMISS_CHOICES.len() {
                            DISMISS_CHOICES[dis_idx].1.clone()
                        } else {
                            DismissMode::Hybrid
                        };

                        // Timeout
                        let timeout_sec = {
                            let len = GetWindowTextLengthW(HWND(state.h_edit_timeout as *mut _));
                            let mut buf = vec![0u16; (len + 1) as usize];
                            GetWindowTextW(HWND(state.h_edit_timeout as *mut _), &mut buf);
                            let s = String::from_utf16_lossy(&buf[..len as usize]);
                            s.trim().parse::<f32>().unwrap_or(state.initial_config.dismiss.timeout_seconds)
                        };

                        // 6. LLM
                        let llm_checked = SendMessageW(HWND(state.h_chk_llm as *mut _), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1;
                        let api_key = {
                            let len = GetWindowTextLengthW(HWND(state.h_edit_api_key as *mut _));
                            let mut buf = vec![0u16; (len + 1) as usize];
                            GetWindowTextW(HWND(state.h_edit_api_key as *mut _), &mut buf);
                            String::from_utf16_lossy(&buf[..len as usize]).trim().to_string()
                        };

                        // Construct updated configuration
                        let mut new_config = state.initial_config.clone();
                        new_config.overlay.position = selected_pos;
                        new_config.overlay.font_size = font_size;
                        new_config.overlay.opacity = opacity;
                        new_config.audio.enabled = tts_checked;
                        new_config.audio.volume = volume;
                        new_config.dismiss.mode = selected_dismiss;
                        new_config.dismiss.timeout_seconds = timeout_sec;
                        new_config.llm.enabled = llm_checked;
                        new_config.llm.api_key = api_key;

                        // Save configuration to both local AppData and root/exe paths
                        if let Err(e) = crate::save_config_everywhere(&new_config) {
                            log::warn!("Failed saving config to disk: {e}");
                        }

                        // Immediately trigger live reload in AppState
                        crate::app::reload_global_config(new_config);

                        let _ = DestroyWindow(hwnd);
                    }
                }
                ID_BTN_CANCEL => {
                    let _ = DestroyWindow(hwnd);
                }
                ID_BTN_OPEN_FILE => {
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
                _ => {}
            }
            LRESULT(0)
        }

        WM_CTLCOLORSTATIC | WM_CTLCOLORDLG | WM_CTLCOLORBTN => {
            let hdc = HDC(wparam.0 as *mut _);
            SetBkMode(hdc, TRANSPARENT);
            LRESULT(GetSysColorBrush(COLOR_BTNFACE).0 as isize)
        }

        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let _ = Box::from_raw(ptr as *mut SettingsControls);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
