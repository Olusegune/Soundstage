//! Piper: a fast, offline, CPU-only neural voice engine. Soundstage does not ship it. The user points Soundstage at their
//! own `piper` program and a folder of voices (`*.onnx` with a matching `*.onnx.json`).
//!
//! Piper voices each have their own licence (many are CC0 or CC-BY, some are not), so every voice starts as
//! `unknown`. Piper has no direction tags, so a tag is "not supported by this voice" and pauses are exact silence.
//! The program is started with an argument list (never a shell); text goes in on stdin; only voices found by
//! scanning the voices folder can be used.
//!
//! VERIFY: the command line below (`--model`, `--output_file`, `--length_scale`) is the one the original Piper
//! releases document. Piper's newer packaging may differ; failures surface as a plain error, not a guess.

use crate::engines::PiperConfig;
use crate::licence::{Licence, LicenceInfo};
use crate::providers::*;
use crate::audio::resample;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Runs a program. Real in the app, replaced in tests.
pub trait Runner: Send + Sync {
    fn run(&self, exe: &Path, args: &[String], stdin: &[u8], timeout: Duration) -> Result<(bool, String), String>;
}

pub struct RealRunner;

impl Runner for RealRunner {
    fn run(&self, exe: &Path, args: &[String], stdin: &[u8], timeout: Duration) -> Result<(bool, String), String> {
        let mut cmd = Command::new(exe);
        cmd.args(args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let mut child = cmd.spawn().map_err(|e| format!("could not start {}: {e}", exe.display()))?;
        if let Some(mut si) = child.stdin.take() {
            let data = stdin.to_vec();
            // Written on its own thread so a program that is slow to read cannot deadlock us.
            std::thread::spawn(move || {
                let _ = si.write_all(&data);
            });
        }
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if start.elapsed() > timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("the voice program took too long and was stopped".into());
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(25)),
                Err(e) => return Err(e.to_string()),
            }
        }
        let mut err = String::new();
        if let Some(mut e) = child.stderr.take() {
            let _ = e.by_ref().take(64 * 1024).read_to_string(&mut err);
        }
        Ok((child.wait().map(|s| s.success()).unwrap_or(false), err))
    }
}

#[derive(Clone, Debug)]
struct PVoice {
    stem: String,
    name: String,
    language: String,
    model: PathBuf,
}

pub struct PiperAdapter {
    exe: PathBuf,
    dir: PathBuf,
    runner: Arc<dyn Runner>,
    voices: Mutex<Option<Vec<PVoice>>>,
}

fn safe_stem(s: &str) -> bool {
    !s.is_empty() && s.len() <= 80 && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// `en_US-lessac-medium` becomes `en-US`; a voice file's own JSON wins when it says.
fn locale_from(stem: &str, json: &serde_json::Value) -> String {
    let raw = json["language"]["code"].as_str().map(String::from).or_else(|| stem.split('-').next().map(String::from)).unwrap_or_default();
    let l = raw.replace('_', "-");
    let ok = {
        let mut p = l.split('-');
        p.next().is_some_and(|a| (2..=3).contains(&a.len()) && a.bytes().all(|b| b.is_ascii_alphabetic())) && p.all(|x| (2..=8).contains(&x.len()) && x.bytes().all(|b| b.is_ascii_alphanumeric()))
    };
    if ok && l.len() <= 20 { l } else { "en".into() }
}

fn pretty(stem: &str) -> String {
    let parts: Vec<&str> = stem.split('-').collect();
    match parts.as_slice() {
        [_, name, quality] => format!("{} ({})", name.replace('_', " "), quality),
        [_, name] => name.replace('_', " "),
        _ => stem.to_string(),
    }
}

impl PiperAdapter {
    pub fn new(cfg: &PiperConfig, runner: Arc<dyn Runner>) -> Result<Self, String> {
        let exe = PathBuf::from(cfg.exe.trim());
        if !exe.is_file() || !exe.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case("piper")) {
            return Err("Choose the piper program (piper.exe).".into());
        }
        let dir = PathBuf::from(cfg.voices_dir.trim());
        if !dir.is_dir() {
            return Err("Choose the folder that holds your Piper voices.".into());
        }
        Ok(PiperAdapter { exe, dir, runner, voices: Mutex::new(None) })
    }

    fn scan(&self) -> Vec<PVoice> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else { return vec![] };
        let mut out: Vec<PVoice> = rd
            .flatten()
            .take(2000)
            .filter_map(|e| {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("onnx") {
                    return None;
                }
                let stem = p.file_stem()?.to_str()?.to_string();
                if !safe_stem(&stem) {
                    return None;
                }
                let json_path = PathBuf::from(format!("{}.json", p.display()));
                let meta = std::fs::metadata(&json_path).ok()?;
                if meta.len() > 1_000_000 {
                    return None;
                }
                let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json_path).ok()?).ok()?;
                Some(PVoice { name: pretty(&stem), language: locale_from(&stem, &json), stem, model: p })
            })
            .collect();
        out.sort_by(|a, b| a.stem.cmp(&b.stem));
        out
    }

    fn voices(&self) -> Vec<PVoice> {
        let mut g = self.voices.lock().unwrap_or_else(|e| e.into_inner());
        g.get_or_insert_with(|| self.scan()).clone()
    }
}

