//! Voice engines the user adds themselves: local servers (Kokoro, Chatterbox, LocalAI, Speaches and anything else that
//! speaks the OpenAI speech API), OpenRouter, and more. Soundstage cannot know what each engine's model can do, so
//! the user declares the voices and which direction tags the engine understands. Those claims are the user's, and the
//! interface says so. Licence always starts as `unknown`, because it depends on the model weights being run.
//!
//! The config file never holds secrets. Keys live in the Windows Credential Manager under `engine:<id>`.

use crate::net::parse_base_url;
use crate::tags::{DELIVERY, NON_VERBAL};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const MAX_ENGINES: usize = 24;
pub const MAX_VOICES: usize = 300;
pub const CONFIG_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct VoiceDef {
    pub id: String,
    pub name: String,
    /// BCP 47, for example `en-US` or `yo-NG`.
    pub language: String,
    pub accent: String,
    pub gender: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SpeechEngine {
    pub id: String,
    pub label: String,
    pub base_url: String,
    pub model: String,
    pub voices: Vec<VoiceDef>,
    /// `wav`, `mp3`, `flac`, `opus` or `aac`. Soundstage decodes all of them.
    pub response_format: String,
    /// Canonical tags the engine understands. Declared by the user, not tested by Soundstage.
    pub tags: Vec<String>,
    /// How a canonical tag is written for this engine. A missing entry means `[tag]`.
    pub tag_map: BTreeMap<String, String>,
    pub needs_key: bool,
    pub licence_note: String,
    pub notes: String,
    /// Optional model name for `/audio/transcriptions` on the same server (a Whisper model, for example).
    pub transcribe_model: String,
}

impl Default for SpeechEngine {
    fn default() -> Self {
        SpeechEngine {
            id: String::new(), label: String::new(), base_url: String::new(), model: String::new(), voices: vec![], response_format: "wav".into(),
            tags: vec![], tag_map: BTreeMap::new(), needs_key: false, licence_note: String::new(), notes: String::new(), transcribe_model: String::new(),
        }
    }
}

/// A job-style engine: fal, Kie (including Suno) or ComfyUI. The user supplies the model and its input template,
/// because input fields differ per model. For ComfyUI the template is the workflow saved in API format.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct JobEngine {
    pub id: String,
    pub label: String,
    /// `fal`, `kie` or `comfy`.
    pub provider: String,
    /// `speech`, `music` or `sfx`.
    pub kind: String,
    /// The fal model id or Kie model name. Not used for ComfyUI.
    pub model: String,
    /// ComfyUI only.
    pub base_url: String,
    pub template: String,
    pub voices: Vec<VoiceDef>,
    pub licence_note: String,
    pub min_s: f32,
    pub max_s: f32,
    pub default_s: f32,
}

impl Default for JobEngine {
    fn default() -> Self {
        JobEngine {
            id: String::new(), label: String::new(), provider: "fal".into(), kind: "music".into(), model: String::new(), base_url: "http://127.0.0.1:8188".into(),
            template: String::new(), voices: vec![], licence_note: String::new(), min_s: 3.0, max_s: 300.0, default_s: 30.0,
        }
    }
}

