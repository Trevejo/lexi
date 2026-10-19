use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use lexi_core::config::AppConfig;
use lexi_core::llm::LlmClient;
use lexi_core::lookup::LookupService;
use lexi_core::models::LookupResult;
use lexi_core::stt_models::AudioRecorder;

pub struct AudioService {
    recorder: AudioRecorder,
    is_active: Arc<AtomicBool>,
}

impl Default for AudioService {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioService {
    pub fn new() -> Self {
        Self {
            recorder: AudioRecorder::new(),
            is_active: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.recorder.is_recording()
    }

    pub fn start_capture(&self, config: &AppConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_recording() {
            return Ok(());
        }

        self.is_active.store(true, Ordering::SeqCst);
        let dev = config.stt.input_device.as_deref();
        self.recorder.start_recording(dev)
    }

    pub fn stop_and_process<F>(&self, config: &AppConfig, lookup_service: Arc<LookupService>, on_result: F)
    where
        F: FnOnce(LookupResult) + Send + 'static,
    {
        if !self.is_active.swap(false, Ordering::SeqCst) {
            return;
        }

        let wav_result = self.recorder.stop_recording();
        let cfg = config.clone();

        thread::spawn(move || {
            let wav_bytes = match wav_result {
                Ok(bytes) => bytes,
                Err(err) => {
                    log::warn!("Audio capture yielded no bytes: {err}");
                    return;
                }
            };

            // WAV header is 44 bytes; require at least 150ms of audio (~4800 bytes)
            if wav_bytes.len() < 4000 {
                log::debug!("Recorded audio buffer too short ({}), skipping", wav_bytes.len());
                return;
            }

            log::info!("Audio capture completed: {} bytes recorded. Running STT pipeline...", wav_bytes.len());

            // 1. STT Speech-to-Text Transcription
            let transcript_opt = transcribe_audio(&wav_bytes, &cfg);
            let transcript = match transcript_opt {
                Some(t) if !t.trim().is_empty() => t.trim().to_string(),
                _ => {
                    log::info!("No clear voice detected in audio sample");
                    return;
                }
            };

            log::info!("Voice transcription recognized: '{}'", transcript);

            // 2. Voice Clarification & Translation (External AI or Offline Dictionary)
            let result = if cfg.llm.enabled && !cfg.llm.api_key.trim().is_empty() {
                let client = LlmClient::new(cfg.llm.clone());
                match client.clarify_voice_transcript(&transcript) {
                    Ok(r) => {
                        log::info!("AI Voice Clarifier succeeded for '{}': {} ({:?})", transcript, r.translation, r.part_of_speech);
                        r
                    }
                    Err(e) => {
                        log::warn!("AI Clarifier failed ({}), falling back to local dictionary", e);
                        lookup_service.lookup(&transcript)
                    }
                }
            } else {
                lookup_service.lookup(&transcript)
            };

            on_result(result);
        });
    }
}

/// Transcribes audio bytes using external Whisper API, Gemini Audio, or local speech runner
fn transcribe_audio(wav_bytes: &[u8], config: &AppConfig) -> Option<String> {
    // A. If external LLM is OpenAI or Groq with API key, use Whisper API
    if config.llm.enabled && !config.llm.api_key.trim().is_empty() {
        if config.llm.provider == "openai" || config.llm.base_url.contains("groq") {
            let endpoint = if config.llm.base_url.contains("groq") {
                "https://api.groq.com/openai/v1/audio/transcriptions"
            } else {
                "https://api.openai.com/v1/audio/transcriptions"
            };

            let model = if config.llm.base_url.contains("groq") {
                "whisper-large-v3-turbo"
            } else {
                "whisper-1"
            };

            if let Ok(text) = call_whisper_multipart_api(endpoint, model, &config.llm.api_key, wav_bytes) {
                return Some(text);
            }
        }

        // B. If Google Gemini, send audio to Gemini for instant transcription
        if config.llm.provider == "gemini" {
            if let Ok(text) = call_gemini_audio_transcription(&config.llm.api_key, wav_bytes) {
                return Some(text);
            }
        }
    }

    // C. Local fallback / mock for offline
    None
}

/// Sends audio WAV to OpenAI or Groq Whisper API via multipart form-data
fn call_whisper_multipart_api(
    endpoint: &str,
    model: &str,
    api_key: &str,
    wav_bytes: &[u8],
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let boundary = "----LexiFormBoundary7MA4YWxkTrZu0gW";
    let mut body = Vec::new();

    // Field: model
    body.extend(format!("--{}\r\n", boundary).as_bytes());
    body.extend(b"Content-Disposition: form-data; name=\"model\"\r\n\r\n");
    body.extend(format!("{}\r\n", model).as_bytes());

    // Field: file
    body.extend(format!("--{}\r\n", boundary).as_bytes());
    body.extend(b"Content-Disposition: form-data; name=\"file\"; filename=\"speech.wav\"\r\n");
    body.extend(b"Content-Type: audio/wav\r\n\r\n");
    body.extend(wav_bytes);
    body.extend(b"\r\n");

    // Close boundary
    body.extend(format!("--{}--\r\n", boundary).as_bytes());

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build();

    let resp = agent.post(endpoint)
        .set("Content-Type", &format!("multipart/form-data; boundary={}", boundary))
        .set("Authorization", &format!("Bearer {}", api_key.trim()))
        .send_bytes(&body)?;

    #[derive(serde::Deserialize)]
    struct WhisperResp {
        text: String,
    }

    let parsed: WhisperResp = resp.into_json()?;
    Ok(parsed.text)
}

/// Sends audio WAV directly to Google Gemini 1.5 Flash via inline data base64
fn call_gemini_audio_transcription(
    api_key: &str,
    wav_bytes: &[u8],
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let endpoint = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
        api_key.trim()
    );

    // Simple base64 encode using standard table
    let b64 = base64_simd_encode(wav_bytes);

    let payload = serde_json::json!({
        "contents": [{
            "parts": [
                {
                    "inlineData": {
                        "mimeType": "audio/wav",
                        "data": b64
                    }
                },
                {
                    "text": "Transcreva exatamente as palavras ditas no áudio acima, especialmente termos em inglês e gírias de jogos. Responda APENAS com a transcrição textual, sem nenhum comentário adicional."
                }
            ]
        }]
    });

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(12))
        .build();

    let resp = agent.post(&endpoint)
        .set("Content-Type", "application/json")
        .send_json(&payload)?;

    #[derive(serde::Deserialize)]
    struct GeminiResp {
        candidates: Option<Vec<Candidate>>,
    }
    #[derive(serde::Deserialize)]
    struct Candidate {
        content: Option<Content>,
    }
    #[derive(serde::Deserialize)]
    struct Content {
        parts: Option<Vec<Part>>,
    }
    #[derive(serde::Deserialize)]
    struct Part {
        text: Option<String>,
    }

    let parsed: GeminiResp = resp.into_json()?;
    let text = parsed.candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|p| p.into_iter().next())
        .and_then(|p| p.text)
        .unwrap_or_default();

    Ok(text)
}

fn base64_simd_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        let c0 = b0 >> 2;
        let c1 = ((b0 & 0b11) << 4) | (b1 >> 4);
        let c2 = ((b1 & 0b1111) << 2) | (b2 >> 6);
        let c3 = b2 & 0b111111;

        out.push(TABLE[c0 as usize] as char);
        out.push(TABLE[c1 as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[c2 as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[c3 as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}
