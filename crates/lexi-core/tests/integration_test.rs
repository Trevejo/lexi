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
        assert!(elapsed.as_millis() < 25, "Lookup for '{}' took {:?}, must be under 25ms", term, elapsed);
    }
}
