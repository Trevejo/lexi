use crate::config::LlmConfig;
use crate::models::{LookupResult, LookupSource};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

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

// Anthropic Claude structures
#[derive(Serialize)]
struct ClaudeMessagesRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Deserialize)]
struct ClaudeMessagesResponse {
    content: Vec<ClaudeContentBlock>,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LlmOutputJson {
    #[serde(default)]
    pub term: Option<String>,
    pub translation: String,
    pub part_of_speech: Option<String>,
    pub definition: String,
    pub example: Option<String>,
}

pub struct LlmClient {
    config: LlmConfig,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    /// Tests the connection to the configured LLM provider and measures latency
    pub fn test_connection(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if self.config.api_key.trim().is_empty() && self.config.provider != "opencode" && self.config.provider != "custom" {
            return Err("A chave de API não foi informada.".into());
        }

        let start = Instant::now();
        let prompt = "Responda apenas com a palavra OK em JSON: {\"status\": \"OK\"}";

        let raw_res = self.send_prompt(
            "Você é um validador de API do Lexi. Responda estritamente em JSON.",
            prompt,
        )?;

        let elapsed = start.elapsed().as_millis();
        if raw_res.contains("OK") || raw_res.contains("ok") || raw_res.contains("status") {
            Ok(format!(
                "✓ Conexão com {} estabelecida com sucesso! ({}ms)",
                self.config.provider.to_uppercase(),
                elapsed
            ))
        } else {
            Ok(format!(
                "✓ Resposta recebida de {} em {}ms",
                self.config.provider.to_uppercase(),
                elapsed
            ))
        }
    }

    /// Performs dictionary lookup or voice clarification using the configured AI model
    pub fn lookup(&self, query: &str, normalized: &str) -> Result<LookupResult, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            return Err("LLM lookup is disabled in settings".into());
        }

        if self.config.api_key.trim().is_empty() && self.config.provider != "opencode" && self.config.provider != "custom" {
            return Err("LLM API key is not configured".into());
        }

        let system_prompt = r#"Você é o interpretador gamer e tradutor avançado do Lexi.
Para o termo, gíria de videogame ou fala do jogador fornecida:
1. Isole e corrija o termo principal em inglês pretendido (ex: remova ruídos de fala como 'o que é', 'mano', corrija 'cool down' -> 'cooldown', 'caitar' -> 'kite').
2. Forneça a tradução direta e explicação aprofundada de mecânica de jogo.
Responda ESTRITAMENTE em formato JSON com estes campos:
{
  "term": "termo correto em inglês (ex: cooldown)",
  "translation": "tradução direta em PT-BR (máximo 5 palavras)",
  "part_of_speech": "categoria gamer (ex: Mecânica de Jogo, Gíria MOBA, Tática FPS)",
  "definition": "explicação contextual clara e rica em PT-BR sobre o que é e como funciona nos jogos (1 a 3 frases)",
  "example": "exemplo prático de jogo em inglês com a tradução em português entre parênteses"
}
Não inclua nenhuma outra palavra além do JSON."#;

        let user_content = format!("Termo/Transcrição: \"{}\"", normalized);
        let raw_content = self.send_prompt(system_prompt, &user_content)?;
        let parsed: LlmOutputJson = parse_llm_json(&raw_content)?;

        let final_term = parsed.term.unwrap_or_else(|| normalized.to_string());

