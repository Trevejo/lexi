#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod keyboard_hook;
mod overlay;
pub mod settings;
mod tray;
mod tts;

use app::Application;
use lexi_core::config::AppConfig;
use std::path::PathBuf;

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::GetLastError;
#[cfg(windows)]
use windows::Win32::System::Threading::CreateMutexW;
#[cfg(windows)]
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

struct FileLogger {
    file1: std::sync::Mutex<Option<std::fs::File>>,
    file2: std::sync::Mutex<Option<std::fs::File>>,
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let msg = format!("[{}] {}\n", record.level(), record.args());
            eprint!("{}", msg);
            use std::io::Write;
            if let Ok(mut lock) = self.file1.lock() {
                if let Some(ref mut f) = *lock {
                    let _ = f.write_all(msg.as_bytes());
                    let _ = f.flush();
                }
            }
            if let Ok(mut lock) = self.file2.lock() {
                if let Some(ref mut f) = *lock {
                    let _ = f.write_all(msg.as_bytes());
                    let _ = f.flush();
                }
            }
        }
    }

    fn flush(&self) {}
}

pub fn get_local_app_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local).join("Lexi");
            let _ = std::fs::create_dir_all(&p);
            return p;
        }
        if let Ok(roaming) = std::env::var("APPDATA") {
            let p = PathBuf::from(roaming).join("Lexi");
            let _ = std::fs::create_dir_all(&p);
            return p;
        }
    }

    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let p = PathBuf::from(home).join(".lexi");
        let _ = std::fs::create_dir_all(&p);
        return p;
    }

    PathBuf::from(".")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup crash reporting so errors are visible instead of silently disappearing
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("Erro crítico no Lexi:\n\n{}", info);
        let _ = std::fs::write("lexi_crash.log", &msg);
        #[cfg(windows)]
        unsafe {
            let wide_msg = to_wide_str(&msg);
            let wide_title = to_wide_str("Lexi - Erro");
            MessageBoxW(None, PCWSTR(wide_msg.as_ptr()), PCWSTR(wide_title.as_ptr()), MB_OK | MB_ICONERROR);
        }
    }));

    #[cfg(windows)]
    unsafe {
        // Enable crisp Per-Monitor V2 DPI scaling on Windows 10/11
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let app_dir = get_local_app_dir();
    let log_path_local = app_dir.join("lexi.log");
    let log_file_local = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path_local)
        .ok();

    let log_file_exe = if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let p = exe_dir.join("lexi.log");
            if p != log_path_local {
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&p)
                    .ok()
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let logger = Box::leak(Box::new(FileLogger {
        file1: std::sync::Mutex::new(log_file_local),
        file2: std::sync::Mutex::new(log_file_exe),
    }));
    let _ = log::set_logger(logger);
    log::set_max_level(log::LevelFilter::Info);

    log::info!("====================================================");
    log::info!("Iniciando Lexi - Tradutor Gamer & Dicionario Nativo");
    log::info!("Pasta de dados local: {:?}", app_dir);

    // Single-instance Mutex check
    #[cfg(windows)]
    unsafe {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let mutex_name: Vec<u16> = OsStr::new("LexiGamingTranslateToolMutex")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let _mutex = CreateMutexW(None, true, PCWSTR(mutex_name.as_ptr()));
        if GetLastError().0 == 183 {
            // ERROR_ALREADY_EXISTS
            let wide_msg = to_wide_str("O Lexi ja esta rodando em segundo plano!\nVerifique o icone proximo ao relogio do Windows.");
            let wide_title = to_wide_str("Lexi");
            MessageBoxW(None, PCWSTR(wide_msg.as_ptr()), PCWSTR(wide_title.as_ptr()), MB_OK);
            return Ok(());
        }
    }

    let (mut config, config_path) = load_best_config(&app_dir);
    log::info!("Configuracao carregada de: {:?}", config_path);

    // Ensure dictionary is on local fast drive (NTFS) to avoid network locking/latency issues
    let local_dict = ensure_local_dictionary(&app_dir);
    config.lookup.dictionary_path = local_dict;
    config.lookup.cache_path = app_dir.join("cache.sqlite");

    let app = Application::new(config);
    if let Err(e) = app.run() {
        let msg = format!("Falha na execucao do Lexi: {}", e);
        let _ = std::fs::write("lexi_error.log", &msg);
        #[cfg(windows)]
        unsafe {
            let wide_msg = to_wide_str(&msg);
            let wide_title = to_wide_str("Lexi - Erro de Execucao");
            MessageBoxW(None, PCWSTR(wide_msg.as_ptr()), PCWSTR(wide_title.as_ptr()), MB_OK | MB_ICONERROR);
        }
    }

    log::info!("Lexi finalizado.");
    Ok(())
}

