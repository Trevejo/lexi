//! Hotkey parsing and modifier detection module for Lexi.
//!
//! Provides virtual key parsing, modifier tracking (Ctrl, Alt, Shift, Win),
//! modifier release detection, and human-friendly display formatting.

use serde::{Deserialize, Serialize};

// Common Windows Virtual-Key (VK) Codes
pub const VK_BACK: u32 = 0x08;
pub const VK_TAB: u32 = 0x09;
pub const VK_RETURN: u32 = 0x0D;
pub const VK_SHIFT: u32 = 0x10;
pub const VK_CONTROL: u32 = 0x11;
pub const VK_MENU: u32 = 0x12; // Alt key
pub const VK_PAUSE: u32 = 0x13;
pub const VK_CAPITAL: u32 = 0x14; // CapsLock
pub const VK_ESCAPE: u32 = 0x1B;
pub const VK_SPACE: u32 = 0x20;
pub const VK_PRIOR: u32 = 0x21; // PageUp
pub const VK_NEXT: u32 = 0x22; // PageDown
pub const VK_END: u32 = 0x23;
pub const VK_HOME: u32 = 0x24;
pub const VK_LEFT: u32 = 0x25;
pub const VK_UP: u32 = 0x26;
pub const VK_RIGHT: u32 = 0x27;
pub const VK_DOWN: u32 = 0x28;
pub const VK_SNAPSHOT: u32 = 0x2C; // PrintScreen
pub const VK_INSERT: u32 = 0x2D;
pub const VK_DELETE: u32 = 0x2E;
pub const VK_LWIN: u32 = 0x5B;
pub const VK_RWIN: u32 = 0x5C;
pub const VK_APPS: u32 = 0x5D;
pub const VK_F1: u32 = 0x70;
pub const VK_F2: u32 = 0x71;
pub const VK_F3: u32 = 0x72;
pub const VK_F4: u32 = 0x73;
pub const VK_F5: u32 = 0x74;
pub const VK_F6: u32 = 0x75;
pub const VK_F7: u32 = 0x76;
pub const VK_F8: u32 = 0x77;
pub const VK_F9: u32 = 0x78;
pub const VK_F10: u32 = 0x79;
pub const VK_F11: u32 = 0x7A;
pub const VK_F12: u32 = 0x7B;
pub const VK_F13: u32 = 0x7C;
pub const VK_F14: u32 = 0x7D;
pub const VK_F15: u32 = 0x7E;
pub const VK_F16: u32 = 0x7F;
pub const VK_F17: u32 = 0x80;
pub const VK_F18: u32 = 0x81;
pub const VK_F19: u32 = 0x82;
pub const VK_F20: u32 = 0x83;
pub const VK_F21: u32 = 0x84;
pub const VK_F22: u32 = 0x85;
pub const VK_F23: u32 = 0x86;
pub const VK_F24: u32 = 0x87;
pub const VK_NUMLOCK: u32 = 0x90;
pub const VK_SCROLL: u32 = 0x91; // ScrollLock
pub const VK_LSHIFT: u32 = 0xA0;
pub const VK_RSHIFT: u32 = 0xA1;
pub const VK_LCONTROL: u32 = 0xA2;
pub const VK_RCONTROL: u32 = 0xA3;
pub const VK_LMENU: u32 = 0xA4;
pub const VK_RMENU: u32 = 0xA5;

/// Represents a hotkey binding with its primary virtual key code and modifier flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HotkeyBinding {
    pub vk: u32,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
}

impl Default for HotkeyBinding {
    fn default() -> Self {
        Self {
            vk: VK_F9,
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
        }
    }
}

impl HotkeyBinding {
    /// Returns true if the released VK code is one of the active modifiers required by this binding:
    /// - Ctrl: `VK_CONTROL` (0x11), `VK_LCONTROL` (0xA2), `VK_RCONTROL` (0xA3)
    /// - Alt: `VK_MENU` (0x12), `VK_LMENU` (0xA4), `VK_RMENU` (0xA5)
    /// - Shift: `VK_SHIFT` (0x10), `VK_LSHIFT` (0xA0), `VK_RSHIFT` (0xA1)
    /// - Win: `VK_LWIN` (0x5B), `VK_RWIN` (0x5C)
    pub fn is_modifier_released(&self, vk: u32) -> bool {
        (self.ctrl && matches!(vk, VK_CONTROL | VK_LCONTROL | VK_RCONTROL))
            || (self.alt && matches!(vk, VK_MENU | VK_LMENU | VK_RMENU))
            || (self.shift && matches!(vk, VK_SHIFT | VK_LSHIFT | VK_RSHIFT))
            || (self.win && matches!(vk, VK_LWIN | VK_RWIN))
    }