impl JobEngine {
    pub fn validate(&self) -> Result<(), String> {
        let name = if self.label.is_empty() { self.id.clone() } else { self.label.clone() };
        if !valid_engine_id(&self.id) {
            return Err("An engine's short name may only use lower-case letters, numbers and hyphens.".into());
        }
        if self.label.trim().is_empty() || !clean_text(&self.label, 60) {
            return Err(format!("“{}” needs a name of up to 60 characters.", self.id));
        }
        if !["speech", "music", "sfx"].contains(&self.kind.as_str()) {
            return Err(format!("{name}: choose speech, music or sound effects."));
        }
        let comfy = self.provider == "comfy";
        match self.provider.as_str() {
            "fal" if !crate::adapters::fal::valid_model_id(&self.model) => return Err(format!("{name}: a fal model id looks like fal-ai/some-model.")),
            "kie" if !crate::adapters::kie::valid_model(&self.model) => return Err(format!("{name}: enter the Kie model name.")),
            "comfy" => {
                parse_base_url(&self.base_url).map_err(|e| format!("{name}: {e}"))?;
            }
            "fal" | "kie" => {}
            _ => return Err(format!("{name}: the provider must be fal, Kie or ComfyUI.")),
        }
        let needs = if self.kind == "speech" { "text" } else { "prompt" };
        let tpl = crate::jobs::parse_template(&self.template, needs, if comfy { crate::jobs::MAX_WORKFLOW_BYTES } else { crate::jobs::MAX_TEMPLATE_BYTES }).map_err(|e| format!("{name}: {e}"))?;
        if comfy {
            crate::adapters::comfy::validate_workflow(&tpl).map_err(|e| format!("{name}: {e}"))?;
        }
        if self.voices.len() > MAX_VOICES {
            return Err(format!("{name}: too many voices (limit {MAX_VOICES})."));
        }
        let mut seen = BTreeSet::new();
        for v in &self.voices {
            v.validate().map_err(|e| format!("{name}: {e}"))?;
            if !seen.insert(v.id.as_str()) {
                return Err(format!("{name}: the voice “{}” is listed twice.", v.id));
            }
        }
        if !clean_text(&self.licence_note, 300) {
            return Err(format!("{name}: notes are limited to 300 characters."));
        }
        if !(self.min_s.is_finite() && self.max_s.is_finite() && self.default_s.is_finite()) || self.min_s <= 0.0 || self.min_s > self.default_s || self.default_s > self.max_s || self.max_s > 3600.0 {
            return Err(format!("{name}: the length limits are not valid."));
        }
        Ok(())
    }

    pub fn limits(&self) -> crate::generate::GenLimits {
        crate::generate::GenLimits { min_s: self.min_s, max_s: self.max_s, default_s: self.default_s, supports_length: crate::jobs::template_uses(&serde_json::from_str(&self.template).unwrap_or_default(), "seconds") }
    }
}

/// Where the user's own Piper program and voices are.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PiperConfig {
    pub exe: String,
    pub voices_dir: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EnginesConfig {
    pub version: u32,
    pub speech: Vec<SpeechEngine>,
    pub piper: Option<PiperConfig>,
    pub jobs: Vec<JobEngine>,
}

impl Default for EnginesConfig {
    fn default() -> Self {
        EnginesConfig { version: CONFIG_VERSION, speech: vec![], piper: None, jobs: vec![] }
    }
}

pub fn valid_engine_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 40 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') && !id.starts_with('-') && !id.ends_with('-')
}

fn clean_text(s: &str, max: usize) -> bool {
    s.chars().count() <= max && !s.chars().any(|c| c.is_control())
}

fn valid_language(l: &str) -> bool {
    let mut parts = l.split(['-', '_']);
    let first = parts.next().unwrap_or("");
    (2..=3).contains(&first.len()) && first.bytes().all(|b| b.is_ascii_alphabetic()) && parts.all(|p| (2..=8).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphanumeric())) && l.len() <= 20
}

impl VoiceDef {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() || !clean_text(&self.id, 80) || !clean_text(&self.name, 80) || !clean_text(&self.accent, 40) || !clean_text(&self.gender, 40) {
            return Err(format!("The voice “{}” has an empty or invalid field.", self.id));
        }
        if !self.language.is_empty() && !valid_language(&self.language) {
            return Err(format!("The voice “{}” has a language code that is not valid. Use something like en-US.", self.id));
        }
        Ok(())
    }
}

