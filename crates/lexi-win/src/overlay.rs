#[cfg(windows)]
use std::mem::size_of;
#[cfg(windows)]
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, GetDC,
    GetMonitorInfoW, MonitorFromWindow, ReleaseDC, SelectObject, SetBkMode, SetTextColor,
    DrawTextW, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    MONITORINFO, MONITOR_DEFAULTTONEAREST, TRANSPARENT, DT_WORDBREAK, DT_LEFT, DT_CALCRECT,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DT_CENTER,
};
#[cfg(windows)]
use windows::Win32::UI::HiDpi::GetDpiForWindow;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetForegroundWindow, GetSystemMetrics,
    RegisterClassW, ShowWindow, UpdateLayeredWindow, HMENU, SM_CXSCREEN, SM_CYSCREEN,
    SW_HIDE, SW_SHOWNOACTIVATE, ULW_ALPHA, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use lexi_core::config::AppConfig;
#[cfg(windows)]
use lexi_core::config::{DismissMode, OverlayPosition};
use lexi_core::models::LookupResult;

#[derive(Clone)]
pub struct OverlayManager {
    hwnd: isize,
    config: Arc<RwLock<AppConfig>>,
    is_visible: Arc<AtomicBool>,
    epoch: Arc<AtomicU64>,
}

#[cfg(windows)]
unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(windows)]
#[inline]
fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

impl OverlayManager {
    pub fn new(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let is_visible = Arc::new(AtomicBool::new(false));
        let epoch = Arc::new(AtomicU64::new(0));
        let config_lock = Arc::new(RwLock::new(config.clone()));

        #[cfg(windows)]
        unsafe {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;

            let class_name: Vec<u16> = OsStr::new("LexiOverlayClass")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(overlay_wnd_proc),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };

            let _ = RegisterClassW(&wnd_class);

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
                PCWSTR(class_name.as_ptr()),
                PCWSTR(null()),
                WS_POPUP,
                0,
                0,
                config.overlay.width,
                config.overlay.max_height,
                HWND(null_mut()),
                HMENU(null_mut()),
                None,
                None,
            )?;

