//! Windows built-in speech (System.Speech / SAPI 5), driven through PowerShell.
//! Free, offline and always present, so a first-time user hears speech immediately.
//! Output terms for Windows voices are not verified, so the licence is `unknown` and the
//! commercial export gate blocks it until the user records their own check.
//!
//! Security notes: the PowerShell script is a fixed constant, passed as -EncodedCommand.
//! User text reaches it only through a JSON file in a private temp folder, and the voice
//! name is checked against the installed-voice list before use.

use crate::audio::read_wav;
use crate::licence::Licence;
use crate::providers::*;
use crate::tags::Token;
use std::sync::Mutex;

const ENUM_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$s = New-Object System.Speech.Synthesis.SpeechSynthesizer
foreach ($v in $s.GetInstalledVoices()) {
  if ($v.Enabled) { $i = $v.VoiceInfo; "{0}|{1}|{2}|{3}" -f $i.Name, $i.Culture.Name, $i.Gender, $i.Age }
}
$s.Dispose()
"#;

const SPEAK_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$j = Get-Content -Raw -Encoding UTF8 -LiteralPath $env:SS_JOB | ConvertFrom-Json
$s = New-Object System.Speech.Synthesis.SpeechSynthesizer
$s.SelectVoice([string]$j.voice)
$s.Rate = [int]$j.rate
$s.SetOutputToWaveFile($env:SS_OUT)
$pb = New-Object System.Speech.Synthesis.PromptBuilder
foreach ($seg in $j.segments) {
  if ($seg.pause_ms -gt 0) { $pb.AppendBreak([TimeSpan]::FromMilliseconds([int]$seg.pause_ms)) }
  else {
    $st = New-Object System.Speech.Synthesis.PromptStyle
    $st.Rate = [System.Speech.Synthesis.PromptRate]([string]$seg.rate)
    $st.Volume = [System.Speech.Synthesis.PromptVolume]([string]$seg.volume)
    $pb.StartStyle($st); $pb.AppendText([string]$seg.text); $pb.EndStyle()
  }
}
$s.Speak($pb)
$s.Dispose()
"#;

pub struct SapiAdapter {
    voices: Mutex<Option<Vec<Voice>>>,
}

impl SapiAdapter {
    pub fn new() -> Self {
        SapiAdapter { voices: Mutex::new(None) }
    }

    fn voices(&self) -> Vec<Voice> {
        let mut g = self.voices.lock().unwrap_or_else(|e| e.into_inner());
        if g.is_none() {
            *g = Some(enumerate().unwrap_or_default());
        }
        g.clone().unwrap_or_default()
    }
}

impl Default for SapiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