pub fn load_best_config(app_dir: &std::path::Path) -> (AppConfig, PathBuf) {
    let local_cfg = app_dir.join("config.toml");

    // First check next to executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let p = exe_dir.join("config.toml");
            if p.exists() {
                if let Ok(cfg) = AppConfig::load_from_path(&p) {
                    let _ = cfg.save_to_path(&local_cfg);
                    return (cfg, p);
                }
            }
        }
    }

    // Check current working directory
    let cwd_cfg = PathBuf::from("config.toml");
    if cwd_cfg.exists() {
        if let Ok(cfg) = AppConfig::load_from_path(&cwd_cfg) {
            let _ = cfg.save_to_path(&local_cfg);
            return (cfg, cwd_cfg);
        }
    }

    // Check AppData local
    if local_cfg.exists() {
        if let Ok(cfg) = AppConfig::load_from_path(&local_cfg) {
            return (cfg, local_cfg);
        }
    }

    // Default
    let default_cfg = AppConfig::default();
    let _ = default_cfg.save_to_path(&local_cfg);
    (default_cfg, local_cfg)
}

pub fn save_config_everywhere(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let app_dir = get_local_app_dir();
    let local_cfg = app_dir.join("config.toml");
    config.save_to_path(&local_cfg)?;

    // Sync to cwd / project root if config.toml exists or Cargo.toml exists
    let cwd_cfg = PathBuf::from("config.toml");
    if cwd_cfg.exists() || std::path::Path::new("Cargo.toml").exists() {
        let _ = config.save_to_path(&cwd_cfg);
    }

    // Sync next to executable if exists
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let exe_cfg = exe_dir.join("config.toml");
            if exe_cfg.exists() {
                let _ = config.save_to_path(&exe_cfg);
            }
        }
    }

    log::info!("Config successfully saved to local AppData and root paths");
    Ok(())
}

fn ensure_local_dictionary(app_dir: &std::path::Path) -> PathBuf {
    let local_dict = app_dir.join("dictionary.sqlite");

    // Check if local dict already exists and is healthy (> 30 MB)
    let needs_copy = match std::fs::metadata(&local_dict) {
        Ok(meta) => meta.len() < 30_000_000,
        Err(_) => true,
    };

    if needs_copy {
        let mut candidates = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                candidates.push(exe_dir.join("dictionary.sqlite"));
                if let Some(parent) = exe_dir.parent() {
                    candidates.push(parent.join("dictionary.sqlite"));
                    if let Some(grandparent) = parent.parent() {
                        candidates.push(grandparent.join("dictionary.sqlite"));
                    }
                }
            }
        }
        candidates.push(PathBuf::from("dictionary.sqlite"));
        candidates.push(PathBuf::from("target/release/dictionary.sqlite"));

        for cand in candidates {
            if cand.exists() {
                if let Ok(meta) = std::fs::metadata(&cand) {
                    if meta.len() >= 30_000_000 {
                        log::info!("Copiando dicionario bundled ({:.1} MB) de {:?} para {:?}",
                            meta.len() as f64 / 1_000_000.0, cand, local_dict);
                        if let Ok(_) = std::fs::copy(&cand, &local_dict) {
                            log::info!("Dicionario copiado com sucesso para o disco local!");
                            return local_dict;
                        }
                    }
                }
            }
        }
    }

    local_dict
}

#[cfg(windows)]
fn to_wide_str(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