    /// Formats the hotkey as a human-friendly string like `'Ctrl + Space'`, `'Alt + F9'`, `'CapsLock'`.
    pub fn to_display_string(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        if self.win {
            parts.push("Win".to_string());
        }

        // Avoid duplicating modifier names if primary key is itself a modifier that's already flagged
        let is_dup_modifier = match self.vk {
            VK_CONTROL | VK_LCONTROL | VK_RCONTROL => self.ctrl,
            VK_MENU | VK_LMENU | VK_RMENU => self.alt,
            VK_SHIFT | VK_LSHIFT | VK_RSHIFT => self.shift,
            VK_LWIN | VK_RWIN => self.win,
            _ => false,
        };

        if !is_dup_modifier {
            if let Some(name) = vk_to_display_name(self.vk) {
                parts.push(name.to_string());
            } else if self.vk != 0 {
                parts.push(format!("0x{:02X}", self.vk));
            }
        }

        if parts.is_empty() {
            "None".to_string()
        } else {
            parts.join(" + ")
        }
    }

    /// Checks if this binding requires any modifiers (Ctrl, Alt, Shift, or Win).
    pub fn has_modifiers(&self) -> bool {
        self.ctrl || self.alt || self.shift || self.win
    }

    /// Returns true if the given modifier states match this binding's required modifiers.
    pub fn matches_modifiers(&self, ctrl: bool, alt: bool, shift: bool, win: bool) -> bool {
        self.ctrl == ctrl && self.alt == alt && self.shift == shift && self.win == win
    }
}

impl std::fmt::Display for HotkeyBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

/// Parses a hotkey string into a [`HotkeyBinding`].
///
/// Features:
/// - Case-insensitive and whitespace-trimming.
/// - Strips UI descriptions in parentheses (e.g. `"F9 (Recomendado)"` -> `"F9"`).
/// - Handles combinations separated by `'+'`, `'-'`, or spaces (e.g. `'Ctrl+Space'`, `'ctrl + space'`, `'ctrl+espaço'`).
/// - Recognizes modifier aliases:
///   - Ctrl: `'ctrl'`, `'control'`, `'strg'`
///   - Alt: `'alt'`, `'menu'`
///   - Shift: `'shift'`
///   - Win: `'win'`, `'super'`, `'cmd'`, `'windows'`, `'meta'`
/// - Supports common keys including Portuguese names (`'espaço'`, `'espaco'`, `'fixa'`).
pub fn parse_hotkey(s: &str) -> HotkeyBinding {
    // Strip trailing comments/labels in parentheses, e.g. "F9 (Recomendado)" -> "F9"
    let clean = if let Some(idx) = s.find('(') {
        &s[..idx]
    } else {
        s
    };

    let trimmed = clean.trim();
    if trimmed.is_empty() {
        return HotkeyBinding::default();
    }

    let mut lower = trimmed.to_lowercase();
    // Normalize compound key names so they aren't split by spaces
    lower = lower
        .replace("caps lock", "capslock")
        .replace("scroll lock", "scrolllock")
        .replace("num lock", "numlock")
        .replace("page up", "pageup")
        .replace("page down", "pagedown")
        .replace("print screen", "printscreen")
        .replace("prt scn", "printscreen")
        .replace("prt sc", "printscreen");

    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut win = false;
    let mut explicit_vk: Option<u32> = None;

    // Split by '+', '-', or whitespace
    let tokens: Vec<&str> = lower
        .split(|c: char| c == '+' || c == '-' || c.is_whitespace())
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect();

    for token in tokens {
        match token {
            "ctrl" | "control" | "strg" => {
                ctrl = true;
            }
            "alt" | "menu" => {
                alt = true;
            }
            "shift" => {
                shift = true;
            }
            "win" | "super" | "cmd" | "windows" | "meta" => {
                win = true;
            }
            other => {
                if let Some(vk) = parse_single_key(other) {
                    explicit_vk = Some(vk);
                }
            }
        }
    }

    let vk = match explicit_vk {
        Some(code) => code,
        None => {
            // If only modifiers were specified, assign that modifier's own VK code
            if ctrl {
                VK_CONTROL
            } else if alt {
                VK_MENU
            } else if shift {
                VK_SHIFT
            } else if win {
                VK_LWIN
            } else {
                VK_F9 // Default fallback
            }
        }
    };

    HotkeyBinding {
        vk,
        ctrl,
        alt,
        shift,
        win,
    }
}