            Ok(Self {
                hwnd: hwnd.0 as isize,
                config: config_lock,
                is_visible,
                epoch,
            })
        }

        #[cfg(not(windows))]
        {
            Ok(Self {
                hwnd: 0,
                config: config_lock,
                is_visible,
                epoch,
            })
        }
    }

    /// Update overlay configuration at runtime
    pub fn update_config(&self, overlay_config: lexi_core::config::OverlayConfig) {
        if let Ok(mut lock) = self.config.write() {
            lock.overlay = overlay_config;
        }
    }

    /// Update dismiss mode at runtime
    pub fn update_dismiss_mode(&self, dismiss_mode: lexi_core::config::DismissMode) {
        if let Ok(mut lock) = self.config.write() {
            lock.dismiss.mode = dismiss_mode;
        }
    }

    /// Update full configuration via shared reference
    #[allow(dead_code)]
    pub fn update_config_shared(&self, config: AppConfig) {
        if let Ok(mut lock) = self.config.write() {
            *lock = config;
        }
    }

    /// Read current configuration
    pub fn get_config(&self) -> AppConfig {
        self.config.read().map(|c| c.clone()).unwrap_or_default()
    }

    /// Show confirmation toast
    pub fn show_toast(&self, message: &str) {
        self.show_temporary_toast(message);
    }

    #[allow(dead_code)]
    pub fn is_visible(&self) -> bool {
        self.is_visible.load(Ordering::SeqCst)
    }

    pub fn show_loading(&self, query: &str) {
        let loading_res = LookupResult {
            query: query.to_string(),
            normalized: query.to_string(),
            translation: "Pesquisando significado...".to_string(),
            part_of_speech: Some("consultando".to_string()),
            definition: format!("Processando o termo \"{}\"...", query),
            example: None,
            source: lexi_core::LookupSource::OfflineDictionary,
        };
        self.render_card(&loading_res);
    }

    pub fn show_result(&self, result: &LookupResult) {
        self.render_card(result);
    }

    /// Show a brief 2-second notification overlay toast (e.g. for config reload confirmation)
    pub fn show_temporary_toast(&self, message: &str) {
        let current_epoch = self.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        self.render_toast(message);

        let overlay = self.clone();
        let my_epoch = current_epoch;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(2000));
            if overlay.epoch.load(Ordering::SeqCst) == my_epoch {
                overlay.hide();
            }
        });
    }

    pub fn hide(&self) {
        self.is_visible.store(false, Ordering::SeqCst);
        #[cfg(windows)]
        unsafe {
            let _ = ShowWindow(HWND(self.hwnd as *mut _), SW_HIDE);
        }
    }

    #[cfg(windows)]
    fn get_dpi_scale(&self) -> f32 {
        let hwnd = HWND(self.hwnd as *mut _);
        let mut dpi = unsafe { GetDpiForWindow(hwnd) };
        if dpi == 0 {
            let fg = unsafe { GetForegroundWindow() };
            dpi = unsafe { GetDpiForWindow(fg) };
        }
        if dpi == 0 {
            dpi = 96;
        }
        (dpi as f32 / 96.0).max(1.0)
    }

    #[cfg(not(windows))]
    fn get_dpi_scale(&self) -> f32 {
        1.0
    }

    #[allow(unused_variables)]
    fn render_card(&self, result: &LookupResult) {
        self.is_visible.store(true, Ordering::SeqCst);
        self.epoch.fetch_add(1, Ordering::SeqCst);

        #[cfg(windows)]
        unsafe {
            let config = self.get_config();
            let dpi_scale = self.get_dpi_scale();

            let width = (config.overlay.width as f32 * dpi_scale).round() as i32;
            let pad_x = (20.0 * dpi_scale).round() as i32;
            let pad_top = (18.0 * dpi_scale).round() as i32;
            let pad_bottom = (16.0 * dpi_scale).round() as i32;
            let content_w = width - pad_x * 2;
            let gap = (8.0 * dpi_scale).round() as i32;

            let screen_dc = GetDC(HWND(null_mut()));
            let mem_dc = CreateCompatibleDC(screen_dc);

            // Setup fonts
            let font_name = to_wide_null_terminated(&config.overlay.font_family);
            let base_font_size = config.overlay.font_size as f32;

            let query_font_size = ((base_font_size * 1.35 * dpi_scale).round() as i32).max(18);
            let trans_font_size = ((base_font_size * 1.25 * dpi_scale).round() as i32).max(17);
            let body_font_size = ((base_font_size * dpi_scale).round() as i32).max(14);
            let badge_font_size = ((base_font_size * 0.75 * dpi_scale).round() as i32).max(11);
            let example_font_size = ((base_font_size * 0.88 * dpi_scale).round() as i32).max(13);
            let footer_font_size = ((base_font_size * 0.78 * dpi_scale).round() as i32).max(11);

            let query_font = CreateFontW(
                query_font_size, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0,
                PCWSTR(font_name.as_ptr()),
            );
            let badge_font = CreateFontW(
                badge_font_size, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0,
                PCWSTR(font_name.as_ptr()),
            );
            let trans_font = CreateFontW(
                trans_font_size, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0,
                PCWSTR(font_name.as_ptr()),
            );
            let body_font = CreateFontW(
                body_font_size, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0,
                PCWSTR(font_name.as_ptr()),
            );
            let example_font = CreateFontW(
                example_font_size, 0, 0, 0, 400, 1, 0, 0, 1, 0, 0, 5, 0,
                PCWSTR(font_name.as_ptr()),
            );
            let footer_font = CreateFontW(
                footer_font_size, 0, 0, 0, 500, 0, 0, 0, 1, 0, 0, 5, 0,
                PCWSTR(font_name.as_ptr()),
            );

            // Measure elements
            let mut query_wide = to_wide_chars(&result.normalized.to_uppercase());
            let mut q_rc = RECT { left: 0, top: 0, right: content_w, bottom: 0 };
            SelectObject(mem_dc, query_font);
            DrawTextW(mem_dc, &mut query_wide, &mut q_rc, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
            let query_w = q_rc.right - q_rc.left;
            let query_h = q_rc.bottom - q_rc.top;

            let (badge_w, badge_h, badge_wide) = if let Some(ref pos) = result.part_of_speech {
                let mut b_wide = to_wide_chars(&pos.to_uppercase());
                let mut b_rc = RECT { left: 0, top: 0, right: content_w, bottom: 0 };
                SelectObject(mem_dc, badge_font);
                DrawTextW(mem_dc, &mut b_wide, &mut b_rc, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
                (b_rc.right - b_rc.left, b_rc.bottom - b_rc.top, Some(b_wide))
            } else {
                (0, 0, None)
            };

            let badge_pill_pad_x = (8.0 * dpi_scale).round() as i32;
            let badge_pill_pad_y = (3.0 * dpi_scale).round() as i32;
            let badge_pill_w = if badge_wide.is_some() { badge_w + badge_pill_pad_x * 2 } else { 0 };
            let badge_pill_h = if badge_wide.is_some() { badge_h + badge_pill_pad_y * 2 } else { 0 };

            let mut trans_wide = to_wide_chars(&format!("→ {}", result.translation));
            let mut t_rc = RECT { left: 0, top: 0, right: content_w, bottom: 0 };
            SelectObject(mem_dc, trans_font);
            DrawTextW(mem_dc, &mut trans_wide, &mut t_rc, DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX);
            let trans_h = (t_rc.bottom - t_rc.top).max((22.0 * dpi_scale) as i32);

            let mut def_wide = to_wide_chars(&result.definition);
            let mut d_rc = RECT { left: 0, top: 0, right: content_w, bottom: 0 };
            SelectObject(mem_dc, body_font);
            DrawTextW(mem_dc, &mut def_wide, &mut d_rc, DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX);
            let def_h = (d_rc.bottom - d_rc.top).max((18.0 * dpi_scale) as i32);

            let (ex_h, ex_wide) = if let Some(ref ex) = result.example {
                let mut e_wide = to_wide_chars(&format!("Ex: {}", ex));
                let mut e_rc = RECT { left: 0, top: 0, right: content_w, bottom: 0 };
                SelectObject(mem_dc, example_font);
                DrawTextW(mem_dc, &mut e_wide, &mut e_rc, DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX);
                ((e_rc.bottom - e_rc.top).max((16.0 * dpi_scale) as i32), Some(e_wide))
            } else {
                (0, None)
            };

            let hint_text = match config.dismiss.mode {
                DismissMode::ReleaseKey => "Solte a tecla para fechar",
                DismissMode::Timeout => "Fechando automaticamente...",
                DismissMode::Hotkey => "Pressione ESC para fechar",
                DismissMode::AnyKey => "Pressione qualquer tecla para fechar",
                DismissMode::Hybrid => "Solte a tecla ou pressione ESC para fechar",
            };
            let mut hint_wide = to_wide_chars(hint_text);
            let mut f_rc = RECT { left: 0, top: 0, right: content_w, bottom: 0 };
            SelectObject(mem_dc, footer_font);
            DrawTextW(mem_dc, &mut hint_wide, &mut f_rc, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
            let foot_w = f_rc.right - f_rc.left;
            let foot_h = (f_rc.bottom - f_rc.top).max((14.0 * dpi_scale) as i32);
            let foot_pill_h = foot_h + (6.0 * dpi_scale).round() as i32;

            // Calculate card height dynamically so text is never truncated
            let header_row_h = query_h.max(badge_pill_h);
            let mut total_needed = pad_top + header_row_h + gap + trans_h + gap + def_h + gap;
            if ex_wide.is_some() {
                total_needed += ex_h + gap;
            }
            total_needed += (10.0 * dpi_scale).round() as i32; // breathing space
            total_needed += foot_pill_h + pad_bottom;

            let min_h = (190.0 * dpi_scale).round() as i32;
            let configured_max_h = (config.overlay.max_height as f32 * dpi_scale).round() as i32;
            let max_h = configured_max_h.max((440.0 * dpi_scale).round() as i32);
            let height = total_needed.clamp(min_h, max_h);

            let (pos_x, pos_y) = self.calculate_position(width, height, dpi_scale);

            // Allocate 32-bit DIB section
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height, // top-down DIB
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits: *mut u8 = null_mut();
            let bitmap = match CreateDIBSection(
                mem_dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits as *mut *mut u8 as *mut *mut _,
                None,
                0,
            ) {
                Ok(b) => b,
                Err(e) => {
                    log::warn!("CreateDIBSection failed: {e}");
                    let _ = DeleteDC(mem_dc);
                    ReleaseDC(HWND(null_mut()), screen_dc);
                    return;
                }
            };

            let old_bitmap = SelectObject(mem_dc, bitmap);
            let pixel_count = (width * height) as usize;
            let slice = std::slice::from_raw_parts_mut(bits as *mut u32, pixel_count);
            slice.fill(0);

            // Draw Card Background & Pill Boxes
            let corner_radius = (12.0 * dpi_scale).round() as i32;
            let bg_alpha = ((config.overlay.opacity * 255.0).clamp(0.0, 255.0)) as u8;

            // Main Card surface: Slate-900 with Indigo-500 border
            draw_rounded_box(
                slice, width, height,
                0, 0, width, height, corner_radius,
                (15, 23, 42, bg_alpha),       // Slate-900
                (99, 102, 241, bg_alpha.max(220)), // Indigo-500
            );

            // Top highlight accent line (Sky-400 / Emerald-400)
            let top_border_h = (1.5 * dpi_scale).round().max(1.0) as i32;
            let highlight_r = 56u8;
            let highlight_g = 189u8;
            let highlight_b = 248u8;
            let highlight_a = bg_alpha.max(220);
            let highlight_pixel = ((highlight_a as u32) << 24)
                | (((highlight_r as u32 * highlight_a as u32 + 127) / 255) << 16)
                | (((highlight_g as u32 * highlight_a as u32 + 127) / 255) << 8)
                | ((highlight_b as u32 * highlight_a as u32 + 127) / 255);

            for y in 0..top_border_h {
                for x in corner_radius..(width - corner_radius) {
                    let idx = (y * width + x) as usize;
                    slice[idx] = highlight_pixel;
                }
            }

            // Draw Part-of-Speech Badge Pill
            let pill_x0 = pad_x + query_w + (12.0 * dpi_scale) as i32;
            let pill_y0 = pad_top + (header_row_h - badge_pill_h) / 2;
            if badge_wide.is_some() {
                draw_rounded_box(
                    slice, width, height,
                    pill_x0, pill_y0, pill_x0 + badge_pill_w, pill_y0 + badge_pill_h,
                    badge_pill_h / 2,
                    (30, 27, 75, 240),   // Indigo-950
                    (99, 102, 241, 240), // Indigo-500
                );
            }

            // Draw subtle divider line above footer
            let foot_pill_w = foot_w + (20.0 * dpi_scale).round() as i32;
            let foot_pill_y0 = height - pad_bottom - foot_pill_h;
            let foot_pill_x0 = pad_x;
            let sep_y = foot_pill_y0 - (8.0 * dpi_scale).round() as i32;

            let sep_alpha = (bg_alpha as u32 * 140) / 255;
            let sep_pixel = (sep_alpha << 24)
                | (((30 * sep_alpha + 127) / 255) << 16)
                | (((41 * sep_alpha + 127) / 255) << 8)
                | ((59 * sep_alpha + 127) / 255);

            if sep_y > 0 && sep_y < height {
                for x in pad_x..(width - pad_x) {
                    slice[(sep_y * width + x) as usize] = sep_pixel;
                }
            }

            // Draw Footer Dismissal Pill
            draw_rounded_box(
                slice, width, height,
                foot_pill_x0, foot_pill_y0, foot_pill_x0 + foot_pill_w, foot_pill_y0 + foot_pill_h,
                (6.0 * dpi_scale).round() as i32,
                (30, 41, 59, 230), // Slate-800
                (51, 65, 85, 230), // Slate-700
            );

            // Snapshot background buffer before GDI drawing to resolve alpha premultiplication
            let mut bg_snapshot = vec![0u32; pixel_count];
            bg_snapshot.copy_from_slice(slice);

            SetBkMode(mem_dc, TRANSPARENT);

            let mut cur_y = pad_top;

            // 1. Header Query Title
            SelectObject(mem_dc, query_font);
            SetTextColor(mem_dc, rgb(248, 250, 252)); // Slate-50 White
            let mut draw_q_rc = RECT {
                left: pad_x,
                top: cur_y,
                right: pad_x + query_w + 10,
                bottom: cur_y + query_h,
            };
            DrawTextW(mem_dc, &mut query_wide, &mut draw_q_rc, DT_SINGLELINE | DT_NOPREFIX);

            // 1b. Header Part of Speech Badge
            if let Some(mut b_wide) = badge_wide {
                SelectObject(mem_dc, badge_font);
                SetTextColor(mem_dc, rgb(199, 210, 254)); // Indigo-200
                let mut draw_b_rc = RECT {
                    left: pill_x0,
                    top: pill_y0,
                    right: pill_x0 + badge_pill_w,
                    bottom: pill_y0 + badge_pill_h,
                };
                DrawTextW(mem_dc, &mut b_wide, &mut draw_b_rc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            }

            cur_y += header_row_h + gap;

            // 2. Translation (High contrast emerald/cyan #38bdf8 / #34d399)
            SelectObject(mem_dc, trans_font);
            SetTextColor(mem_dc, rgb(56, 189, 248)); // Sky-400
            let mut draw_t_rc = RECT {
                left: pad_x,
                top: cur_y,
                right: width - pad_x,
                bottom: cur_y + trans_h + 10,
            };
            DrawTextW(mem_dc, &mut trans_wide, &mut draw_t_rc, DT_WORDBREAK | DT_LEFT | DT_NOPREFIX);

            cur_y += trans_h + gap;

            // 3. Definition / Context explanation (Slate-100/200 wrapped with DT_WORDBREAK)
            SelectObject(mem_dc, body_font);
            SetTextColor(mem_dc, rgb(226, 232, 240)); // Slate-200
            let max_def_bottom = sep_y - (6.0 * dpi_scale) as i32;
            let mut draw_d_rc = RECT {
                left: pad_x,
                top: cur_y,
                right: width - pad_x,
                bottom: max_def_bottom,
            };
            let actual_def_h = DrawTextW(mem_dc, &mut def_wide, &mut draw_d_rc, DT_WORDBREAK | DT_LEFT | DT_NOPREFIX);

            cur_y += actual_def_h + gap;

            // 4. Example (if present, italicized with "Ex:" prefix in slate-400)
            if let Some(mut e_wide) = ex_wide {
                if cur_y < max_def_bottom {
                    SelectObject(mem_dc, example_font);
                    SetTextColor(mem_dc, rgb(148, 163, 184)); // Slate-400
                    let mut draw_e_rc = RECT {
                        left: pad_x,
                        top: cur_y,
                        right: width - pad_x,
                        bottom: max_def_bottom,
                    };
                    let _ = DrawTextW(mem_dc, &mut e_wide, &mut draw_e_rc, DT_WORDBREAK | DT_LEFT | DT_NOPREFIX);
                }
            }

            // 5. Dismissal Hint in Footer Pill
            SelectObject(mem_dc, footer_font);
            SetTextColor(mem_dc, rgb(148, 163, 184)); // Slate-400
            let mut draw_f_rc = RECT {
                left: foot_pill_x0,
                top: foot_pill_y0,
                right: foot_pill_x0 + foot_pill_w,
                bottom: foot_pill_y0 + foot_pill_h,
            };
            DrawTextW(mem_dc, &mut hint_wide, &mut draw_f_rc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

            // Fix alpha channel premultiplication for layered window
            // Any pixel modified by GDI text rendering is set to full opacity (A=255)
            // ensuring smooth, crisp rendering over fullscreen games without dark borders.
            for i in 0..pixel_count {
                let current_rgb = slice[i] & 0x00FF_FFFF;
                let initial_rgb = bg_snapshot[i] & 0x00FF_FFFF;
                if current_rgb != initial_rgb {
                    let r = (current_rgb >> 16) & 0xFF;
                    let g = (current_rgb >> 8) & 0xFF;
                    let b = current_rgb & 0xFF;
                    slice[i] = (255 << 24) | (r << 16) | (g << 8) | b;
                }
            }

            let pt_src = POINT { x: 0, y: 0 };
            let pt_dst = POINT { x: pos_x, y: pos_y };
            let sz = SIZE { cx: width, cy: height };
            let blend = BLENDFUNCTION {
                BlendOp: 0,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: 1, // AC_SRC_ALPHA
            };

            let _ = UpdateLayeredWindow(
                HWND(self.hwnd as *mut _),
                screen_dc,
                Some(&pt_dst),
                Some(&sz),
                mem_dc,
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            let _ = ShowWindow(HWND(self.hwnd as *mut _), SW_SHOWNOACTIVATE);

            // Clean up GDI objects
            let _ = DeleteObject(query_font);
            let _ = DeleteObject(badge_font);
            let _ = DeleteObject(trans_font);
            let _ = DeleteObject(body_font);
            let _ = DeleteObject(example_font);
            let _ = DeleteObject(footer_font);
            SelectObject(mem_dc, old_bitmap);
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(mem_dc);
            ReleaseDC(HWND(null_mut()), screen_dc);
        }
    }

    #[allow(unused_variables)]
    fn render_toast(&self, message: &str) {
        self.is_visible.store(true, Ordering::SeqCst);

        #[cfg(windows)]
        unsafe {
            let dpi_scale = self.get_dpi_scale();
            let config = self.get_config();

            let width = (380.0 * dpi_scale).round() as i32;
            let height = (68.0 * dpi_scale).round() as i32;
            let (pos_x, pos_y) = self.calculate_position(width, height, dpi_scale);

            let screen_dc = GetDC(HWND(null_mut()));
            let mem_dc = CreateCompatibleDC(screen_dc);

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits: *mut u8 = null_mut();
            let bitmap = match CreateDIBSection(
                mem_dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits as *mut *mut u8 as *mut *mut _,
                None,
                0,
            ) {
                Ok(b) => b,
                Err(e) => {
                    log::warn!("CreateDIBSection for toast failed: {e}");
                    let _ = DeleteDC(mem_dc);
                    ReleaseDC(HWND(null_mut()), screen_dc);
                    return;
                }
            };

            let old_bitmap = SelectObject(mem_dc, bitmap);
            let pixel_count = (width * height) as usize;
            let slice = std::slice::from_raw_parts_mut(bits as *mut u32, pixel_count);
            slice.fill(0);

            let corner_radius = (12.0 * dpi_scale).round() as i32;
            draw_rounded_box(
                slice, width, height,
                0, 0, width, height, corner_radius,
                (15, 23, 42, 245),       // Slate-900
                (52, 211, 153, 255),     // Emerald-400 accent border
            );

            let mut bg_snapshot = vec![0u32; pixel_count];
            bg_snapshot.copy_from_slice(slice);

            SetBkMode(mem_dc, TRANSPARENT);
            let font_name = to_wide_null_terminated(&config.overlay.font_family);

            let badge_font = CreateFontW(
                ((12.0 * dpi_scale).round() as i32).max(11), 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0,
                PCWSTR(font_name.as_ptr()),
            );
            let msg_font = CreateFontW(
                ((14.0 * dpi_scale).round() as i32).max(12), 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0,
                PCWSTR(font_name.as_ptr()),
            );

            // Icon / App badge
            SelectObject(mem_dc, badge_font);
            SetTextColor(mem_dc, rgb(52, 211, 153)); // Emerald-400
            let mut icon_wide = to_wide_chars("✓ LEXI");
            let icon_pad_x = (18.0 * dpi_scale).round() as i32;
            let mut icon_rc = RECT {
                left: icon_pad_x,
                top: 0,
                right: icon_pad_x + (70.0 * dpi_scale) as i32,
                bottom: height,
            };
            DrawTextW(mem_dc, &mut icon_wide, &mut icon_rc, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

            // Toast message text
            SelectObject(mem_dc, msg_font);
            SetTextColor(mem_dc, rgb(241, 245, 249)); // Slate-100
            let mut msg_wide = to_wide_chars(message);
            let msg_left = icon_rc.right + (6.0 * dpi_scale) as i32;
            let mut msg_rc = RECT {
                left: msg_left,
                top: 0,
                right: width - (16.0 * dpi_scale) as i32,
                bottom: height,
            };
            DrawTextW(mem_dc, &mut msg_wide, &mut msg_rc, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

            for i in 0..pixel_count {
                let current_rgb = slice[i] & 0x00FF_FFFF;
                let initial_rgb = bg_snapshot[i] & 0x00FF_FFFF;
                if current_rgb != initial_rgb {
                    let r = (current_rgb >> 16) & 0xFF;
                    let g = (current_rgb >> 8) & 0xFF;
                    let b = current_rgb & 0xFF;
                    slice[i] = (255 << 24) | (r << 16) | (g << 8) | b;
                }
            }

            let pt_src = POINT { x: 0, y: 0 };
            let pt_dst = POINT { x: pos_x, y: pos_y };
            let sz = SIZE { cx: width, cy: height };
            let blend = BLENDFUNCTION {
                BlendOp: 0,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: 1,
            };

            let _ = UpdateLayeredWindow(
                HWND(self.hwnd as *mut _),
                screen_dc,
                Some(&pt_dst),
                Some(&sz),
                mem_dc,
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            let _ = ShowWindow(HWND(self.hwnd as *mut _), SW_SHOWNOACTIVATE);

            let _ = DeleteObject(badge_font);
            let _ = DeleteObject(msg_font);
            SelectObject(mem_dc, old_bitmap);
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(mem_dc);
            ReleaseDC(HWND(null_mut()), screen_dc);
        }
    }

    #[cfg(windows)]
    unsafe fn calculate_position(&self, width: i32, height: i32, dpi_scale: f32) -> (i32, i32) {
        let fg = GetForegroundWindow();
        let monitor = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);

        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };

        let (mon_x, mon_y, mon_w, mon_h) = if GetMonitorInfoW(monitor, &mut mi).as_bool() {
            let rc = mi.rcWork;
            (rc.left, rc.top, rc.right - rc.left, rc.bottom - rc.top)
        } else {
            (0, 0, GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
        };

        let margin_x = (40.0 * dpi_scale) as i32;
        let margin_y = (40.0 * dpi_scale) as i32;
        let bottom_margin = (60.0 * dpi_scale) as i32;

        let position = self.get_config().overlay.position;
        match position {
            OverlayPosition::TopCenter => (mon_x + (mon_w - width) / 2, mon_y + margin_y),
            OverlayPosition::TopRight => (mon_x + mon_w - width - margin_x, mon_y + margin_y),
            OverlayPosition::TopLeft => (mon_x + margin_x, mon_y + margin_y),
            OverlayPosition::Center => (mon_x + (mon_w - width) / 2, mon_y + (mon_h - height) / 2),
            OverlayPosition::BottomCenter => (mon_x + (mon_w - width) / 2, mon_y + mon_h - height - bottom_margin),
            OverlayPosition::MiddleLeft => (mon_x + (40.0 * dpi_scale) as i32, mon_y + (mon_h - height) / 2),
            OverlayPosition::MiddleRight => (mon_x + mon_w - width - (40.0 * dpi_scale) as i32, mon_y + (mon_h - height) / 2),
        }
    }

    #[cfg(not(windows))]
    fn calculate_position(&self, _width: i32, _height: i32, _dpi_scale: f32) -> (i32, i32) {
        (0, 0)
    }
}

/// Helper to draw a filled rounded box with border in premultiplied ARGB
#[cfg(windows)]
fn draw_rounded_box(
    slice: &mut [u32],
    w: i32,
    h: i32,
    rx0: i32,
    ry0: i32,
    rx1: i32,
    ry1: i32,
    radius: i32,
    fill_color: (u8, u8, u8, u8),
    border_color: (u8, u8, u8, u8),
) {
    let (fr, fg, fb, fa) = fill_color;
    let (br, bg, bb, ba) = border_color;
    let fill_pixel = ((fa as u32) << 24)
        | (((fr as u32 * fa as u32 + 127) / 255) << 16)
        | (((fg as u32 * fa as u32 + 127) / 255) << 8)
        | ((fb as u32 * fa as u32 + 127) / 255);
    let border_pixel = ((ba as u32) << 24)
        | (((br as u32 * ba as u32 + 127) / 255) << 16)
        | (((bg as u32 * ba as u32 + 127) / 255) << 8)
        | ((bb as u32 * ba as u32 + 127) / 255);

    let box_w = rx1 - rx0;
    let box_h = ry1 - ry0;
    let r = radius.min(box_w / 2).min(box_h / 2).max(1);

    for y in ry0..ry1 {
        if y < 0 || y >= h { continue; }
        for x in rx0..rx1 {
            if x < 0 || x >= w { continue; }
            let dx = if x < rx0 + r {
                (rx0 + r - 1) - x
            } else if x >= rx1 - r {
                x - (rx1 - r)
            } else {
                0
            };
            let dy = if y < ry0 + r {
                (ry0 + r - 1) - y
            } else if y >= ry1 - r {
                y - (ry1 - r)
            } else {
                0
            };

            let dist_sq = dx * dx + dy * dy;
            if dist_sq > r * r {
                continue;
            }

            let idx = (y * w + x) as usize;
            let is_edge = (dist_sq >= (r - 1) * (r - 1) && (dx > 0 || dy > 0))
                || x == rx0 || x == rx1 - 1 || y == ry0 || y == ry1 - 1;

            if is_edge {
                slice[idx] = border_pixel;
            } else {
                slice[idx] = fill_pixel;
            }
        }
    }
}

#[cfg(windows)]
fn to_wide_chars(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().collect()
}

#[cfg(windows)]
fn to_wide_null_terminated(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}
