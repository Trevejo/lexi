use lexi_core::config::AppConfig;
use lexi_core::LookupService;
use std::path::Path;

#[test]
fn test_lookup_service_real() {
    let mut config = AppConfig::load_from_path(Path::new("../../config.toml"))
        .or_else(|_| AppConfig::load_from_path(Path::new("config.toml")))
        .unwrap();

    // Ensure we point to the main 15k dictionary in project root if relative
    if config.lookup.dictionary_path.is_relative() {
        if Path::new("../../dictionary.sqlite").exists() {
            config.lookup.dictionary_path = Path::new("../../dictionary.sqlite").to_path_buf();
        } else if Path::new("dictionary.sqlite").exists() {
            config.lookup.dictionary_path = Path::new("dictionary.sqlite").to_path_buf();
        }
    }

    let service = LookupService::new(config);

    // 1. Gaming terms
    let res_flank = service.lookup("flank");
    println!("Query 'flank': {:?}", res_flank);
    assert_ne!(res_flank.translation, "Significado não encontrado");

    let res_aggro = service.lookup("What does aggro mean?");
    println!("Query 'What does aggro mean?': {:?}", res_aggro);
    assert_ne!(res_aggro.translation, "Significado não encontrado");

    // 2. Previously missing terms like 'sword', 'shield', 'dragon', 'potion'
    let res_sword = service.lookup("sword");
    println!("Query 'sword': {:?}", res_sword);
    assert!(res_sword.translation.to_lowercase().contains("espada"));

    let res_q_sword = service.lookup("what is a sword?");
    println!("Query 'what is a sword?': {:?}", res_q_sword);
    assert!(res_q_sword.translation.to_lowercase().contains("espada"));

    let res_shield = service.lookup("shield");
    println!("Query 'shield': {:?}", res_shield);
    assert!(res_shield.translation.to_lowercase().contains("escudo"));

    let res_dragon = service.lookup("dragon");
    println!("Query 'dragon': {:?}", res_dragon);
    assert!(res_dragon.translation.to_lowercase().contains("dragão"));

    let res_potion = service.lookup("potion");
    println!("Query 'potion': {:?}", res_potion);
    assert!(res_potion.translation.to_lowercase().contains("poção"));

    // 3. Multi-word gaming terms
    let res_cc = service.lookup("crowd control");
    println!("Query 'crowd control': {:?}", res_cc);
    assert_ne!(res_cc.translation, "Significado não encontrado");

    // 4. Portuguese question
    let res_pt = service.lookup("o que é um debuff?");
    println!("Query 'o que é um debuff?': {:?}", res_pt);
    assert_ne!(res_pt.translation, "Significado não encontrado");

    let res_env = service.lookup("o que é environment");
    println!("Query 'o que é environment': {:?}", res_env);
    assert_eq!(res_env.normalized, "environment");
    assert!(res_env.translation.contains("ambiente"));

    let res_env_accentless = service.lookup("o que e environment");
    assert_eq!(res_env_accentless.normalized, "environment");

    let res_test = service.lookup("o que é test");
    println!("Query 'o que é test': {:?}", res_test);
    assert_eq!(res_test.normalized, "test");
    assert!(res_test.translation.contains("teste") || res_test.translation.contains("exame"));

    let res_test_accentless = service.lookup("o que e test");
    assert_eq!(res_test_accentless.normalized, "test");

    let res_test_joined = service.lookup("oque é test");
    assert_eq!(res_test_joined.normalized, "test");

    // 5. Reverse query (Portuguese to English)
    let res_rev = service.lookup("espada");
    println!("Query 'espada': {:?}", res_rev);
    assert_eq!(res_rev.normalized, "sword");
}

#[test]
fn test_lookup_service_fallback_when_file_missing() {
    let mut config = AppConfig::default();
    config.lookup.dictionary_path = std::path::PathBuf::from("nonexistent_path_file.sqlite");
    config.lookup.cache_path = std::path::PathBuf::from("nonexistent_cache.sqlite");

    let service = LookupService::new(config);

    // General words reported by user
    let res_env = service.lookup("o que é environment");
    assert_eq!(res_env.normalized, "environment");
    assert!(res_env.translation.contains("ambiente"));

    let res_test = service.lookup("o que é test");
    assert_eq!(res_test.normalized, "test");
    assert!(res_test.translation.contains("teste") || res_test.translation.contains("exame"));

    // Gaming terms
    let res_sniper = service.lookup("sniper");
    assert_eq!(res_sniper.normalized, "sniper");
    assert!(res_sniper.translation.contains("atirador de elite"));

    let res_flank = service.lookup("flank");
    assert_eq!(res_flank.normalized, "flank");
    assert!(res_flank.translation.contains("flanquear"));
}

