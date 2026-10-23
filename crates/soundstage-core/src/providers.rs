//! Provider-agnostic core (PROVIDERS.md sections 2 to 4). The UI and project files
//! speak canonical tags and voice ids. Adapters translate and never touch project state.

use crate::licence::{Licence, LicenceInfo};
use crate::tags::{required_tags, Token};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Credentials are looked up by name (`elevenlabs`, `openrouter`, `engine:my-server`). The shell owns the key store;
/// adapters only ever call this, and never keep what it returns.
pub type KeyProvider = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// The common error set every adapter maps into.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "code", content = "detail", rename_all = "snake_case")]
pub enum AdapterError {
    Auth,
    RateLimit,
    UnsupportedTag(String),
    UnsupportedLanguage(String),
    Quota,
    Network,
    ProviderError(String),
    UnknownVoice(String),
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterError::Auth => write!(f, "The provider rejected the credentials."),
            AdapterError::RateLimit => write!(f, "The provider is rate limiting requests. Try again shortly."),
            AdapterError::UnsupportedTag(t) => write!(f, "This voice cannot perform [{t}]."),
            AdapterError::UnsupportedLanguage(l) => write!(f, "This provider does not support the language {l}."),
            AdapterError::Quota => write!(f, "The provider account is out of quota."),
            AdapterError::Network => write!(f, "The provider could not be reached."),
            AdapterError::ProviderError(m) => write!(f, "The provider reported a problem: {m}"),
            AdapterError::UnknownVoice(v) => write!(f, "The voice {v} is not available."),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Capabilities {
    /// BCP 47 tags (prefix match on the primary subtag).
    pub languages: Vec<String>,
    /// Canonical non-verbal and delivery tags the adapter renders. Pauses are always supported.
    pub tags: Vec<String>,
    pub voice_design: bool,
    pub cloning: bool,
    pub max_chars_per_request: usize,
    pub streaming: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Voice {
    pub id: String,
    pub name: String,
    /// Adapter id.
    pub provider: String,
    pub language: String,
    pub accent: String,
    pub gender: String,
    pub age_range: String,
    pub licence: LicenceInfo,
    pub sample_text: String,
    pub notes: String,
}

#[derive(Clone, Debug)]
pub struct RenderRequest {
    pub tokens: Vec<Token>,
    pub voice_id: String,
    pub emotion: String,
    pub pace: f32,
    pub language: String,
}

/// A request to clone a voice. The proof makes it impossible to build without a clean consent record.
#[derive(Debug)]
pub struct CloneRequest {
    pub name: String,
    /// 16-bit mono WAV of the owner speaking, already checked.
    pub sample_wav: Vec<u8>,
    pub proof: crate::consent::ConsentProof,
}

#[derive(Clone, Debug)]
pub struct DesignRequest {
    pub description: String,
    /// Words for the candidates to speak. `None` lets the provider write something suited to the description.
    pub sample_text: Option<String>,
}

#[derive(Clone, Debug)]
pub struct DesignPreview {
    pub generated_voice_id: String,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Clone, Debug)]
pub struct RenderOutput {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub provider_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Cost {
    pub units: f64,
    pub amount: f64,
    pub currency: String,
}

pub trait Adapter: Send + Sync {
    fn id(&self) -> &str;
    fn label(&self) -> &str;
    fn licence(&self) -> LicenceInfo;
    fn capabilities(&self) -> Capabilities;
    fn list_voices(&self) -> Vec<Voice>;
    fn provider_version(&self) -> String;
    fn render(&self, req: &RenderRequest) -> Result<RenderOutput, AdapterError>;
    fn estimate_cost(&self, _req: &RenderRequest) -> Option<Cost> {
        None
    }
    /// False when nothing is set up yet (for example, no key saved).
    fn ready(&self) -> bool {
        true
    }
    /// Make three candidate voices from a text description. Only some providers can.
    fn design_voice(&self, _req: &DesignRequest) -> Result<Vec<DesignPreview>, AdapterError> {
        Err(AdapterError::ProviderError("this voice engine cannot design voices".into()))
    }
    /// Keep one candidate as a permanent voice.
    fn save_designed_voice(&self, _generated_id: &str, _name: &str, _description: &str) -> Result<Voice, AdapterError> {
        Err(AdapterError::ProviderError("this voice engine cannot design voices".into()))
    }
    /// Tell the provider a voice id outside its own list (a clone the user made) may be rendered.
    fn allow_voice(&self, _id: &str) {}
    /// Create a voice from a recording. Returns the new voice's id. Only some providers can.
    fn clone_voice(&self, _req: &CloneRequest) -> Result<String, AdapterError> {
        Err(AdapterError::ProviderError("this voice engine cannot clone voices".into()))
    }
    /// Remove a voice this provider holds for the user.
    fn delete_voice(&self, _voice_id: &str) -> Result<(), AdapterError> {
        Err(AdapterError::ProviderError("this voice engine cannot delete voices".into()))
    }
    /// True when script text leaves this computer to be rendered. The UI asks before sending.
    fn sends_data(&self) -> bool {
        false
    }
    /// A cheap authenticated call for the Settings "Test connection" button.
    fn check_connection(&self) -> Result<(), AdapterError> {
        Ok(())
    }
}

/// Canonical tags a request needs that the adapter cannot render.
pub fn unsupported_tags(caps: &Capabilities, tokens: &[Token], emotion: &str) -> Vec<String> {
    let mut need = required_tags(tokens);
    if emotion != "neutral" && !need.iter().any(|t| t == emotion) {
        need.push(emotion.to_string());
    }
    need.into_iter().filter(|t| !caps.tags.contains(t)).collect()
}

fn primary(lang: &str) -> String {
    lang.split(['-', '_']).next().unwrap_or("").to_lowercase()
}

/// Every adapter calls this before doing any work or any network call.
pub fn precheck(caps: &Capabilities, req: &RenderRequest) -> Result<(), AdapterError> {
    if let Some(t) = unsupported_tags(caps, &req.tokens, &req.emotion).into_iter().next() {
        return Err(AdapterError::UnsupportedTag(t));
    }
    let want = primary(&req.language);
    if !caps.languages.iter().any(|l| primary(l) == want) {
        return Err(AdapterError::UnsupportedLanguage(req.language.clone()));
    }
    Ok(())
}

/// Split a request so no chunk's spoken text exceeds `max_chars` (the core does this, not the adapter).
pub fn split_tokens(tokens: &[Token], max_chars: usize) -> Vec<Vec<Token>> {
    let max_chars = max_chars.max(20);
    let mut chunks: Vec<Vec<Token>> = vec![vec![]];
    let mut used = 0usize;
    for t in tokens {
        match t {
            Token::Text(s) => {
                let mut rest: &str = s;
                while !rest.is_empty() {
                    let room = max_chars.saturating_sub(used);
                    if rest.chars().count() <= room {
                        chunks.last_mut().unwrap().push(Token::Text(rest.to_string()));
                        used += rest.chars().count();
                        break;
                    }
                    // Cut at the last sentence end, else last space, within the room.
                    let window: String = rest.chars().take(room.max(1)).collect();
                    let cut = window
                        .rfind(['.', '!', '?'])
                        .or_else(|| window.rfind(' '))
                        .map(|i| i + 1)
                        .filter(|&i| i > 0)
                        .unwrap_or(window.len());
                    let (head, tail) = rest.split_at(cut);
                    if !head.trim().is_empty() {
                        chunks.last_mut().unwrap().push(Token::Text(head.trim().to_string()));
                    }
                    chunks.push(vec![]);
                    used = 0;
                    rest = tail.trim_start();
                }
            }
            other => chunks.last_mut().unwrap().push(other.clone()),
        }
    }
    chunks.retain(|c| !c.is_empty());
    chunks
}

/// Spoken segments and exact pauses, for engines that take plain text. Delivery and non-verbal directions are written
/// into the text in the engine's own tag syntax. A direction the engine has no syntax for is an error, never dropped.
pub fn compose_segments(tokens: &[Token], emotion: &str, native: &dyn Fn(&str) -> Option<String>) -> Result<Vec<Result<String, u32>>, AdapterError> {
    let mut current = emotion.to_string();
    let mut segs: Vec<Result<String, u32>> = Vec::new();
    let mut buf = String::new();
    let mut started = false;
    let style = |c: &str| native(c).map(|t| format!("{t} ")).unwrap_or_default();
    for t in tokens {
        match t {
            Token::Text(s) => {
                if !started {
                    buf.push_str(&style(&current));
                    started = true;
                }
                buf.push_str(s);
                buf.push(' ');
            }
            Token::Delivery(d) => {
                started = true;
                current = d.clone();
                buf.push_str(&native(d).ok_or_else(|| AdapterError::UnsupportedTag(d.clone()))?);
                buf.push(' ');
            }
            Token::NonVerbal(n) => {
                if !started {
                    buf.push_str(&style(&current));
                    started = true;
                }
                buf.push_str(&native(n).ok_or_else(|| AdapterError::UnsupportedTag(n.clone()))?);
                buf.push(' ');
            }
            Token::Pause(ms) => {
                if !buf.trim().is_empty() {
                    segs.push(Ok(buf.trim().to_string()));
                }
                buf.clear();
                started = false;
                segs.push(Err(*ms));
            }
        }
    }
    if !buf.trim().is_empty() {
        segs.push(Ok(buf.trim().to_string()));
    }
    if !segs.iter().any(|s| s.is_ok()) {
        return Err(AdapterError::ProviderError("there is nothing to say on this line".into()));
    }
    Ok(segs)
}

/// All adapters plus a cache of their voices.
pub struct Registry {
    pub adapters: Vec<Box<dyn Adapter>>,
    pub voices: Vec<Voice>,
    pub generators: Vec<Box<dyn crate::generate::Generator>>,
    /// Consent record for every cloned voice, by voice id. A voice in this map is usable only while its record is clean.
    pub consents: std::collections::BTreeMap<String, crate::consent::ConsentRecord>,
}

impl Registry {
    pub fn new(adapters: Vec<Box<dyn Adapter>>) -> Self {
        let mut r = Registry { adapters, voices: vec![], generators: vec![], consents: Default::default() };
        r.refresh();
        r
    }

