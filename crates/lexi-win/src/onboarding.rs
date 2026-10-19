#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::ptr::null_mut;
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::Arc;
use std::sync::Mutex;
#[cfg(windows)]
use std::thread;

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    CreateFontW, CreateSolidBrush, SetBkColor, SetBkMode, SetTextColor,
    DEFAULT_CHARSET, HDC, TRANSPARENT,
};

#[cfg(windows)]
const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}
#[cfg(windows)]
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, ICC_BAR_CLASSES, ICC_PROGRESS_CLASS, ICC_STANDARD_CLASSES,
    INITCOMMONCONTROLSEX,
};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, GetWindowLongPtrW, IsWindow, PostQuitMessage, RegisterClassW,
    SendMessageW, SetForegroundWindow, SetWindowLongPtrW, SetWindowTextW, ShowWindow,
    BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX, BS_AUTORADIOBUTTON, BS_DEFPUSHBUTTON,
    BS_GROUPBOX, BS_PUSHBUTTON, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, CBS_DROPDOWNLIST,
    GWLP_USERDATA, HMENU, MSG, SM_CXSCREEN, SM_CYSCREEN, SW_RESTORE, SW_SHOW,
    WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLORDLG, WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX,
    WM_CTLCOLORSTATIC, WM_DESTROY,
    WM_SETFONT, WNDCLASSW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_MINIMIZEBOX,
    WS_OVERLAPPED, WS_SYSMENU, WS_VISIBLE,
};

use lexi_core::config::AppConfig;
#[cfg(windows)]
use lexi_core::stt_models::{download_stt_model, list_audio_input_devices, ModelCatalog};

#[allow(dead_code)]
static ONBOARDING_HWND: Mutex<Option<usize>> = Mutex::new(None);

const PBM_SETRANGE32: u32 = 1030;
const PBM_SETPOS: u32 = 1026;

const ID_BTN_DOWNLOAD: usize = 3001;
const ID_BTN_FINISH: usize = 3002;
const ID_RADIO_PTT: usize = 3003;
const ID_RADIO_TOGGLE: usize = 3004;

#[cfg(windows)]
#[allow(dead_code)]
struct OnboardingControls {
    h_combo_model: usize,
    h_btn_download: usize,
    h_progress: usize,
    h_lbl_status: usize,
    h_combo_device: usize,
    h_radio_ptt: usize,
    h_radio_toggle: usize,
    h_combo_hotkey: usize,
    h_chk_ai: usize,
    h_combo_ai_provider: usize,
    h_edit_ai_key: usize,
    download_canceller: Arc<AtomicBool>,
}

#[cfg(windows)]
fn to_wide_str(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

pub fn open_onboarding_window_if_needed(config: &AppConfig) {
    if config.onboarding.completed {
        return;
    }

    #[cfg(windows)]
    {
        if let Ok(mut lock) = ONBOARDING_HWND.lock() {
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
            run_onboarding_window();
        });
    }

    #[cfg(not(windows))]
    {
        log::info!("Onboarding wizard requested (non-Windows platform mock)");
    }
}