/// Parses an individual key name into its Windows Virtual-Key (VK) code.
fn parse_single_key(key: &str) -> Option<u32> {
    // Function keys F1..F24
    if key.starts_with('f') && key.len() >= 2 {
        if let Ok(num) = key[1..].parse::<u32>() {
            if (1..=24).contains(&num) {
                return Some(VK_F1 + (num - 1));
            }
        }
    }

    match key {
        // Space
        "space" | "espaco" | "espaço" => Some(VK_SPACE),
        // Tab
        "tab" | "tabulacao" | "tabulação" => Some(VK_TAB),
        // Enter / Return
        "enter" | "return" | "retorno" => Some(VK_RETURN),
        // Escape
        "esc" | "escape" => Some(VK_ESCAPE),
        // Backspace
        "backspace" | "back" => Some(VK_BACK),
        // CapsLock
        "capslock" | "caps" | "fixa" | "capital" => Some(VK_CAPITAL),
        // ScrollLock
        "scrolllock" | "scroll" => Some(VK_SCROLL),
        // NumLock
        "numlock" => Some(VK_NUMLOCK),
        // Pause
        "pause" | "pausa" | "break" => Some(VK_PAUSE),
        // Insert / Delete
        "insert" | "ins" => Some(VK_INSERT),
        "delete" | "del" => Some(VK_DELETE),
        // Home / End
        "home" | "inicio" | "início" => Some(VK_HOME),
        "end" | "fim" => Some(VK_END),
        // Page Up / Down
        "pageup" | "pgup" | "prior" => Some(VK_PRIOR),
        "pagedown" | "pgdn" | "next" => Some(VK_NEXT),
        // Arrow keys
        "up" | "cima" => Some(VK_UP),
        "down" | "baixo" => Some(VK_DOWN),
        "left" | "esquerda" => Some(VK_LEFT),
        "right" | "direita" => Some(VK_RIGHT),
        // PrintScreen
        "printscreen" | "prtscn" | "prt_scn" | "snapshot" | "print" => Some(VK_SNAPSHOT),
        // Application / Context menu key
        "apps" | "app" | "contextmenu" => Some(VK_APPS),
        // Numpad 0..9
        "numpad0" | "num0" => Some(0x60),
        "numpad1" | "num1" => Some(0x61),
        "numpad2" | "num2" => Some(0x62),
        "numpad3" | "num3" => Some(0x63),
        "numpad4" | "num4" => Some(0x64),
        "numpad5" | "num5" => Some(0x65),
        "numpad6" | "num6" => Some(0x66),
        "numpad7" | "num7" => Some(0x67),
        "numpad8" | "num8" => Some(0x68),
        "numpad9" | "num9" => Some(0x69),
        // Numpad operators
        "numpadmultiply" | "nummul" => Some(0x6A),
        "numpadadd" | "numadd" => Some(0x6B),
        "numpadsubtract" | "numsub" => Some(0x6D),
        "numpaddecimal" | "numdec" | "numpaddot" => Some(0x6E),
        "numpaddivide" | "numdiv" => Some(0x6F),
        // Modifiers as explicit keys
        "ctrl" | "control" | "strg" => Some(VK_CONTROL),
        "alt" | "menu" => Some(VK_MENU),
        "shift" => Some(VK_SHIFT),
        "win" | "super" | "cmd" => Some(VK_LWIN),
        // OEM punctuation keys
        "semicolon" | ";" => Some(0xBA),
        "plus" | "equal" | "=" => Some(0xBB),
        "comma" | "," => Some(0xBC),
        "minus" | "dash" => Some(0xBD),
        "period" | "dot" | "." => Some(0xBE),
        "slash" | "/" => Some(0xBF),
        "tilde" | "backquote" | "`" => Some(0xC0),
        "bracketleft" | "[" => Some(0xDB),
        "backslash" | "\\" => Some(0xDC),
        "bracketright" | "]" => Some(0xDD),
        "quote" | "'" => Some(0xDE),
        _ => {
            // Single character check (A-Z, 0-9)
            if key.len() == 1 {
                let c = key.chars().next().unwrap();
                if c.is_ascii_alphabetic() {
                    let upper = c.to_ascii_uppercase();
                    return Some(upper as u32);
                } else if c.is_ascii_digit() {
                    return Some(c as u32);
                }
            }
            None
        }
    }
}

