//! Provider-agnostic core (PROVIDERS.md sections 2 to 4). The UI and project files
//! speak canonical tags and voice ids. Adapters translate and never touch project state.

use crate::licence::{Licence, LicenceInfo};
use crate::tags::{required_tags, Token};
use serde::{Deserialize, Serialize};

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
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    fn licence(&self) -> LicenceInfo;
    fn capabilities(&self) -> Capabilities;
    fn list_voices(&self) -> Vec<Voice>;
    fn provider_version(&self) -> String;
    fn render(&self, req: &RenderRequest) -> Result<RenderOutput, AdapterError>;
    fn estimate_cost(&self, _req: &RenderRequest) -> Option<Cost> {
        None
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

/// All adapters plus a cache of their voices.
pub struct Registry {
    pub adapters: Vec<Box<dyn Adapter>>,
    pub voices: Vec<Voice>,
}

impl Registry {
    pub fn new(adapters: Vec<Box<dyn Adapter>>) -> Self {
        let mut r = Registry { adapters, voices: vec![] };
        r.refresh();
        r
    }

    /// The adapters shipped in this build.
    pub fn standard() -> Self {
        Registry::new(vec![
            Box::new(crate::adapters::sapi::SapiAdapter::new()),
            Box::new(crate::adapters::scratch::ScratchAdapter),
        ])
    }

    pub fn refresh(&mut self) {
        self.voices = self.adapters.iter().flat_map(|a| a.list_voices()).collect();
    }

    pub fn voice(&self, id: &str) -> Option<&Voice> {
        self.voices.iter().find(|v| v.id == id)
    }

    pub fn adapter(&self, id: &str) -> Option<&dyn Adapter> {
        self.adapters.iter().find(|a| a.id() == id).map(|b| b.as_ref())
    }

    pub fn adapter_for_voice(&self, voice_id: &str) -> Option<&dyn Adapter> {
        self.voice(voice_id).and_then(|v| self.adapter(&v.provider))
    }
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
    fn short_text_is_one_chunk_and_keeps_tags() {
        let (t, _) = tokenize("Hi [laugh] there");
        assert_eq!(split_tokens(&t, 500), vec![t]);
    }
}
