//! Whisper on this computer, through the `whisper-cli.exe` program that ships with Soundstage (whisper.cpp, MIT licence).
//!
//! Nothing is sent anywhere. The recording is written to a private temporary folder, the program is started with an argument
//! list (never a shell), and its JSON answer is read back. The speech models are large, so they are not in the installer:
//! the person downloads one inside Soundstage, and each is checked against a fixed size and SHA-256 before it is used.

use crate::adapters::piper::Runner;
use crate::licence::{Licence, LicenceInfo};
use crate::providers::*;
use crate::transcribe::{Transcript, Word};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub struct ModelSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    pub file: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
    pub url: &'static str,
    pub english_only: bool,
}

/// Fixed models. Sizes and hashes were read from the host on 2026-10-24; a file that does not match is deleted, never used.
pub const MODELS: &[ModelSpec] = &[
    ModelSpec { id: "tiny.en", label: "Fast, English only", blurb: "Quickest and smallest. Fine for clear speech; it can miss names.", file: "ggml-tiny.en.bin", bytes: 77_704_715, sha256: "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f", url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin", english_only: true },
    ModelSpec { id: "base.en", label: "Better, English only", blurb: "More accurate, still quick on most computers. A good first choice for English.", file: "ggml-base.en.bin", bytes: 147_964_211, sha256: "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002", url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin", english_only: true },
    ModelSpec { id: "tiny", label: "Fast, many languages", blurb: "Understands about a hundred languages, with less accuracy in the smaller ones.", file: "ggml-tiny.bin", bytes: 77_691_713, sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21", url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin", english_only: false },
    ModelSpec { id: "base", label: "Better, many languages", blurb: "The best choice here for languages other than English. Still weaker in smaller languages such as Yoruba, Igbo and Hausa.", file: "ggml-base.bin", bytes: 147_951_465, sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe", url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin", english_only: false },
];

/// Hosts a model may be fetched from, including the file hosts the first address hands off to.
pub const MODEL_HOSTS: &[&str] = &["huggingface.co", ".huggingface.co", "hf.co", ".hf.co"];

pub fn model(id: &str) -> Option<&'static ModelSpec> {
    MODELS.iter().find(|m| m.id == id)
}

pub fn model_path(dir: &Path, m: &ModelSpec) -> PathBuf {
    dir.join(m.file)
}

/// A model counts as installed only when the file has exactly the expected size. Its hash was checked when it was downloaded.
pub fn is_installed(dir: &Path, m: &ModelSpec) -> bool {
    std::fs::metadata(model_path(dir, m)).is_ok_and(|f| f.is_file() && f.len() == m.bytes)
}

/// The best installed model for a language: English-only models only for English, and larger before smaller.
pub fn choose_model(dir: &Path, language: Option<&str>) -> Result<&'static ModelSpec, String> {
    let english = matches!(language, None | Some("") | Some("en") | Some("auto"));
    let order: &[&str] = if english { &["base.en", "tiny.en", "base", "tiny"] } else { &["base", "tiny"] };
    order.iter().filter_map(|id| model(id)).find(|m| is_installed(dir, m)).ok_or_else(|| {
        if english {
            "Download an offline speech model first (Audio tools, Offline transcription).".to_string()
        } else {
            "That language needs a many-languages model. Download one in Audio tools, Offline transcription.".to_string()
        }
    })
}

fn clean(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect::<String>().trim().chars().take(80).collect()
}