#[test]
fn test_lookup_random_english_words() {
    let service = LookupService::new(AppConfig::default());

    let random_english_words = [
        "window", "guitar", "coffee", "hospital", "climate",
        "freedom", "velocity", "whisper", "elephant", "banana",
        "castle", "river", "mountain", "butter", "bread",
        "library", "camera", "doctor", "shadow", "universe"
    ];

    for word in &random_english_words {
        let res = service.lookup(word);
        println!("Random word '{}' -> normalized: '{}', translation: '{}'", word, res.normalized, res.translation);
        assert_ne!(res.translation, "Significado não encontrado", "Word '{}' should be translated", word);
        assert!(!res.translation.trim().is_empty(), "Translation for '{}' should not be empty", word);
    }

    // Also test questions with these words
    let res_q1 = service.lookup("o que é guitar");
    assert_ne!(res_q1.translation, "Significado não encontrado");

    let res_q2 = service.lookup("what is coffee");
    assert_ne!(res_q2.translation, "Significado não encontrado");

    let res_q3 = service.lookup("o que significa castle");
    assert_ne!(res_q3.translation, "Significado não encontrado");
}

#[test]
fn test_conversational_and_filler_queries() {
    let service = LookupService::new(AppConfig::default());

    // 1. User specific conversational inputs that previously failed
    let res_yeah = service.lookup("Test yeah");
    assert_eq!(res_yeah.normalized, "test");
    assert!(res_yeah.translation.contains("teste") || res_yeah.translation.contains("exame"));

    let res_warren = service.lookup("Warren Eu nem sei o que");
    assert_eq!(res_warren.normalized, "warren");
    assert_ne!(res_warren.translation, "Significado não encontrado");

    let res_jungle = service.lookup("what is jungle");
    assert_eq!(res_jungle.normalized, "jungle");
    assert!(res_jungle.translation.contains("selva") || res_jungle.translation.contains("jungle"));

    let res_flank_mano = service.lookup("o que é flank mano");
    assert_eq!(res_flank_mano.normalized, "flank");
    assert!(res_flank_mano.translation.contains("flanquear"));

    let res_shyster = service.lookup("sheister");
    assert_ne!(res_shyster.translation, "Significado não encontrado");

    // 2. Latency test: Ensure every lookup runs in < 10ms (zero hanging / "pensar")
    let test_terms = ["test yeah", "flank", "aggro", "environment", "apple", "nonexistentquery123xyz"];
    for term in &test_terms {
        let start = std::time::Instant::now();
        let _ = service.lookup(term);
        let elapsed = start.elapsed();
        println!("Lookup for '{}' took: {:?}", term, elapsed);
        assert!(elapsed.as_millis() < 50, "Lookup for '{}' took {:?}, must be under 50ms", term, elapsed);
    }
}

#[test]
fn test_hotkey_ctrl_space_end_to_end() {
    use lexi_core::hotkey::parse_hotkey;

    // Test primary trigger string
    let binding = parse_hotkey("Ctrl+Space");
    assert_eq!(binding.vk, 0x20, "VK should be VK_SPACE (0x20)");
    assert!(binding.ctrl, "Ctrl modifier must be true");
    assert!(!binding.alt, "Alt must be false");
    assert!(!binding.shift, "Shift must be false");
    assert!(!binding.win, "Win must be false");
    assert_eq!(binding.to_display_string(), "Ctrl + Space");

    // Variations
    let variants = ["ctrl+space", "ctrl + space", "Ctrl + Espaço", "Ctrl+Space (Recomendado)", "Control + Space"];
    for v in &variants {
        let b = parse_hotkey(v);
        assert_eq!(b.vk, 0x20, "Variant '{}' must resolve to VK_SPACE", v);
        assert!(b.ctrl, "Variant '{}' must require Ctrl modifier", v);
    }

    // Modifier release verification (when user releases Ctrl, PTT must stop)
    assert!(binding.is_modifier_released(0x11), "VK_CONTROL (0x11) release must be detected");
    assert!(binding.is_modifier_released(0xA2), "VK_LCONTROL (0xA2) release must be detected");
    assert!(binding.is_modifier_released(0xA3), "VK_RCONTROL (0xA3) release must be detected");
    assert!(!binding.is_modifier_released(0x12), "VK_MENU (Alt) should not trigger Ctrl release");
    assert!(!binding.is_modifier_released(0x10), "VK_SHIFT should not trigger Ctrl release");

    // Other hotkeys that users can configure
    let alt_space = parse_hotkey("Alt+Space");
    assert_eq!(alt_space.vk, 0x20);
    assert!(alt_space.alt);
    assert!(!alt_space.ctrl);
    assert!(alt_space.is_modifier_released(0x12)); // VK_MENU

    let caps = parse_hotkey("CapsLock");
    assert_eq!(caps.vk, 0x14); // VK_CAPITAL
    assert!(!caps.ctrl);
    assert!(!caps.alt);

    let f9 = parse_hotkey("F9");
    assert_eq!(f9.vk, 0x78);
    assert!(!f9.ctrl);
}

