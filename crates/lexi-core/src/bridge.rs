use crate::config::BridgeConfig;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use tiny_http::{Header, Response, Server, StatusCode};

pub struct HandyBridgeServer {
    config: BridgeConfig,
    running: Arc<AtomicBool>,
}

impl HandyBridgeServer {
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self, query_sender: Sender<String>) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let server = Server::http(&addr).map_err(|e| format!("Failed to bind Handy bridge on {}: {}", addr, e))?;
        log::info!("Handy bridge listening on http://{}", addr);

        self.running.store(true, Ordering::SeqCst);
        let running = Arc::clone(&self.running);
        let model_name = self.config.model_name.clone();

        thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                let mut request = match server.recv() {
                    Ok(rq) => rq,
                    Err(_) => break,
                };

                let url = request.url().to_string();
                let method = request.method().as_str().to_string();

                log::debug!("Bridge received request: {} {}", method, url);

                if (url.ends_with("/models") || url == "/models") && method == "GET" {
                    let models_json = serde_json::json!({
                        "object": "list",
                        "data": [
                            {
                                "id": model_name,
                                "object": "model",
                                "created": 1720000000,
                                "owned_by": "lexi"
                            }
                        ]
                    });
                    let data = models_json.to_string();
                    let response = Response::from_string(data)
                        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                    let _ = request.respond(response);
                    continue;
                }

                if (url.ends_with("/chat/completions") || url.ends_with("/completions")) && method == "POST" {
                    let mut body_str = String::new();
                    let _ = request.as_reader().read_to_string(&mut body_str);
                    log::info!("Handy bridge POST body: '{}'", body_str);

                    let recognized_text = extract_user_text(&body_str);
                    if let Some(text) = recognized_text {
                        if !text.trim().is_empty() {
                            log::info!("Handy transcription extracted: '{}'", text);
                            let _ = query_sender.send(text);
                        }
                    }

                    // Respond with EMPTY assistant content so Handy WILL NOT PASTE ANYTHING into the game
                    let response_json = serde_json::json!({
                        "id": "chatcmpl-lexi",
                        "object": "chat.completion",
                        "created": 1720000000,
                        "model": model_name,
                        "choices": [
                            {
                                "index": 0,
                                "message": {
                                    "role": "assistant",
                                    "content": ""
                                },
                                "finish_reason": "stop"
                            }
                        ]
                    });

                    let data = response_json.to_string();
                    let response = Response::from_string(data)
                        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                    let _ = request.respond(response);
                    continue;
                }

                // Default 404
                let response = Response::from_string("Not Found")
                    .with_status_code(StatusCode(404));
                let _ = request.respond(response);
            }
        });

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

fn extract_user_text(json_body: &str) -> Option<String> {
    let parsed: Value = serde_json::from_str(json_body).ok()?;

    // 1. Check messages array (chat completions)
    if let Some(messages) = parsed.get("messages").and_then(|m| m.as_array()) {
        for msg in messages.iter().rev() {
            if msg.get("role").and_then(|r| r.as_str()) == Some("user") {
                if let Some(text) = extract_content_string(msg.get("content")) {
                    return Some(text);
                }
            }
        }

        // Fallback inside messages
        for msg in messages.iter().rev() {
            if let Some(text) = extract_content_string(msg.get("content")) {
                return Some(text);
            }
        }
    }

    // 2. Check prompt (completions endpoint)
    if let Some(prompt) = extract_content_string(parsed.get("prompt")) {
        return Some(prompt);
    }

    // 3. Check input / text field
    if let Some(input) = extract_content_string(parsed.get("input")) {
        return Some(input);
    }
    if let Some(text) = extract_content_string(parsed.get("text")) {
        return Some(text);
    }

    None
}

fn extract_content_string(val: Option<&Value>) -> Option<String> {
    let v = val?;
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = v.as_array() {
        // Can be array of strings or array of parts [{"type": "text", "text": "..."}]
        let mut parts = Vec::new();
        for item in arr {
            if let Some(s) = item.as_str() {
                parts.push(s.to_string());
            } else if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                parts.push(text.to_string());
            }
        }
        if !parts.is_empty() {
            return Some(parts.join(" "));
        }
    }
    None
}