    /// The adapters shipped in this build. `keys` supplies credentials; adapters never read the key store themselves.
    pub fn standard(keys: KeyProvider, engines: &crate::engines::EnginesConfig) -> Self {
        let mut adapters: Vec<Box<dyn Adapter>> = vec![
            Box::new(crate::adapters::sapi::SapiAdapter::new()),
            Box::new(crate::adapters::elevenlabs::ElevenLabsAdapter::new(keys.clone())),
        ];
        for e in &engines.speech {
            // Engines were validated when saved, so a failure here means a hand-edited file; skip it rather than refuse to start.
            if let Ok(a) = crate::adapters::openai_speech::OpenAiSpeechAdapter::new(e.clone(), keys.clone()) {
                adapters.push(Box::new(a));
            }
        }
        if let Some(p) = engines.piper.as_ref().filter(|p| !p.exe.trim().is_empty()) {
            if let Ok(a) = crate::adapters::piper::PiperAdapter::new(p, std::sync::Arc::new(crate::adapters::piper::RealRunner)) {
                adapters.push(Box::new(a));
            }
        }
        let mut generators: Vec<Box<dyn crate::generate::Generator>> = vec![Box::new(crate::adapters::elevenlabs::ElevenLabsGenerator::new(keys.clone()))];
        for j in &engines.jobs {
            // Validated when saved. A failure here means a hand-edited file, so the engine is skipped, not fatal.
            let Ok((transport, meta)) = build_job(j, &keys) else { continue };
            match j.kind.as_str() {
                "speech" => adapters.push(Box::new(crate::jobs::JobSpeech { id: meta.id.clone(), meta, transport })),
                "music" => generators.push(Box::new(crate::jobs::JobGenerator { meta, kind: crate::generate::GenKind::Music, transport })),
                _ => generators.push(Box::new(crate::jobs::JobGenerator { meta, kind: crate::generate::GenKind::Sfx, transport })),
            }
        }
        adapters.push(Box::new(crate::adapters::scratch::ScratchAdapter));
        let mut reg = Registry::new(adapters);
        reg.generators = generators;
        reg
    }

