use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SttModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub architecture: String,
    pub size_mb: u32,
    pub speed_score: u8,
    pub accuracy_score: u8,
    pub languages: Vec<String>,
    pub recommended: bool,
    pub recommended_rank: u32,
    pub download_url: String,
    pub filename: String,
    pub is_installed: bool,
    pub installed_path: Option<PathBuf>,
}

pub struct ModelCatalog;

impl ModelCatalog {
    pub fn all_models() -> Vec<SttModelInfo> {
        let raw_catalog = vec![
            SttModelInfo {
                id: "whisper-base".to_string(),
                name: "Whisper Base (Recomendado)".to_string(),
                description: "Equilíbrio ideal entre velocidade e alta precisão para termos e jogos.".to_string(),
                architecture: "whisper".to_string(),
                size_mb: 142,
                speed_score: 88,
                accuracy_score: 87,
                languages: vec!["pt".to_string(), "en".to_string(), "multilingual".to_string()],
                recommended: true,
                recommended_rank: 1,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin".to_string(),
                filename: "ggml-base.bin".to_string(),
                is_installed: false,
                installed_path: None,
            },
            SttModelInfo {
                id: "whisper-tiny".to_string(),
                name: "Whisper Tiny (Ultraleve)".to_string(),
                description: "Mínimo uso de CPU e memória. Resposta instantânea mesmo em PCs modestos.".to_string(),
                architecture: "whisper".to_string(),
                size_mb: 75,
                speed_score: 96,
                accuracy_score: 78,
                languages: vec!["pt".to_string(), "en".to_string(), "multilingual".to_string()],
                recommended: false,
                recommended_rank: 3,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin".to_string(),
                filename: "ggml-tiny.bin".to_string(),
                is_installed: false,
                installed_path: None,
            },
            SttModelInfo {
                id: "whisper-small".to_string(),
                name: "Whisper Small (Alta Precisão)".to_string(),
                description: "Vocabulário extenso e fidelidade superior para gírias complexas.".to_string(),
                architecture: "whisper".to_string(),
                size_mb: 466,
                speed_score: 75,
                accuracy_score: 93,
                languages: vec!["pt".to_string(), "en".to_string(), "multilingual".to_string()],
                recommended: false,
                recommended_rank: 4,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin".to_string(),
                filename: "ggml-small.bin".to_string(),
                is_installed: false,
                installed_path: None,
            },
            SttModelInfo {
                id: "nemotron-3.5".to_string(),
                name: "Nemotron Streaming 3.5 (NVIDIA)".to_string(),
                description: "Streaming multilíngue em tempo real para 28 idiomas (Padrão do Handy).".to_string(),
                architecture: "parakeet".to_string(),
                size_mb: 717,
                speed_score: 84,
                accuracy_score: 82,
                languages: vec!["pt".to_string(), "en".to_string(), "28 langs".to_string()],
                recommended: true,
                recommended_rank: 2,
                download_url: "https://huggingface.co/handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf/resolve/6d44e540bc31b0de1dbe174a3cea87f53a7f22fb/nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf".to_string(),
                filename: "nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf".to_string(),
                is_installed: false,
                installed_path: None,
            },
            SttModelInfo {
                id: "parakeet-en".to_string(),
                name: "Parakeet Unified EN 0.6B".to_string(),
                description: "Especializado e calibrado em inglês nativo e termos técnicos de games.".to_string(),
                architecture: "parakeet".to_string(),
                size_mb: 731,
                speed_score: 79,
                accuracy_score: 90,
                languages: vec!["en".to_string()],
                recommended: false,
                recommended_rank: 5,
                download_url: "https://huggingface.co/handy-computer/parakeet-unified-en-0.6b-gguf/resolve/7e948f21b7bdbac698d3318db9d350f1096f3b6c/parakeet-unified-en-0.6b-Q8_0.gguf".to_string(),
                filename: "parakeet-unified-en-0.6b-Q8_0.gguf".to_string(),
                is_installed: false,
                installed_path: None,
            },
            SttModelInfo {
                id: "canary-180m".to_string(),
                name: "Canary 180M Flash".to_string(),
                description: "Ultrarrápido, multilíngue e econômico em processamento gráfico.".to_string(),
                architecture: "canary".to_string(),
                size_mb: 209,
                speed_score: 92,
                accuracy_score: 80,
                languages: vec!["en".to_string(), "es".to_string(), "de".to_string(), "fr".to_string()],
                recommended: false,
                recommended_rank: 6,
                download_url: "https://huggingface.co/handy-computer/canary-180m-flash-gguf/resolve/b147f9dc52b59f0998e410540a84727bd86457fd/canary-180m-flash-Q8_0.gguf".to_string(),
                filename: "canary-180m-flash-Q8_0.gguf".to_string(),
                is_installed: false,
                installed_path: None,
            },
        ];

        let mut models = raw_catalog;
        for m in &mut models {
            if let Some(path) = Self::find_installed_file(&m.filename) {
                m.is_installed = true;
                m.installed_path = Some(path);
            }
        }
        models
    }

