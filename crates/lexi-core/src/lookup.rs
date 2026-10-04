use crate::cache::Cache;
use crate::config::{AppConfig, LookupMode};
use crate::dictionary::Dictionary;
use crate::llm::LlmClient;
use crate::models::{LookupResult, LookupSource};
use crate::normalize::normalize_query;
use std::sync::{Arc, Mutex};

pub struct LookupService {
    config: AppConfig,
    dict: Option<Arc<Mutex<Dictionary>>>,
    cache: Option<Arc<Mutex<Cache>>>,
    llm: Option<LlmClient>,
}

impl LookupService {
    pub fn new(config: AppConfig) -> Self {
        let dict = if config.lookup.mode != LookupMode::LlmOnly {
            log::info!("Opening dictionary at: {:?}", config.lookup.dictionary_path);
            let mut opened_dict = Dictionary::open(&config.lookup.dictionary_path).ok();

            // Try candidate relative paths if default failed
            if opened_dict.is_none() {
                let candidates = [
                    std::path::PathBuf::from("dictionary.sqlite"),
                    std::path::PathBuf::from("../dictionary.sqlite"),
                    std::path::PathBuf::from("../../dictionary.sqlite"),
                ];
                for p in &candidates {
                    if p.exists() {
                        if let Ok(d) = Dictionary::open(p) {
                            log::info!("Opened dictionary from candidate path: {:?}", p);
                            opened_dict = Some(d);
                            break;
                        }
                    }
                }
            }

            // Try next to running executable
            if opened_dict.is_none() {
                if let Ok(exe) = std::env::current_exe() {
                    if let Some(exe_dir) = exe.parent() {
                        let p = exe_dir.join("dictionary.sqlite");
                        if p.exists() {
                            opened_dict = Dictionary::open(&p).ok();
                            if opened_dict.is_some() {
                                log::info!("Opened dictionary next to exe: {:?}", p);
                            }
                        }
                        if opened_dict.is_none() {
                            let mut curr = exe_dir;
                            for _ in 0..3 {
                                if let Some(parent) = curr.parent() {
                                    let p = parent.join("dictionary.sqlite");
                                    if p.exists() {
                                        opened_dict = Dictionary::open(&p).ok();
                                        if opened_dict.is_some() {
                                            log::info!("Opened dictionary in parent dir: {:?}", p);
                                            break;
                                        }
                                    }
                                    curr = parent;
                                }
                            }
                        }
                    }
                }
            }

            // Ensure opened dictionary actually has entries; fallback to in-memory if empty
            let has_entries = opened_dict.as_ref().map_or(0, |d| d.entry_count()) > 0;
            if !has_entries {
                log::warn!("On-disk dictionary could not be opened or has 0 entries; initializing in-memory dictionary with bundled seeds");
                opened_dict = Dictionary::in_memory().ok();
            }

            if let Some(ref d) = opened_dict {
                log::info!("Dictionary successfully ready with {} entries", d.entry_count());
            } else {
                log::error!("CRITICAL: Failed to initialize even in-memory dictionary!");
            }

            opened_dict.map(|d| Arc::new(Mutex::new(d)))
        } else {
            None
        };

        let cache = Cache::open(&config.lookup.cache_path)
            .or_else(|_| Cache::in_memory())
            .map(|c| Arc::new(Mutex::new(c)))
            .ok();

        let llm = if config.lookup.mode != LookupMode::OfflineOnly 
            && config.llm.enabled 
            && !config.llm.api_key.trim().is_empty() 
        {
            Some(LlmClient::new(config.llm.clone()))
        } else {
            None
        };


        Self {
            config,
            dict,
            cache,
            llm,
        }
    }