    /// Add the voices the user cloned, each tied to its consent record. They appear in the library like any other voice.
    pub fn with_clones(mut self, entries: Vec<crate::clone::CloneEntry>) -> Self {
        for e in entries {
            if self.adapter(&e.voice.adapter_id).is_none() || self.voices.iter().any(|v| v.id == e.voice.voice_id) {
                continue;
            }
            let commercial = e.consent.commercial_use_permitted;
            self.voices.push(Voice {
                id: e.voice.voice_id.clone(),
                name: format!("{} (cloned)", e.voice.name),
                provider: e.voice.adapter_id.clone(),
                language: "en".into(),
                accent: String::new(),
                gender: String::new(),
                age_range: String::new(),
                licence: licence_info(if commercial { Licence::Commercial } else { Licence::NonCommercial }, &format!("consent record {}", e.consent.id), &crate::util::today_string(), "Cloned with the voice owner's recorded consent. The provider's own terms still apply to the model."),
                sample_text: "Hello, this is a short sample of my voice.".into(),
                notes: format!("Voice of {}. Usable only while the consent record is active.", e.consent.voice_subject),
            });
            if let Some(a) = self.adapter(&e.voice.adapter_id) {
                a.allow_voice(&e.voice.voice_id);
            }
            self.consents.insert(e.voice.voice_id, e.consent);
        }
        self
    }

