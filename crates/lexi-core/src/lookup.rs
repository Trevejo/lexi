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
}