/// whisper.cpp's JSON: `{ result: { language }, transcription: [{ offsets: { from, to }, text }] }`, one entry per word when
/// started with one-word segments. Empty pieces and bracketed markers such as `[BLANK_AUDIO]` are dropped.
pub fn parse_output(v: &serde_json::Value) -> Result<Transcript, String> {
    let items = v["transcription"].as_array().ok_or("The transcript was not in the expected format.")?;
    let mut words = Vec::new();
    for it in items {
        let text = clean(it["text"].as_str().unwrap_or(""));
        let (from, to) = (it["offsets"]["from"].as_f64(), it["offsets"]["to"].as_f64());
        let marker = (text.starts_with('[') && text.ends_with(']')) || (text.starts_with('(') && text.ends_with(')'));
        if text.is_empty() || marker {
            continue;
        }
        if let (Some(a), Some(b)) = (from, to) {
            if a >= 0.0 && b >= a {
                words.push(Word { text, start: (a / 1000.0) as f32, end: (b / 1000.0) as f32 });
            }
        }
    }
    let text = words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
    let language = v["result"]["language"].as_str().filter(|l| (2..=8).contains(&l.len()) && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')).unwrap_or("").to_string();
    Ok(Transcript { text, language, words })
}

pub struct WhisperAdapter {
    exe: PathBuf,
    models: PathBuf,
    runner: Arc<dyn Runner>,
}

impl WhisperAdapter {
    pub fn new(exe: PathBuf, models: PathBuf, runner: Arc<dyn Runner>) -> Self {
        WhisperAdapter { exe, models, runner }
    }

    fn installed_any(&self) -> bool {
        MODELS.iter().any(|m| is_installed(&self.models, m))
    }
}

impl Adapter for WhisperAdapter {
    fn id(&self) -> &str {
        "whisper-local"
    }
    fn label(&self) -> &str {
        "Whisper (on this computer)"
    }
    fn licence(&self) -> LicenceInfo {
        licence_info(Licence::Commercial, "https://github.com/ggml-org/whisper.cpp (MIT) and https://github.com/openai/whisper (MIT)", "2026-10-24", "Whisper's code and weights are MIT licensed. A transcript of your own recording carries no third-party terms.")
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities { languages: vec![], tags: vec![], voice_design: false, cloning: false, max_chars_per_request: 1, streaming: false }
    }
    fn list_voices(&self) -> Vec<Voice> {
        Vec::new()
    }
    fn provider_version(&self) -> String {
        "whisper.cpp/v1.9.2".into()
    }
    fn render(&self, _req: &RenderRequest) -> Result<RenderOutput, AdapterError> {
        Err(AdapterError::ProviderError("Whisper turns speech into text; it cannot speak".into()))
    }
    fn ready(&self) -> bool {
        self.exe.is_file() && self.installed_any()
    }
    fn can_transcribe(&self) -> bool {
        self.ready()
    }
    fn transcribe(&self, wav: &[u8], language: Option<&str>) -> Result<Transcript, AdapterError> {
        let fail = |m: &str| AdapterError::ProviderError(m.to_string());
        if wav.len() < 1000 || wav.len() > 200 * 1024 * 1024 || !wav.starts_with(b"RIFF") {
            return Err(fail("the recording is not valid"));
        }
        if !self.exe.is_file() {
            return Err(fail("the offline speech program is missing. Reinstall Soundstage"));
        }
        let lang = language.map(str::trim).filter(|l| (2..=3).contains(&l.len()) && l.bytes().all(|b| b.is_ascii_lowercase()));
        let spec = choose_model(&self.models, lang).map_err(|m| AdapterError::ProviderError(m))?;
        // English-only models are always told English; others detect the language unless one was named.
        let lang_arg = if spec.english_only { "en" } else { lang.unwrap_or("auto") };

        let dir = std::env::temp_dir().join(crate::util::new_id("whisper"));
        std::fs::create_dir_all(&dir).map_err(|_| fail("could not make a temporary folder"))?;
        let result = (|| {
            let input = dir.join("in.wav");
            std::fs::write(&input, wav).map_err(|_| fail("could not write the recording"))?;
            let prefix = dir.join("out");
            let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
            let args: Vec<String> = [
                "-m", &model_path(&self.models, spec).to_string_lossy(), "-f", &input.to_string_lossy(), "-oj", "-of", &prefix.to_string_lossy(),
                "-ml", "1", "-sow", "-np", "-t", &threads.to_string(), "-l", lang_arg,
            ]
            .iter()
            .map(|s| s.to_string())
            .collect();
            let (ok, err) = self.runner.run(&self.exe, &args, &[], Duration::from_secs(45 * 60)).map_err(|e| AdapterError::ProviderError(e))?;
            let json = dir.join("out.json");
            if !ok || !json.is_file() {
                let line = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("it did not finish");
                return Err(AdapterError::ProviderError(format!("the offline speech program failed: {}", line.chars().take(160).collect::<String>())));
            }
            if std::fs::metadata(&json).map_or(true, |m| m.len() > 32 * 1024 * 1024) {
                return Err(fail("the transcript was larger than expected"));
            }
            let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&json).map_err(|_| fail("could not read the transcript"))?).map_err(|_| fail("the transcript was not understood"))?;
            parse_output(&v).map_err(AdapterError::ProviderError)
        })();
        let _ = std::fs::remove_dir_all(&dir);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Fake {
        calls: Mutex<Vec<Vec<String>>>,
        json: String,
        ok: bool,
    }
    impl Runner for Fake {
        fn run(&self, _exe: &Path, args: &[String], _stdin: &[u8], _t: Duration) -> Result<(bool, String), String> {
            self.calls.lock().unwrap().push(args.to_vec());
            let prefix = args[args.iter().position(|a| a == "-of").unwrap() + 1].clone();
            if self.ok {
                std::fs::write(format!("{prefix}.json"), &self.json).unwrap();
            }
            Ok((self.ok, "whisper_init: failed to load model\n".into()))
        }
    }

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(crate::util::new_id("whisper-test"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn install(dir: &Path, id: &str) {
        let m = model(id).unwrap();
        let f = std::fs::File::create(model_path(dir, m)).unwrap();
        f.set_len(m.bytes).unwrap();
    }

    const OUT: &str = r#"{"result":{"language":"en"},"transcription":[{"offsets":{"from":0,"to":150},"text":""},{"offsets":{"from":150,"to":360},"text":" Good"},{"offsets":{"from":360,"to":900},"text":" morning."},{"offsets":{"from":900,"to":1200},"text":" [BLANK_AUDIO]"},{"offsets":{"from":1200,"to":1500},"text":" Ama"}]}"#;

    fn wav() -> Vec<u8> {
        let mut w = b"RIFF".to_vec();
        w.extend(vec![1u8; 4000]);
        w
    }

    fn adapter(dir: &Path, fake: Arc<Fake>) -> (WhisperAdapter, PathBuf) {
        let exe = dir.join("whisper-cli.exe");
        std::fs::write(&exe, b"x").unwrap();
        (WhisperAdapter::new(exe.clone(), dir.to_path_buf(), fake), exe)
    }

    #[test]
    fn output_is_read_into_words_and_markers_are_dropped() {
        let t = parse_output(&serde_json::from_str(OUT).unwrap()).unwrap();
        assert_eq!(t.text, "Good morning. Ama");
        assert_eq!(t.words.len(), 3);
        assert_eq!(t.language, "en");
        assert!((t.words[0].start - 0.15).abs() < 1e-6 && (t.words[1].end - 0.9).abs() < 1e-6);
        assert!(parse_output(&serde_json::json!({"nope": 1})).is_err());
    }

    #[test]
    fn it_is_ready_only_with_the_program_and_a_model_of_the_exact_size() {
        let d = tmp();
        let fake = Arc::new(Fake { calls: Default::default(), json: OUT.into(), ok: true });
        let (a, _) = adapter(&d, fake);
        assert!(!a.ready() && !a.can_transcribe());
        std::fs::write(model_path(&d, model("tiny.en").unwrap()), b"truncated download").unwrap();
        assert!(!a.ready(), "a model with the wrong size is not installed");
        install(&d, "tiny.en");
        assert!(a.ready());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn the_right_model_is_chosen_for_the_language() {
        let d = tmp();
        install(&d, "tiny.en");
        assert_eq!(choose_model(&d, None).unwrap().id, "tiny.en");
        assert!(choose_model(&d, Some("fr")).is_err(), "an English-only model is never used for French");
        install(&d, "base.en");
        assert_eq!(choose_model(&d, Some("en")).unwrap().id, "base.en");
        install(&d, "tiny");
        assert_eq!(choose_model(&d, Some("yo")).unwrap().id, "tiny");
        install(&d, "base");
        assert_eq!(choose_model(&d, Some("yo")).unwrap().id, "base");
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn the_program_is_started_with_an_argument_list_and_the_temporary_folder_is_removed() {
        let d = tmp();
        install(&d, "base.en");
        let fake = Arc::new(Fake { calls: Default::default(), json: OUT.into(), ok: true });
        let (a, _) = adapter(&d, fake.clone());
        let t = a.transcribe(&wav(), Some("en")).unwrap();
        assert_eq!(t.words.len(), 3);
        let args = fake.calls.lock().unwrap()[0].clone();
        assert!(args.windows(2).any(|w| w[0] == "-l" && w[1] == "en") && args.contains(&"-oj".to_string()) && args.contains(&"-ml".to_string()));
        assert!(args.iter().any(|a| a.ends_with("ggml-base.en.bin")));
        let work = PathBuf::from(&args[args.iter().position(|a| a == "-f").unwrap() + 1]).parent().unwrap().to_path_buf();
        assert!(!work.exists(), "the private temporary folder is removed");
        // An odd language code is ignored rather than passed on.
        a.transcribe(&wav(), Some("e n; calc")).unwrap();
        let args2 = fake.calls.lock().unwrap()[1].clone();
        assert!(args2.windows(2).any(|w| w[0] == "-l" && w[1] == "en"));
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn failures_are_plain_and_bad_input_never_starts_the_program() {
        let d = tmp();
        install(&d, "tiny.en");
        let bad = Arc::new(Fake { calls: Default::default(), json: String::new(), ok: false });
        let (a, _) = adapter(&d, bad.clone());
        let e = a.transcribe(&wav(), None).unwrap_err().to_string();
        assert!(e.contains("failed to load model"), "{e}");
        assert!(a.transcribe(b"nope", None).is_err());
        assert_eq!(bad.calls.lock().unwrap().len(), 1, "only the valid recording started the program");
        let none = WhisperAdapter::new(d.join("missing.exe"), d.clone(), bad);
        assert!(!none.ready());
        assert!(none.transcribe(&wav(), None).is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn the_model_list_is_consistent() {
        for m in MODELS {
            assert!(m.url.starts_with("https://huggingface.co/") && m.url.ends_with(m.file));
            assert_eq!(m.sha256.len(), 64);
            assert!(m.bytes > 50_000_000 && m.bytes < 500_000_000);
            assert_eq!(m.english_only, m.id.ends_with(".en"));
        }
        let ids: std::collections::BTreeSet<_> = MODELS.iter().map(|m| m.id).collect();
        assert_eq!(ids.len(), MODELS.len());
    }
}
