use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub bridge: BridgeConfig,
    #[serde(default)]
    pub lookup: LookupConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub dismiss: DismissConfig,
    #[serde(default)]
    pub overlay: OverlayConfig,
    #[serde(default)]
    pub audio: AudioConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            bridge: BridgeConfig::default(),
            lookup: LookupConfig::default(),
            llm: LlmConfig::default(),
            dismiss: DismissConfig::default(),
            overlay: OverlayConfig::default(),
            audio: AudioConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn load_from_path(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let config: AppConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = AppConfig::default();
            config.save_to_path(path)?;
            Ok(config)
        }
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub host: String,
    pub port: u16,
    pub model_name: String,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 47823,
            model_name: "lexi".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LookupMode {
    Hybrid,
    OfflineOnly,
    LlmOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupConfig {
    pub mode: LookupMode,
    pub dictionary_path: PathBuf,
    pub cache_path: PathBuf,
}

impl Default for LookupConfig {
    fn default() -> Self {
        Self {
            mode: LookupMode::Hybrid,
            dictionary_path: PathBuf::from("dictionary.sqlite"),
            cache_path: PathBuf::from("cache.sqlite"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub enabled: bool,
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout_seconds: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: "custom".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
            timeout_seconds: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DismissMode {
    /// Dismiss as soon as the trigger key or keyboard key is released
    ReleaseKey,
    /// Dismiss automatically after timeout_seconds
    Timeout,
    /// Dismiss only when pressing the dismiss hotkey
    Hotkey,
    /// Dismiss on any keyboard press (without consuming game input)
    AnyKey,
    /// Hybrid: timeout + release key + hotkey
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DismissConfig {
    pub mode: DismissMode,
    pub timeout_seconds: f32,
    pub dismiss_hotkey: String,
    pub trigger_key_to_watch: String,
}

impl Default for DismissConfig {
    fn default() -> Self {
        Self {
            mode: DismissMode::Hybrid,
            timeout_seconds: 8.0,
            dismiss_hotkey: "Escape".to_string(),
            trigger_key_to_watch: "F13".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverlayPosition {
    TopCenter,
    TopRight,
    TopLeft,
    Center,
    BottomCenter,
    MiddleLeft,
    MiddleRight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub enabled: bool,
    pub position: OverlayPosition,
    pub width: i32,
    pub max_height: i32,
    pub font_family: String,
    pub font_size: i32,
    pub opacity: f32,
    pub fade_duration_ms: u32,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: OverlayPosition::TopCenter,
            width: 440,
            max_height: 240,
            font_family: "Segoe UI".to_string(),
            font_size: 15,
            opacity: 0.94,
            fade_duration_ms: 120,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub enabled: bool,
    pub only_fullscreen: bool,
    pub voice: String,
    pub volume: u32,
    pub rate: i32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            only_fullscreen: false,
            voice: "pt-BR".to_string(),
            volume: 85,
            rate: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_position_serde() {
        let positions = vec![
            (OverlayPosition::TopCenter, "\"top_center\""),
            (OverlayPosition::TopRight, "\"top_right\""),
            (OverlayPosition::TopLeft, "\"top_left\""),
            (OverlayPosition::Center, "\"center\""),
            (OverlayPosition::BottomCenter, "\"bottom_center\""),
            (OverlayPosition::MiddleLeft, "\"middle_left\""),
            (OverlayPosition::MiddleRight, "\"middle_right\""),
        ];

        for (pos, json_str) in positions {
            let serialized = serde_json::to_string(&pos).expect("serialize pos");
            assert_eq!(serialized, json_str);

            let deserialized: OverlayPosition =
                serde_json::from_str(json_str).expect("deserialize pos");
            assert_eq!(deserialized, pos);
        }
    }

    #[test]
    fn test_overlay_position_toml() {
        let toml_str = r#"
            enabled = true
            position = "middle_left"
            width = 440
            max_height = 240
            font_family = "Segoe UI"
            font_size = 15
            opacity = 0.94
            fade_duration_ms = 120
        "#;
        let cfg: OverlayConfig = toml::from_str(toml_str).expect("parse toml");
        assert_eq!(cfg.position, OverlayPosition::MiddleLeft);

        let toml_str2 = r#"
            enabled = true
            position = "middle_right"
            width = 440
            max_height = 240
            font_family = "Segoe UI"
            font_size = 15
            opacity = 0.94
            fade_duration_ms = 120
        "#;
        let cfg2: OverlayConfig = toml::from_str(toml_str2).expect("parse toml");
        assert_eq!(cfg2.position, OverlayPosition::MiddleRight);
    }
}
