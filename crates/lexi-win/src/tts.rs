use lexi_core::config::AudioConfig;
use lexi_core::models::LookupResult;
#[cfg(windows)]
use std::process::Command;
use std::sync::{Arc, RwLock};
use std::thread;

pub struct TtsManager {
    config: Arc<RwLock<AudioConfig>>,
}

impl TtsManager {
    pub fn new(config: AudioConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    pub fn update_config(&self, new_config: AudioConfig) {
        if let Ok(mut cfg) = self.config.write() {
            *cfg = new_config;
            log::info!(
                "TtsManager audio config updated (enabled: {}, only_fullscreen: {}, volume: {})",
                cfg.enabled,
                cfg.only_fullscreen,
                cfg.volume
            );
        }
    }

    pub fn speak_result(&self, result: &LookupResult) {
        let config = match self.config.read() {
            Ok(cfg) => cfg.clone(),
            Err(_) => return,
        };

        if !config.enabled {
            return;
        }

        // When only_fullscreen is true, only speak if a fullscreen window is detected.
        // When only_fullscreen is false, always speak regardless of window mode (borderless, windowed, desktop).
        #[cfg(windows)]
        if config.only_fullscreen && !is_fullscreen_detected() {
            log::debug!("TTS skipped: only_fullscreen is active and foreground window is not fullscreen");
            return;
        }

        let term = result.normalized.clone();
        let translation = result.translation.clone();
        let volume = config.volume.min(100);
        let rate = config.rate;
        let voice_preference = config.voice.clone();

        // Run speech asynchronously in background thread so UI never stalls
        thread::spawn(move || {
            let speech_text = format!("{}: {}", term, translation);
            speak_windows_sapi(&speech_text, volume, rate, &voice_preference);
        });
    }
}

#[cfg(windows)]
fn is_fullscreen_detected() -> bool {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetDesktopWindow, GetForegroundWindow, GetShellWindow, GetWindowRect,
    };

    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() || fg == GetDesktopWindow() || fg == GetShellWindow() {
            return false;
        }

        let mut window_rect = RECT::default();
        if GetWindowRect(fg, &mut window_rect).is_err() {
            return false;
        }

        let monitor = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };

        if GetMonitorInfoW(monitor, &mut mi).as_bool() {
            window_rect.left <= mi.rcMonitor.left
                && window_rect.top <= mi.rcMonitor.top
                && window_rect.right >= mi.rcMonitor.right
                && window_rect.bottom >= mi.rcMonitor.bottom
        } else {
            false
        }
    }
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn is_fullscreen_detected() -> bool {
    true
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(windows)]
fn speak_windows_sapi(text: &str, volume: u32, rate: i32, voice_preference: &str) {
    // 1. Primary: WScript with VBScript.
    // wscript.exe is a GUI subsystem process that starts in <20ms without any console window flash.
    if speak_via_wscript(text, volume, rate, voice_preference).is_ok() {
        return;
    }

    // 2. Fallback: PowerShell with CREATE_NO_WINDOW and -WindowStyle Hidden
    speak_via_powershell(text, volume, rate, voice_preference);
}

#[cfg(windows)]
fn speak_via_wscript(text: &str, volume: u32, rate: i32, voice_pref: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::windows::process::CommandExt;

    let escaped_text = text.replace('"', "\"\"").replace('\r', " ").replace('\n', " ");

    let vbs_content = format!(
r#"Set voice = CreateObject("SAPI.SpVoice")
voice.Volume = {volume}
voice.Rate = {rate}
pref = "{voice_pref}"
If pref <> "" Then
    For Each v In voice.GetVoices
        desc = v.GetDescription
        If InStr(1, desc, pref, 1) > 0 Or (pref = "pt-BR" And (InStr(1, desc, "Portuguese", 1) > 0 Or InStr(1, desc, "Brasil", 1) > 0 Or InStr(1, desc, "Brazil", 1) > 0)) Then
            Set voice.Voice = v
            Exit For
        End If
    Next
End If
voice.Speak "{escaped_text}"
"#,
        volume = volume.min(100),
        rate = rate.clamp(-10, 10),
        voice_pref = voice_pref.replace('"', ""),
        escaped_text = escaped_text
    );

    let temp_file = std::env::temp_dir().join(format!(
        "lexi_tts_{}_{}.vbs",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    std::fs::write(&temp_file, vbs_content)?;

    let status = Command::new("wscript.exe")
        .args(["//nologo", "//B"])
        .arg(&temp_file)
        .creation_flags(CREATE_NO_WINDOW)
        .status();

    let _ = std::fs::remove_file(&temp_file);

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("wscript exited with code: {:?}", s.code()).into()),
        Err(e) => Err(Box::new(e)),
    }
}

#[cfg(windows)]
fn speak_via_powershell(text: &str, volume: u32, rate: i32, voice_pref: &str) {
    use std::os::windows::process::CommandExt;

    let script = format!(
        "$v = New-Object -ComObject SAPI.SpVoice; $v.Volume = {}; $v.Rate = {}; \
         $pref = '{}'; \
         if ($pref) {{ foreach ($voice in $v.GetVoices()) {{ $d = $voice.GetDescription(); if ($d -like \"*$pref*\" -or ($pref -eq 'pt-BR' -and ($d -like '*Portuguese*' -or $d -like '*Brazil*'))) {{ $v.Voice = $voice; break }} }} }} \
         $v.Speak('{}')",
        volume.min(100),
        rate.clamp(-10, 10),
        voice_pref.replace('\'', "''"),
        text.replace('\'', "''")
    );

    let _ = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

#[cfg(not(windows))]
fn speak_windows_sapi(text: &str, volume: u32, rate: i32, voice_preference: &str) {
    log::info!(
        "[TTS Mock] Speaking: {} (volume: {}, rate: {}, voice: {})",
        text,
        volume,
        rate,
        voice_preference
    );
}