        Ok(LookupResult {
            query: query.to_string(),
            normalized: final_term,
            translation: parsed.translation,
            part_of_speech: parsed.part_of_speech,
            definition: parsed.definition,
            example: parsed.example,
            source: LookupSource::Llm,
        })
    }

    /// Voice Clarification & Translation: takes raw STT transcription and disambiguates user intent
    pub fn clarify_voice_transcript(&self, transcript: &str) -> Result<LookupResult, Box<dyn std::error::Error + Send + Sync>> {
        self.lookup(transcript, transcript)
    }

    /// Internal HTTP dispatcher supporting Claude, Gemini, OpenAI, OpenCode, and Custom providers
    fn send_prompt(&self, system_prompt: &str, user_content: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let timeout = Duration::from_secs(self.config.timeout_seconds.max(2));
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(timeout)
            .timeout_read(timeout)
            .build();

        let provider = self.config.provider.to_lowercase();

        if provider == "claude" || provider == "anthropic" {
            let endpoint = if self.config.base_url.is_empty() || self.config.base_url.contains("openai") {
                "https://api.anthropic.com/v1/messages".to_string()
            } else if self.config.base_url.ends_with("/messages") {
                self.config.base_url.clone()
            } else {
                format!("{}/messages", self.config.base_url.trim_end_matches('/'))
            };

            let req_body = ClaudeMessagesRequest {
                model: if self.config.model.is_empty() || self.config.model.starts_with("gpt") || self.config.model.starts_with("gemini") {
                    "claude-3-5-haiku-20241022".to_string()
                } else {
                    self.config.model.clone()
                },
                max_tokens: 1024,
                system: system_prompt.to_string(),
                messages: vec![ChatMessage {
                    role: "user".to_string(),
                    content: user_content.to_string(),
                }],
                temperature: 0.2,
            };

            let resp = agent.post(&endpoint)
                .set("Content-Type", "application/json")
                .set("x-api-key", self.config.api_key.trim())
                .set("anthropic-version", "2023-06-01")
                .send_json(&req_body)?;

            let claude_resp: ClaudeMessagesResponse = resp.into_json()?;
            let text = claude_resp.content
                .into_iter()
                .filter(|c| c.content_type == "text" || !c.text.is_empty())
                .map(|c| c.text)
                .collect::<Vec<_>>()
                .join("");

            if text.is_empty() {
                return Err("Empty response content from Claude".into());
            }
            return Ok(text);
        }

        // Gemini (using official OpenAI-compatible endpoint or Google Generative Language)
        let endpoint = if provider == "gemini" || provider == "google" {
            if self.config.base_url.is_empty() || self.config.base_url == "https://api.openai.com/v1" {
                "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions".to_string()
            } else if self.config.base_url.ends_with("/chat/completions") {
                self.config.base_url.clone()
            } else {
                format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'))
            }
        } else if provider == "openai" {
            if self.config.base_url.is_empty() {
                "https://api.openai.com/v1/chat/completions".to_string()
            } else if self.config.base_url.ends_with("/chat/completions") {
                self.config.base_url.clone()
            } else {
                format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'))
            }
        } else if provider == "opencode" {
            if self.config.base_url.is_empty() {
                "http://127.0.0.1:8317/v1/chat/completions".to_string()
            } else if self.config.base_url.ends_with("/chat/completions") {
                self.config.base_url.clone()
            } else {
                format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'))
            }
        } else {
            // Custom or OpenRouter or Ollama
            if self.config.base_url.ends_with("/chat/completions") {
                self.config.base_url.clone()
            } else {
                format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'))
            }
        };

        let effective_model = if provider == "gemini" && (self.config.model.is_empty() || self.config.model.starts_with("gpt")) {
            "gemini-1.5-flash".to_string()
        } else if provider == "openai" && (self.config.model.is_empty() || self.config.model.starts_with("gemini")) {
            "gpt-4o-mini".to_string()
        } else if self.config.model.is_empty() {
            "gpt-4o-mini".to_string()
        } else {
            self.config.model.clone()
        };

        let request_payload = ChatCompletionRequest {
            model: effective_model,
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_content.to_string(),
                },
            ],
            temperature: 0.2,
            response_format: Some(ResponseFormat {
                format_type: "json_object".to_string(),
            }),
        };

        let mut req = agent.post(&endpoint)
            .set("Content-Type", "application/json");

        if !self.config.api_key.trim().is_empty() {
            req = req.set("Authorization", &format!("Bearer {}", self.config.api_key.trim()));
        }

        let response = req.send_json(&request_payload)?;
        let response_body: ChatCompletionResponse = response.into_json()?;

        let raw_content = response_body
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or("No response choice from LLM")?;

        Ok(raw_content)
    }
}

pub fn parse_llm_json(raw: &str) -> Result<LlmOutputJson, Box<dyn std::error::Error + Send + Sync>> {
    let trimmed = raw.trim();
    let json_str = if let Some(stripped) = trimmed.strip_prefix("```json") {
        stripped.strip_suffix("```").unwrap_or(stripped).trim()
    } else if let Some(stripped) = trimmed.strip_prefix("```") {
        stripped.strip_suffix("```").unwrap_or(stripped).trim()
    } else {
        trimmed
    };

    // If there is surrounding text before the first '{', find it
    let start_idx = json_str.find('{').unwrap_or(0);
    let end_idx = json_str.rfind('}').map(|i| i + 1).unwrap_or(json_str.len());
    let slice = &json_str[start_idx..end_idx];

    let result: LlmOutputJson = serde_json::from_str(slice)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_llm_json_standard() {
        let raw = r#"{"term": "cooldown", "translation": "Tempo de recarga", "part_of_speech": "Mecânica", "definition": "Tempo de espera antes de reusar habilidade.", "example": "Flash is on cooldown (Flash em recarga)"}"#;
        let parsed = parse_llm_json(raw).expect("parse json");
        assert_eq!(parsed.term.as_deref(), Some("cooldown"));
        assert_eq!(parsed.translation, "Tempo de recarga");
    }

    #[test]
    fn test_parse_llm_json_with_markdown_fences() {
        let raw = "```json\n{\n  \"translation\": \"Golpe surpresa\",\n  \"part_of_speech\": \"Gíria MOBA\",\n  \"definition\": \"Ataque surpresa\",\n  \"example\": \"Gank mid\"\n}\n```";
        let parsed = parse_llm_json(raw).expect("parse markdown json");
        assert_eq!(parsed.translation, "Golpe surpresa");
        assert_eq!(parsed.term, None);
    }
}
