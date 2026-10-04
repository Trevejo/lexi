use crate::config::LlmConfig;
use crate::models::{LookupResult, LookupSource};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct LlmOutputJson {
    translation: String,
    part_of_speech: Option<String>,
    definition: String,
    example: Option<String>,
}

pub struct LlmClient {
    config: LlmConfig,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    pub fn lookup(&self, query: &str, normalized: &str) -> Result<LookupResult, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled || self.config.api_key.trim().is_empty() {
            return Err("LLM lookup is disabled or API key is not configured".into());
        }

        let system_prompt = r#"Você é um tradutor gamer e dicionário ultra-conciso de Inglês para Português do Brasil.
Para o termo, expressão, gíria gamer ou frase fornecida, responda ESTRITAMENTE em formato JSON com os seguintes campos:
{
  "translation": "tradução direta em PT-BR (máximo 5 palavras)",
  "part_of_speech": "classe (ex: verbo, gíria gamer, substantivo)",
  "definition": "explicação curta e clara em PT-BR no contexto de jogos (1 ou 2 frases)",
  "example": "frase de exemplo em inglês com a tradução entre parênteses"
}
Não inclua nenhuma outra palavra além do JSON."#;

        let user_content = format!("Termo/Frase: \"{}\"", normalized);

        let endpoint = if self.config.base_url.ends_with("/chat/completions") {
            self.config.base_url.clone()
        } else {
            format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'))
        };

        let request_payload = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_content,
                },
            ],
            temperature: 0.2,
            response_format: Some(ResponseFormat {
                format_type: "json_object".to_string(),
            }),
        };

        let timeout = Duration::from_secs(self.config.timeout_seconds.max(1));
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(timeout)
            .timeout_read(timeout)
            .build();

        let mut req = agent.post(&endpoint)
            .set("Content-Type", "application/json");

        if !self.config.api_key.is_empty() {
            req = req.set("Authorization", &format!("Bearer {}", self.config.api_key.trim()));
        }

        let response = req.send_json(&request_payload)?;
        let response_body: ChatCompletionResponse = response.into_json()?;

        let raw_content = response_body
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or("No response choice from LLM")?;

        let parsed: LlmOutputJson = parse_llm_json(&raw_content)?;

        Ok(LookupResult {
            query: query.to_string(),
            normalized: normalized.to_string(),
            translation: parsed.translation,
            part_of_speech: parsed.part_of_speech,
            definition: parsed.definition,
            example: parsed.example,
            source: LookupSource::Llm,
        })
    }
}

fn parse_llm_json(raw: &str) -> Result<LlmOutputJson, Box<dyn std::error::Error + Send + Sync>> {
    let trimmed = raw.trim();
    // Strip markdown fences ```json ... ``` if model outputted them
    let json_str = if let Some(stripped) = trimmed.strip_prefix("```json") {
        stripped.strip_suffix("```").unwrap_or(stripped).trim()
    } else if let Some(stripped) = trimmed.strip_prefix("```") {
        stripped.strip_suffix("```").unwrap_or(stripped).trim()
    } else {
        trimmed
    };

    let result: LlmOutputJson = serde_json::from_str(json_str)?;
    Ok(result)
}