#[cfg(windows)]
fn run_ps(script: &str, envs: &[(&str, &std::path::Path)]) -> Result<String, AdapterError> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let utf16: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let mut cmd = Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &b64(&utf16)])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().map_err(|e| AdapterError::ProviderError(format!("could not start PowerShell: {e}")))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() > Duration::from_secs(90) => {
                let _ = child.kill();
                return Err(AdapterError::Network);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(40)),
            Err(e) => return Err(AdapterError::ProviderError(e.to_string())),
        }
    }
    let out = child.wait_with_output().map_err(|e| AdapterError::ProviderError(e.to_string()))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let first = err.lines().find(|l| !l.trim().is_empty()).unwrap_or("speech engine failed");
        return Err(AdapterError::ProviderError(first.chars().take(200).collect()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[cfg(not(windows))]
fn run_ps(_script: &str, _envs: &[(&str, &std::path::Path)]) -> Result<String, AdapterError> {
    Err(AdapterError::ProviderError("Windows speech is only available on Windows".into()))
}

fn accent_for(culture: &str) -> String {
    match culture {
        "en-US" => "American",
        "en-GB" => "British",
        "en-AU" => "Australian",
        "en-IN" => "Indian",
        "en-CA" => "Canadian",
        "en-NG" => "Nigerian",
        "en-PH" => "Filipino",
        "en-ZA" => "South African",
        "en-IE" => "Irish",
        other => return other.to_string(),
    }
    .to_string()
}

fn enumerate() -> Result<Vec<Voice>, AdapterError> {
    let out = run_ps(ENUM_SCRIPT, &[])?;
    Ok(out
        .lines()
        .filter_map(|l| {
            let p: Vec<&str> = l.trim().split('|').collect();
            if p.len() != 4 || p[0].is_empty() {
                return None;
            }
            Some(Voice {
                id: format!("sapi:{}", p[0]),
                name: p[0].to_string(),
                provider: "windows-speech".into(),
                language: p[1].to_string(),
                accent: accent_for(p[1]),
                gender: p[2].to_lowercase(),
                age_range: p[3].to_lowercase(),
                licence: licence_info(
                    Licence::Unknown,
                    "",
                    "",
                    "Built-in Windows voice. Terms for commercial use of its output are not verified.",
                ),
                sample_text: "Good morning. This is how I sound.".into(),
                notes: "Offline. Cannot perform laughs, sighs or other non-verbal sounds.".into(),
            })
        })
        .collect())
}

/// (PromptRate, PromptVolume) names for a delivery style.
fn style(emotion: &str) -> (&'static str, &'static str) {
    match emotion {
        "whisper" => ("Slow", "ExtraSoft"),
        "shout" => ("Fast", "ExtraLoud"),
        "excited" => ("Fast", "Loud"),
        "sad" => ("Slow", "Soft"),
        "tired" => ("ExtraSlow", "Soft"),
        _ => ("Medium", "Medium"),
    }
}

impl Adapter for SapiAdapter {
    fn id(&self) -> &'static str {
        "windows-speech"
    }
    fn label(&self) -> &'static str {
        "Windows voices (offline)"
    }
    fn licence(&self) -> crate::licence::LicenceInfo {
        licence_info(Licence::Unknown, "", "", "Windows built-in speech. Output terms not verified.")
    }
    fn capabilities(&self) -> Capabilities {
        let mut languages: Vec<String> = self.voices().iter().map(|v| v.language.clone()).collect();
        languages.sort();
        languages.dedup();
        Capabilities {
            languages,
            // Prosody only (rate and volume). No claim about accent or emotion quality.
            tags: ["whisper", "shout", "excited", "sad", "tired"].iter().map(|s| s.to_string()).collect(),
            voice_design: false,
            cloning: false,
            max_chars_per_request: 4000,
            streaming: false,
        }
    }
    fn list_voices(&self) -> Vec<Voice> {
        self.voices()
    }
    fn provider_version(&self) -> String {
        "windows-sapi-5".into()
    }
    fn render(&self, req: &RenderRequest) -> Result<RenderOutput, AdapterError> {
        precheck(&self.capabilities(), req)?;
        let voice = self
            .voices()
            .into_iter()
            .find(|v| v.id == req.voice_id)
            .ok_or_else(|| AdapterError::UnknownVoice(req.voice_id.clone()))?;
        let mut current = req.emotion.clone();
        let mut segments = Vec::new();
        for t in &req.tokens {
            match t {
                Token::Text(s) => {
                    let (rate, volume) = style(&current);
                    segments.push(serde_json::json!({ "text": s, "rate": rate, "volume": volume }));
                }
                Token::Delivery(d) => current = d.clone(),
                Token::Pause(ms) => segments.push(serde_json::json!({ "pause_ms": ms })),
                Token::NonVerbal(t) => return Err(AdapterError::UnsupportedTag(t.clone())),
            }
        }
        if !segments.iter().any(|s| s.get("text").is_some()) {
            return Err(AdapterError::ProviderError("there is nothing to say on this line".into()));
        }
        let rate = ((req.pace - 1.0) * 10.0).round().clamp(-10.0, 10.0) as i32;
        let job = serde_json::json!({ "voice": voice.name, "rate": rate, "segments": segments });

        let dir = std::env::temp_dir().join(crate::util::new_id("soundstage-sapi"));
        std::fs::create_dir_all(&dir).map_err(|e| AdapterError::ProviderError(e.to_string()))?;
        let (job_path, out_path) = (dir.join("job.json"), dir.join("out.wav"));
        let result = (|| {
            std::fs::write(&job_path, job.to_string()).map_err(|e| AdapterError::ProviderError(e.to_string()))?;
            run_ps(SPEAK_SCRIPT, &[("SS_JOB", job_path.as_path()), ("SS_OUT", out_path.as_path())])?;
            read_wav(&out_path).map_err(AdapterError::ProviderError)
        })();
        let _ = std::fs::remove_dir_all(&dir);
        let d = result?;
        Ok(RenderOutput {
            samples: d.channels.into_iter().next().unwrap_or_default(),
            sample_rate: d.rate,
            provider_version: format!("windows-sapi-5/{}", voice.name),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(b64(b""), "");
        assert_eq!(b64(b"f"), "Zg==");
        assert_eq!(b64(b"fo"), "Zm8=");
        assert_eq!(b64(b"foo"), "Zm9v");
        assert_eq!(b64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn styles_cover_every_supported_tag() {
        for t in ["whisper", "shout", "excited", "sad", "tired"] {
            assert_ne!(style(t), ("Medium", "Medium"), "{t}");
        }
    }

    /// Runs against the real speech engine when this machine has voices (Windows).
    #[test]
    fn passes_the_adapter_contract_on_this_machine() {
        let a = SapiAdapter::new();
        let Some(v) = a.list_voices().into_iter().find(|v| v.language.starts_with("en")) else {
            eprintln!("no English Windows voice installed; skipping");
            return;
        };
        contract::run(&a, &v, "sneeze");
    }

    #[test]
    fn unknown_voice_name_is_never_passed_to_the_engine() {
        let a = SapiAdapter::new();
        let Some(v) = a.list_voices().into_iter().next() else { return };
        let req = RenderRequest {
            tokens: crate::tags::tokenize("hi").0,
            voice_id: "sapi:'; Remove-Item C:\\ ; '".into(),
            emotion: "neutral".into(),
            pace: 1.0,
            language: v.language,
        };
        assert!(matches!(a.render(&req), Err(AdapterError::UnknownVoice(_))));
    }
}