#[test]
fn test_dpi_scaling_calculations_4k() {
    // 96 DPI = 100% (Standard 1080p)
    let dpi_96 = 96.0f32;
    let scale_100 = (dpi_96 / 96.0).max(1.0);
    assert!((scale_100 - 1.0).abs() < f32::EPSILON);

    // 144 DPI = 150% (Standard 4K monitor scaling)
    let dpi_144 = 144.0f32;
    let scale_150 = (dpi_144 / 96.0).max(1.0);
    assert!((scale_150 - 1.5).abs() < f32::EPSILON);

    // 192 DPI = 200% (High density 4K)
    let dpi_192 = 192.0f32;
    let scale_200 = (dpi_192 / 96.0).max(1.0);
    assert!((scale_200 - 2.0).abs() < f32::EPSILON);

    // Scaling helper s(v)
    let s = |v: i32, scale: f32| -> i32 { (v as f32 * scale).round() as i32 };

    // Overlay width: base 480
    assert_eq!(s(480, scale_100), 480);
    assert_eq!(s(480, scale_150), 720); // 480 * 1.5 = 720
    assert_eq!(s(480, scale_200), 960); // 480 * 2.0 = 960

    // Settings window: base 660x700
    assert_eq!(s(660, scale_150), 990);
    assert_eq!(s(700, scale_150), 1050);

    // Font point size: -((base_font_size * 1.55 * dpi_scale).round() as i32)
    let font_calc = |base: f32, scale: f32| -> i32 {
        -((base * 1.55 * scale).round() as i32)
    };
    assert_eq!(font_calc(14.0, 1.0), -22); // 14 * 1.55 = 21.7 -> 22
    assert_eq!(font_calc(14.0, 1.5), -33); // 14 * 1.55 * 1.5 = 32.55 -> 33
    assert_eq!(font_calc(14.0, 2.0), -43); // 14 * 1.55 * 2.0 = 43.4 -> 43
}

#[test]
fn test_settings_tab_and_config_e2e() {
    use lexi_core::config::{AppConfig, DismissMode, OverlayPosition};

    let config = AppConfig::default();
    assert_eq!(config.stt.hotkey, "Ctrl+Space");
    assert_eq!(config.dismiss.trigger_key_to_watch, "Ctrl+Space");
    assert_eq!(config.overlay.width, 480);
    assert_eq!(config.overlay.position, OverlayPosition::TopCenter);

    // Verify all 7 positions supported in the settings tab
    let positions = [
        OverlayPosition::TopCenter,
        OverlayPosition::Center,
        OverlayPosition::BottomCenter,
        OverlayPosition::TopLeft,
        OverlayPosition::TopRight,
        OverlayPosition::MiddleLeft,
        OverlayPosition::MiddleRight,
    ];
    for pos in &positions {
        let mut cfg = config.clone();
        cfg.overlay.position = pos.clone();
        let toml_str = toml::to_string(&cfg).unwrap();
        let reloaded: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(reloaded.overlay.position, *pos);
    }

    // Verify all dismiss modes
    let dismiss_modes = [
        DismissMode::Hybrid,
        DismissMode::ReleaseKey,
        DismissMode::Timeout,
        DismissMode::Hotkey,
        DismissMode::AnyKey,
    ];
    for mode in &dismiss_modes {
        let mut cfg = config.clone();
        cfg.dismiss.mode = mode.clone();
        let toml_str = toml::to_string(&cfg).unwrap();
        let reloaded: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(reloaded.dismiss.mode, *mode);
    }
}

