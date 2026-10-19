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
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    CreateFontW, GetSysColorBrush, SetBkMode,
    COLOR_BTNFACE, DEFAULT_CHARSET, HDC, TRANSPARENT,
};
#[cfg(windows)]
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, ICC_BAR_CLASSES, ICC_PROGRESS_CLASS, ICC_STANDARD_CLASSES,
    ICC_TAB_CLASSES, INITCOMMONCONTROLSEX, TBM_SETPOS, TBM_SETRANGE,
};

#[cfg(windows)]
const TBM_GETPOS: u32 = 1024;
const PBM_SETRANGE32: u32 = 1030;
const PBM_SETPOS: u32 = 1026;

const TCM_FIRST: u32 = 0x1300;
const TCM_INSERTITEMW: u32 = TCM_FIRST + 62;
const TCM_GETCURSEL: u32 = TCM_FIRST + 11;
const TCN_FIRST: u32 = 0xFFFFFD00;
const TCN_SELCHANGE: u32 = TCN_FIRST - 551;

#[cfg(windows)]
use windows::Win32::UI::Controls::NMHDR;
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;

#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetMessageW, GetSystemMetrics, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
    IsWindow, PostQuitMessage, RegisterClassW, SendMessageW, SetForegroundWindow,
    SetWindowLongPtrW, SetWindowTextW, ShowWindow,
    BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX, BS_AUTORADIOBUTTON, BS_DEFPUSHBUTTON,
    BS_GROUPBOX, BS_PUSHBUTTON, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, CBS_DROPDOWNLIST,
    ES_AUTOHSCROLL, GWLP_USERDATA, HMENU, MSG, SM_CXSCREEN, SM_CYSCREEN,
    SW_HIDE, SW_RESTORE, SW_SHOW, WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLORDLG,
    WM_CTLCOLORSTATIC, WM_DESTROY, WM_HSCROLL, WM_NOTIFY, WM_SETFONT, WNDCLASSW,
    WS_BORDER, WS_CAPTION, WS_CHILD, WS_CLIPSIBLINGS, WS_MINIMIZEBOX, WS_OVERLAPPED,
    WS_SYSMENU, WS_VISIBLE, WS_VSCROLL,
};

use lexi_core::config::{AppConfig, DismissMode, OverlayPosition};
#[cfg(windows)]
use lexi_core::llm::LlmClient;
#[cfg(windows)]
use lexi_core::stt_models::{download_stt_model, list_audio_input_devices, ModelCatalog};

#[allow(dead_code)]
static SETTINGS_HWND: Mutex<Option<usize>> = Mutex::new(None);

const ID_TAB_CONTROL: usize = 1000;
const ID_BTN_SAVE: usize = 2001;
const ID_BTN_CANCEL: usize = 2002;
const ID_BTN_OPEN_FILE: usize = 2003;
const ID_BTN_DOWNLOAD_STT: usize = 2004;
const ID_BTN_TEST_AI: usize = 2005;
const ID_BTN_DEMO_OVERLAY: usize = 2006;
const ID_COMBO_AI_PROVIDER: usize = 2007;

const POSITION_CHOICES: &[(&str, OverlayPosition)] = &[
    ("Topo Central (top_center)", OverlayPosition::TopCenter),
    ("Centro da Tela (center)", OverlayPosition::Center),
    ("Inferior Central (bottom_center)", OverlayPosition::BottomCenter),
    ("Topo Esquerda (top_left)", OverlayPosition::TopLeft),
    ("Topo Direita (top_right)", OverlayPosition::TopRight),
    ("Meio Esquerda (middle_left)", OverlayPosition::MiddleLeft),
    ("Meio Direita (middle_right)", OverlayPosition::MiddleRight),
];

const DISMISS_CHOICES: &[(&str, DismissMode)] = &[
    ("Híbrido (Tempo limite + Soltar tecla)", DismissMode::Hybrid),
    ("Soltar Tecla de Trigger (Release Key)", DismissMode::ReleaseKey),
    ("Tempo Limite Apenas (Timeout)", DismissMode::Timeout),
    ("Atalho de Fechar (Hotkey - Esc)", DismissMode::Hotkey),
    ("Qualquer Tecla (Any Key)", DismissMode::AnyKey),
];

const SCALE_CHOICES: &[(&str, f32)] = &[
    ("100% (Padrão 1080p)", 1.0),
    ("125% (Médio)", 1.25),
    ("150% (Recomendado para 4K)", 1.5),
    ("175% (Grande)", 1.75),
    ("200% (Extra Grande)", 2.0),
];

#[cfg(windows)]
#[repr(C)]
struct TCITEMW {
    mask: u32,
    dw_state: u32,
    dw_state_mask: u32,
    psz_text: *mut u16,
    cch_text_max: i32,
    i_image: i32,
    l_param: LPARAM,
}

#[cfg(windows)]
#[allow(dead_code)]
struct SettingsControls {
    h_tab: usize,
    // Tab groups (for visibility toggle)
    tab_controls: Vec<Vec<usize>>,

    // Tab 0: STT & Audio
    h_combo_stt_model: usize,
    h_btn_stt_download: usize,
    h_progress_stt: usize,
    h_lbl_stt_status: usize,
    h_combo_mic: usize,
    h_radio_ptt: usize,
    h_radio_toggle: usize,
    h_combo_hotkey: usize,

