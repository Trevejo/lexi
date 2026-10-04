use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LookupSource {
    Cache,
    OfflineDictionary,
    Llm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupResult {
    pub query: String,
    pub normalized: String,
    pub translation: String,
    pub part_of_speech: Option<String>,
    pub definition: String,
    pub example: Option<String>,
    pub source: LookupSource,
}

impl LookupResult {
    pub fn empty(query: &str) -> Self {
        Self {
            query: query.to_string(),
            normalized: query.to_string(),
            translation: "Não encontrado".to_string(),
            part_of_speech: None,
            definition: "Nenhuma definição encontrada no dicionário offline ou IA.".to_string(),
            example: None,
            source: LookupSource::OfflineDictionary,
        }
    }
}
