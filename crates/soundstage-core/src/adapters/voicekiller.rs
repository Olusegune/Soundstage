//! VoiceKiller (https://voicekiller.com/api-docs): text to speech with about two thousand voices, plus speech from voices
//! the user cloned on voicekiller.com.
//!
//! Read from the provider's own documentation on 2026-10-23 (all three endpoints are POST, authenticated by the
//! `api-key` and `userid` headers):
//! - `/api/generate-speech/voicelist`: no body, plain text, one voice name per line.
//! - `/api/generate-speech`: JSON `{ script (max 6000), voice_name, instructions (max 2000, optional) }`, returns MP3.
//! - `/api/generate-speech/clone`: JSON `{ script, voice_name }` for a voice already cloned on the website, returns MP3.
//!
//! Decisions that follow from that:
//! - There are no inline sound tags (laugh, sigh), so those are refused before any request. Delivery (whisper, excited and
//!   so on) and pace are sent through the `instructions` field, one request per stretch of speech in one style.
//! - Clones cannot be created through the API, so a clone is *registered* here by name, and the consent gate applies to it
//!   exactly as it does to a clone made in the app.
//! - The key and user id are looked up by name from the key store and only ever go in headers. Nothing is followed on a
//!   redirect, and the host is fixed.

use crate::audio::resample;
use crate::decode::decode_bytes;
use crate::licence::{Licence, LicenceInfo};
use crate::net::{error_from_status, read_limited};
use crate::providers::*;
use crate::tags::{Token, DELIVERY};
use std::sync::Mutex;
use std::time::Duration;

const HOST: &str = "https://voicekiller.com";
pub const CLONE_PREFIX: &str = "voicekiller-clone:";
const MAX_SCRIPT: usize = 6000;
const MAX_AUDIO_BYTES: usize = 25 * 1024 * 1024;
const MAX_LIST_BYTES: usize = 2 * 1024 * 1024;
const MAX_VOICES: usize = 4000;

pub struct VoicekillerAdapter {
    keys: KeyProvider,
    base: String,
    agent: ureq::Agent,
    voices: Mutex<Option<Vec<Voice>>>,
    extra: Mutex<Vec<String>>,
}

fn licence() -> LicenceInfo {
    LicenceInfo {
        licence: Licence::Unknown,
        source: Some("https://voicekiller.com".into()),
        checked: None,
        notes: "Commercial terms depend on your VoiceKiller plan. Read them and record your check before a commercial export.".into(),
        conditions: vec![],
    }
}

/// A voice name is shown to people and sent back in a JSON body, so it only needs to be sane text.
fn valid_name(n: &str) -> bool {
    let t = n.trim();
    !t.is_empty() && t.chars().count() <= 80 && !t.chars().any(|c| c.is_control())
}

/// `Authoritative Advisor 1 (en-GB)` is a British English voice; names without a locale are taken as English.
fn language_of(name: &str) -> String {
    let Some(open) = name.rfind('(') else { return "en".into() };
    let inner = name[open + 1..].trim_end_matches(')');
    let mut it = inner.split('-');
    let (lang, region) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
    let ok = (2..=3).contains(&lang.len()) && lang.bytes().all(|b| b.is_ascii_lowercase()) && region.len() <= 4 && region.bytes().all(|b| b.is_ascii_alphanumeric());
    if ok && inner.len() <= 8 { inner.to_string() } else { "en".into() }
}

fn style_phrase(style: &str) -> Option<&'static str> {
    Some(match style {
        "whisper" => "whispering softly",
        "shout" => "loud, as if shouting",
        "excited" => "excited and energetic",
        "sad" => "sad and subdued",
        "tired" => "tired and low on energy",
        "sarcastic" => "sarcastic",
        "curious" => "curious",
        "serious" => "serious and measured",
        _ => return None,
    })
}