impl SpeechEngine {
    pub fn validate(&self) -> Result<(), String> {
        let name = if self.label.is_empty() { &self.id } else { &self.label };
        if !valid_engine_id(&self.id) {
            return Err("An engine's short name may only use lower-case letters, numbers and hyphens.".into());
        }
        if self.label.trim().is_empty() || !clean_text(&self.label, 60) {
            return Err(format!("“{}” needs a name of up to 60 characters.", self.id));
        }
        parse_base_url(&self.base_url).map_err(|e| format!("{name}: {e}"))?;
        if self.model.trim().is_empty() || !clean_text(&self.model, 120) {
            return Err(format!("{name}: enter the model name the server expects."));
        }
        if !self.transcribe_model.is_empty() && !clean_text(&self.transcribe_model, 120) {
            return Err(format!("{name}: the transcription model name is not valid."));
        }
        if !["wav", "mp3", "flac", "opus", "aac"].contains(&self.response_format.as_str()) {
            return Err(format!("{name}: the audio format must be wav, mp3, flac, opus or aac."));
        }
        if self.voices.len() > MAX_VOICES {
            return Err(format!("{name}: too many voices (limit {MAX_VOICES})."));
        }
        let mut seen = BTreeSet::new();
        for v in &self.voices {
            v.validate().map_err(|e| format!("{name}: {e}"))?;
            if !seen.insert(v.id.as_str()) {
                return Err(format!("{name}: the voice “{}” is listed twice.", v.id));
            }
        }
        for t in &self.tags {
            if !NON_VERBAL.contains(&t.as_str()) && !DELIVERY.contains(&t.as_str()) {
                return Err(format!("{name}: “{t}” is not a Soundstage tag."));
            }
        }
        for (k, v) in &self.tag_map {
            if !self.tags.contains(k) || v.trim().is_empty() || !clean_text(v, 40) {
                return Err(format!("{name}: the written form of [{k}] is not valid."));
            }
        }
        if !clean_text(&self.licence_note, 300) || !clean_text(&self.notes, 300) {
            return Err(format!("{name}: notes are limited to 300 characters."));
        }
        Ok(())
    }
}