    // Tab 1: AI
    h_chk_ai: usize,
    h_combo_ai_provider: usize,
    h_edit_ai_key: usize,
    h_edit_ai_model: usize,
    h_edit_ai_url: usize,
    h_btn_test_ai: usize,
    h_lbl_ai_test_result: usize,

    // Tab 2: Overlay
    h_combo_pos: usize,
    h_combo_scale: usize,
    h_track_font: usize,
    h_lbl_font_val: usize,
    h_track_opacity: usize,
    h_lbl_opacity_val: usize,
    h_combo_dismiss: usize,
    h_edit_timeout: usize,
    h_btn_demo: usize,

    // Tab 3: TTS
    h_chk_tts: usize,
    h_track_volume: usize,
    h_lbl_volume_val: usize,

    stt_canceller: Arc<AtomicBool>,
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
            dwICC: ICC_BAR_CLASSES | ICC_STANDARD_CLASSES | ICC_PROGRESS_CLASS | ICC_TAB_CLASSES,
        };
        let _ = InitCommonControlsEx(&icce);

        let class_name = to_wide_str("LexiModernSettingsDialogClass");
        let wnd_class = WNDCLASSW {
            lpfnWndProc: Some(settings_wnd_proc),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
            ..Default::default()
        };

        let _ = RegisterClassW(&wnd_class);

        let win_w = 640;
        let win_h = 670;
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let pos_x = (screen_w - win_w) / 2;
        let pos_y = (screen_h - win_h) / 2;

        let title = to_wide_str("Configurações - Lexi (Tradutor & Interpretador Gamer)");
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
            let progress_class = to_wide_str("msctls_progress32");
            let tab_class = to_wide_str("SysTabControl32");

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

            // Main Tab Control
            let h_tab = make_ctrl(&tab_class, "", WS_CHILD.0 | WS_VISIBLE.0 | WS_CLIPSIBLINGS.0, 15, 12, 595, 535, ID_TAB_CONTROL);
            SendMessageW(h_tab, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));

            // Insert 4 tabs
            let tab_names = [
                "🎙️  Voz & Modelos STT",
                "🤖  Inteligência Artificial",
                "🖥️  Overlay no Jogo",
                "🔊  Leitura (TTS)",
            ];
            for (idx, name) in tab_names.iter().enumerate() {
                let mut wide_title = to_wide_str(name);
                let mut item = TCITEMW {
                    mask: 1, // TCIF_TEXT
                    dw_state: 0,
                    dw_state_mask: 0,
                    psz_text: wide_title.as_mut_ptr(),
                    cch_text_max: wide_title.len() as i32,
                    i_image: -1,
                    l_param: LPARAM(0),
                };
                SendMessageW(h_tab, TCM_INSERTITEMW, WPARAM(idx), LPARAM(&mut item as *mut _ as isize));
            }

            // Vectors to track which controls belong to which tab
            let mut tab0_ctrls = Vec::new();
            let mut tab1_ctrls = Vec::new();
            let mut tab2_ctrls = Vec::new();
            let mut tab3_ctrls = Vec::new();

            // ==========================================
            // TAB 0: Voz & Modelos STT
            // ==========================================
            let g0_model = make_ctrl(&button_class, " Modelos de Reconhecimento de Fala (Speech-to-Text) ", WS_CHILD.0 | WS_VISIBLE.0 | BS_GROUPBOX as u32, 28, 50, 568, 195, 0);
            SendMessageW(g0_model, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));
            tab0_ctrls.push(g0_model.0 as usize);

            let l0_info = make_ctrl(&static_class, "Baixe e selecione o modelo de voz que transcreve sua fala nos jogos:", WS_CHILD.0 | WS_VISIBLE.0, 42, 75, 540, 18, 0);
            tab0_ctrls.push(l0_info.0 as usize);

            let h_combo_stt_model = make_ctrl(&combo_class, "", WS_CHILD.0 | WS_VISIBLE.0 | WS_VSCROLL.0 | CBS_DROPDOWNLIST as u32, 42, 98, 410, 200, 301);
            tab0_ctrls.push(h_combo_stt_model.0 as usize);

            let catalog = ModelCatalog::all_models();
            let mut active_model_idx = 0;
            for (i, m) in catalog.iter().enumerate() {
                let status_str = if m.is_installed { "✓ Baixado" } else { "Não baixado" };
                let label = format!("{} ({} MB) - [{}]", m.name, m.size_mb, status_str);
                let l_wide = to_wide_str(&label);
                SendMessageW(h_combo_stt_model, CB_ADDSTRING, WPARAM(0), LPARAM(l_wide.as_ptr() as isize));
                if m.id == config.stt.selected_model || m.filename == config.stt.selected_model {
                    active_model_idx = i;
                }
            }
            SendMessageW(h_combo_stt_model, CB_SETCURSEL, WPARAM(active_model_idx), LPARAM(0));

            let h_btn_stt_download = make_ctrl(&button_class, "Baixar Modelo", WS_CHILD.0 | WS_VISIBLE.0 | BS_PUSHBUTTON as u32, 460, 96, 122, 28, ID_BTN_DOWNLOAD_STT);
            SendMessageW(h_btn_stt_download, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));
            tab0_ctrls.push(h_btn_stt_download.0 as usize);

            let h_progress_stt = make_ctrl(&progress_class, "", WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0, 42, 138, 540, 18, 302);
            SendMessageW(h_progress_stt, PBM_SETRANGE32, WPARAM(0), LPARAM(100));
            tab0_ctrls.push(h_progress_stt.0 as usize);

            let h_lbl_stt_status = make_ctrl(&static_class, "Status: Modelos com '✓ Baixado' funcionam de forma 100% autônoma e local.", WS_CHILD.0 | WS_VISIBLE.0, 42, 162, 540, 40, 303);
            tab0_ctrls.push(h_lbl_stt_status.0 as usize);

            let g0_audio = make_ctrl(&button_class, " Microfone & Atalho de Ativação ", WS_CHILD.0 | WS_VISIBLE.0 | BS_GROUPBOX as u32, 28, 255, 568, 240, 0);
            SendMessageW(g0_audio, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));
            tab0_ctrls.push(g0_audio.0 as usize);

            let l0_mic = make_ctrl(&static_class, "Microfone de Entrada:", WS_CHILD.0 | WS_VISIBLE.0, 42, 285, 150, 18, 0);
            tab0_ctrls.push(l0_mic.0 as usize);
            let h_combo_mic = make_ctrl(&combo_class, "", WS_CHILD.0 | WS_VISIBLE.0 | WS_VSCROLL.0 | CBS_DROPDOWNLIST as u32, 200, 282, 380, 180, 304);
            tab0_ctrls.push(h_combo_mic.0 as usize);
            let devices = list_audio_input_devices();
            let mut selected_mic_idx = 0;
            for (idx, dev) in devices.iter().enumerate() {
                let d_wide = to_wide_str(dev);
                SendMessageW(h_combo_mic, CB_ADDSTRING, WPARAM(0), LPARAM(d_wide.as_ptr() as isize));
                if let Some(ref saved) = config.stt.input_device {
                    if saved == dev {
                        selected_mic_idx = idx;
                    }
                }
            }
            SendMessageW(h_combo_mic, CB_SETCURSEL, WPARAM(selected_mic_idx), LPARAM(0));

            let l0_act = make_ctrl(&static_class, "Modo de Ativação:", WS_CHILD.0 | WS_VISIBLE.0, 42, 325, 150, 18, 0);
            tab0_ctrls.push(l0_act.0 as usize);
            let h_radio_ptt = make_ctrl(&button_class, "Push-to-Talk (Segurar tecla para falar, soltar para traduzir)", WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTORADIOBUTTON as u32, 200, 322, 380, 20, 305);
            tab0_ctrls.push(h_radio_ptt.0 as usize);
            let h_radio_toggle = make_ctrl(&button_class, "Alternar / Toggle (Pressione uma vez para falar, outra para enviar)", WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTORADIOBUTTON as u32, 200, 346, 380, 20, 306);
            tab0_ctrls.push(h_radio_toggle.0 as usize);
            if config.stt.activation_mode == "toggle" {
                SendMessageW(h_radio_toggle, BM_SETCHECK, WPARAM(1), LPARAM(0));
            } else {
                SendMessageW(h_radio_ptt, BM_SETCHECK, WPARAM(1), LPARAM(0));
            }

            let l0_key = make_ctrl(&static_class, "Tecla de Gatilho:", WS_CHILD.0 | WS_VISIBLE.0, 42, 385, 150, 18, 0);
            tab0_ctrls.push(l0_key.0 as usize);
            let h_combo_hotkey = make_ctrl(&combo_class, "", WS_CHILD.0 | WS_VISIBLE.0 | WS_VSCROLL.0 | CBS_DROPDOWNLIST as u32, 200, 382, 220, 180, 307);
            tab0_ctrls.push(h_combo_hotkey.0 as usize);
            let hotkey_choices = ["F9", "CapsLock", "F13", "Ctrl+Space", "F8", "F10", "ScrollLock"];
            let mut selected_hk_idx = 0;
            for (idx, hk) in hotkey_choices.iter().enumerate() {
                let h_wide = to_wide_str(hk);
                SendMessageW(h_combo_hotkey, CB_ADDSTRING, WPARAM(0), LPARAM(h_wide.as_ptr() as isize));
                if *hk == config.stt.hotkey {
                    selected_hk_idx = idx;
                }
            }
            SendMessageW(h_combo_hotkey, CB_SETCURSEL, WPARAM(selected_hk_idx), LPARAM(0));

            let l0_tip = make_ctrl(&static_class, "Dica: F9 e CapsLock são perfeitos para jogos competitivos (sem conflitar com WASD).", WS_CHILD.0 | WS_VISIBLE.0, 42, 425, 538, 20, 0);
            tab0_ctrls.push(l0_tip.0 as usize);

            // ==========================================
            // TAB 1: Inteligência Artificial (IA)
            // ==========================================
            let g1_ai = make_ctrl(&button_class, " Configuração de Inteligência Artificial Externa ", WS_CHILD.0 | BS_GROUPBOX as u32, 28, 50, 568, 445, 0);
            SendMessageW(g1_ai, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));
            tab1_ctrls.push(g1_ai.0 as usize);

            let h_chk_ai = make_ctrl(&button_class, "Ativar tratamento de voz e contexto gamer avançado por IA", WS_CHILD.0 | BS_AUTOCHECKBOX as u32, 42, 78, 540, 22, 401);
            SendMessageW(h_chk_ai, BM_SETCHECK, WPARAM(if config.llm.enabled { 1 } else { 0 }), LPARAM(0));
            tab1_ctrls.push(h_chk_ai.0 as usize);

            let l1_prov = make_ctrl(&static_class, "Provedor de IA:", WS_CHILD.0, 42, 116, 150, 18, 0);
            tab1_ctrls.push(l1_prov.0 as usize);
            let h_combo_ai_provider = make_ctrl(&combo_class, "", WS_CHILD.0 | WS_VSCROLL.0 | CBS_DROPDOWNLIST as u32, 200, 113, 260, 180, ID_COMBO_AI_PROVIDER);
            tab1_ctrls.push(h_combo_ai_provider.0 as usize);

            let ai_providers = [
                ("Google Gemini (Recomendado - Ultra Rápido)", "gemini"),
                ("Anthropic Claude (Haiku / Sonnet)", "claude"),
                ("OpenAI (GPT-4o-mini / GPT-4o)", "openai"),
                ("OpenCode / Local AI (127.0.0.1:8317)", "opencode"),
                ("Personalizado (Qualquer API REST)", "custom"),
            ];
            let mut selected_prov_idx = 0;
            for (idx, (label, slug)) in ai_providers.iter().enumerate() {
                let l_wide = to_wide_str(label);
                SendMessageW(h_combo_ai_provider, CB_ADDSTRING, WPARAM(0), LPARAM(l_wide.as_ptr() as isize));
                if *slug == config.llm.provider.to_lowercase() {
                    selected_prov_idx = idx;
                }
            }
            SendMessageW(h_combo_ai_provider, CB_SETCURSEL, WPARAM(selected_prov_idx), LPARAM(0));

            let l1_key = make_ctrl(&static_class, "Chave de API (Key):", WS_CHILD.0, 42, 155, 150, 18, 0);
            tab1_ctrls.push(l1_key.0 as usize);
            let h_edit_ai_key = make_ctrl(&edit_class, &config.llm.api_key, WS_CHILD.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32, 200, 152, 380, 24, 402);
            tab1_ctrls.push(h_edit_ai_key.0 as usize);

            let l1_model = make_ctrl(&static_class, "Nome do Modelo:", WS_CHILD.0, 42, 192, 150, 18, 0);
            tab1_ctrls.push(l1_model.0 as usize);
            let h_edit_ai_model = make_ctrl(&edit_class, &config.llm.model, WS_CHILD.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32, 200, 189, 380, 24, 403);
            tab1_ctrls.push(h_edit_ai_model.0 as usize);

            let l1_url = make_ctrl(&static_class, "URL Base do Endpoint:", WS_CHILD.0, 42, 230, 150, 18, 0);
            tab1_ctrls.push(l1_url.0 as usize);
            let h_edit_ai_url = make_ctrl(&edit_class, &config.llm.base_url, WS_CHILD.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32, 200, 227, 380, 24, 404);
            tab1_ctrls.push(h_edit_ai_url.0 as usize);

            let h_btn_test_ai = make_ctrl(&button_class, "Testar Conexão com a IA", WS_CHILD.0 | BS_PUSHBUTTON as u32, 200, 265, 180, 32, ID_BTN_TEST_AI);
            SendMessageW(h_btn_test_ai, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));
            tab1_ctrls.push(h_btn_test_ai.0 as usize);

            let h_lbl_ai_test_result = make_ctrl(&static_class, "Clique no botão acima para validar sua chave de API e medir a latência.", WS_CHILD.0, 42, 310, 538, 45, 405);
            tab1_ctrls.push(h_lbl_ai_test_result.0 as usize);

            let l1_box_info = make_ctrl(&static_class, "Garantia de Funcionamento:\nMesmo sem conexão com a internet ou sem chave de IA cadastrada, o Lexi continua traduzindo instantaneamente em <5ms usando o banco de dados embutido com 300.000+ termos.", WS_CHILD.0, 42, 380, 538, 55, 0);
            tab1_ctrls.push(l1_box_info.0 as usize);

            // ==========================================
            // TAB 2: Overlay no Jogo
            // ==========================================
            let g2_ov = make_ctrl(&button_class, " Posicionamento & Visual do Overlay ", WS_CHILD.0 | BS_GROUPBOX as u32, 28, 50, 568, 445, 0);
            SendMessageW(g2_ov, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));
            tab2_ctrls.push(g2_ov.0 as usize);

            let l2_pos = make_ctrl(&static_class, "Posição na Tela:", WS_CHILD.0, 42, 80, 150, 18, 0);
            tab2_ctrls.push(l2_pos.0 as usize);
            let h_combo_pos = make_ctrl(&combo_class, "", WS_CHILD.0 | WS_VSCROLL.0 | CBS_DROPDOWNLIST as u32, 200, 77, 380, 220, 501);
            tab2_ctrls.push(h_combo_pos.0 as usize);
            let mut selected_pos_idx = 0;
            for (idx, (label, pos)) in POSITION_CHOICES.iter().enumerate() {
                let l_wide = to_wide_str(label);
                SendMessageW(h_combo_pos, CB_ADDSTRING, WPARAM(0), LPARAM(l_wide.as_ptr() as isize));
                if *pos == config.overlay.position {
                    selected_pos_idx = idx;
                }
            }
            SendMessageW(h_combo_pos, CB_SETCURSEL, WPARAM(selected_pos_idx), LPARAM(0));

            let l2_scale = make_ctrl(&static_class, "Escala / Zoom (4K):", WS_CHILD.0, 42, 118, 150, 18, 0);
            tab2_ctrls.push(l2_scale.0 as usize);
            let h_combo_scale = make_ctrl(&combo_class, "", WS_CHILD.0 | WS_VSCROLL.0 | CBS_DROPDOWNLIST as u32, 200, 115, 380, 180, 502);
            tab2_ctrls.push(h_combo_scale.0 as usize);
            let mut selected_scale_idx = 2; // Default 150%
            for (idx, (label, s_val)) in SCALE_CHOICES.iter().enumerate() {
                let l_wide = to_wide_str(label);
                SendMessageW(h_combo_scale, CB_ADDSTRING, WPARAM(0), LPARAM(l_wide.as_ptr() as isize));
                if (*s_val - config.overlay.scale).abs() < 0.01 {
                    selected_scale_idx = idx;
                }
            }
            SendMessageW(h_combo_scale, CB_SETCURSEL, WPARAM(selected_scale_idx), LPARAM(0));

            let l2_font = make_ctrl(&static_class, "Tamanho da Fonte:", WS_CHILD.0, 42, 156, 150, 18, 0);
            tab2_ctrls.push(l2_font.0 as usize);
            let h_track_font = make_ctrl(&track_class, "", WS_CHILD.0 | 0x0001, 200, 152, 280, 28, 503);
            tab2_ctrls.push(h_track_font.0 as usize);
            let h_lbl_font_val = make_ctrl(&static_class, &format!("{} px", config.overlay.font_size), WS_CHILD.0, 495, 156, 80, 18, 0);
            tab2_ctrls.push(h_lbl_font_val.0 as usize);
            let range_font = ((12u32 & 0xFFFF) | ((36u32 & 0xFFFF) << 16)) as isize;
            SendMessageW(h_track_font, TBM_SETRANGE, WPARAM(1), LPARAM(range_font));
            SendMessageW(h_track_font, TBM_SETPOS, WPARAM(1), LPARAM(config.overlay.font_size as isize));

            let l2_op = make_ctrl(&static_class, "Opacidade do Card:", WS_CHILD.0, 42, 196, 150, 18, 0);
            tab2_ctrls.push(l2_op.0 as usize);
            let h_track_opacity = make_ctrl(&track_class, "", WS_CHILD.0 | 0x0001, 200, 192, 280, 28, 504);
            tab2_ctrls.push(h_track_opacity.0 as usize);
            let op_pct = ((config.overlay.opacity * 100.0).round() as i32).clamp(50, 100);
            let h_lbl_opacity_val = make_ctrl(&static_class, &format!("{} %", op_pct), WS_CHILD.0, 495, 196, 80, 18, 0);
            tab2_ctrls.push(h_lbl_opacity_val.0 as usize);
            let range_op = ((50u32 & 0xFFFF) | ((100u32 & 0xFFFF) << 16)) as isize;
            SendMessageW(h_track_opacity, TBM_SETRANGE, WPARAM(1), LPARAM(range_op));
            SendMessageW(h_track_opacity, TBM_SETPOS, WPARAM(1), LPARAM(op_pct as isize));

            let l2_dis = make_ctrl(&static_class, "Modo de Fechamento:", WS_CHILD.0, 42, 238, 150, 18, 0);
            tab2_ctrls.push(l2_dis.0 as usize);
            let h_combo_dismiss = make_ctrl(&combo_class, "", WS_CHILD.0 | WS_VSCROLL.0 | CBS_DROPDOWNLIST as u32, 200, 235, 380, 180, 505);
            tab2_ctrls.push(h_combo_dismiss.0 as usize);
            let mut selected_dismiss_idx = 0;
            for (idx, (label, mode)) in DISMISS_CHOICES.iter().enumerate() {
                let l_wide = to_wide_str(label);
                SendMessageW(h_combo_dismiss, CB_ADDSTRING, WPARAM(0), LPARAM(l_wide.as_ptr() as isize));
                if *mode == config.dismiss.mode {
                    selected_dismiss_idx = idx;
                }
            }
            SendMessageW(h_combo_dismiss, CB_SETCURSEL, WPARAM(selected_dismiss_idx), LPARAM(0));

            let l2_time = make_ctrl(&static_class, "Tempo Limite (s):", WS_CHILD.0, 42, 276, 150, 18, 0);
            tab2_ctrls.push(l2_time.0 as usize);
            let h_edit_timeout = make_ctrl(&edit_class, &format!("{:.1}", config.dismiss.timeout_seconds), WS_CHILD.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32, 200, 273, 70, 24, 506);
            tab2_ctrls.push(h_edit_timeout.0 as usize);
            let l2_sec = make_ctrl(&static_class, "segundos para fechar automaticamente", WS_CHILD.0, 280, 276, 300, 18, 0);
            tab2_ctrls.push(l2_sec.0 as usize);

            let h_btn_demo = make_ctrl(&button_class, "Testar Exibição do Overlay Agora 🎮", WS_CHILD.0 | BS_PUSHBUTTON as u32, 200, 320, 280, 34, ID_BTN_DEMO_OVERLAY);
            SendMessageW(h_btn_demo, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));
            tab2_ctrls.push(h_btn_demo.0 as usize);

            // ==========================================
            // TAB 3: Leitura por Voz (TTS)
            // ==========================================
            let g3_tts = make_ctrl(&button_class, " Leitura e Pronúncia por Voz (TTS) ", WS_CHILD.0 | BS_GROUPBOX as u32, 28, 50, 568, 220, 0);
            SendMessageW(g3_tts, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));
            tab3_ctrls.push(g3_tts.0 as usize);

            let h_chk_tts = make_ctrl(&button_class, "Ativar pronúncia em inglês do termo traduzido", WS_CHILD.0 | BS_AUTOCHECKBOX as u32, 42, 80, 540, 22, 601);
            SendMessageW(h_chk_tts, BM_SETCHECK, WPARAM(if config.audio.enabled { 1 } else { 0 }), LPARAM(0));
            tab3_ctrls.push(h_chk_tts.0 as usize);

            let l3_vol = make_ctrl(&static_class, "Volume da Voz:", WS_CHILD.0, 42, 125, 150, 18, 0);
            tab3_ctrls.push(l3_vol.0 as usize);
            let h_track_volume = make_ctrl(&track_class, "", WS_CHILD.0 | 0x0001, 200, 121, 280, 28, 602);
            tab3_ctrls.push(h_track_volume.0 as usize);
            let vol_val = config.audio.volume.min(100);
            let h_lbl_volume_val = make_ctrl(&static_class, &format!("{} %", vol_val), WS_CHILD.0, 495, 125, 80, 18, 0);
            tab3_ctrls.push(h_lbl_volume_val.0 as usize);
            let range_vol = ((0u32 & 0xFFFF) | ((100u32 & 0xFFFF) << 16)) as isize;
            SendMessageW(h_track_volume, TBM_SETRANGE, WPARAM(1), LPARAM(range_vol));
            SendMessageW(h_track_volume, TBM_SETPOS, WPARAM(1), LPARAM(vol_val as isize));

            // ==========================================
            // BOTTOM CONTROLS
            // ==========================================
            let h_btn_save = make_ctrl(&button_class, "Salvar e Aplicar", WS_CHILD.0 | WS_VISIBLE.0 | BS_DEFPUSHBUTTON as u32, 15, 565, 160, 38, ID_BTN_SAVE);
            SendMessageW(h_btn_save, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));

            make_ctrl(&button_class, "Cancelar", WS_CHILD.0 | WS_VISIBLE.0 | BS_PUSHBUTTON as u32, 185, 565, 110, 38, ID_BTN_CANCEL);
            make_ctrl(&button_class, "Abrir config.toml", WS_CHILD.0 | WS_VISIBLE.0 | BS_PUSHBUTTON as u32, 305, 565, 170, 38, ID_BTN_OPEN_FILE);

            let state = Box::new(SettingsControls {
                h_tab: h_tab.0 as usize,
                tab_controls: vec![tab0_ctrls, tab1_ctrls, tab2_ctrls, tab3_ctrls],
                h_combo_stt_model: h_combo_stt_model.0 as usize,
                h_btn_stt_download: h_btn_stt_download.0 as usize,
                h_progress_stt: h_progress_stt.0 as usize,
                h_lbl_stt_status: h_lbl_stt_status.0 as usize,
                h_combo_mic: h_combo_mic.0 as usize,
                h_radio_ptt: h_radio_ptt.0 as usize,
                h_radio_toggle: h_radio_toggle.0 as usize,
                h_combo_hotkey: h_combo_hotkey.0 as usize,
                h_chk_ai: h_chk_ai.0 as usize,
                h_combo_ai_provider: h_combo_ai_provider.0 as usize,
                h_edit_ai_key: h_edit_ai_key.0 as usize,
                h_edit_ai_model: h_edit_ai_model.0 as usize,
                h_edit_ai_url: h_edit_ai_url.0 as usize,
                h_btn_test_ai: h_btn_test_ai.0 as usize,
                h_lbl_ai_test_result: h_lbl_ai_test_result.0 as usize,
                h_combo_pos: h_combo_pos.0 as usize,
                h_combo_scale: h_combo_scale.0 as usize,
                h_track_font: h_track_font.0 as usize,
                h_lbl_font_val: h_lbl_font_val.0 as usize,
                h_track_opacity: h_track_opacity.0 as usize,
                h_lbl_opacity_val: h_lbl_opacity_val.0 as usize,
                h_combo_dismiss: h_combo_dismiss.0 as usize,
                h_edit_timeout: h_edit_timeout.0 as usize,
                h_btn_demo: h_btn_demo.0 as usize,
                h_chk_tts: h_chk_tts.0 as usize,
                h_track_volume: h_track_volume.0 as usize,
                h_lbl_volume_val: h_lbl_volume_val.0 as usize,
                stt_canceller: Arc::new(AtomicBool::new(false)),
                initial_config: config,
            });

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
            LRESULT(0)
        }

        WM_NOTIFY => {
            let nmhdr = *(lparam.0 as *const NMHDR);
            if nmhdr.idFrom == ID_TAB_CONTROL && nmhdr.code == TCN_SELCHANGE {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr != 0 {
                    let state = &*(ptr as *const SettingsControls);
                    let sel = SendMessageW(HWND(state.h_tab as *mut _), TCM_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                    for (tab_idx, ctrl_list) in state.tab_controls.iter().enumerate() {
                        let show_cmd = if tab_idx == sel { SW_SHOW } else { SW_HIDE };
                        for &ctrl_hwnd_val in ctrl_list {
                            let _ = ShowWindow(HWND(ctrl_hwnd_val as *mut _), show_cmd);
                        }
                    }
                }
            }
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

            if ptr != 0 {
                let state = &*(ptr as *const SettingsControls);

                match cmd_id {
                    ID_COMBO_AI_PROVIDER => {
                        let code = ((wparam.0 >> 16) & 0xFFFF) as usize;
                        if code == 1 { // CBN_SELCHANGE
                            let sel = SendMessageW(HWND(state.h_combo_ai_provider as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                            let defaults = [
                                ("gemini-1.5-flash", "https://generativelanguage.googleapis.com/v1beta/openai"),
                                ("claude-3-5-haiku-20241022", "https://api.anthropic.com/v1"),
                                ("gpt-4o-mini", "https://api.openai.com/v1"),
                                ("hermes-local", "http://127.0.0.1:8317/v1"),
                                ("custom-model", "https://api.openai.com/v1"),
                            ];
                            if let Some(&(m, u)) = defaults.get(sel) {
                                let m_w = to_wide_str(m);
                                let u_w = to_wide_str(u);
                                let _ = SetWindowTextW(HWND(state.h_edit_ai_model as *mut _), PCWSTR(m_w.as_ptr()));
                                let _ = SetWindowTextW(HWND(state.h_edit_ai_url as *mut _), PCWSTR(u_w.as_ptr()));
                            }
                        }
                    }

                    ID_BTN_DOWNLOAD_STT => {
                        let sel_idx = SendMessageW(HWND(state.h_combo_stt_model as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        let catalog = ModelCatalog::all_models();
                        if let Some(target_model) = catalog.get(sel_idx) {
                            let model_id = target_model.id.clone();
                            let h_progress_val = state.h_progress_stt;
                            let h_lbl_val = state.h_lbl_stt_status;
                            let h_btn_val = state.h_btn_stt_download;
                            let canceller = Arc::clone(&state.stt_canceller);
                            canceller.store(false, Ordering::Relaxed);

                            let _ = EnableWindow(HWND(h_btn_val as *mut _), false);
                            let initial_txt = to_wide_str(&format!("Baixando {} ({:.0} MB)... Por favor aguarde.", target_model.name, target_model.size_mb));
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
                                        Ok(_path) => {
                                            SendMessageW(HWND(h_progress_val as *mut _), PBM_SETPOS, WPARAM(100), LPARAM(0));
                                            let ok_msg = format!("✓ Download concluído com sucesso! Modelo ativo e pronto para uso.");
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

                    ID_BTN_TEST_AI => {
                        let mut test_cfg = state.initial_config.llm.clone();

                        let ai_idx = SendMessageW(HWND(state.h_combo_ai_provider as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        let provider_slugs = ["gemini", "claude", "openai", "opencode", "custom"];
                        if let Some(&p) = provider_slugs.get(ai_idx) {
                            test_cfg.provider = p.to_string();
                        }

                        let len_key = GetWindowTextLengthW(HWND(state.h_edit_ai_key as *mut _));
                        let mut buf_key = vec![0u16; (len_key + 1) as usize];
                        GetWindowTextW(HWND(state.h_edit_ai_key as *mut _), &mut buf_key);
                        test_cfg.api_key = String::from_utf16_lossy(&buf_key[..len_key as usize]).trim().to_string();

                        let len_mod = GetWindowTextLengthW(HWND(state.h_edit_ai_model as *mut _));
                        let mut buf_mod = vec![0u16; (len_mod + 1) as usize];
                        GetWindowTextW(HWND(state.h_edit_ai_model as *mut _), &mut buf_mod);
                        test_cfg.model = String::from_utf16_lossy(&buf_mod[..len_mod as usize]).trim().to_string();

                        let len_url = GetWindowTextLengthW(HWND(state.h_edit_ai_url as *mut _));
                        let mut buf_url = vec![0u16; (len_url + 1) as usize];
                        GetWindowTextW(HWND(state.h_edit_ai_url as *mut _), &mut buf_url);
                        test_cfg.base_url = String::from_utf16_lossy(&buf_url[..len_url as usize]).trim().to_string();

                        let h_lbl_val = state.h_lbl_ai_test_result;
                        let h_btn_val = state.h_btn_test_ai;
                        let _ = EnableWindow(HWND(h_btn_val as *mut _), false);
                        let test_wait = to_wide_str("Conectando ao provedor de IA... Aguarde.");
                        let _ = SetWindowTextW(HWND(h_lbl_val as *mut _), PCWSTR(test_wait.as_ptr()));

                        thread::spawn(move || {
                            let client = LlmClient::new(test_cfg);
                            let test_result = client.test_connection();

                            unsafe {
                                let _ = EnableWindow(HWND(h_btn_val as *mut _), true);
                                match test_result {
                                    Ok(msg) => {
                                        let w_msg = to_wide_str(&msg);
                                        let _ = SetWindowTextW(HWND(h_lbl_val as *mut _), PCWSTR(w_msg.as_ptr()));
                                    }
                                    Err(err) => {
                                        let err_msg = format!("✗ Falha na conexão: {err}");
                                        let w_msg = to_wide_str(&err_msg);
                                        let _ = SetWindowTextW(HWND(h_lbl_val as *mut _), PCWSTR(w_msg.as_ptr()));
                                    }
                                }
                            }
                        });
                    }

                    ID_BTN_DEMO_OVERLAY => {
                        let demo_result = lexi_core::models::LookupResult {
                            query: "clutch".to_string(),
                            normalized: "clutch".to_string(),
                            translation: "vitória sob pressão / virada espetacular".to_string(),
                            part_of_speech: Some("gíria gamer".to_string()),
                            definition: "Momento decisivo em que um jogador vence uma rodada ou situação crítica quando está em desvantagem numérica (ex.: 1v3).".to_string(),
                            example: Some("What an insane clutch to win the tournament round! (Que virada insana!)".to_string()),
                            source: lexi_core::models::LookupSource::OfflineDictionary,
                        };
                        crate::app::post_demo_result(demo_result);
                    }

                    ID_BTN_SAVE => {
                        let mut new_config = state.initial_config.clone();

                        // 1. Overlay position
                        let pos_idx = SendMessageW(HWND(state.h_combo_pos as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        if let Some(&(_, ref pos)) = POSITION_CHOICES.get(pos_idx) {
                            new_config.overlay.position = pos.clone();
                        }

                        // Scale
                        let scale_idx = SendMessageW(HWND(state.h_combo_scale as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        if let Some(&(_, s_val)) = SCALE_CHOICES.get(scale_idx) {
                            new_config.overlay.scale = s_val;
                        }

                        // Font size
                        let font_size = SendMessageW(HWND(state.h_track_font as *mut _), TBM_GETPOS, WPARAM(0), LPARAM(0)).0 as i32;
                        new_config.overlay.font_size = font_size;

                        // Opacity
                        let op_pct = SendMessageW(HWND(state.h_track_opacity as *mut _), TBM_GETPOS, WPARAM(0), LPARAM(0)).0 as f32;
                        new_config.overlay.opacity = (op_pct / 100.0).clamp(0.5, 1.0);

                        // Dismiss mode
                        let dis_idx = SendMessageW(HWND(state.h_combo_dismiss as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        if let Some(&(_, ref mode)) = DISMISS_CHOICES.get(dis_idx) {
                            new_config.dismiss.mode = mode.clone();
                        }

                        // Timeout
                        let len_t = GetWindowTextLengthW(HWND(state.h_edit_timeout as *mut _));
                        let mut buf_t = vec![0u16; (len_t + 1) as usize];
                        GetWindowTextW(HWND(state.h_edit_timeout as *mut _), &mut buf_t);
                        let s_t = String::from_utf16_lossy(&buf_t[..len_t as usize]);
                        new_config.dismiss.timeout_seconds = s_t.trim().parse::<f32>().unwrap_or(8.0);

                        // 2. STT & Audio
                        let model_idx = SendMessageW(HWND(state.h_combo_stt_model as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        let catalog = ModelCatalog::all_models();
                        if let Some(m) = catalog.get(model_idx) {
                            new_config.stt.selected_model = m.id.clone();
                        }

                        let is_ptt = SendMessageW(HWND(state.h_radio_ptt as *mut _), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1;
                        new_config.stt.activation_mode = if is_ptt { "push_to_talk".to_string() } else { "toggle".to_string() };

                        let hk_idx = SendMessageW(HWND(state.h_combo_hotkey as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        let hotkey_choices = ["F9", "CapsLock", "F13", "Ctrl+Space", "F8", "F10", "ScrollLock"];
                        if let Some(&hk) = hotkey_choices.get(hk_idx) {
                            new_config.stt.hotkey = hk.to_string();
                        }

                        let mic_idx = SendMessageW(HWND(state.h_combo_mic as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        let devices = list_audio_input_devices();
                        if let Some(dev) = devices.get(mic_idx) {
                            new_config.stt.input_device = Some(dev.clone());
                        }

                        // 3. AI
                        let ai_checked = SendMessageW(HWND(state.h_chk_ai as *mut _), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1;
                        new_config.llm.enabled = ai_checked;

                        let prov_idx = SendMessageW(HWND(state.h_combo_ai_provider as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                        let provider_slugs = ["gemini", "claude", "openai", "opencode", "custom"];
                        if let Some(&p) = provider_slugs.get(prov_idx) {
                            new_config.llm.provider = p.to_string();
                        }

                        let len_key = GetWindowTextLengthW(HWND(state.h_edit_ai_key as *mut _));
                        let mut buf_key = vec![0u16; (len_key + 1) as usize];
                        GetWindowTextW(HWND(state.h_edit_ai_key as *mut _), &mut buf_key);
                        new_config.llm.api_key = String::from_utf16_lossy(&buf_key[..len_key as usize]).trim().to_string();

                        let len_mod = GetWindowTextLengthW(HWND(state.h_edit_ai_model as *mut _));
                        let mut buf_mod = vec![0u16; (len_mod + 1) as usize];
                        GetWindowTextW(HWND(state.h_edit_ai_model as *mut _), &mut buf_mod);
                        new_config.llm.model = String::from_utf16_lossy(&buf_mod[..len_mod as usize]).trim().to_string();

                        let len_url = GetWindowTextLengthW(HWND(state.h_edit_ai_url as *mut _));
                        let mut buf_url = vec![0u16; (len_url + 1) as usize];
                        GetWindowTextW(HWND(state.h_edit_ai_url as *mut _), &mut buf_url);
                        new_config.llm.base_url = String::from_utf16_lossy(&buf_url[..len_url as usize]).trim().to_string();

                        // 4. TTS
                        let tts_checked = SendMessageW(HWND(state.h_chk_tts as *mut _), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1;
                        new_config.audio.enabled = tts_checked;
                        let volume = SendMessageW(HWND(state.h_track_volume as *mut _), TBM_GETPOS, WPARAM(0), LPARAM(0)).0 as u32;
                        new_config.audio.volume = volume;

                        if let Err(e) = crate::save_config_everywhere(&new_config) {
                            log::warn!("Failed saving config: {e}");
                        }
                        crate::app::reload_global_config(new_config);

                        let _ = DestroyWindow(hwnd);
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
                let state = Box::from_raw(ptr as *mut SettingsControls);
                state.stt_canceller.store(true, Ordering::Relaxed);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