#[cfg(windows)]
fn run_onboarding_window() {
    unsafe {
        let icce = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_BAR_CLASSES | ICC_STANDARD_CLASSES | ICC_PROGRESS_CLASS,
        };
        let _ = InitCommonControlsEx(&icce);

        let dpi = {
            let d = windows::Win32::UI::HiDpi::GetDpiForSystem();
            if d != 0 { d } else { 96 }
        };
        let scale = (dpi as f32 / 96.0).max(1.0);
        let s = |v: i32| -> i32 { (v as f32 * scale).round() as i32 };

        let win_w = s(660);
        let win_h = s(700);
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let pos_x = (screen_w - win_w) / 2;
        let pos_y = (screen_h - win_h) / 2;

        let title = to_wide_str("Bem-vindo ao Lexi! - Configuração Inicial");
        let class_name = to_wide_str("LexiOnboardingDialogClass");
        let purple_bg_brush = CreateSolidBrush(rgb(23, 16, 35));
        let wnd_class = WNDCLASSW {
            lpfnWndProc: Some(onboarding_wnd_proc),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hbrBackground: purple_bg_brush,
            ..Default::default()
        };

        let _ = RegisterClassW(&wnd_class);

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
                log::error!("Failed to create onboarding window: {e}");
                return;
            }
        };

        if let Ok(mut lock) = ONBOARDING_HWND.lock() {
            *lock = Some(hwnd.0 as usize);
        }

        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(null_mut()), 0, 0).as_bool() {
            let _ = windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if let Ok(mut lock) = ONBOARDING_HWND.lock() {
            *lock = None;
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn onboarding_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        windows::Win32::UI::WindowsAndMessaging::WM_CREATE => {
            let dpi = windows::Win32::UI::HiDpi::GetDpiForSystem();
            let scale = if dpi != 0 { (dpi as f32 / 96.0).max(1.0) } else { 1.0 };
            let s = |v: i32| -> i32 { (v as f32 * scale).round() as i32 };

            let font_name = to_wide_str("Segoe UI");
            let font_title = -((18.0 * scale).round() as i32).min(-16);
            let font_bold = -((13.0 * scale).round() as i32).min(-12);
            let font_h = -((13.0 * scale).round() as i32).min(-12);

            let hfont_title = CreateFontW(
                font_title, 0, 0, 0, 700, 0, 0, 0,
                DEFAULT_CHARSET.0 as u32, 0, 0, 0, 0,
                PCWSTR(font_name.as_ptr()),
            );
            let hfont_bold = CreateFontW(
                font_bold, 0, 0, 0, 700, 0, 0, 0,
                DEFAULT_CHARSET.0 as u32, 0, 0, 0, 0,
                PCWSTR(font_name.as_ptr()),
            );
            let hfont = CreateFontW(
                font_h, 0, 0, 0, 400, 0, 0, 0,
                DEFAULT_CHARSET.0 as u32, 0, 0, 0, 0,
                PCWSTR(font_name.as_ptr()),
            );

            let static_class = to_wide_str("STATIC");
            let button_class = to_wide_str("BUTTON");
            let combo_class = to_wide_str("COMBOBOX");
            let edit_class = to_wide_str("EDIT");
            let progress_class = to_wide_str("msctls_progress32");

            let make_ctrl = |class: &[u16], text: &str, style: u32, x: i32, y: i32, w: i32, h: i32, id: usize| -> HWND {
                let t_wide = to_wide_str(text);
                let h_ctl = CreateWindowExW(
                    Default::default(),
                    PCWSTR(class.as_ptr()),
                    PCWSTR(t_wide.as_ptr()),
                    windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(style),
                    s(x), s(y), s(w), s(h),
                    hwnd,
                    HMENU(id as *mut _),
                    None,
                    None,
                ).unwrap_or(HWND(null_mut()));
                SendMessageW(h_ctl, WM_SETFONT, WPARAM(hfont.0 as usize), LPARAM(1));
                h_ctl
            };

            // Header Banner
            let h_title = make_ctrl(&static_class, "Bem-vindo ao Lexi! 🎮", WS_CHILD.0 | WS_VISIBLE.0, 20, 15, 580, 26, 0);
            SendMessageW(h_title, WM_SETFONT, WPARAM(hfont_title.0 as usize), LPARAM(1));

            make_ctrl(&static_class, "Seu tradutor e interpretador gamer por voz autossuficiente para jogos em tela cheia.", WS_CHILD.0 | WS_VISIBLE.0, 20, 42, 580, 20, 0);

            // ==========================================
            // ETAPA 1: Modelo de Voz (STT)
            // ==========================================
            let g1 = make_ctrl(&button_class, " 1. Modelo de Reconhecimento de Voz (Speech-to-Text) ", WS_CHILD.0 | WS_VISIBLE.0 | BS_GROUPBOX as u32, 18, 70, 584, 165, 0);
            SendMessageW(g1, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));

            make_ctrl(&static_class, "Para transcrever sua voz durante a partida, selecione um modelo de voz:", WS_CHILD.0 | WS_VISIBLE.0, 32, 94, 550, 18, 0);

            let h_combo_model = make_ctrl(&combo_class, "", WS_CHILD.0 | WS_VISIBLE.0 | CBS_DROPDOWNLIST as u32, 32, 116, 400, 180, 101);
            let catalog = ModelCatalog::all_models();
            let mut recommended_idx = 0;
            for (i, m) in catalog.iter().enumerate() {
                let status_str = if m.is_installed { "✓ Já baixado" } else { "Não baixado" };
                let label = format!("{} ({} MB) - [{}]", m.name, m.size_mb, status_str);
                let l_wide = to_wide_str(&label);
                SendMessageW(h_combo_model, CB_ADDSTRING, WPARAM(0), LPARAM(l_wide.as_ptr() as isize));
                if m.recommended && recommended_idx == 0 {
                    recommended_idx = i;
                }
            }
            SendMessageW(h_combo_model, CB_SETCURSEL, WPARAM(recommended_idx), LPARAM(0));

            let h_btn_download = make_ctrl(&button_class, "Baixar Modelo", WS_CHILD.0 | WS_VISIBLE.0 | BS_PUSHBUTTON as u32, 442, 114, 145, 28, ID_BTN_DOWNLOAD);
            SendMessageW(h_btn_download, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));

            // Native Progress Bar
            let h_progress = make_ctrl(&progress_class, "", WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0, 32, 155, 555, 18, 102);
            SendMessageW(h_progress, PBM_SETRANGE32, WPARAM(0), LPARAM(100));
            SendMessageW(h_progress, PBM_SETPOS, WPARAM(0), LPARAM(0));

            let h_lbl_status = make_ctrl(&static_class, "Status: Clique em 'Baixar Modelo' ou use o modelo já detectado.", WS_CHILD.0 | WS_VISIBLE.0, 32, 178, 555, 45, 103);

            // ==========================================
            // ETAPA 2: Microfone & Tecla de Gatilho
            // ==========================================
            let g2 = make_ctrl(&button_class, " 2. Microfone & Atalho de Ativação no Jogo ", WS_CHILD.0 | WS_VISIBLE.0 | BS_GROUPBOX as u32, 18, 245, 584, 155, 0);
            SendMessageW(g2, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));

            make_ctrl(&static_class, "Microfone de Entrada:", WS_CHILD.0 | WS_VISIBLE.0, 32, 270, 160, 18, 0);
            let h_combo_device = make_ctrl(&combo_class, "", WS_CHILD.0 | WS_VISIBLE.0 | CBS_DROPDOWNLIST as u32, 200, 267, 385, 160, 104);
            let devices = list_audio_input_devices();
            for dev in &devices {
                let d_wide = to_wide_str(dev);
                SendMessageW(h_combo_device, CB_ADDSTRING, WPARAM(0), LPARAM(d_wide.as_ptr() as isize));
            }
            SendMessageW(h_combo_device, CB_SETCURSEL, WPARAM(0), LPARAM(0));

            make_ctrl(&static_class, "Modo de Ativação:", WS_CHILD.0 | WS_VISIBLE.0, 32, 305, 160, 18, 0);
            let h_radio_ptt = make_ctrl(&button_class, "Push-to-Talk (Segure a tecla para falar, solte para traduzir)", WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTORADIOBUTTON as u32, 200, 302, 385, 20, ID_RADIO_PTT);
            SendMessageW(h_radio_ptt, BM_SETCHECK, WPARAM(1), LPARAM(0));

            let h_radio_toggle = make_ctrl(&button_class, "Alternar (Pressione uma vez para gravar, outra para enviar)", WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTORADIOBUTTON as u32, 200, 324, 385, 20, ID_RADIO_TOGGLE);

            make_ctrl(&static_class, "Tecla de Gatilho:", WS_CHILD.0 | WS_VISIBLE.0, 32, 355, 160, 18, 0);
            let h_combo_hotkey = make_ctrl(&combo_class, "", WS_CHILD.0 | WS_VISIBLE.0 | CBS_DROPDOWNLIST as u32, 200, 352, 200, 160, 105);
            let hotkeys = ["Ctrl+Space (Recomendado)", "F9", "CapsLock (Fácil alcance)", "F13", "Alt+Space", "F8", "F10", "ScrollLock"];
            for hk in &hotkeys {
                let h_wide = to_wide_str(hk);
                SendMessageW(h_combo_hotkey, CB_ADDSTRING, WPARAM(0), LPARAM(h_wide.as_ptr() as isize));
            }
            SendMessageW(h_combo_hotkey, CB_SETCURSEL, WPARAM(0), LPARAM(0));

            // ==========================================
            // ETAPA 3: Inteligência Artificial (Opcional)
            // ==========================================
            let g3 = make_ctrl(&button_class, " 3. Conectar Inteligência Artificial Externa (Opcional) ", WS_CHILD.0 | WS_VISIBLE.0 | BS_GROUPBOX as u32, 18, 410, 584, 150, 0);
            SendMessageW(g3, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));

            let h_chk_ai = make_ctrl(&button_class, "Ativar refinamento de voz e contexto gamer avançado por IA", WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTOCHECKBOX as u32, 32, 432, 550, 22, 106);
            SendMessageW(h_chk_ai, BM_SETCHECK, WPARAM(1), LPARAM(0));

            make_ctrl(&static_class, "Provedor de IA:", WS_CHILD.0 | WS_VISIBLE.0, 32, 465, 130, 18, 0);
            let h_combo_ai_provider = make_ctrl(&combo_class, "", WS_CHILD.0 | WS_VISIBLE.0 | CBS_DROPDOWNLIST as u32, 170, 462, 220, 150, 107);
            let providers = ["Google Gemini (Recomendado)", "Anthropic Claude", "OpenAI (GPT-4o)", "OpenCode / Local AI", "Personalizado (API)"];
            for p in &providers {
                let p_wide = to_wide_str(p);
                SendMessageW(h_combo_ai_provider, CB_ADDSTRING, WPARAM(0), LPARAM(p_wide.as_ptr() as isize));
            }
            SendMessageW(h_combo_ai_provider, CB_SETCURSEL, WPARAM(0), LPARAM(0));

            make_ctrl(&static_class, "Chave de API:", WS_CHILD.0 | WS_VISIBLE.0, 32, 498, 130, 18, 0);
            let h_edit_ai_key = make_ctrl(&edit_class, "", WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0 | 0x0080, 170, 495, 415, 24, 108);

            make_ctrl(&static_class, "* Sem chave de IA, o Lexi usa o dicionário local embutido de 300.000+ termos instantaneamente.", WS_CHILD.0 | WS_VISIBLE.0, 32, 530, 555, 18, 0);

            // ==========================================
            // FOOTER BUTTON
            // ==========================================
            let h_btn_finish = make_ctrl(&button_class, "Concluir e Começar a Jogar! 🚀", WS_CHILD.0 | WS_VISIBLE.0 | BS_DEFPUSHBUTTON as u32, 18, 580, 584, 42, ID_BTN_FINISH);
            SendMessageW(h_btn_finish, WM_SETFONT, WPARAM(hfont_title.0 as usize), LPARAM(1));

            let state = Box::new(OnboardingControls {
                h_combo_model: h_combo_model.0 as usize,
                h_btn_download: h_btn_download.0 as usize,
                h_progress: h_progress.0 as usize,
                h_lbl_status: h_lbl_status.0 as usize,
                h_combo_device: h_combo_device.0 as usize,
                h_radio_ptt: h_radio_ptt.0 as usize,
                h_radio_toggle: h_radio_toggle.0 as usize,
                h_combo_hotkey: h_combo_hotkey.0 as usize,
                h_chk_ai: h_chk_ai.0 as usize,
                h_combo_ai_provider: h_combo_ai_provider.0 as usize,
                h_edit_ai_key: h_edit_ai_key.0 as usize,
                download_canceller: Arc::new(AtomicBool::new(false)),
            });

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
            LRESULT(0)
        }

        WM_COMMAND => {
            let cmd_id = (wparam.0 & 0xFFFF) as usize;
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);

            if ptr != 0 {
                let state = &*(ptr as *const OnboardingControls);

                match cmd_id {
                    ID_BTN_DOWNLOAD => {
                        let sel_idx = SendMessageW(HWND(state.h_combo_model as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        let catalog = ModelCatalog::all_models();
                        if let Some(target_model) = catalog.get(sel_idx) {
                            let model_id = target_model.id.clone();
                            let h_progress_val = state.h_progress;
                            let h_lbl_val = state.h_lbl_status;
                            let h_btn_val = state.h_btn_download;
                            let canceller = Arc::clone(&state.download_canceller);
                            canceller.store(false, Ordering::Relaxed);

                            let _ = EnableWindow(HWND(h_btn_val as *mut _), false);
                            let initial_txt = to_wide_str(&format!("Baixando {}... Por favor aguarde.", target_model.name));
                            let _ = SetWindowTextW(HWND(h_lbl_val as *mut _), PCWSTR(initial_txt.as_ptr()));

                            thread::spawn(move || {
                                let res = download_stt_model(&model_id, canceller, move |upd| {
                                    unsafe {
                                        SendMessageW(HWND(h_progress_val as *mut _), PBM_SETPOS, WPARAM(upd.percent as usize), LPARAM(0));
                                        let status_msg = format!("Baixando: {:.1}% ({:.1} MB/s)", upd.percent, upd.speed_mb_s);
                                        let wide = to_wide_str(&status_msg);
                                        let _ = SetWindowTextW(HWND(h_lbl_val as *mut _), PCWSTR(wide.as_ptr()));
                                    }
                                });

                                unsafe {
                                    let _ = EnableWindow(HWND(h_btn_val as *mut _), true);
                                    match res {
                                        Ok(path) => {
                                            SendMessageW(HWND(h_progress_val as *mut _), PBM_SETPOS, WPARAM(100), LPARAM(0));
                                            let ok_msg = format!("✓ Download concluído com sucesso!\nSalvo em: {:?}", path.file_name().unwrap_or_default());
                                            let wide = to_wide_str(&ok_msg);
                                            let _ = SetWindowTextW(HWND(h_lbl_val as *mut _), PCWSTR(wide.as_ptr()));
                                        }
                                        Err(err) => {
                                            let err_msg = format!("Falha no download: {err}");
                                            let wide = to_wide_str(&err_msg);
                                            let _ = SetWindowTextW(HWND(h_lbl_val as *mut _), PCWSTR(wide.as_ptr()));
                                        }
                                    }
                                }
                            });
                        }
                    }

                    ID_BTN_FINISH => {
                        let mut config = crate::app::get_current_config();

                        // 1. Model
                        let sel_idx = SendMessageW(HWND(state.h_combo_model as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        let catalog = ModelCatalog::all_models();
                        if let Some(m) = catalog.get(sel_idx) {
                            config.stt.selected_model = m.id.clone();
                        }

                        // 2. Activation mode
                        let is_ptt = SendMessageW(HWND(state.h_radio_ptt as *mut _), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1;
                        config.stt.activation_mode = if is_ptt { "push_to_talk".to_string() } else { "toggle".to_string() };

                        // 3. Hotkey
                        let hk_idx = SendMessageW(HWND(state.h_combo_hotkey as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        let hotkey_vals = ["Ctrl+Space", "F9", "CapsLock", "F13", "Alt+Space", "F8", "F10", "ScrollLock"];
                        if let Some(&hk) = hotkey_vals.get(hk_idx) {
                            config.stt.hotkey = hk.to_string();
                        }

                        // 4. AI config
                        let ai_checked = SendMessageW(HWND(state.h_chk_ai as *mut _), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1;
                        config.llm.enabled = ai_checked;

                        let ai_idx = SendMessageW(HWND(state.h_combo_ai_provider as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        let provider_slugs = ["gemini", "claude", "openai", "opencode", "custom"];
                        if let Some(&p) = provider_slugs.get(ai_idx) {
                            config.llm.provider = p.to_string();
                        }

                        let len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextLengthW(HWND(state.h_edit_ai_key as *mut _));
                        let mut buf = vec![0u16; (len + 1) as usize];
                        windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(HWND(state.h_edit_ai_key as *mut _), &mut buf);
                        let key_str = String::from_utf16_lossy(&buf[..len as usize]).trim().to_string();
                        if !key_str.is_empty() {
                            config.llm.api_key = key_str;
                        }

                        // Mark onboarding complete
                        config.onboarding.completed = true;

                        let _ = crate::save_config_everywhere(&config);
                        crate::app::reload_global_config(config);

                        let _ = DestroyWindow(hwnd);
                    }

                    _ => {}
                }
            }

            LRESULT(0)
        }

        WM_CTLCOLORDLG => {
            let hbrush = CreateSolidBrush(rgb(23, 16, 35));
            LRESULT(hbrush.0 as isize)
        }

        WM_CTLCOLORSTATIC => {
            let hdc = HDC(wparam.0 as *mut _);
            SetBkMode(hdc, TRANSPARENT);
            SetTextColor(hdc, rgb(235, 225, 252)); // Lavender RGB(235, 225, 252)
            let hbrush = CreateSolidBrush(rgb(23, 16, 35));
            LRESULT(hbrush.0 as isize)
        }

        WM_CTLCOLOREDIT => {
            let hdc = HDC(wparam.0 as *mut _);
            SetBkColor(hdc, rgb(38, 26, 60)); // Dark Purple
            SetTextColor(hdc, rgb(255, 255, 255)); // White
            let hbrush = CreateSolidBrush(rgb(38, 26, 60));
            LRESULT(hbrush.0 as isize)
        }

        WM_CTLCOLORLISTBOX => {
            let hdc = HDC(wparam.0 as *mut _);
            SetBkColor(hdc, rgb(38, 26, 60));
            SetTextColor(hdc, rgb(255, 255, 255));
            let hbrush = CreateSolidBrush(rgb(38, 26, 60));
            LRESULT(hbrush.0 as isize)
        }

        WM_CTLCOLORBTN => {
            let hbrush = CreateSolidBrush(rgb(23, 16, 35));
            LRESULT(hbrush.0 as isize)
        }

        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let state = Box::from_raw(ptr as *mut OnboardingControls);
                state.download_canceller.store(true, Ordering::Relaxed);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