/// Returns a human-friendly display name for a known VK code.
pub fn vk_to_display_name(vk: u32) -> Option<&'static str> {
    match vk {
        VK_BACK => Some("Backspace"),
        VK_TAB => Some("Tab"),
        VK_RETURN => Some("Enter"),
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT => Some("Shift"),
        VK_CONTROL | VK_LCONTROL | VK_RCONTROL => Some("Ctrl"),
        VK_MENU | VK_LMENU | VK_RMENU => Some("Alt"),
        VK_PAUSE => Some("Pause"),
        VK_CAPITAL => Some("CapsLock"),
        VK_ESCAPE => Some("Esc"),
        VK_SPACE => Some("Space"),
        VK_PRIOR => Some("PageUp"),
        VK_NEXT => Some("PageDown"),
        VK_END => Some("End"),
        VK_HOME => Some("Home"),
        VK_LEFT => Some("Left"),
        VK_UP => Some("Up"),
        VK_RIGHT => Some("Right"),
        VK_DOWN => Some("Down"),
        VK_SNAPSHOT => Some("PrintScreen"),
        VK_INSERT => Some("Insert"),
        VK_DELETE => Some("Delete"),
        0x30 => Some("0"),
        0x31 => Some("1"),
        0x32 => Some("2"),
        0x33 => Some("3"),
        0x34 => Some("4"),
        0x35 => Some("5"),
        0x36 => Some("6"),
        0x37 => Some("7"),
        0x38 => Some("8"),
        0x39 => Some("9"),
        0x41 => Some("A"),
        0x42 => Some("B"),
        0x43 => Some("C"),
        0x44 => Some("D"),
        0x45 => Some("E"),
        0x46 => Some("F"),
        0x47 => Some("G"),
        0x48 => Some("H"),
        0x49 => Some("I"),
        0x4A => Some("J"),
        0x4B => Some("K"),
        0x4C => Some("L"),
        0x4D => Some("M"),
        0x4E => Some("N"),
        0x4F => Some("O"),
        0x50 => Some("P"),
        0x51 => Some("Q"),
        0x52 => Some("R"),
        0x53 => Some("S"),
        0x54 => Some("T"),
        0x55 => Some("U"),
        0x56 => Some("V"),
        0x57 => Some("W"),
        0x58 => Some("X"),
        0x59 => Some("Y"),
        0x5A => Some("Z"),
        VK_LWIN | VK_RWIN => Some("Win"),
        VK_APPS => Some("Apps"),
        0x60 => Some("Numpad 0"),
        0x61 => Some("Numpad 1"),
        0x62 => Some("Numpad 2"),
        0x63 => Some("Numpad 3"),
        0x64 => Some("Numpad 4"),
        0x65 => Some("Numpad 5"),
        0x66 => Some("Numpad 6"),
        0x67 => Some("Numpad 7"),
        0x68 => Some("Numpad 8"),
        0x69 => Some("Numpad 9"),
        0x6A => Some("Numpad *"),
        0x6B => Some("Numpad +"),
        0x6D => Some("Numpad -"),
        0x6E => Some("Numpad ."),
        0x6F => Some("Numpad /"),
        VK_F1 => Some("F1"),
        VK_F2 => Some("F2"),
        VK_F3 => Some("F3"),
        VK_F4 => Some("F4"),
        VK_F5 => Some("F5"),
        VK_F6 => Some("F6"),
        VK_F7 => Some("F7"),
        VK_F8 => Some("F8"),
        VK_F9 => Some("F9"),
        VK_F10 => Some("F10"),
        VK_F11 => Some("F11"),
        VK_F12 => Some("F12"),
        VK_F13 => Some("F13"),
        VK_F14 => Some("F14"),
        VK_F15 => Some("F15"),
        VK_F16 => Some("F16"),
        VK_F17 => Some("F17"),
        VK_F18 => Some("F18"),
        VK_F19 => Some("F19"),
        VK_F20 => Some("F20"),
        VK_F21 => Some("F21"),
        VK_F22 => Some("F22"),
        VK_F23 => Some("F23"),
        VK_F24 => Some("F24"),
        VK_NUMLOCK => Some("NumLock"),
        VK_SCROLL => Some("ScrollLock"),
        0xBA => Some(";"),
        0xBB => Some("+"),
        0xBC => Some(","),
        0xBD => Some("-"),
        0xBE => Some("."),
        0xBF => Some("/"),
        0xC0 => Some("`"),
        0xDB => Some("["),
        0xDC => Some("\\"),
        0xDD => Some("]"),
        0xDE => Some("'"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ctrl_space_variations() {
        let expected = HotkeyBinding {
            vk: VK_SPACE,
            ctrl: true,
            alt: false,
            shift: false,
            win: false,
        };

        let inputs = [
            "Ctrl+Space",
            "ctrl + space",
            "Ctrl+espaco",
            "ctrl+espaço",
            "CTRL - SPACE",
            "ctrl   space",
            "control+space",
            "strg + space",
        ];

        for input in inputs {
            let parsed = parse_hotkey(input);
            assert_eq!(
                parsed, expected,
                "Failed to parse variation '{}': expected {:?}, got {:?}",
                input, expected, parsed
            );
            assert_eq!(parsed.to_display_string(), "Ctrl + Space");
        }
    }

    #[test]
    fn test_parse_single_keys() {
        assert_eq!(
            parse_hotkey("F9"),
            HotkeyBinding {
                vk: VK_F9,
                ctrl: false,
                alt: false,
                shift: false,
                win: false,
            }
        );
        assert_eq!(parse_hotkey("F9").to_display_string(), "F9");

        assert_eq!(
            parse_hotkey("CapsLock"),
            HotkeyBinding {
                vk: VK_CAPITAL,
                ctrl: false,
                alt: false,
                shift: false,
                win: false,
            }
        );
        assert_eq!(parse_hotkey("CapsLock").to_display_string(), "CapsLock");
        assert_eq!(parse_hotkey("caps lock").to_display_string(), "CapsLock");

        assert_eq!(
            parse_hotkey("F13"),
            HotkeyBinding {
                vk: VK_F13,
                ctrl: false,
                alt: false,
                shift: false,
                win: false,
            }
        );
        assert_eq!(parse_hotkey("F13").to_display_string(), "F13");

        assert_eq!(
            parse_hotkey("ScrollLock"),
            HotkeyBinding {
                vk: VK_SCROLL,
                ctrl: false,
                alt: false,
                shift: false,
                win: false,
            }
        );
        assert_eq!(parse_hotkey("ScrollLock").to_display_string(), "ScrollLock");
        assert_eq!(parse_hotkey("scroll lock").to_display_string(), "ScrollLock");

        assert_eq!(parse_hotkey("Tab").vk, VK_TAB);
        assert_eq!(parse_hotkey("Tab").to_display_string(), "Tab");
        assert_eq!(parse_hotkey("Space").vk, VK_SPACE);
        assert_eq!(parse_hotkey("Space").to_display_string(), "Space");
    }

    #[test]
    fn test_parse_modifier_combinations() {
        // Alt+Space
        let alt_space = parse_hotkey("Alt+Space");
        assert_eq!(
            alt_space,
            HotkeyBinding {
                vk: VK_SPACE,
                ctrl: false,
                alt: true,
                shift: false,
                win: false,
            }
        );
        assert_eq!(alt_space.to_display_string(), "Alt + Space");

        // Shift+F9
        let shift_f9 = parse_hotkey("Shift+F9");
        assert_eq!(
            shift_f9,
            HotkeyBinding {
                vk: VK_F9,
                ctrl: false,
                alt: false,
                shift: true,
                win: false,
            }
        );
        assert_eq!(shift_f9.to_display_string(), "Shift + F9");

        // Ctrl+Shift+Space
        let ctrl_shift_space = parse_hotkey("Ctrl+Shift+Space");
        assert_eq!(
            ctrl_shift_space,
            HotkeyBinding {
                vk: VK_SPACE,
                ctrl: true,
                alt: false,
                shift: true,
                win: false,
            }
        );
        assert_eq!(ctrl_shift_space.to_display_string(), "Ctrl + Shift + Space");

        // Win / Super / Cmd
        let win_space = parse_hotkey("Win+Space");
        assert_eq!(win_space.win, true);
        assert_eq!(win_space.vk, VK_SPACE);
        assert_eq!(win_space.to_display_string(), "Win + Space");

        let super_space = parse_hotkey("super+space");
        assert_eq!(super_space, win_space);

        let cmd_space = parse_hotkey("cmd + space");
        assert_eq!(cmd_space, win_space);

        // Menu alias for Alt
        let menu_space = parse_hotkey("menu+space");
        assert_eq!(menu_space, alt_space);
    }

    #[test]
    fn test_f1_through_f24() {
        for n in 1..=24 {
            let str_f = format!("F{}", n);
            let parsed = parse_hotkey(&str_f);
            assert_eq!(parsed.vk, VK_F1 + (n - 1));
            assert_eq!(parsed.to_display_string(), str_f);
        }
    }

    #[test]
    fn test_is_modifier_released() {
        // Ctrl+Space
        let ctrl_space = parse_hotkey("Ctrl+Space");
        assert!(ctrl_space.is_modifier_released(VK_CONTROL));
        assert!(ctrl_space.is_modifier_released(VK_LCONTROL));
        assert!(ctrl_space.is_modifier_released(VK_RCONTROL));
        assert!(!ctrl_space.is_modifier_released(VK_MENU));
        assert!(!ctrl_space.is_modifier_released(VK_LMENU));
        assert!(!ctrl_space.is_modifier_released(VK_SHIFT));
        assert!(!ctrl_space.is_modifier_released(VK_LSHIFT));
        assert!(!ctrl_space.is_modifier_released(VK_LWIN));
        assert!(!ctrl_space.is_modifier_released(VK_SPACE));

        // Alt+Space
        let alt_space = parse_hotkey("Alt+Space");
        assert!(alt_space.is_modifier_released(VK_MENU));
        assert!(alt_space.is_modifier_released(VK_LMENU));
        assert!(alt_space.is_modifier_released(VK_RMENU));
        assert!(!alt_space.is_modifier_released(VK_CONTROL));
        assert!(!alt_space.is_modifier_released(VK_SHIFT));

        // Shift+F9
        let shift_f9 = parse_hotkey("Shift+F9");
        assert!(shift_f9.is_modifier_released(VK_SHIFT));
        assert!(shift_f9.is_modifier_released(VK_LSHIFT));
        assert!(shift_f9.is_modifier_released(VK_RSHIFT));
        assert!(!shift_f9.is_modifier_released(VK_CONTROL));
        assert!(!shift_f9.is_modifier_released(VK_MENU));
        assert!(!shift_f9.is_modifier_released(VK_F9));

        // No modifiers (e.g. F9)
        let f9 = parse_hotkey("F9");
        assert!(!f9.is_modifier_released(VK_CONTROL));
        assert!(!f9.is_modifier_released(VK_MENU));
        assert!(!f9.is_modifier_released(VK_SHIFT));
        assert!(!f9.is_modifier_released(VK_LWIN));
    }

    #[test]
    fn test_display_string_formatting() {
        assert_eq!(parse_hotkey("Ctrl+Space").to_display_string(), "Ctrl + Space");
        assert_eq!(parse_hotkey("Alt+F9").to_display_string(), "Alt + F9");
        assert_eq!(parse_hotkey("CapsLock").to_display_string(), "CapsLock");
        assert_eq!(parse_hotkey("Shift+F9").to_display_string(), "Shift + F9");
        assert_eq!(
            parse_hotkey("Ctrl+Shift+Space").to_display_string(),
            "Ctrl + Shift + Space"
        );
        assert_eq!(parse_hotkey("F13").to_display_string(), "F13");
        assert_eq!(parse_hotkey("ScrollLock").to_display_string(), "ScrollLock");
    }

    #[test]
    fn test_parentheses_and_edge_cases() {
        assert_eq!(parse_hotkey("F9 (Recomendado)").vk, VK_F9);
        assert_eq!(parse_hotkey("CapsLock (Fácil alcance)").vk, VK_CAPITAL);
        assert_eq!(parse_hotkey("Ctrl + Espaço (Recomendado)").vk, VK_SPACE);
        assert!(parse_hotkey("Ctrl + Espaço (Recomendado)").ctrl);

        // Empty string
        assert_eq!(parse_hotkey("").vk, VK_F9);
        assert_eq!(parse_hotkey("   ").vk, VK_F9);

        // Single modifier
        let ctrl_only = parse_hotkey("Ctrl");
        assert_eq!(ctrl_only.ctrl, true);
        assert_eq!(ctrl_only.vk, VK_CONTROL);
        assert_eq!(ctrl_only.to_display_string(), "Ctrl");

        let alt_only = parse_hotkey("Alt");
        assert_eq!(alt_only.alt, true);
        assert_eq!(alt_only.vk, VK_MENU);
        assert_eq!(alt_only.to_display_string(), "Alt");
    }

    #[test]
    fn test_serde_roundtrip() {
        let binding = parse_hotkey("Ctrl+Shift+Space");
        let json = serde_json::to_string(&binding).expect("serialize");
        let deserialized: HotkeyBinding = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(binding, deserialized);
    }
}