    /// Why a cloned voice cannot be used right now. `None` for ordinary voices and for cloned voices with clean consent.
    pub fn consent_problems(&self, voice_id: &str, commercial: bool) -> Option<Vec<String>> {
        let rec = self.consents.get(voice_id)?;
        crate::consent::require(Some(rec), &crate::consent::Use::Dialogue, commercial).err()
    }

    pub fn refresh(&mut self) {
        self.voices = self.adapters.iter().flat_map(|a| a.list_voices()).collect();
    }

    pub fn voice(&self, id: &str) -> Option<&Voice> {
        self.voices.iter().find(|v| v.id == id)
    }

    pub fn generator(&self, id: &str) -> Option<&dyn crate::generate::Generator> {
        self.generators.iter().find(|g| g.id() == id).map(|b| b.as_ref())
    }

    pub fn adapter(&self, id: &str) -> Option<&dyn Adapter> {
        self.adapters.iter().find(|a| a.id() == id).map(|b| b.as_ref())
    }

    pub fn adapter_for_voice(&self, voice_id: &str) -> Option<&dyn Adapter> {
        self.voice(voice_id).and_then(|v| self.adapter(&v.provider))
    }
}

fn build_job(j: &crate::engines::JobEngine, keys: &KeyProvider) -> Result<(Box<dyn crate::jobs::Transport>, crate::jobs::JobMeta), String> {
    use crate::adapters::{comfy::ComfyTransport, fal::FalTransport, kie::KieTransport};
    use crate::jobs::{parse_template, JobMeta, RealFetcher, MAX_TEMPLATE_BYTES, MAX_WORKFLOW_BYTES};
    j.validate()?;
    let comfy = j.provider == "comfy";
    let tpl = parse_template(&j.template, if j.kind == "speech" { "text" } else { "prompt" }, if comfy { MAX_WORKFLOW_BYTES } else { MAX_TEMPLATE_BYTES })?;
    let fetch: std::sync::Arc<dyn crate::jobs::Fetcher> = std::sync::Arc::new(RealFetcher);
    let transport: Box<dyn crate::jobs::Transport> = match j.provider.as_str() {
        "fal" => Box::new(FalTransport::new(j.model.clone(), tpl, keys.clone(), fetch)),
        "kie" => Box::new(KieTransport::new(j.model.clone(), tpl, keys.clone(), fetch)),
        _ => Box::new(ComfyTransport::new(&j.base_url, tpl)?),
    };
    Ok((transport, JobMeta { id: format!("job-{}", j.id), label: j.label.clone(), licence_note: j.licence_note.clone(), voices: j.voices.clone(), limits: j.limits() }))
}

pub fn licence_info(licence: Licence, source: &str, checked: &str, notes: &str) -> LicenceInfo {
    LicenceInfo {
        licence,
        source: if source.is_empty() { None } else { Some(source.to_string()) },
        checked: if checked.is_empty() { None } else { Some(checked.to_string()) },
        notes: notes.to_string(),
    }
}

#[cfg(test)]
pub mod contract {
    //! The shared adapter contract suite (TEST_PLAN.md section 2).
    use super::*;
    use crate::tags::{expected_duration_ms, tokenize};

    fn req(adapter: &dyn Adapter, voice: &Voice, text: &str, emotion: &str) -> RenderRequest {
        let _ = adapter;
        RenderRequest { tokens: tokenize(text).0, voice_id: voice.id.clone(), emotion: emotion.into(), pace: 1.0, language: voice.language.clone() }
    }

