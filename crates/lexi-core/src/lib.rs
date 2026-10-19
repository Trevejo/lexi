pub mod bridge;
pub mod cache;
pub mod config;
pub mod dictionary;
pub mod llm;
pub mod models;
pub mod normalize;
pub mod lookup;
pub mod stt_models;

pub use bridge::HandyBridgeServer;
pub use cache::Cache;
pub use config::AppConfig;
pub use dictionary::Dictionary;
pub use llm::LlmClient;
pub use models::{LookupResult, LookupSource};
pub use normalize::normalize_query;
pub use lookup::LookupService;
pub use stt_models::{
    download_stt_model, list_audio_input_devices, AudioRecorder, DownloadStatusUpdate,
    ModelCatalog, SttModelInfo,
};