/// The `instructions` text for a style and pace. `None` when there is nothing to say (neutral at normal speed).
pub fn instructions_for(style: &str, pace: f32) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(p) = style_phrase(style) {
        parts.push(format!("Speak in a tone that is {p}."));
    }
    if pace >= 1.25 {
        parts.push("Speak much faster than normal.".into());
    } else if pace > 1.05 {
        parts.push("Speak a little faster than normal.".into());
    } else if pace <= 0.75 {
        parts.push("Speak much slower than normal.".into());
    } else if pace < 0.95 {
        parts.push("Speak a little slower than normal.".into());
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

impl VoicekillerAdapter {
    pub fn new(keys: KeyProvider) -> Self {
        Self::build(keys, HOST.to_string())
    }

    fn build(keys: KeyProvider, base: String) -> Self {
        let agent = crate::net::cloud_agent_builder().redirects(0).timeout_connect(Duration::from_secs(10)).timeout_write(Duration::from_secs(30)).timeout_read(Duration::from_secs(120)).user_agent("Soundstage").build();
        VoicekillerAdapter { keys, base, agent, voices: Mutex::new(None), extra: Mutex::new(Vec::new()) }
    }

    #[cfg(test)]
    fn for_test(keys: KeyProvider, base: String) -> Self {
        Self::build(keys, base)
    }

    fn credential(&self, name: &str) -> Result<String, AdapterError> {
        let v = (self.keys)(name).ok_or(AdapterError::Auth)?;
        let v = v.trim().to_string();
        // Anything that could smuggle header characters is treated as no credential at all.
        if v.is_empty() || v.len() > 512 || !v.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(AdapterError::Auth);
        }
        Ok(v)
    }

    fn has_keys(&self) -> bool {
        self.credential("voicekiller").is_ok() && self.credential("voicekiller:userid").is_ok()
    }

    fn post(&self, path: &str, body: Option<&serde_json::Value>) -> Result<ureq::Response, AdapterError> {
        let (key, user) = (self.credential("voicekiller")?, self.credential("voicekiller:userid")?);
        let req = self.agent.post(&format!("{}{}", self.base, path)).set("api-key", &key).set("userid", &user);
        // One attempt: characters are billed, so a blind retry could pay twice.
        let res = match body {
            Some(b) => req.set("content-type", "application/json").send_string(&b.to_string()),
            None => req.call(),
        };
        match res {
            Ok(r) if r.status() == 200 => Ok(r),
            Ok(r) => Err(AdapterError::ProviderError(format!("unexpected HTTP {}", r.status()))),
            Err(ureq::Error::Status(code, r)) => Err(error_from_status(code, r)),
            Err(ureq::Error::Transport(t)) => { crate::net::note_transport(&t); Err(AdapterError::Network) }
        }
    }

    fn fetch_voices(&self) -> Result<Vec<Voice>, AdapterError> {
        let bytes = read_limited(self.post("/api/generate-speech/voicelist", None)?, MAX_LIST_BYTES)?;
        let text = String::from_utf8_lossy(&bytes);
        let mut seen = std::collections::BTreeSet::new();
        let mut out = Vec::new();
        for line in text.lines() {
            let name = line.trim();
            if !valid_name(name) || !seen.insert(name.to_string()) {
                continue;
            }
            out.push(Voice {
                id: format!("voicekiller:{name}"),
                name: name.to_string(),
                provider: "voicekiller".into(),
                language: language_of(name),
                accent: "Not stated".into(),
                gender: "Not stated".into(),
                age_range: "Not stated".into(),
                licence: licence(),
                sample_text: "Good morning. This is how I sound.".into(),
                notes: "Script text is sent to VoiceKiller. Delivery and pace are passed as instructions, so they are a request, not a guarantee.".into(),
            });
            if out.len() >= MAX_VOICES {
                break;
            }
        }
        if out.is_empty() {
            return Err(AdapterError::ProviderError("the voice list was empty".into()));
        }
        Ok(out)
    }

    fn voices(&self) -> Vec<Voice> {
        let mut g = self.voices.lock().unwrap_or_else(|e| e.into_inner());
        if g.is_none() {
            *g = Some(if self.has_keys() { self.fetch_voices().unwrap_or_default() } else { Vec::new() });
        }
        g.clone().unwrap_or_default()
    }

    /// Speak one stretch of text in one style. Returns mono samples and their rate.
    fn speak(&self, text: &str, voice_name: &str, clone: bool, style: &str, pace: f32) -> Result<(Vec<f32>, u32), AdapterError> {
        let text = text.trim();
        if text.is_empty() {
            return Ok((Vec::new(), 24_000));
        }
        if text.chars().count() > MAX_SCRIPT {
            return Err(AdapterError::ProviderError("that line is too long for this voice".into()));
        }
        let mut body = serde_json::json!({ "script": text, "voice_name": voice_name });
        let path = if clone {
            // The clone endpoint documents only these two fields, so nothing else is sent.
            "/api/generate-speech/clone"
        } else {
            if let Some(i) = instructions_for(style, pace) {
                body["instructions"] = serde_json::json!(i);
            }
            "/api/generate-speech"
        };
        let bytes = read_limited(self.post(path, Some(&body))?, MAX_AUDIO_BYTES)?;
        let d = decode_bytes(&bytes, Some("mp3")).map_err(|_| AdapterError::ProviderError("VoiceKiller did not return audio Soundstage could read".into()))?;
        let n = d.channels[0].len();
        let mono: Vec<f32> = (0..n).map(|i| d.channels.iter().map(|c| c.get(i).copied().unwrap_or(0.0)).sum::<f32>() / d.channels.len() as f32).collect();
        Ok((mono, d.rate))
    }
}