    pub fn lookup(&self, raw_query: &str) -> LookupResult {
        let normalized = normalize_query(raw_query);
        if normalized.is_empty() {
            return LookupResult::empty(raw_query);
        }

        // 1. Check cache first
        if let Some(ref cache_lock) = self.cache {
            if let Ok(cache) = cache_lock.lock() {
                if let Some(mut cached) = cache.get(&normalized) {
                    cached.query = raw_query.to_string();
                    log::info!("Lookup for '{}' served from cache", normalized);
                    return cached;
                }
            }
        }

        // 2. Check offline dictionary (if enabled)
        if self.config.lookup.mode != LookupMode::LlmOnly {
            if let Some(ref dict_lock) = self.dict {
                if let Ok(dict) = dict_lock.lock() {
                    if let Some(res) = dict.lookup(&normalized) {
                        log::info!("Lookup for '{}' found in offline dictionary", normalized);
                        // Store to cache
                        self.store_cache(&res);
                        return res;
                    }

                    // 2b. Smart token fallback for multi-word conversational queries:
                    // When speech recognition captures extra filler words or conversational clauses,
                    // filter out stop words and search for substantive keywords and compound terms.
                    if normalized.contains(' ') {
                        let words: Vec<&str> = normalized.split_whitespace().collect();
                        let substantive: Vec<&str> = words
                            .iter()
                            .copied()
                            .filter(|w| !is_stop_word(w) && w.len() >= 2)
                            .collect();

                        // Check 2-word pairs first (e.g. compound terms like "choke point")
                        for pair in substantive.windows(2) {
                            let compound = format!("{} {}", pair[0], pair[1]);
                            if let Some(mut res) = dict.lookup(&compound) {
                                res.query = raw_query.to_string();
                                log::info!("Lookup for '{}' resolved via multi-word compound '{}'", raw_query, compound);
                                self.store_cache(&res);
                                return res;
                            }
                        }

                        // Check individual substantive words (e.g. "warren", "test", "target")
                        for word in &substantive {
                            if let Some(mut res) = dict.lookup(word) {
                                res.query = raw_query.to_string();
                                log::info!("Lookup for '{}' resolved via token fallback '{}'", raw_query, word);
                                self.store_cache(&res);
                                return res;
                            }
                        }
                    }
                }
            }
        }

        // 3. Fallback to LLM (if enabled and offline had no hit, or if multi-word query)
        if let Some(ref llm) = self.llm {
            log::info!("Querying LLM for '{}'", normalized);
            match llm.lookup(raw_query, &normalized) {
                Ok(llm_res) => {
                    log::info!("LLM lookup succeeded for '{}'", normalized);
                    self.store_cache(&llm_res);
                    return llm_res;
                }
                Err(err) => {
                    log::warn!("LLM lookup failed for '{}': {}", normalized, err);
                }
            }
        }

        // 4. Default empty result
        LookupResult {
            query: raw_query.to_string(),
            normalized: normalized.clone(),
            translation: "Significado não encontrado".to_string(),
            part_of_speech: None,
            definition: format!("O termo \"{}\" não foi encontrado no dicionário offline.", normalized),
            example: None,
            source: LookupSource::OfflineDictionary,
        }
    }

    fn store_cache(&self, res: &LookupResult) {
        if let Some(ref cache_lock) = self.cache {
            if let Ok(cache) = cache_lock.lock() {
                if let Err(e) = cache.set(res) {
                    log::warn!("Failed to store query into cache: {e}");
                }
            }
        }
    }
}

fn is_stop_word(w: &str) -> bool {
    matches!(
        w.to_lowercase().as_str(),
        // English
        "a" | "an" | "the" | "in" | "on" | "at" | "to" | "for" | "with" | "from" | "of" | "by"
        | "about" | "as" | "into" | "like" | "through" | "after" | "over" | "between" | "out"
        | "is" | "are" | "was" | "were" | "be" | "been" | "being" | "have" | "has" | "had"
        | "do" | "does" | "did" | "can" | "could" | "will" | "would" | "shall" | "should"
        | "it" | "its" | "this" | "that" | "these" | "those" | "what" | "which" | "who"
        | "how" | "why" | "when" | "where" | "all" | "any" | "both" | "each" | "few" | "more"
        | "and" | "or" | "but" | "not" | "yeah" | "yes" | "ok" | "okay" | "just" | "so"
        // Portuguese
        | "o" | "os" | "um" | "uma" | "uns" | "umas" | "de" | "da" | "dos" | "das"
        | "em" | "no" | "na" | "nos" | "nas" | "por" | "pra" | "pro" | "para" | "pelo" | "pela"
        | "com" | "sem" | "sob" | "sobre" | "que" | "oq" | "e" | "é" | "eh" | "se" | "me" | "te"
        | "eu" | "tu" | "ele" | "ela" | "nós" | "vós" | "eles" | "elas" | "meu" | "minha" | "seu" | "sua"
        | "nem" | "sei" | "sabe" | "tipo" | "cara" | "mano" | "né" | "ne" | "então" | "entao"
        | "aí" | "ai" | "ou" | "mas" | "já" | "ja" | "não" | "nao" | "como" | "quando" | "onde"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_service() {
        let config = AppConfig::default();
        let service = LookupService::new(config);
        let res = service.lookup("flank");
        assert_eq!(res.normalized, "flank");
        assert!(res.translation.contains("flanquear"));
        let res2 = service.lookup("environment");
        assert_eq!(res2.normalized, "environment");
        assert!(res2.translation.contains("ambiente"));
    }

    #[test]
    fn test_multi_word_token_fallback() {
        let config = AppConfig::default();
        let service = LookupService::new(config);
        let res_yeah = service.lookup("Test yeah");
        assert_eq!(res_yeah.normalized, "test");
        assert!(res_yeah.translation.contains("teste") || res_yeah.translation.contains("exame"));

        let res_warren = service.lookup("Warren Eu nem sei o que");
        assert_eq!(res_warren.normalized, "warren");
        assert_ne!(res_warren.translation, "Significado não encontrado");
    }
}