impl Adapter for PiperAdapter {
    fn id(&self) -> &str {
        "piper"
    }
    fn label(&self) -> &str {
        "Piper (offline)"
    }
    fn licence(&self) -> LicenceInfo {
        LicenceInfo { licence: Licence::Unknown, source: None, checked: None, notes: "Each Piper voice has its own licence. Read the voice's model card and record your check.".into() }
    }
    fn capabilities(&self) -> Capabilities {
        let mut languages: Vec<String> = self.voices().iter().map(|v| v.language.clone()).collect();
        languages.sort();
        languages.dedup();
        Capabilities { languages, tags: vec![], voice_design: false, cloning: false, max_chars_per_request: 3000, streaming: false }
    }
    fn list_voices(&self) -> Vec<Voice> {
        self.voices()
            .into_iter()
            .map(|v| Voice {
                id: format!("piper:{}", v.stem),
                name: v.name,
                provider: "piper".into(),
                accent: v.language.split('-').nth(1).map(|r| r.to_uppercase()).unwrap_or_else(|| "Not stated".into()),
                language: v.language,
                gender: "Not stated".into(),
                age_range: "Not stated".into(),
                licence: self.licence(),
                sample_text: "Good morning. This is how I sound.".into(),
                notes: "Offline. Runs on this computer. No sighs, laughs or other directions. Speed and pauses only.".into(),
            })
            .collect()
    }
    fn provider_version(&self) -> String {
        "piper-cli".into()
    }
    fn check_connection(&self) -> Result<(), AdapterError> {
        match self.runner.run(&self.exe, &["--help".into()], b"", Duration::from_secs(20)) {
            Ok(_) => Ok(()),
            Err(m) => Err(AdapterError::ProviderError(m)),
        }
    }
    fn render(&self, req: &RenderRequest) -> Result<RenderOutput, AdapterError> {
        precheck(&self.capabilities(), req)?;
        let stem = req.voice_id.strip_prefix("piper:").unwrap_or("");
        let voice = self.voices().into_iter().find(|v| v.stem == stem).ok_or_else(|| AdapterError::UnknownVoice(req.voice_id.chars().take(60).collect()))?;
        let segs = compose_segments(&req.tokens, &req.emotion, &|_| None)?;
        let tmp = std::env::temp_dir().join(crate::util::new_id("soundstage-piper"));
        std::fs::create_dir_all(&tmp).map_err(|e| AdapterError::ProviderError(e.to_string()))?;
        let result = (|| {
            let mut pieces: Vec<Result<(Vec<f32>, u32), u32>> = Vec::new();
            for (i, s) in segs.into_iter().enumerate() {
                match s {
                    Err(ms) => pieces.push(Err(ms)),
                    Ok(text) => {
                        let out = tmp.join(format!("seg{i}.wav"));
                        let args = vec![
                            "--model".to_string(), voice.model.to_string_lossy().to_string(),
                            "--output_file".to_string(), out.to_string_lossy().to_string(),
                            "--length_scale".to_string(), format!("{:.2}", 1.0 / req.pace.clamp(0.5, 2.0)),
                        ];
                        let line = format!("{}\n", text.replace(['\r', '\n'], " "));
                        let (ok, err) = self.runner.run(&self.exe, &args, line.as_bytes(), Duration::from_secs(180)).map_err(AdapterError::ProviderError)?;
                        if !ok || !out.is_file() {
                            let first = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("piper failed");
                            return Err(AdapterError::ProviderError(format!("Piper could not speak this line: {}", first.chars().take(160).collect::<String>())));
                        }
                        let d = crate::audio::read_wav(&out).map_err(AdapterError::ProviderError)?;
                        pieces.push(Ok((d.channels.into_iter().next().unwrap_or_default(), d.rate)));
                    }
                }
            }
            let rate = pieces.iter().find_map(|p| p.as_ref().ok().map(|(_, r)| *r)).unwrap_or(22_050);
            let mut samples = Vec::new();
            for p in pieces {
                match p {
                    Ok((s, r)) => samples.extend(if r == rate { s } else { resample(&s, r, rate) }),
                    Err(ms) => samples.extend(std::iter::repeat(0.0).take((ms as usize * rate as usize) / 1000)),
                }
            }
            Ok(RenderOutput { samples, sample_rate: rate, provider_version: format!("piper/{}", voice.stem) })
        })();
        let _ = std::fs::remove_dir_all(&tmp);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::write_wav_16_mono;
    use crate::tags::tokenize;
    use crate::util::new_id;

    /// Pretends to be piper: records what it was asked and writes a tone to the --output_file path.
    #[derive(Default)]
    struct Fake {
        calls: Mutex<Vec<(Vec<String>, String)>>,
        fail: bool,
    }
    impl Runner for Fake {
        fn run(&self, _exe: &Path, args: &[String], stdin: &[u8], _t: Duration) -> Result<(bool, String), String> {
            self.calls.lock().unwrap().push((args.to_vec(), String::from_utf8_lossy(stdin).to_string()));
            if self.fail {
                return Ok((false, "error: model failed to load\n".into()));
            }
            if let Some(i) = args.iter().position(|a| a == "--output_file") {
                let words = String::from_utf8_lossy(stdin).split_whitespace().count().max(1);
                let tone: Vec<f32> = (0..words * 350 * 22_050 / 1000).map(|n| 0.3 * (n as f32 * 0.07).sin()).collect();
                write_wav_16_mono(Path::new(&args[i + 1]), &tone, 22_050).unwrap();
            }
            Ok((true, String::new()))
        }
    }

    fn setup() -> (PathBuf, PiperConfig) {
        let d = std::env::temp_dir().join(new_id("ss-piper"));
        std::fs::create_dir_all(d.join("voices dir")).unwrap();
        std::fs::write(d.join("piper.exe"), b"x").unwrap();
        for (stem, lang) in [("en_US-lessac-medium", "en_US"), ("yo_NG-example-low", "yo_NG")] {
            std::fs::write(d.join("voices dir").join(format!("{stem}.onnx")), b"model").unwrap();
            std::fs::write(d.join("voices dir").join(format!("{stem}.onnx.json")), format!(r#"{{"language":{{"code":"{lang}"}},"audio":{{"sample_rate":22050}}}}"#)).unwrap();
        }
        // Things that must be ignored: no json, bad name, damaged json.
        std::fs::write(d.join("voices dir").join("orphan.onnx"), b"m").unwrap();
        std::fs::write(d.join("voices dir").join("bad name!.onnx"), b"m").unwrap();
        std::fs::write(d.join("voices dir").join("bad name!.onnx.json"), b"{}").unwrap();
        std::fs::write(d.join("voices dir").join("broken.onnx"), b"m").unwrap();
        std::fs::write(d.join("voices dir").join("broken.onnx.json"), b"{ nope").unwrap();
        let cfg = PiperConfig { exe: d.join("piper.exe").to_string_lossy().to_string(), voices_dir: d.join("voices dir").to_string_lossy().to_string() };
        (d, cfg)
    }
    fn request(text: &str, voice: &str, pace: f32) -> RenderRequest {
        RenderRequest { tokens: tokenize(text).0, voice_id: voice.into(), emotion: "neutral".into(), pace, language: "en-US".into() }
    }

    #[test]
    fn voices_are_found_in_the_folder_and_odd_files_are_ignored() {
        let (d, cfg) = setup();
        let a = PiperAdapter::new(&cfg, Arc::new(Fake::default())).unwrap();
        let v = a.list_voices();
        assert_eq!(v.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), vec!["piper:en_US-lessac-medium", "piper:yo_NG-example-low"]);
        assert_eq!((v[0].language.as_str(), v[0].name.as_str(), v[0].accent.as_str()), ("en-US", "lessac (medium)", "US"));
        assert_eq!(v[1].language, "yo-NG");
        assert!(v.iter().all(|x| x.licence.licence == Licence::Unknown), "every voice has its own licence");
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn passes_the_adapter_contract() {
        let (d, cfg) = setup();
        let a = PiperAdapter::new(&cfg, Arc::new(Fake::default())).unwrap();
        contract::run(&a, &a.list_voices()[0], "sneeze");
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn the_command_line_is_exact_and_text_goes_in_on_stdin() {
        let (d, cfg) = setup();
        let fake = Arc::new(Fake::default());
        let a = PiperAdapter::new(&cfg, fake.clone()).unwrap();
        let out = a.render(&request("hello [pause:500] there world", "piper:en_US-lessac-medium", 2.0)).unwrap();
        let calls = fake.calls.lock().unwrap();
        assert_eq!(calls.len(), 2, "one run per spoken segment");
        let (args, stdin) = &calls[0];
        assert_eq!(args[0], "--model");
        assert!(args[1].ends_with("en_US-lessac-medium.onnx"));
        assert_eq!((args[2].as_str(), args[4].as_str(), args[5].as_str()), ("--output_file", "--length_scale", "0.50"));
        assert_eq!(stdin, "hello\n");
        assert_eq!(calls[1].1, "there world\n");
        let hello = 350 * 22_050 / 1000;
        assert!((out.samples.len() as i64 - (hello + 11_025 + 2 * hello) as i64).abs() < 5);
        assert_eq!(out.provider_version, "piper/en_US-lessac-medium");
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn tags_are_refused_before_piper_is_run_and_unknown_voices_are_never_passed_on() {
        let (d, cfg) = setup();
        let fake = Arc::new(Fake::default());
        let a = PiperAdapter::new(&cfg, fake.clone()).unwrap();
        assert_eq!(a.render(&request("hi [sigh]", "piper:en_US-lessac-medium", 1.0)).unwrap_err(), AdapterError::UnsupportedTag("sigh".into()));
        for id in ["piper:../../evil", "piper:nope", "piper:", "other:en_US-lessac-medium", "piper:orphan"] {
            assert!(matches!(a.render(&request("hi", id, 1.0)), Err(AdapterError::UnknownVoice(_))), "{id}");
        }
        assert!(fake.calls.lock().unwrap().is_empty());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn failures_become_a_plain_error_and_temp_files_are_cleaned_up() {
        let (d, cfg) = setup();
        let fake = Arc::new(Fake { fail: true, ..Default::default() });
        let a = PiperAdapter::new(&cfg, fake.clone()).unwrap();
        match a.render(&request("hello", "piper:en_US-lessac-medium", 1.0)) {
            Err(AdapterError::ProviderError(m)) => assert!(m.contains("model failed to load"), "{m}"),
            other => panic!("{other:?}"),
        }
        let args = fake.calls.lock().unwrap()[0].0.clone();
        let out = PathBuf::from(&args[args.iter().position(|x| x == "--output_file").unwrap() + 1]);
        assert!(!out.parent().unwrap().exists(), "this render's temporary folder must be removed even when it fails");
        // and on success too
        let ok = Arc::new(Fake::default());
        PiperAdapter::new(&cfg, ok.clone()).unwrap().render(&request("hello", "piper:en_US-lessac-medium", 1.0)).unwrap();
        let a2 = ok.calls.lock().unwrap()[0].0.clone();
        let out2 = PathBuf::from(&a2[a2.iter().position(|x| x == "--output_file").unwrap() + 1]);
        assert!(!out2.parent().unwrap().exists());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn the_program_and_folder_must_be_what_they_claim() {
        let (d, mut cfg) = setup();
        assert!(PiperAdapter::new(&cfg, Arc::new(Fake::default())).is_ok());
        std::fs::write(d.join("notpiper.exe"), b"x").unwrap();
        cfg.exe = d.join("notpiper.exe").to_string_lossy().to_string();
        assert!(PiperAdapter::new(&cfg, Arc::new(Fake::default())).is_err());
        cfg.exe = d.join("piper.exe").to_string_lossy().to_string();
        cfg.voices_dir = d.join("missing").to_string_lossy().to_string();
        assert!(PiperAdapter::new(&cfg, Arc::new(Fake::default())).is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[cfg(windows)]
    #[test]
    fn the_real_runner_reports_success_failure_and_timeouts() {
        let cmd = Path::new("C:\\Windows\\System32\\cmd.exe");
        if !cmd.is_file() { return; }
        let r = RealRunner;
        assert!(r.run(cmd, &["/C".into(), "exit 0".into()], b"", Duration::from_secs(10)).unwrap().0);
        assert!(!r.run(cmd, &["/C".into(), "exit 3".into()], b"", Duration::from_secs(10)).unwrap().0);
        assert!(r.run(cmd, &["/C".into(), "ping -n 6 127.0.0.1 >nul".into()], b"", Duration::from_millis(300)).unwrap_err().contains("too long"));
        assert!(r.run(Path::new("C:\\no\\such.exe"), &[], b"", Duration::from_secs(1)).is_err());
    }
}