impl Adapter for VoicekillerAdapter {
    fn id(&self) -> &str {
        "voicekiller"
    }
    fn label(&self) -> &str {
        "VoiceKiller"
    }
    fn licence(&self) -> LicenceInfo {
        licence()
    }
    fn capabilities(&self) -> Capabilities {
        let mut languages: Vec<String> = self.voices().iter().map(|v| v.language.split('-').next().unwrap_or("en").to_string()).collect();
        languages.push("en".into());
        languages.sort();
        languages.dedup();
        // Delivery styles go through the instructions field. Inline sounds (laugh, sigh) do not exist here.
        Capabilities { languages, tags: DELIVERY.iter().map(|s| s.to_string()).collect(), voice_design: false, cloning: false, max_chars_per_request: 5500, streaming: false }
    }
    fn list_voices(&self) -> Vec<Voice> {
        self.voices()
    }
    fn provider_version(&self) -> String {
        "voicekiller/api-v1".into()
    }
    fn sends_data(&self) -> bool {
        true
    }
    fn ready(&self) -> bool {
        self.has_keys()
    }
    fn can_register_clone(&self) -> bool {
        true
    }
    fn allow_voice(&self, id: &str) {
        let mut g = self.extra.lock().unwrap_or_else(|e| e.into_inner());
        if !g.iter().any(|v| v == id) && g.len() < 500 {
            g.push(id.to_string());
        }
    }
    fn check_connection(&self) -> Result<(), AdapterError> {
        let r = self.post("/api/generate-speech/voicelist", None)?;
        let b = read_limited(r, MAX_LIST_BYTES)?;
        if b.iter().all(|c| c.is_ascii_whitespace()) {
            return Err(AdapterError::ProviderError("the voice list was empty".into()));
        }
        Ok(())
    }
    fn render(&self, req: &RenderRequest) -> Result<RenderOutput, AdapterError> {
        precheck(&self.capabilities(), req)?;
        // Inline sounds are refused up front (they are not in `capabilities`), but never trust that alone.
        if let Some(t) = req.tokens.iter().find_map(|t| if let Token::NonVerbal(n) = t { Some(n.clone()) } else { None }) {
            return Err(AdapterError::UnsupportedTag(t));
        }
        let (name, clone) = match req.voice_id.strip_prefix(CLONE_PREFIX) {
            Some(n) => (n.to_string(), true),
            None => (req.voice_id.strip_prefix("voicekiller:").unwrap_or("").to_string(), false),
        };
        let known = if clone { self.extra.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|v| *v == req.voice_id) } else { self.voices().iter().any(|v| v.id == req.voice_id) };
        if !valid_name(&name) || !known {
            return Err(AdapterError::UnknownVoice(req.voice_id.chars().take(60).collect()));
        }
        // Group the line into stretches of speech that share one style, with exact silences between them.
        let mut style = req.emotion.clone();
        let mut pieces: Vec<Result<(Vec<f32>, u32), u32>> = Vec::new();
        let mut buf = String::new();
        let flush = |buf: &mut String, style: &str, pieces: &mut Vec<Result<(Vec<f32>, u32), u32>>| -> Result<(), AdapterError> {
            if !buf.trim().is_empty() {
                pieces.push(Ok(self.speak(buf, &name, clone, style, req.pace)?));
            }
            buf.clear();
            Ok(())
        };
        for t in &req.tokens {
            match t {
                Token::Text(s) => {
                    buf.push_str(s);
                    buf.push(' ');
                }
                Token::Delivery(d) => {
                    flush(&mut buf, &style, &mut pieces)?;
                    style = d.clone();
                }
                Token::Pause(ms) => {
                    flush(&mut buf, &style, &mut pieces)?;
                    pieces.push(Err(*ms));
                }
                Token::NonVerbal(n) => return Err(AdapterError::UnsupportedTag(n.clone())),
            }
        }
        flush(&mut buf, &style, &mut pieces)?;
        let rate = pieces.iter().find_map(|p| p.as_ref().ok().map(|(_, r)| *r)).unwrap_or(24_000);
        let mut samples = Vec::new();
        for p in pieces {
            match p {
                Ok((s, r)) => samples.extend(if r == rate { s } else { resample(&s, r, rate) }),
                Err(ms) => samples.extend(std::iter::repeat(0.0).take((ms as usize * rate as usize) / 1000)),
            }
        }
        if samples.is_empty() {
            return Err(AdapterError::ProviderError("VoiceKiller returned no audio".into()));
        }
        Ok(RenderOutput { samples, sample_rate: rate, provider_version: self.provider_version() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tags::tokenize;
    use crate::testutil::{start, Fake, Handler};
    use std::sync::Arc;

    const KEY: &str = "vk-test-DO-NOT-LEAK-999";
    const USER: &str = "user-4242";

    fn keys() -> KeyProvider {
        Arc::new(|n| match n {
            "voicekiller" => Some(KEY.into()),
            "voicekiller:userid" => Some(USER.into()),
            _ => None,
        })
    }

    fn wav_for(text: &str) -> Vec<u8> {
        let words = text.split_whitespace().count().max(1);
        let n = words * 350 * 24_000 / 1000;
        let mut cur = std::io::Cursor::new(Vec::new());
        {
            let spec = hound::WavSpec { channels: 1, sample_rate: 24_000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
            let mut w = hound::WavWriter::new(&mut cur, spec).unwrap();
            for i in 0..n {
                w.write_sample((8000.0 * (i as f32 * 0.06).sin()) as i16).unwrap();
            }
            w.finalize().unwrap();
        }
        cur.into_inner()
    }

    fn happy() -> Handler {
        Box::new(|r, _| {
            let authed = r.headers.iter().any(|(k, v)| k == "api-key" && v == KEY) && r.headers.iter().any(|(k, v)| k == "userid" && v == USER);
            if !authed {
                return (401, b"Unauthorized: Invalid or missing API key".to_vec());
            }
            match (r.method.as_str(), r.path.as_str()) {
                ("POST", "/api/generate-speech/voicelist") => (200, b"Jett\nBella\nAuthoritative Advisor 1 (en-GB)\nCall Center Agent 4 (ja-JP)\nJett\n\n  \nBad\x07Name\nZ".to_vec()),
                ("POST", "/api/generate-speech") | ("POST", "/api/generate-speech/clone") => {
                    let v: serde_json::Value = serde_json::from_str(&r.body).unwrap_or_default();
                    if r.path.ends_with("/clone") && v["voice_name"] != "My Clone" {
                        return (404, b"No cloned voice found for this user.".to_vec());
                    }
                    (200, wav_for(v["script"].as_str().unwrap_or("")))
                }
                _ => (404, b"{}".to_vec()),
            }
        })
    }

    fn adapter(f: &Fake) -> VoicekillerAdapter {
        VoicekillerAdapter::for_test(keys(), f.base.clone())
    }

    fn req(a: &VoicekillerAdapter, voice: &str, text: &str, emotion: &str, pace: f32) -> RenderRequest {
        let _ = a;
        RenderRequest { tokens: tokenize(text).0, voice_id: voice.into(), emotion: emotion.into(), pace, language: "en-GB".into() }
    }

    fn posts(f: &Fake) -> Vec<crate::testutil::Req> {
        f.hits.lock().unwrap().iter().filter(|r| r.path == "/api/generate-speech" || r.path.ends_with("/clone")).cloned().collect()
    }

    #[test]
    fn lists_voices_with_languages_and_drops_bad_or_repeated_names() {
        let f = start(happy());
        let a = adapter(&f);
        let v = a.list_voices();
        let names: Vec<&str> = v.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, vec!["Jett", "Bella", "Authoritative Advisor 1 (en-GB)", "Call Center Agent 4 (ja-JP)", "Z"]);
        assert_eq!(v[2].language, "en-GB");
        assert_eq!(v[3].language, "ja-JP");
        assert_eq!(v[0].language, "en");
        assert_eq!(v[0].id, "voicekiller:Jett");
        assert!(a.capabilities().languages.contains(&"ja".to_string()));
        assert_eq!(v[0].licence.licence, Licence::Unknown);
    }

    #[test]
    fn credentials_go_in_headers_only() {
        let f = start(happy());
        let a = adapter(&f);
        a.render(&req(&a, "voicekiller:Jett", "Hello there my friend", "neutral", 1.0)).unwrap();
        for h in f.hits.lock().unwrap().iter() {
            assert!(h.headers.iter().any(|(k, v)| k == "api-key" && v == KEY) && h.headers.iter().any(|(k, v)| k == "userid" && v == USER));
            assert!(!h.body.contains(KEY) && !h.body.contains(USER) && !h.path.contains(KEY) && !h.path.contains(USER));
        }
        let none = VoicekillerAdapter::for_test(Arc::new(|_| None), f.base.clone());
        assert!(!none.ready() && none.list_voices().is_empty());
        assert_eq!(none.check_connection().unwrap_err(), AdapterError::Auth);
        let bad = VoicekillerAdapter::for_test(Arc::new(|n| Some(if n == "voicekiller" { "k\r\nx-evil: 1".into() } else { USER.into() })), f.base.clone());
        assert_eq!(bad.check_connection().unwrap_err(), AdapterError::Auth, "a key with header characters is refused");
    }

    #[test]
    fn style_and_pace_travel_as_instructions_one_request_per_style() {
        let f = start(happy());
        let a = adapter(&f);
        let r = a.render(&req(&a, "voicekiller:Bella", "I am calm. [whisper] Now I whisper. [pause:long] Done", "neutral", 1.3)).unwrap();
        let p = posts(&f);
        assert_eq!(p.len(), 3, "calm, whisper, and the text after the pause");
        let body = |i: usize| serde_json::from_str::<serde_json::Value>(&p[i].body).unwrap();
        assert!(body(0)["instructions"].as_str().unwrap().contains("much faster") && !body(0)["instructions"].as_str().unwrap().contains("whisper"));
        assert!(body(1)["instructions"].as_str().unwrap().contains("whispering"));
        assert_eq!(body(0)["voice_name"], "Bella");
        assert!(r.samples.len() > 24_000, "the exact pause is silence between the pieces");
        assert!(instructions_for("neutral", 1.0).is_none());
    }

    #[test]
    fn inline_sounds_are_refused_before_any_request() {
        let f = start(happy());
        let a = adapter(&f);
        let before = f.hits.lock().unwrap().len();
        assert_eq!(a.render(&req(&a, "voicekiller:Jett", "Hello [laugh] there", "neutral", 1.0)).unwrap_err(), AdapterError::UnsupportedTag("laugh".into()));
        assert!(a.render(&req(&a, "voicekiller:Jett", "Hello", "neutral", 1.0)).is_ok());
        assert!(f.hits.lock().unwrap().len() > before);
        assert!(posts(&f).iter().all(|h| !h.body.contains("laugh")));
    }

    #[test]
    fn unknown_voices_and_bad_auth_are_plain_errors() {
        let f = start(happy());
        let a = adapter(&f);
        assert!(matches!(a.render(&req(&a, "voicekiller:Nobody", "Hello", "neutral", 1.0)), Err(AdapterError::UnknownVoice(_))));
        assert!(matches!(a.render(&req(&a, "elevenlabs:x", "Hello", "neutral", 1.0)), Err(AdapterError::UnknownVoice(_))));
        let wrong = VoicekillerAdapter::for_test(Arc::new(|n| Some(if n == "voicekiller" { "wrong".into() } else { USER.into() })), f.base.clone());
        assert_eq!(wrong.check_connection().unwrap_err(), AdapterError::Auth);
    }

    #[test]
    fn a_registered_clone_uses_the_clone_endpoint_and_sends_no_instructions() {
        let f = start(happy());
        let a = adapter(&f);
        let id = format!("{CLONE_PREFIX}My Clone");
        assert!(matches!(a.render(&req(&a, &id, "Hello", "neutral", 1.0)), Err(AdapterError::UnknownVoice(_))), "not usable until registered through the consent path");
        a.allow_voice(&id);
        a.render(&req(&a, &id, "[whisper] Hello there friend", "neutral", 1.2)).unwrap();
        let p = posts(&f);
        assert_eq!(p[0].path, "/api/generate-speech/clone");
        let body: serde_json::Value = serde_json::from_str(&p[0].body).unwrap();
        assert_eq!(body.as_object().unwrap().len(), 2, "only script and voice_name are sent to the clone endpoint");
        let wrong = format!("{CLONE_PREFIX}Other");
        a.allow_voice(&wrong);
        assert!(matches!(a.render(&req(&a, &wrong, "Hello", "neutral", 1.0)), Err(AdapterError::UnknownVoice(_))), "a 404 is an unknown voice");
    }

    #[test]
    fn it_passes_the_shared_adapter_contract() {
        let f = start(happy());
        let a = adapter(&f);
        let v = a.list_voices().into_iter().find(|v| v.name == "Authoritative Advisor 1 (en-GB)").unwrap();
        contract::run(&a, &v, "cough");
    }

    #[test]
    fn garbage_audio_and_empty_lists_are_not_mistaken_for_success() {
        let junk = start(Box::new(|r, _| if r.path.ends_with("voicelist") { (200, b"Jett".to_vec()) } else { (200, b"<html>not audio at all, just a web page</html>".to_vec()) }));
        let a = VoicekillerAdapter::for_test(keys(), junk.base.clone());
        assert!(a.render(&req(&a, "voicekiller:Jett", "Hello", "neutral", 1.0)).is_err());
        let empty = start(Box::new(|_, _| (200, b"  \n".to_vec())));
        let b = VoicekillerAdapter::for_test(keys(), empty.base.clone());
        assert!(b.check_connection().is_err() && b.list_voices().is_empty());
    }
}