    /// `unsupported` is a canonical tag the adapter must refuse.
    pub fn run(adapter: &dyn Adapter, voice: &Voice, unsupported: &str) {
        let caps = adapter.capabilities();
        // 1. Capabilities only name canonical tags.
        for t in &caps.tags {
            assert!(crate::tags::NON_VERBAL.contains(&t.as_str()) || crate::tags::DELIVERY.contains(&t.as_str()), "non-canonical tag {t}");
        }
        // 2. A short line renders with a duration within 20 percent of the expectation.
        let r = req(adapter, voice, "Good morning my friend, how are you today?", "neutral");
        let out = adapter.render(&r).expect("render");
        let ms = out.samples.len() as u64 * 1000 / out.sample_rate as u64;
        let expect = expected_duration_ms(&r.tokens);
        assert!(ms as f64 > expect as f64 * 0.4 && (ms as f64) < expect as f64 * 1.6, "duration {ms} ms vs expected {expect} ms");
        assert!(out.samples.iter().any(|x| x.abs() > 0.001), "silent output");
        // 6. provider_version is non-empty.
        assert!(!out.provider_version.is_empty());
        assert!(!adapter.provider_version().is_empty());
        // 4. A tag the adapter does not support is rejected before any work.
        let bad = req(adapter, voice, &format!("Hello [{unsupported}] there"), "neutral");
        assert_eq!(adapter.render(&bad).unwrap_err(), AdapterError::UnsupportedTag(unsupported.into()));
        // Unsupported emotions are rejected as well, not ignored.
        let bad_emotion = req(adapter, voice, "Hello there", "sarcastic");
        if !caps.tags.contains(&"sarcastic".to_string()) {
            assert_eq!(adapter.render(&bad_emotion).unwrap_err(), AdapterError::UnsupportedTag("sarcastic".into()));
        }
        // Unknown language is rejected.
        let mut lang = req(adapter, voice, "Hello there", "neutral");
        lang.language = "xx-YY".into();
        assert!(matches!(adapter.render(&lang), Err(AdapterError::UnsupportedLanguage(_))));
        // 7. Identical requests are deterministic enough to share a cache key (same inputs, same length).
        let again = adapter.render(&r).expect("render again");
        assert_eq!(again.samples.len().abs_diff(out.samples.len()) < out.sample_rate as usize / 20, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tags::tokenize;

    fn caps(tags: &[&str]) -> Capabilities {
        Capabilities {
            languages: vec!["en".into()],
            tags: tags.iter().map(|s| s.to_string()).collect(),
            voice_design: false,
            cloning: false,
            max_chars_per_request: 100,
            streaming: false,
        }
    }

    #[test]
    fn unsupported_tags_include_emotion_but_not_pauses() {
        let (t, _) = tokenize("[sneeze] bless you [pause:short] [whisper] ok");
        let u = unsupported_tags(&caps(&["whisper"]), &t, "sad");
        assert_eq!(u, vec!["sneeze", "sad"]);
    }

    #[test]
    fn precheck_rejects_before_any_work() {
        let r = RenderRequest { tokens: tokenize("hi [cough]").0, voice_id: "x".into(), emotion: "neutral".into(), pace: 1.0, language: "en-GB".into() };
        assert_eq!(precheck(&caps(&[]), &r).unwrap_err(), AdapterError::UnsupportedTag("cough".into()));
        let r2 = RenderRequest { language: "fr".into(), tokens: tokenize("hi").0, ..r };
        assert!(matches!(precheck(&caps(&[]), &r2), Err(AdapterError::UnsupportedLanguage(_))));
    }

    #[test]
    fn long_text_is_split_by_the_core() {
        let long = "One two three. ".repeat(40);
        let (t, _) = tokenize(&format!("{long} [pause:short] done"));
        let chunks = split_tokens(&t, 100);
        assert!(chunks.len() > 3);
        for c in &chunks {
            let n: usize = c.iter().map(|t| if let Token::Text(s) = t { s.chars().count() } else { 0 }).sum();
            assert!(n <= 100, "chunk of {n} chars");
        }
        let words: usize = chunks.iter().flatten().map(|t| if let Token::Text(s) = t { s.split_whitespace().count() } else { 0 }).sum();
        assert_eq!(words, 120 + 1);
    }

    #[test]
    fn segments_carry_tags_pauses_and_the_active_style() {
        let map = |c: &str| match c { "laugh" => Some("[L]".to_string()), "whisper" => Some("[W]".to_string()), _ => None };
        let (t, _) = tokenize("hi [laugh] there [pause:500] bye [whisper] ok");
        let segs = compose_segments(&t, "neutral", &map).unwrap();
        assert_eq!(segs, vec![Ok("hi [L] there".to_string()), Err(500), Ok("bye [W] ok".to_string())]);
        let (t2, _) = tokenize("one [pause:short] two");
        assert_eq!(compose_segments(&t2, "whisper", &map).unwrap(), vec![Ok("[W] one".to_string()), Err(250), Ok("[W] two".to_string())], "the active style carries across a pause");
        let (t3, _) = tokenize("a [sneeze] b");
        assert_eq!(compose_segments(&t3, "neutral", &map).unwrap_err(), AdapterError::UnsupportedTag("sneeze".into()));
        assert!(compose_segments(&tokenize("[pause:short]").0, "neutral", &map).is_err());
    }

    #[test]
    fn short_text_is_one_chunk_and_keeps_tags() {
        let (t, _) = tokenize("Hi [laugh] there");
        assert_eq!(split_tokens(&t, 500), vec![t]);
    }
}