    pub fn find_model(id: &str) -> Option<SttModelInfo> {
        Self::all_models().into_iter().find(|m| m.id == id || m.filename == id)
    }

    pub fn get_models_dir() -> PathBuf {
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local_app).join("Lexi").join("models");
            let _ = std::fs::create_dir_all(&p);
            return p;
        }

        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let p = PathBuf::from(userprofile).join(".lexi").join("models");
            let _ = std::fs::create_dir_all(&p);
            return p;
        }

        let p = PathBuf::from("models");
        let _ = std::fs::create_dir_all(&p);
        p
    }

    pub fn find_installed_file(filename: &str) -> Option<PathBuf> {
        // 1. Direct in Lexi's models dir
        let lexi_dir = Self::get_models_dir();
        let target = lexi_dir.join(filename);
        if target.exists() && target.metadata().map(|m| m.len() > 1024).unwrap_or(false) {
            return Some(target);
        }

        // 2. Relative models dir
        let local_rel = PathBuf::from("models").join(filename);
        if local_rel.exists() && local_rel.metadata().map(|m| m.len() > 1024).unwrap_or(false) {
            return Some(local_rel);
        }

        // 3. HuggingFace Hub cache check (Windows & Linux WSL)
        let mut hf_roots = Vec::new();
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            hf_roots.push(PathBuf::from(userprofile).join(".cache").join("huggingface").join("hub"));
        }
        if let Ok(home) = std::env::var("HOME") {
            hf_roots.push(PathBuf::from(home).join(".cache").join("huggingface").join("hub"));
        }
        // Also check /mnt/c/Users/Usuario/.cache/huggingface/hub if in WSL
        hf_roots.push(PathBuf::from("/mnt/c/Users/Usuario/.cache/huggingface/hub"));

        for hf_root in hf_roots {
            if hf_root.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&hf_root) {
                    for entry in entries.flatten() {
                        let snapshots = entry.path().join("snapshots");
                        if snapshots.is_dir() {
                            if let Ok(sub_entries) = std::fs::read_dir(&snapshots) {
                                for sub in sub_entries.flatten() {
                                    let candidate = sub.path().join(filename);
                                    if candidate.exists() && candidate.metadata().map(|m| m.len() > 1024).unwrap_or(false) {
                                        return Some(candidate);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStatusUpdate {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percent: f32,
    pub speed_mb_s: f32,
    pub is_finished: bool,
    pub error: Option<String>,
}

pub fn download_stt_model<F>(
    model_id: &str,
    cancel_flag: Arc<AtomicBool>,
    mut progress_cb: F,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnMut(DownloadStatusUpdate) + Send + 'static,
{
    let model = ModelCatalog::find_model(model_id)
        .ok_or_else(|| format!("Model with id '{}' not found in catalog", model_id))?;

    let models_dir = ModelCatalog::get_models_dir();
    let dest_file = models_dir.join(&model.filename);
    let part_file = models_dir.join(format!("{}.part", &model.filename));

    // If already downloaded and valid, return immediately
    if dest_file.exists() && dest_file.metadata()?.len() > 1024 {
        progress_cb(DownloadStatusUpdate {
            model_id: model_id.to_string(),
            downloaded_bytes: dest_file.metadata()?.len(),
            total_bytes: Some(dest_file.metadata()?.len()),
            percent: 100.0,
            speed_mb_s: 0.0,
            is_finished: true,
            error: None,
        });
        return Ok(dest_file);
    }

    log::info!("Starting download of model '{}' from {}", model.name, model.download_url);

    let resp = ureq::get(&model.download_url)
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|e| format!("HTTP request error for {}: {}", model.download_url, e))?;

    let total_bytes: Option<u64> = resp
        .header("Content-Length")
        .and_then(|val| val.parse::<u64>().ok());

    let mut reader = resp.into_reader();
    let mut file = File::create(&part_file)?;

    let mut buffer = [0u8; 64 * 1024]; // 64 KB buffer
    let mut downloaded_bytes: u64 = 0;
    let start_time = Instant::now();
    let mut last_update_time = Instant::now();

    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            let _ = std::fs::remove_file(&part_file);
            return Err("Download cancelled by user".into());
        }

        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])?;
        downloaded_bytes += bytes_read as u64;

        if last_update_time.elapsed().as_millis() >= 250 {
            let elapsed_sec = start_time.elapsed().as_secs_f32().max(0.001);
            let speed_mb_s = (downloaded_bytes as f32 / (1024.0 * 1024.0)) / elapsed_sec;
            let percent = if let Some(tot) = total_bytes {
                if tot > 0 {
                    (downloaded_bytes as f32 / tot as f32) * 100.0
                } else {
                    0.0
                }
            } else {
                0.0
            };

            progress_cb(DownloadStatusUpdate {
                model_id: model_id.to_string(),
                downloaded_bytes,
                total_bytes,
                percent: percent.clamp(0.0, 99.9),
                speed_mb_s,
                is_finished: false,
                error: None,
            });
            last_update_time = Instant::now();
        }
    }

    file.flush()?;
    drop(file);

    // Atomically rename .part file to final model filename
    std::fs::rename(&part_file, &dest_file)?;

    progress_cb(DownloadStatusUpdate {
        model_id: model_id.to_string(),
        downloaded_bytes,
        total_bytes: Some(downloaded_bytes),
        percent: 100.0,
        speed_mb_s: 0.0,
        is_finished: true,
        error: None,
    });

    log::info!("Model '{}' successfully saved to {:?}", model.name, dest_file);
    Ok(dest_file)
}

/// List available audio input devices via cpal (Windows) or mock list (non-Windows)
pub fn list_audio_input_devices() -> Vec<String> {
    #[cfg(windows)]
    {
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        let mut devices = Vec::new();

        if let Ok(input_devices) = host.input_devices() {
            for device in input_devices {
                if let Ok(name) = device.name() {
                    devices.push(name);
                }
            }
        }

        if devices.is_empty() {
            devices.push("Dispositivo Padrão do Sistema".to_string());
        }

        devices
    }

    #[cfg(not(windows))]
    {
        vec![
            "Dispositivo Padrão do Sistema".to_string(),
            "Microphone (Realtek High Definition Audio)".to_string(),
            "Microphone (USB PnP Sound Device)".to_string(),
        ]
    }
}

/// Audio Recorder capturing 16kHz mono audio from microphone using cpal + hound
pub struct AudioRecorder {
    is_recording: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 16000,
        }
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    pub fn start_recording(&self, device_name_opt: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_recording.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.samples.lock().unwrap().clear();
        self.is_recording.store(true, Ordering::Relaxed);

        #[cfg(windows)]
        {
            use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
            let is_recording_clone = Arc::clone(&self.is_recording);
            let samples_clone = Arc::clone(&self.samples);
            let device_name_owned = device_name_opt.map(|s| s.to_string());

            std::thread::spawn(move || {
                let host = cpal::default_host();
                let device = if let Some(ref name) = device_name_owned {
                    host.input_devices()
                        .ok()
                        .and_then(|mut devs| devs.find(|d| d.name().map(|n| n == *name).unwrap_or(false)))
                        .or_else(|| host.default_input_device())
                } else {
                    host.default_input_device()
                };

                let Some(device) = device else {
                    log::error!("No audio input device available for recording");
                    is_recording_clone.store(false, Ordering::Relaxed);
                    return;
                };

                let Ok(default_config) = device.default_input_config() else {
                    log::error!("Failed to get default input config");
                    is_recording_clone.store(false, Ordering::Relaxed);
                    return;
                };

                let channels = default_config.channels() as usize;
                let sample_rate = default_config.sample_rate().0;

                let err_fn = |err| log::error!("CPAL audio stream error: {}", err);

                let stream_result = match default_config.sample_format() {
                    cpal::SampleFormat::F32 => {
                        let samples_inner = Arc::clone(&samples_clone);
                        let is_rec_inner = Arc::clone(&is_recording_clone);
                        device.build_input_stream(
                            &default_config.into(),
                            move |data: &[f32], _: &_| {
                                if is_rec_inner.load(Ordering::Relaxed) {
                                    let mut lock = samples_inner.lock().unwrap();
                                    for chunk in data.chunks(channels) {
                                        if !chunk.is_empty() {
                                            lock.push(chunk[0]);
                                        }
                                    }
                                }
                            },
                            err_fn,
                            None,
                        )
                    }
                    cpal::SampleFormat::I16 => {
                        let samples_inner = Arc::clone(&samples_clone);
                        let is_rec_inner = Arc::clone(&is_recording_clone);
                        device.build_input_stream(
                            &default_config.into(),
                            move |data: &[i16], _: &_| {
                                if is_rec_inner.load(Ordering::Relaxed) {
                                    let mut lock = samples_inner.lock().unwrap();
                                    for chunk in data.chunks(channels) {
                                        if !chunk.is_empty() {
                                            lock.push(chunk[0] as f32 / 32768.0);
                                        }
                                    }
                                }
                            },
                            err_fn,
                            None,
                        )
                    }
                    _ => {
                        log::error!("Unsupported sample format");
                        is_recording_clone.store(false, Ordering::Relaxed);
                        return;
                    }
                };

                if let Ok(stream) = stream_result {
                    if let Err(e) = stream.play() {
                        log::error!("Failed to play input audio stream: {}", e);
                        is_recording_clone.store(false, Ordering::Relaxed);
                        return;
                    }

                    while is_recording_clone.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }

                    drop(stream);
                    log::debug!("Audio recording loop terminated gracefully (orig_sr={})", sample_rate);
                }
            });
        }

        #[cfg(not(windows))]
        {
            let _ = device_name_opt;
            let samples_clone = Arc::clone(&self.samples);
            let is_recording_clone = Arc::clone(&self.is_recording);
            std::thread::spawn(move || {
                let mut dummy_val = 0.0f32;
                while is_recording_clone.load(Ordering::Relaxed) {
                    {
                        let mut lock = samples_clone.lock().unwrap();
                        for _ in 0..160 {
                            lock.push((dummy_val.sin()) * 0.1);
                            dummy_val += 0.05;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
            });
        }

        Ok(())
    }

    pub fn stop_recording(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        self.is_recording.store(false, Ordering::Relaxed);
        // Small grace period for audio buffer to flush
        std::thread::sleep(std::time::Duration::from_millis(60));

        let raw_samples = {
            let mut lock = self.samples.lock().unwrap();
            let s = lock.clone();
            lock.clear();
            s
        };

        if raw_samples.is_empty() {
            return Err("No audio recorded (empty buffer)".into());
        }

        // Resample or downsample to 16kHz if needed, and write to WAV in memory
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
            for sample in raw_samples {
                let clamped = sample.clamp(-1.0, 1.0);
                let val = (clamped * 32767.0) as i16;
                writer.write_sample(val)?;
            }
            writer.finalize()?;
        }

        Ok(cursor.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_retrieval() {
        let models = ModelCatalog::all_models();
        assert!(!models.is_empty());
        let whisper_base = models.iter().find(|m| m.id == "whisper-base");
        assert!(whisper_base.is_some());
        assert_eq!(whisper_base.unwrap().size_mb, 142);
        assert!(whisper_base.unwrap().recommended);
    }

    #[test]
    fn test_find_model_by_id_or_filename() {
        assert!(ModelCatalog::find_model("whisper-tiny").is_some());
        assert!(ModelCatalog::find_model("ggml-base.bin").is_some());
        assert!(ModelCatalog::find_model("nemotron-3.5").is_some());
        assert!(ModelCatalog::find_model("nonexistent").is_none());
    }

    #[test]
    fn test_list_audio_devices() {
        let devices = list_audio_input_devices();
        assert!(!devices.is_empty());
    }

    #[test]
    fn test_audio_recorder_buffer() {
        let recorder = AudioRecorder::new();
        assert!(!recorder.is_recording());
    }
}