impl EnginesConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.speech.len() + self.jobs.len() > MAX_ENGINES {
            return Err(format!("Too many engines (limit {MAX_ENGINES})."));
        }
        let mut ids = BTreeSet::new();
        for e in &self.speech {
            e.validate()?;
            if !ids.insert(e.id.as_str()) {
                return Err(format!("Two engines are named “{}”.", e.id));
            }
        }
        for j in &self.jobs {
            j.validate()?;
            if !ids.insert(j.id.as_str()) {
                return Err(format!("Two engines are named “{}”.", j.id));
            }
        }
        if let Some(p) = &self.piper {
            if !clean_text(&p.exe, 400) || !clean_text(&p.voices_dir, 400) {
                return Err("The Piper paths are not valid.".into());
            }
        }
        Ok(())
    }

    /// A missing file is an empty config. A damaged one is an error the UI can show, never a crash.
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(t) => {
                if t.len() > 2_000_000 {
                    return Err("The engines file is too large.".into());
                }
                let c: EnginesConfig = serde_json::from_str(&t).map_err(|e| format!("The engines file is damaged: {e}"))?;
                if c.version > CONFIG_VERSION {
                    return Err("The engines file was saved by a newer version of Soundstage.".into());
                }
                c.validate()?;
                Ok(c)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(EnginesConfig::default()),
            Err(e) => Err(format!("Could not read the engines file: {e}")),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self).map_err(|e| e.to_string())?).map_err(|e| format!("Could not save the engines: {e}"))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("Could not save the engines: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn engine() -> SpeechEngine {
        SpeechEngine {
            id: "kokoro".into(), label: "Kokoro (local)".into(), base_url: "http://127.0.0.1:8880/v1".into(), model: "kokoro".into(),
            voices: vec![VoiceDef { id: "af_bella".into(), name: "Bella".into(), language: "en-US".into(), accent: "American".into(), gender: "female".into() }],
            ..Default::default()
        }
    }

    #[test]
    fn a_good_engine_validates_and_round_trips_through_disk() {
        let mut c = EnginesConfig::default();
        c.speech.push(engine());
        c.validate().unwrap();
        let p = std::env::temp_dir().join(crate::util::new_id("ss-eng")).join("engines.json");
        c.save(&p).unwrap();
        assert_eq!(EnginesConfig::load(&p).unwrap(), c);
        assert_eq!(EnginesConfig::load(&p.with_file_name("missing.json")).unwrap(), EnginesConfig::default());
        std::fs::remove_dir_all(p.parent().unwrap()).ok();
    }

    #[test]
    fn bad_engines_are_refused_with_a_reason() {
        let cases: Vec<(&str, Box<dyn Fn(&mut SpeechEngine)>)> = vec![
            ("name", Box::new(|e| e.id = "Bad Id!".into())),
            ("address", Box::new(|e| e.base_url = "http://example.com/v1".into())),
            ("model", Box::new(|e| e.model = "".into())),
            ("format", Box::new(|e| e.response_format = "exe".into())),
            ("tag", Box::new(|e| e.tags = vec!["explode".into()])),
            ("tag map", Box::new(|e| { e.tags = vec!["laugh".into()]; e.tag_map.insert("cough".into(), "[cough]".into()); })),
            ("voice", Box::new(|e| e.voices[0].language = "english!".into())),
            ("dup", Box::new(|e| e.voices.push(e.voices[0].clone()))),
            ("control", Box::new(|e| e.label = "a\nb".into())),
        ];
        for (name, f) in cases {
            let mut e = engine();
            f(&mut e);
            assert!(e.validate().is_err(), "{name} should be refused");
        }
        let mut c = EnginesConfig::default();
        c.speech = vec![engine(), engine()];
        assert!(c.validate().unwrap_err().contains("Two engines"));
    }

    #[test]
    fn damaged_or_newer_files_are_errors_not_crashes() {
        let d = std::env::temp_dir().join(crate::util::new_id("ss-eng2"));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("a.json"), "{ nope").unwrap();
        assert!(EnginesConfig::load(&d.join("a.json")).unwrap_err().contains("damaged"));
        std::fs::write(d.join("b.json"), r#"{"version": 99, "speech": []}"#).unwrap();
        assert!(EnginesConfig::load(&d.join("b.json")).unwrap_err().contains("newer"));
        std::fs::remove_dir_all(d).ok();
    }

    fn job() -> JobEngine {
        JobEngine { id: "suno".into(), label: "Suno via Kie".into(), provider: "kie".into(), kind: "music".into(), model: "ai-music-api/generate".into(),
            template: r#"{"custom_mode":false,"instrumental":"{{instrumental}}","model":"V6","prompt":"{{prompt}}"}"#.into(), ..Default::default() }
    }

    #[test]
    fn job_engines_are_validated_per_provider() {
        job().validate().unwrap();
        let mut fal = JobEngine { provider: "fal".into(), model: "fal-ai/lyria2".into(), template: r#"{"prompt":"{{prompt}}"}"#.into(), ..job() };
        fal.validate().unwrap();
        fal.model = "not a model".into();
        assert!(fal.validate().unwrap_err().contains("fal model id"));
        let comfy = JobEngine { provider: "comfy".into(), kind: "speech".into(), model: String::new(), template: r#"{"1":{"class_type":"X","inputs":{"t":"{{text}}"}}}"#.into(), ..job() };
        comfy.validate().unwrap();
        for bad in [
            JobEngine { template: r#"{"prompt":"hello"}"#.into(), ..job() },
            JobEngine { template: "nope".into(), ..job() },
            JobEngine { kind: "video".into(), ..job() },
            JobEngine { provider: "other".into(), ..job() },
            JobEngine { model: "a b".into(), ..job() },
            JobEngine { min_s: 0.0, ..job() },
            JobEngine { max_s: 99_999.0, ..job() },
            JobEngine { base_url: "http://example.com".into(), ..comfy.clone() },
            JobEngine { template: r#"{"1":{"inputs":{"t":"{{text}}"}}}"#.into(), ..comfy.clone() },
        ] {
            assert!(bad.validate().is_err(), "{bad:?}");
        }
        let mut c = EnginesConfig::default();
        c.jobs = vec![job()];
        c.speech = vec![engine()];
        c.validate().unwrap();
        c.speech[0].id = "suno".into();
        assert!(c.validate().unwrap_err().contains("Two engines"), "ids are unique across every kind of engine");
    }

    #[test]
    fn ids_for_engine_keys_are_safe() {
        for ok in ["kokoro", "my-server-2"] { assert!(valid_engine_id(ok)); }
        for bad in ["", "-a", "a-", "A", "a b", "a:b", "../x", &"a".repeat(41)] { assert!(!valid_engine_id(bad), "{bad}"); }
    }
}
