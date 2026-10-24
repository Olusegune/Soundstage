//! ElevenLabs cloud voices (Eleven v3, raw 24 kHz PCM).
//!
//! Facts below were checked against ElevenLabs' public documentation on 2026-10-23 and are
//! logged in docs/PROVIDERS.md. Sources: /docs/api-reference/text-to-speech/convert,
//! /docs/api-reference/voices/search, /docs/eleven-api/resources/errors,
//! /docs/overview/capabilities/text-to-speech/best-practices.
//!
//! Security design (decided before coding):
//! - The host is pinned to https://api.elevenlabs.io and redirects are never followed.
//! - The API key comes from an injected `KeyProvider`, is read per call, and is never stored,
//!   logged, put in a URL, or included in an error.
//! - Voice ids are validated before they reach a URL path.
//! - Response bodies are size-capped. Errors keep only a short machine code, not the body.
//! - Voices in the account that are cloned, professional or "famous" are not listed: using them
//!   would bypass the consent gate (SAFETY_LICENSING.md section 2).

use crate::licence::{Licence, LicenceInfo};
use crate::generate::{clamp_seconds, GenKind, GenLimits, GenOutput, GenRequest, Generator};
use crate::net::read_limited;
use crate::providers::*;
use crate::providers::KeyProvider;
use crate::tags::Token;
use std::io::Read;
use std::sync::Mutex;
use std::time::Duration;


const HOST: &str = "https://api.elevenlabs.io";
const MODEL: &str = "eleven_v3";
const FORMAT: &str = "pcm_24000";
const RATE: u32 = 24_000;
/// About ten minutes of 16-bit mono audio. Anything larger is refused.
const MAX_AUDIO_BYTES: usize = RATE as usize * 2 * 600;
const MAX_JSON_BYTES: usize = 4 * 1024 * 1024;
const MAX_VOICE_PAGES: usize = 10;
const MAX_CLONE_BYTES: usize = 10 * 1024 * 1024;
const MAX_STT_BYTES: usize = 80 * 1024 * 1024;

use crate::net::multipart;
const RETRIES: u32 = 3;

/// Canonical tag to Eleven v3 audio tag. Only tags ElevenLabs documents are mapped; the rest
/// (cough, sneeze, sniff, gasp, chuckle, breath, shout, sad, tired, serious) are unsupported.
pub fn native_tag(canonical: &str) -> Option<&'static str> {
    Some(match canonical {
        "laugh" => "[laughs]",
        "sigh" => "[sighs]",
        "whisper" => "[whispers]",
        "excited" => "[excited]",
        "sarcastic" => "[sarcastic]",
        "curious" => "[curious]",
        "cry" => "[crying]",
        _ => return None,
    })
}

pub struct ElevenLabsAdapter {
    keys: KeyProvider,
    base: String,
    agent: ureq::Agent,
    extra: Mutex<Vec<String>>,
    voices: Mutex<Option<Vec<Voice>>>,
    retry_delay: Duration,
    max_audio_bytes: usize,
}

impl ElevenLabsAdapter {
    pub fn new(keys: KeyProvider) -> Self {
        Self::build(keys, HOST.to_string(), Duration::from_secs(1), MAX_AUDIO_BYTES)
    }

    fn build(keys: KeyProvider, base: String, retry_delay: Duration, max_audio_bytes: usize) -> Self {
        let agent = crate::net::cloud_agent_builder()
            .redirects(0)
            .timeout_connect(Duration::from_secs(10))
            .timeout_write(Duration::from_secs(30))
            .timeout_read(Duration::from_secs(90))
            .user_agent("Soundstage")
            .build();
        ElevenLabsAdapter { keys, base, agent, voices: Mutex::new(None), extra: Mutex::new(Vec::new()), retry_delay, max_audio_bytes }
    }

    #[cfg(test)]
    fn for_test(keys: KeyProvider, base: String, max_audio_bytes: usize) -> Self {
        Self::build(keys, base, Duration::ZERO, max_audio_bytes)
    }

    fn key(&self) -> Result<String, AdapterError> {
        let k = (self.keys)("elevenlabs").ok_or(AdapterError::Auth)?;
        let k = k.trim().to_string();
        // A key that could smuggle header characters is treated as no key at all.
        if k.is_empty() || k.len() > 512 || !k.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(AdapterError::Auth);
        }
        Ok(k)
    }

    fn has_key(&self) -> bool {
        self.key().is_ok()
    }

    fn call(&self, method: &str, path: &str, body: Option<&serde_json::Value>) -> Result<ureq::Response, AdapterError> {
        let key = self.key()?;
        let url = format!("{}{}", self.base, path);
        let mut attempt = 0;
        loop {
            let req = self.agent.request(method, &url).set("xi-api-key", &key);
            let res = match body {
                Some(b) => req.set("content-type", "application/json").send_string(&b.to_string()),
                None => req.call(),
            };
            match res {
                Ok(r) if r.status() == 200 => return Ok(r),
                Ok(r) => return Err(AdapterError::ProviderError(format!("unexpected HTTP {}", r.status()))),
                Err(ureq::Error::Status(code, r)) => {
                    let err = map_status(code, r);
                    if err == AdapterError::RateLimit && attempt < RETRIES {
                        attempt += 1;
                        std::thread::sleep(self.retry_delay * 2u32.pow(attempt - 1));
                        continue;
                    }
                    return Err(err);
                }
                Err(ureq::Error::Transport(t)) => { crate::net::note_transport(&t); return Err(AdapterError::Network); }
            }
        }
    }

    /// One multipart upload, one attempt: a retry after a timeout could be billed or created twice.
    fn post_form(&self, path: &str, fields: &[(&str, &str)], file: (&str, &str, &str, &[u8])) -> Result<ureq::Response, AdapterError> {
        let key = self.key()?;
        let (boundary, body) = multipart(fields, file);
        match self.agent.request("POST", &format!("{}{}", self.base, path)).set("xi-api-key", &key).set("content-type", &format!("multipart/form-data; boundary={boundary}")).send_bytes(&body) {
            Ok(r) if r.status() == 200 => Ok(r),
            Ok(r) => Err(AdapterError::ProviderError(format!("unexpected HTTP {}", r.status()))),
            Err(ureq::Error::Status(code, r)) => Err(map_status(code, r)),
            Err(ureq::Error::Transport(t)) => { crate::net::note_transport(&t); Err(AdapterError::Network) }
        }
    }

    fn voices(&self) -> Vec<Voice> {
        let mut g = self.voices.lock().unwrap_or_else(|e| e.into_inner());
        if g.is_none() {
            // No key means no network call. A failed fetch is retried on the next registry refresh.
            *g = Some(if self.has_key() { self.fetch_voices().unwrap_or_default() } else { Vec::new() });
        }
        g.clone().unwrap_or_default()
    }

    fn fetch_voices(&self) -> Result<Vec<Voice>, AdapterError> {
        let mut out = Vec::new();
        let mut token: Option<String> = None;
        for _ in 0..MAX_VOICE_PAGES {
            let mut path = "/v2/voices?page_size=100".to_string();
            if let Some(t) = &token {
                path.push_str(&format!("&next_page_token={}", url_encode(t)));
            }
            let v = read_json(self.call("GET", &path, None)?)?;
            for item in v.get("voices").and_then(|x| x.as_array()).into_iter().flatten() {
                if let Some(voice) = parse_voice(item) {
                    out.push(voice);
                }
            }
            token = v.get("next_page_token").and_then(|t| t.as_str()).map(String::from);
            if !v.get("has_more").and_then(|h| h.as_bool()).unwrap_or(false) || token.is_none() {
                break;
            }
        }
        Ok(out)
    }

    fn speak(&self, voice_id: &str, text: &str, pace: f32) -> Result<Vec<f32>, AdapterError> {
        let mut body = serde_json::json!({ "text": text, "model_id": MODEL });
        if (pace - 1.0).abs() > 0.01 {
            body["voice_settings"] = serde_json::json!({ "speed": pace.clamp(0.7, 1.2) });
        }
        let path = format!("/v1/text-to-speech/{voice_id}?output_format={FORMAT}");
        let r = self.call("POST", &path, Some(&body))?;
        let mut bytes = Vec::new();
        r.into_reader().take(self.max_audio_bytes as u64 + 1).read_to_end(&mut bytes).map_err(|_| AdapterError::Network)?;
        if bytes.len() > self.max_audio_bytes {
            return Err(AdapterError::ProviderError("the audio was larger than expected".into()));
        }
        if bytes.len() < 2 || bytes.len() % 2 != 0 {
            return Err(AdapterError::ProviderError("the provider returned no usable audio".into()));
        }
        Ok(bytes.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0).collect())
    }
}

/// Short machine code from an error body, limited to a safe alphabet. The body itself is never kept.
fn error_code(r: ureq::Response) -> String {
    let mut buf = Vec::new();
    let _ = r.into_reader().take(16 * 1024).read_to_end(&mut buf);
    let v: serde_json::Value = serde_json::from_slice(&buf).unwrap_or_default();
    let d = &v["detail"];
    let raw = d["code"].as_str().or_else(|| d["status"].as_str()).unwrap_or("");
    raw.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(60).collect()
}

fn map_status(code: u16, r: ureq::Response) -> AdapterError {
    let c = error_code(r);
    if c.contains("quota") || c.contains("insufficient_credits") || code == 402 {
        return AdapterError::Quota;
    }
    match code {
        401 | 403 => AdapterError::Auth,
        404 => AdapterError::UnknownVoice(if c.is_empty() { "voice".into() } else { c }),
        422 => AdapterError::ProviderError("the text or settings were rejected".into()),
        429 => AdapterError::RateLimit,
        _ => AdapterError::ProviderError(format!("HTTP {code} {c}").trim().to_string()),
    }
}

fn read_json(r: ureq::Response) -> Result<serde_json::Value, AdapterError> {
    let mut buf = Vec::new();
    r.into_reader().take(MAX_JSON_BYTES as u64 + 1).read_to_end(&mut buf).map_err(|_| AdapterError::Network)?;
    if buf.len() > MAX_JSON_BYTES {
        return Err(AdapterError::ProviderError("the response was larger than expected".into()));
    }
    serde_json::from_slice(&buf).map_err(|_| AdapterError::ProviderError("the response was not understood".into()))
}

fn url_encode(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

fn valid_voice_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn primary(lang: &str) -> String {
    lang.split(['-', '_']).next().unwrap_or("").to_lowercase()
}

fn parse_voice(v: &serde_json::Value) -> Option<Voice> {
    let id = v["voice_id"].as_str().filter(|i| valid_voice_id(i))?;
    // Cloned, professional and famous voices need a consent record we do not have. Never listed.
    if !matches!(v["category"].as_str(), Some("premade") | Some("generated")) {
        return None;
    }
    let label = |k: &str| v["labels"][k].as_str().map(|s| s.chars().take(40).collect::<String>());
    let language = label("language")
        .or_else(|| v["verified_languages"][0]["language"].as_str().map(String::from))
        .map(|l| primary(&l))
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| "en".into());
    let name: String = v["name"].as_str().unwrap_or("Voice").chars().take(80).collect();
    Some(Voice {
        id: format!("elevenlabs:{id}"),
        name,
        provider: "elevenlabs".into(),
        language,
        accent: label("accent").unwrap_or_else(|| "Not stated".into()),
        gender: label("gender").unwrap_or_else(|| "Not stated".into()),
        age_range: label("age").unwrap_or_else(|| "Not stated".into()),
        licence: licence(),
        sample_text: "Good morning. This is how I sound.".into(),
        notes: "Cloud voice. Script text is sent to ElevenLabs. Tags follow ElevenLabs' documentation and have not yet been listening-tested.".into(),
    })
}

fn licence() -> LicenceInfo {
    LicenceInfo {
        licence: Licence::Unknown,
        source: None,
        checked: None,
        notes: "Commercial rights depend on your ElevenLabs plan. Read the current terms and record your check.".into(),
    }
}

impl Adapter for ElevenLabsAdapter {
    fn id(&self) -> &'static str {
        "elevenlabs"
    }
    fn label(&self) -> &'static str {
        "ElevenLabs (cloud)"
    }
    fn licence(&self) -> LicenceInfo {
        licence()
    }
    fn capabilities(&self) -> Capabilities {
        let mut languages: Vec<String> = self.voices().iter().map(|v| v.language.clone()).collect();
        languages.sort();
        languages.dedup();
        Capabilities {
            languages,
            tags: ["laugh", "sigh", "whisper", "excited", "sarcastic", "curious", "cry"].iter().map(|s| s.to_string()).collect(),
            voice_design: true,
            cloning: true,
            // Eleven v3 allows 5,000 characters; leave room for the tags the adapter adds.
            max_chars_per_request: 4500,
            streaming: false,
        }
    }
    fn list_voices(&self) -> Vec<Voice> {
        self.voices()
    }
    fn provider_version(&self) -> String {
        format!("elevenlabs/{MODEL}/{FORMAT}")
    }
    fn sends_data(&self) -> bool {
        true
    }
    fn ready(&self) -> bool {
        self.has_key()
    }
    fn check_connection(&self) -> Result<(), AdapterError> {
        self.call("GET", "/v2/voices?page_size=1", None).map(|_| ())
    }
    fn design_voice(&self, req: &DesignRequest) -> Result<Vec<DesignPreview>, AdapterError> {
        let d = req.description.split_whitespace().collect::<Vec<_>>().join(" ");
        let n = d.chars().count();
        if !(20..=1000).contains(&n) || d.chars().any(|c| c.is_control()) {
            return Err(AdapterError::ProviderError("describe the voice in 20 to 1000 characters".into()));
        }
        if let Some(r) = crate::safety::check_design(&d) {
            return Err(AdapterError::ProviderError(format!("{} {}", r.message, r.alternative)));
        }
        let mut body = serde_json::json!({ "voice_description": d, "model_id": "eleven_multilingual_ttv_v2" });
        match req.sample_text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
            Some(t) if (100..=1000).contains(&t.chars().count()) && !t.chars().any(|c| c.is_control() && c != '\n') => body["text"] = serde_json::json!(t),
            Some(_) => return Err(AdapterError::ProviderError("the sample text must be 100 to 1000 characters, or left empty".into())),
            None => body["auto_generate_text"] = serde_json::json!(true),
        }
        let v = crate::net::read_json(self.call("POST", "/v1/text-to-voice/design?output_format=mp3_44100_128", Some(&body))?, 40 << 20)?;
        let mut out = Vec::new();
        for p in v["previews"].as_array().into_iter().flatten().take(5) {
            let id = p["generated_voice_id"].as_str().filter(|i| valid_voice_id(i));
            let bytes = p["audio_base_64"].as_str().and_then(crate::net::base64_decode);
            let (Some(id), Some(bytes)) = (id, bytes) else { continue };
            if bytes.len() > 12 * 1024 * 1024 {
                continue;
            }
            if let Ok(dec) = crate::decode::decode_bytes(&bytes, Some("mp3")) {
                out.push(DesignPreview { generated_voice_id: id.to_string(), samples: dec.channels.into_iter().next().unwrap_or_default(), sample_rate: dec.rate });
            }
        }
        if out.is_empty() {
            return Err(AdapterError::ProviderError("the provider returned no usable previews".into()));
        }
        Ok(out)
    }
    fn can_transcribe(&self) -> bool {
        self.has_key()
    }
    fn transcribe(&self, wav: &[u8], language: Option<&str>) -> Result<crate::transcribe::Transcript, AdapterError> {
        if wav.len() < 1000 || wav.len() > MAX_STT_BYTES || !wav.starts_with(b"RIFF") {
            return Err(AdapterError::ProviderError("the recording is not valid".into()));
        }
        let lang = language.map(str::trim).filter(|l| (2..=3).contains(&l.len()) && l.bytes().all(|b| b.is_ascii_lowercase()));
        let mut fields = vec![("model_id", "scribe_v2"), ("timestamps_granularity", "word")];
        if let Some(l) = lang {
            fields.push(("language_code", l));
        }
        let v = crate::net::read_json(self.post_form("/v1/speech-to-text", &fields, ("file", "audio.wav", "audio/wav", wav))?, 32 << 20)?;
        crate::transcribe::parse_elevenlabs(&v).map_err(AdapterError::ProviderError)
    }
    fn can_convert_voice(&self) -> bool {
        self.has_key()
    }
    fn convert_voice(&self, wav: &[u8], voice_id: &str, _proof: &crate::consent::ConsentProof) -> Result<(Vec<f32>, u32), AdapterError> {
        if wav.len() < 1000 || wav.len() > MAX_STT_BYTES || !wav.starts_with(b"RIFF") {
            return Err(AdapterError::ProviderError("the recording is not valid".into()));
        }
        let bare = voice_id.strip_prefix("elevenlabs:").unwrap_or("");
        let known = self.voices().iter().any(|v| v.id == voice_id) || self.extra.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|v| v == voice_id);
        if !valid_voice_id(bare) || !known {
            return Err(AdapterError::UnknownVoice(voice_id.chars().take(60).collect()));
        }
        let bytes = read_limited(self.post_form(&format!("/v1/voice-changer/{bare}"), &[("model_id", "eleven_multilingual_sts_v2"), ("remove_background_noise", "false")], ("audio", "audio.wav", "audio/wav", wav))?, MAX_AUDIO_BYTES * 8)?;
        let d = crate::decode::decode_bytes(&bytes, Some("mp3")).map_err(|_| AdapterError::ProviderError("the provider did not return audio Soundstage could read".into()))?;
        let n = d.channels[0].len();
        Ok(((0..n).map(|i| d.channels.iter().map(|c| c.get(i).copied().unwrap_or(0.0)).sum::<f32>() / d.channels.len() as f32).collect(), d.rate))
    }
    fn can_isolate(&self) -> bool {
        self.has_key()
    }
    fn isolate_audio(&self, wav: &[u8]) -> Result<(Vec<f32>, u32), AdapterError> {
        if wav.len() < 1000 || wav.len() > MAX_STT_BYTES || !wav.starts_with(b"RIFF") {
            return Err(AdapterError::ProviderError("the recording is not valid".into()));
        }
        let bytes = read_limited(self.post_form("/v1/audio-isolation", &[], ("audio", "audio.wav", "audio/wav", wav))?, MAX_AUDIO_BYTES * 8)?;
        let d = crate::decode::decode_bytes(&bytes, Some("mp3")).map_err(|_| AdapterError::ProviderError("the provider did not return audio Soundstage could read".into()))?;
        let n = d.channels[0].len();
        Ok(((0..n).map(|i| d.channels.iter().map(|c| c.get(i).copied().unwrap_or(0.0)).sum::<f32>() / d.channels.len() as f32).collect(), d.rate))
    }
    fn allow_voice(&self, id: &str) {
        let mut g = self.extra.lock().unwrap_or_else(|e| e.into_inner());
        if !g.iter().any(|v| v == id) && g.len() < 500 {
            g.push(id.to_string());
        }
    }
    fn clone_voice(&self, req: &CloneRequest) -> Result<String, AdapterError> {
        let name: String = req.name.split_whitespace().collect::<Vec<_>>().join(" ");
        if name.is_empty() || name.chars().count() > 60 || name.chars().any(|c| c.is_control()) {
            return Err(AdapterError::ProviderError("the voice name is not valid".into()));
        }
        if req.sample_wav.len() < 1000 || req.sample_wav.len() > MAX_CLONE_BYTES || !req.sample_wav.starts_with(b"RIFF") {
            return Err(AdapterError::ProviderError("the recording is not valid".into()));
        }
        let (boundary, body) = multipart(&[("name", &name), ("remove_background_noise", "false"), ("description", "Created in Soundstage.")], ("files", "sample.wav", "audio/wav", &req.sample_wav));
        // One attempt only: a retry after a timeout could create the voice twice.
        let key = self.key()?;
        let res = self.agent.request("POST", &format!("{}/v1/voices/add", self.base)).set("xi-api-key", &key).set("content-type", &format!("multipart/form-data; boundary={boundary}")).send_bytes(&body);
        let res = match res {
            Ok(r) if r.status() == 200 => r,
            Ok(r) => return Err(AdapterError::ProviderError(format!("unexpected HTTP {}", r.status()))),
            Err(ureq::Error::Status(code, r)) => return Err(map_status(code, r)),
            Err(ureq::Error::Transport(t)) => { crate::net::note_transport(&t); return Err(AdapterError::Network); }
        };
        let v = crate::net::read_json(res, 1 << 20)?;
        let vid = v["voice_id"].as_str().filter(|i| valid_voice_id(i)).ok_or_else(|| AdapterError::ProviderError("the provider did not confirm the voice".into()))?;
        *self.voices.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(format!("elevenlabs:{vid}"))
    }
    fn delete_voice(&self, voice_id: &str) -> Result<(), AdapterError> {
        let bare = voice_id.strip_prefix("elevenlabs:").unwrap_or("");
        if !valid_voice_id(bare) {
            return Err(AdapterError::ProviderError("that voice id is not valid".into()));
        }
        self.call("DELETE", &format!("/v1/voices/{bare}"), None)?;
        *self.voices.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.extra.lock().unwrap_or_else(|e| e.into_inner()).retain(|v| v != voice_id);
        Ok(())
    }
    fn save_designed_voice(&self, generated_id: &str, name: &str, description: &str) -> Result<Voice, AdapterError> {
        let name: String = name.split_whitespace().collect::<Vec<_>>().join(" ");
        let desc: String = description.split_whitespace().collect::<Vec<_>>().join(" ");
        if !valid_voice_id(generated_id) || name.is_empty() || name.chars().count() > 80 || name.chars().any(|c| c.is_control()) || !(20..=1000).contains(&desc.chars().count()) {
            return Err(AdapterError::ProviderError("the name or description is not valid".into()));
        }
        let body = serde_json::json!({ "voice_name": name, "voice_description": desc, "generated_voice_id": generated_id });
        let v = crate::net::read_json(self.call("POST", "/v1/text-to-voice", Some(&body))?, 1 << 20)?;
        let vid = v["voice_id"].as_str().filter(|i| valid_voice_id(i)).ok_or_else(|| AdapterError::ProviderError("the provider did not confirm the voice".into()))?;
        // The saved voice is new, so the cached list is stale.
        *self.voices.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(Voice {
            id: format!("elevenlabs:{vid}"), name, provider: "elevenlabs".into(), language: "en".into(), accent: "Designed".into(), gender: "Not stated".into(), age_range: "Not stated".into(),
            licence: licence(), sample_text: "Good morning. This is how I sound.".into(),
            notes: "A voice you designed. It uses one voice slot in your ElevenLabs account.".into(),
        })
    }
    fn render(&self, req: &RenderRequest) -> Result<RenderOutput, AdapterError> {
        // Validate everything locally before any network call.
        precheck(&self.capabilities(), req)?;
        let bare = req.voice_id.strip_prefix("elevenlabs:").unwrap_or("");
        let known = self.voices().iter().any(|v| v.id == req.voice_id) || self.extra.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|v| *v == req.voice_id);
        if !valid_voice_id(bare) || !known {
            return Err(AdapterError::UnknownVoice(req.voice_id.chars().take(60).collect()));
        }
        // v3 has no SSML breaks, so pauses are exact silence inserted here between segments.
        let mut current = req.emotion.clone();
        let mut segs: Vec<Result<String, u32>> = Vec::new();
        let mut buf = String::new();
        let mut started = false;
        let start_style = |current: &str| native_tag(current).map(|t| format!("{t} ")).unwrap_or_default();
        for t in &req.tokens {
            match t {
                Token::Text(s) => {
                    if !started {
                        buf.push_str(&start_style(&current));
                        started = true;
                    }
                    buf.push_str(s);
                    buf.push(' ');
                }
                Token::Delivery(d) => {
                    started = true;
                    current = d.clone();
                    buf.push_str(native_tag(d).ok_or_else(|| AdapterError::UnsupportedTag(d.clone()))?);
                    buf.push(' ');
                }
                Token::NonVerbal(n) => {
                    if !started {
                        buf.push_str(&start_style(&current));
                        started = true;
                    }
                    buf.push_str(native_tag(n).ok_or_else(|| AdapterError::UnsupportedTag(n.clone()))?);
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
        let mut samples: Vec<f32> = Vec::new();
        for s in segs {
            match s {
                Ok(text) => samples.extend(self.speak(bare, &text, req.pace)?),
                Err(ms) => samples.extend(std::iter::repeat(0.0).take((ms as usize * RATE as usize) / 1000)),
            }
        }
        Ok(RenderOutput { samples, sample_rate: RATE, provider_version: self.provider_version() })
    }
}

/// ElevenLabs music (`/v1/music`) and sound effects (`/v1/sound-generation`). Facts verified from the public docs on
/// 2026-10-23: music length 3 to 600 seconds, effects 0.5 to 30 seconds, output as downloadable audio.
pub struct ElevenLabsGenerator {
    inner: ElevenLabsAdapter,
}

const MAX_GEN_BYTES: usize = 60 * 1024 * 1024;
const MUSIC: GenLimits = GenLimits { min_s: 3.0, max_s: 300.0, default_s: 30.0, supports_length: true };
const SFX: GenLimits = GenLimits { min_s: 0.5, max_s: 30.0, default_s: 4.0, supports_length: true };

impl ElevenLabsGenerator {
    pub fn new(keys: KeyProvider) -> Self {
        ElevenLabsGenerator { inner: ElevenLabsAdapter::new(keys) }
    }
    #[cfg(test)]
    fn for_test(keys: KeyProvider, base: String) -> Self {
        ElevenLabsGenerator { inner: ElevenLabsAdapter::for_test(keys, base, MAX_AUDIO_BYTES) }
    }
}

impl Generator for ElevenLabsGenerator {
    fn id(&self) -> &str {
        "elevenlabs-audio"
    }
    fn label(&self) -> &str {
        "ElevenLabs music and effects"
    }
    fn kinds(&self) -> Vec<GenKind> {
        vec![GenKind::Music, GenKind::Sfx]
    }
    fn limits(&self, kind: GenKind) -> GenLimits {
        if kind == GenKind::Music { MUSIC } else { SFX }
    }
    fn licence(&self) -> LicenceInfo {
        LicenceInfo { licence: Licence::Unknown, source: None, checked: None, notes: "Rights to ElevenLabs music and sound effects depend on your plan. Read their current terms and record your check before a commercial export.".into() }
    }
    fn notes(&self) -> String {
        "Your description is sent to ElevenLabs and uses your plan's credits. Soundstage cannot show the cost in advance.".into()
    }
    fn sends_data(&self) -> bool {
        true
    }
    fn ready(&self) -> bool {
        self.inner.has_key()
    }
    fn check_connection(&self) -> Result<(), AdapterError> {
        self.inner.check_connection()
    }
    fn generate(&self, req: &GenRequest) -> Result<GenOutput, AdapterError> {
        let secs = clamp_seconds(&self.limits(req.kind), req.seconds);
        let (path, body, version) = match req.kind {
            GenKind::Music => (
                "/v1/music?output_format=mp3_44100_128",
                serde_json::json!({ "prompt": req.prompt, "music_length_ms": (secs * 1000.0).round() as u32, "force_instrumental": req.instrumental }),
                "elevenlabs/music",
            ),
            GenKind::Sfx => ("/v1/sound-generation?output_format=mp3_44100_128", serde_json::json!({ "text": req.prompt, "duration_seconds": secs }), "elevenlabs/sound-generation"),
        };
        let bytes = read_limited(self.inner.call("POST", path, Some(&body))?, MAX_GEN_BYTES)?;
        let audio = crate::decode::to_stereo_48k(&bytes, Some("mp3")).map_err(|_| AdapterError::ProviderError("the provider did not return audio Soundstage could read".into()))?;
        Ok(GenOutput { audio, provider_version: version.into() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tags::tokenize;
    use std::sync::Arc;

    use crate::testutil::{start, Fake, Handler, Req};

    const KEY: &str = "test-key-DO-NOT-LEAK-12345";
    fn keys() -> KeyProvider { Arc::new(|_| Some(KEY.to_string())) }

    fn pcm_for(text: &str) -> Vec<u8> {
        let words = text.split_whitespace().filter(|w| !w.starts_with('[') && !w.ends_with(']')).count().max(1);
        (0..(words * 350 * RATE as usize / 1000)).flat_map(|n| ((8000.0 * (n as f32 * 0.07).sin()) as i16).to_le_bytes()).collect()
    }

    fn voices_json() -> Vec<u8> {
        serde_json::json!({
            "voices": [
                { "voice_id": "VoiceAAA111", "name": "Ada", "category": "premade", "labels": { "accent": "british", "gender": "female", "age": "young", "language": "en" } },
                { "voice_id": "VoiceBBB222", "name": "Cloned Person", "category": "cloned", "labels": {} },
                { "voice_id": "Voice/../evil", "name": "Bad Id", "category": "premade", "labels": {} },
                { "voice_id": "VoiceCCC333", "name": "Famous One", "category": "famous", "labels": {} },
                { "voice_id": "VoiceDDD444", "name": "Designed", "category": "generated", "labels": { "language": "yo-NG" } }
            ],
            "has_more": false, "next_page_token": null
        }).to_string().into_bytes()
    }

    fn happy() -> Handler {
        Box::new(|r, _| {
            if r.path.starts_with("/v2/voices") { (200, voices_json()) }
            else if r.path.starts_with("/v1/text-to-speech/") {
                let v: serde_json::Value = serde_json::from_str(&r.body).unwrap_or_default();
                (200, pcm_for(v["text"].as_str().unwrap_or("")))
            } else { (404, b"{}".to_vec()) }
        })
    }

    fn adapter(f: &Fake) -> ElevenLabsAdapter { ElevenLabsAdapter::for_test(keys(), f.base.clone(), MAX_AUDIO_BYTES) }
    fn request(text: &str, emotion: &str, voice: &str) -> RenderRequest {
        RenderRequest { tokens: tokenize(text).0, voice_id: voice.into(), emotion: emotion.into(), pace: 1.0, language: "en".into() }
    }
    fn tts_hits(f: &Fake) -> Vec<Req> { f.hits.lock().unwrap().iter().filter(|r| r.method == "POST").cloned().collect() }

    #[test]
    fn passes_the_adapter_contract_against_a_fake_server() {
        let f = start(happy());
        let a = adapter(&f);
        let v = a.list_voices().into_iter().find(|v| v.name == "Ada").unwrap();
        contract::run(&a, &v, "sneeze");
    }

    #[test]
    fn only_safe_premade_and_generated_voices_are_listed() {
        let f = start(happy());
        let names: Vec<String> = adapter(&f).list_voices().into_iter().map(|v| v.name).collect();
        assert_eq!(names, vec!["Ada", "Designed"], "cloned, famous and badly-named voices must never appear");
        let d = adapter(&f).list_voices().into_iter().find(|v| v.name == "Designed").unwrap();
        assert_eq!(d.language, "yo");
        assert_eq!(d.licence.licence, Licence::Unknown);
    }

    #[test]
    fn no_key_means_no_network_call_and_an_auth_error() {
        let f = start(happy());
        let a = ElevenLabsAdapter::for_test(Arc::new(|_| None), f.base.clone(), MAX_AUDIO_BYTES);
        assert!(a.list_voices().is_empty());
        assert_eq!(a.check_connection().unwrap_err(), AdapterError::Auth);
        assert!(f.hits.lock().unwrap().is_empty());
    }

    #[test]
    fn header_injection_keys_are_refused_locally() {
        let f = start(happy());
        let a = ElevenLabsAdapter::for_test(Arc::new(|_| Some("abc\r\nx-evil: 1".into())), f.base.clone(), MAX_AUDIO_BYTES);
        assert_eq!(a.check_connection().unwrap_err(), AdapterError::Auth);
        assert!(f.hits.lock().unwrap().is_empty());
    }

    #[test]
    fn key_goes_in_the_header_never_the_url_or_body() {
        let f = start(happy());
        let a = adapter(&f);
        let v = a.list_voices().remove(0);
        a.render(&request("hello there", "neutral", &v.id)).unwrap();
        for h in f.hits.lock().unwrap().iter() {
            assert!(h.headers.iter().any(|(k, val)| k == "xi-api-key" && val == KEY));
            assert!(!h.path.contains(KEY) && !h.body.contains(KEY));
        }
    }

    #[test]
    fn tags_map_to_v3_audio_tags_and_pauses_become_exact_silence() {
        let f = start(happy());
        let a = adapter(&f);
        let v = a.list_voices().remove(0);
        let out = a.render(&request("[laugh] hi [whisper] there [pause:500] bye", "neutral", &v.id)).unwrap();
        let bodies: Vec<serde_json::Value> = tts_hits(&f).iter().map(|r| serde_json::from_str(&r.body).unwrap()).collect();
        assert_eq!(bodies.len(), 2, "one request per spoken segment");
        assert_eq!(bodies[0]["text"], "[laughs] hi [whispers] there");
        assert_eq!(bodies[1]["text"], "[whispers] bye", "the active style carries across a pause");
        assert_eq!(bodies[0]["model_id"], "eleven_v3");
        assert!(tts_hits(&f)[0].path.contains("output_format=pcm_24000"));
        let spoken = bodies.iter().map(|b| pcm_for(b["text"].as_str().unwrap()).len() / 2).sum::<usize>();
        assert_eq!(out.samples.len(), spoken + RATE as usize / 2);
    }

    #[test]
    fn unsupported_tags_are_refused_before_any_request() {
        let f = start(happy());
        let a = adapter(&f);
        let v = a.list_voices().remove(0);
        for tag in ["sneeze", "cough", "sniff", "gasp", "chuckle", "breath", "shout"] {
            let e = a.render(&request(&format!("hi [{tag}]"), "neutral", &v.id)).unwrap_err();
            assert_eq!(e, AdapterError::UnsupportedTag(tag.into()));
        }
        assert_eq!(a.render(&request("hi", "sad", &v.id)).unwrap_err(), AdapterError::UnsupportedTag("sad".into()));
        assert!(tts_hits(&f).is_empty());
    }

    #[test]
    fn unknown_or_malformed_voice_ids_never_reach_the_network() {
        let f = start(happy());
        let a = adapter(&f);
        let _ = a.list_voices();
        for id in ["elevenlabs:VoiceBBB222", "elevenlabs:../../x", "elevenlabs:", "other:VoiceAAA111"] {
            assert!(matches!(a.render(&request("hi", "neutral", id)), Err(AdapterError::UnknownVoice(_))), "{id}");
        }
        assert!(tts_hits(&f).is_empty());
    }

    fn status_handler(code: u16, body: &'static str) -> Handler {
        Box::new(move |r, _| if r.path.starts_with("/v2/voices") { (200, voices_json()) } else { (code, body.as_bytes().to_vec()) })
    }

    #[test]
    fn provider_errors_map_to_the_common_set_without_leaking_the_key() {
        for (code, body, want) in [
            (401, r#"{"detail":{"code":"invalid_api_key","message":"bad"}}"#, AdapterError::Auth),
            (401, r#"{"detail":{"status":"quota_exceeded","message":"x"}}"#, AdapterError::Quota),
            (402, r#"{"detail":{"code":"insufficient_credits"}}"#, AdapterError::Quota),
            (404, r#"{"detail":{"code":"voice_not_found"}}"#, AdapterError::UnknownVoice("voice_not_found".into())),
        ] {
            let f = start(status_handler(code, body));
            let a = adapter(&f);
            let v = a.list_voices().remove(0);
            let e = a.render(&request("hi", "neutral", &v.id)).unwrap_err();
            assert_eq!(e, want, "status {code}");
            assert!(!format!("{e:?} {e}").contains(KEY));
        }
        let f = start(status_handler(422, r#"{"detail":[{"msg":"echo hi secret text"}]}"#));
        let a = adapter(&f);
        let v = a.list_voices().remove(0);
        let e = a.render(&request("hi", "neutral", &v.id)).unwrap_err();
        assert!(matches!(&e, AdapterError::ProviderError(m) if !m.contains("secret")));
        let f = start(status_handler(500, "{}"));
        let a = adapter(&f);
        let v = a.list_voices().remove(0);
        assert!(matches!(a.render(&request("hi", "neutral", &v.id)), Err(AdapterError::ProviderError(_))));
    }

    #[test]
    fn rate_limits_retry_with_backoff_then_succeed_or_give_up() {
        let flaky: Handler = Box::new(|r, _| {
            if r.path.starts_with("/v2/voices") { return (200, voices_json()); }
            static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            if N.fetch_add(1, std::sync::atomic::Ordering::SeqCst) < 2 { (429, br#"{"detail":{"code":"system_busy"}}"#.to_vec()) } else { (200, pcm_for("hello")) }
        });
        let f = start(flaky);
        let a = adapter(&f);
        let v = a.list_voices().remove(0);
        assert!(a.render(&request("hello", "neutral", &v.id)).is_ok());
        assert_eq!(tts_hits(&f).len(), 3);

        let f = start(status_handler(429, r#"{"detail":{"code":"rate_limit_exceeded"}}"#));
        let a = adapter(&f);
        let v = a.list_voices().remove(0);
        assert_eq!(a.render(&request("hello", "neutral", &v.id)).unwrap_err(), AdapterError::RateLimit);
        assert_eq!(tts_hits(&f).len(), 1 + RETRIES as usize, "gives up after the retry budget");
    }

    #[test]
    fn redirects_are_never_followed() {
        let evil = start(happy());
        let target = evil.base.clone();
        let f = start(Box::new(move |r, _| {
            if r.path.starts_with("/v2/voices") { (200, voices_json()) } else { (302, format!("{target}/stolen").into_bytes()) }
        }));
        let a = adapter(&f);
        let v = a.list_voices().remove(0);
        assert!(a.render(&request("hi", "neutral", &v.id)).is_err());
        assert!(evil.hits.lock().unwrap().is_empty(), "no request may reach another host");
    }

    #[test]
    fn oversized_or_malformed_audio_is_refused() {
        let f = start(Box::new(|r, _| if r.path.starts_with("/v2/voices") { (200, voices_json()) } else { (200, vec![1u8; 5000]) }));
        let a = ElevenLabsAdapter::for_test(keys(), f.base.clone(), 4000);
        let v = a.list_voices().remove(0);
        assert!(matches!(a.render(&request("hi", "neutral", &v.id)), Err(AdapterError::ProviderError(_))));
        let f = start(Box::new(|r, _| if r.path.starts_with("/v2/voices") { (200, voices_json()) } else { (200, vec![1u8; 3]) }));
        let a = adapter(&f);
        let v = a.list_voices().remove(0);
        assert!(matches!(a.render(&request("hi", "neutral", &v.id)), Err(AdapterError::ProviderError(_))));
    }

    #[test]
    fn unreachable_provider_is_a_network_error() {
        let a = ElevenLabsAdapter::for_test(keys(), "http://127.0.0.1:1".into(), MAX_AUDIO_BYTES);
        assert_eq!(a.check_connection().unwrap_err(), AdapterError::Network);
    }

    #[test]
    fn production_adapter_is_pinned_to_the_elevenlabs_host() {
        let a = ElevenLabsAdapter::new(keys());
        assert_eq!(a.base, "https://api.elevenlabs.io");
        assert!(a.sends_data());
    }

    fn wav_bytes(secs: f32) -> Vec<u8> {
        let mut cur = std::io::Cursor::new(Vec::new());
        {
            let spec = hound::WavSpec { channels: 1, sample_rate: 44_100, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
            let mut w = hound::WavWriter::new(&mut cur, spec).unwrap();
            for i in 0..(secs * 44_100.0) as usize { w.write_sample((9000.0 * (i as f32 * 0.05).sin()) as i16).unwrap(); }
            w.finalize().unwrap();
        }
        cur.into_inner()
    }

    #[test]
    fn music_and_effects_use_the_documented_requests_and_come_back_as_48k_stereo() {
        let f = start(Box::new(|r, _| if r.path.starts_with("/v1/music") || r.path.starts_with("/v1/sound-generation") { (200, wav_bytes(1.0)) } else { (404, b"{}".to_vec()) }));
        let g = ElevenLabsGenerator::for_test(keys(), f.base.clone());
        assert!(g.ready() && g.sends_data());
        let out = g.generate(&GenRequest { kind: GenKind::Music, prompt: "warm piano".into(), seconds: Some(45.0), instrumental: true }).unwrap();
        assert!((out.audio.len() as i64 - 48_000).abs() < 100, "{}", out.audio.len());
        assert_eq!(out.audio.l.len(), out.audio.r.len());
        g.generate(&GenRequest { kind: GenKind::Sfx, prompt: "door slam".into(), seconds: Some(500.0), instrumental: false }).unwrap();
        let hits = f.hits.lock().unwrap();
        let m: serde_json::Value = serde_json::from_str(&hits[0].body).unwrap();
        assert_eq!((m["prompt"].as_str(), m["music_length_ms"].as_u64(), m["force_instrumental"].as_bool()), (Some("warm piano"), Some(45_000), Some(true)));
        assert!(hits[0].path.contains("output_format=mp3_44100_128"));
        let s: serde_json::Value = serde_json::from_str(&hits[1].body).unwrap();
        assert_eq!((s["text"].as_str(), s["duration_seconds"].as_f64()), (Some("door slam"), Some(30.0)), "effect length is clamped to the documented maximum");
        for h in hits.iter() {
            assert!(h.headers.iter().any(|(k, v)| k == "xi-api-key" && v == KEY));
            assert!(!h.path.contains(KEY) && !h.body.contains(KEY));
        }
    }

    #[test]
    fn generators_without_a_key_make_no_request_and_failures_are_plain() {
        let f = start(Box::new(|_, _| (402, br#"{"detail":{"code":"insufficient_credits"}}"#.to_vec())));
        let none = ElevenLabsGenerator::for_test(Arc::new(|_| None), f.base.clone());
        assert!(!none.ready());
        let req = GenRequest { kind: GenKind::Music, prompt: "x y z".into(), seconds: None, instrumental: false };
        assert_eq!(none.generate(&req).unwrap_err(), AdapterError::Auth);
        assert!(f.hits.lock().unwrap().is_empty());
        let g = ElevenLabsGenerator::for_test(keys(), f.base.clone());
        let e = g.generate(&req).unwrap_err();
        assert_eq!(e, AdapterError::Quota);
        assert!(!format!("{e:?}").contains(KEY));
        let junk = start(Box::new(|_, _| (200, b"<html>nope</html>".to_vec())));
        assert!(matches!(ElevenLabsGenerator::for_test(keys(), junk.base.clone()).generate(&req), Err(AdapterError::ProviderError(_))));
    }

    #[test]
    fn voice_design_returns_previews_and_saves_one_as_a_permanent_voice() {
        let wav = wav_bytes(0.5);
        let b64 = {
            const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut o = String::new();
            for c in wav.chunks(3) {
                let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
                o.push(T[(n >> 18) as usize & 63] as char); o.push(T[(n >> 12) as usize & 63] as char);
                o.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' }); o.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
            }
            o
        };
        let b = b64.clone();
        let f = start(Box::new(move |r, _| {
            if r.path.starts_with("/v1/text-to-voice/design") {
                (200, serde_json::json!({"previews":[{"generated_voice_id":"GenAAA111","audio_base_64":b},{"generated_voice_id":"Gen/bad","audio_base_64":b},{"generated_voice_id":"GenBBB222","audio_base_64":"!!!not base64"},{"generated_voice_id":"GenCCC333","audio_base_64":b}],"text":"x"}).to_string().into_bytes())
            } else if r.method == "POST" && r.path == "/v1/text-to-voice" {
                (200, serde_json::json!({"voice_id":"SavedVoice999","category":"generated"}).to_string().into_bytes())
            } else if r.path.starts_with("/v2/voices") { (200, voices_json()) } else { (404, b"{}".to_vec()) }
        }));
        let a = adapter(&f);
        assert!(a.capabilities().voice_design);
        let prev = a.design_voice(&DesignRequest { description: "a calm older woman with a warm Lagos accent".into(), sample_text: None }).unwrap();
        assert_eq!(prev.iter().map(|p| p.generated_voice_id.as_str()).collect::<Vec<_>>(), vec!["GenAAA111", "GenCCC333"], "unsafe ids and undecodable previews are dropped");
        assert_eq!(prev[0].sample_rate, 44_100);
        let sent: serde_json::Value = serde_json::from_str(&f.hits.lock().unwrap().iter().find(|h| h.path.starts_with("/v1/text-to-voice/design")).unwrap().body).unwrap();
        assert_eq!((sent["model_id"].as_str(), sent["auto_generate_text"].as_bool()), (Some("eleven_multilingual_ttv_v2"), Some(true)));
        assert!(!sent.to_string().contains(KEY));
        let v = a.save_designed_voice("GenAAA111", "Mama Ade", "a calm older woman with a warm Lagos accent").unwrap();
        assert_eq!((v.id.as_str(), v.licence.licence), ("elevenlabs:SavedVoice999", Licence::Unknown));
        let saved: serde_json::Value = serde_json::from_str(&f.hits.lock().unwrap().iter().find(|h| h.method == "POST" && h.path == "/v1/text-to-voice").unwrap().body).unwrap();
        assert_eq!((saved["voice_name"].as_str(), saved["generated_voice_id"].as_str()), (Some("Mama Ade"), Some("GenAAA111")));
    }

    #[test]
    fn cloning_uploads_only_audio_and_a_name_and_the_clone_renders_after_registration() {
        let f = start(Box::new(|r, _| {
            if r.method == "POST" && r.path == "/v1/voices/add" { (200, serde_json::json!({"voice_id":"CloneVoice777"}).to_string().into_bytes()) }
            else if r.method == "DELETE" && r.path == "/v1/voices/CloneVoice777" { (200, b"{}".to_vec()) }
            else if r.path.starts_with("/v2/voices") { (200, voices_json()) }
            else if r.path.starts_with("/v1/text-to-speech/CloneVoice777") { (200, vec![0u8; 4000]) }
            else { (404, b"{}".to_vec()) }
        }));
        let a = adapter(&f);
        assert!(a.capabilities().cloning);
        let rec = crate::clone::build_record(&crate::clone::Attestation { subject: "Ama Okafor".into(), subject_is_self: true, adult: true, alive: true, not_public_figure: true, agrees: true, purpose: "Narration".into(), commercial: false, expires: None }, "consent-t").unwrap();
        let proof = crate::consent::proof(&rec, &crate::consent::Use::Dialogue, false).unwrap();
        let mut wav = b"RIFF".to_vec();
        wav.extend(vec![7u8; 5000]);
        let id = a.clone_voice(&CloneRequest { name: "Ama".into(), sample_wav: wav.clone(), proof }).unwrap();
        assert_eq!(id, "elevenlabs:CloneVoice777");
        let hit = f.hits.lock().unwrap().iter().find(|h| h.path == "/v1/voices/add").cloned().unwrap();
        assert!(hit.headers.iter().any(|(k, v)| k == "content-type" && v.starts_with("multipart/form-data; boundary=")));
        assert!(hit.body.contains("name=\"name\"") && hit.body.contains("Ama"), "name is sent");
        for private in ["Okafor", "consent", "Narration", "subject"] {
            assert!(!hit.body.contains(private), "{private} must never be uploaded");
        }
        assert!(!hit.body.contains(KEY));
        // Not in the account list, so it only renders once registered through the consent-checked path.
        a.allow_voice(&id);
        assert!(a.delete_voice(&id).is_ok());
        assert!(a.delete_voice("elevenlabs:../x").is_err());
        let bad_proof = crate::consent::proof(&rec, &crate::consent::Use::PerformanceReference, false);
        assert!(bad_proof.is_err(), "a proof cannot be made for a use the record does not permit");
        let tiny = crate::consent::proof(&rec, &crate::consent::Use::Dialogue, false).unwrap();
        assert!(a.clone_voice(&CloneRequest { name: "x".into(), sample_wav: b"nope".to_vec(), proof: tiny }).is_err(), "bad audio never reaches the network");
    }

    #[test]
    fn transcription_and_cleaning_upload_audio_only_and_parse_the_answer() {
        let wav = {
            let mut w = b"RIFF".to_vec();
            w.extend(vec![3u8; 4000]);
            w
        };
        let f = start(Box::new(|r, _| match r.path.as_str() {
            "/v1/speech-to-text" => (200, serde_json::json!({"text":"Hello there","language_code":"eng","words":[{"text":"Hello","type":"word","start":0.0,"end":0.4},{"text":"there","type":"word","start":0.5,"end":0.9}]}).to_string().into_bytes()),
            "/v1/audio-isolation" => (200, b"not audio".to_vec()),
            _ => (404, b"{}".to_vec()),
        }));
        let a = adapter(&f);
        assert!(a.can_transcribe() && a.can_isolate());
        let t = a.transcribe(&wav, Some("en")).unwrap();
        assert_eq!((t.text.as_str(), t.words.len()), ("Hello there", 2));
        let hit = f.hits.lock().unwrap().iter().find(|h| h.path == "/v1/speech-to-text").cloned().unwrap();
        assert!(hit.body.contains("name=\"model_id\"") && hit.body.contains("scribe_v2") && hit.body.contains("name=\"language_code\""));
        assert!(!hit.body.contains(KEY));
        assert!(a.transcribe(b"nope", None).is_err(), "bad audio never reaches the network");
        assert!(a.transcribe(&wav, Some("e n; drop")).is_ok(), "an odd language code is ignored, not sent");
        assert!(a.isolate_audio(&wav).is_err(), "a non-audio answer is an error, not silence");
        let none = ElevenLabsAdapter::for_test(Arc::new(|_| None), f.base.clone(), MAX_AUDIO_BYTES);
        assert!(!none.can_transcribe());
    }

    #[test]
    fn the_voice_changer_needs_proof_a_known_voice_and_real_audio() {
        let wavfile = |secs: f32| {
            let n = (secs * 22_050.0) as usize;
            let mut cur = std::io::Cursor::new(Vec::new());
            {
                let spec = hound::WavSpec { channels: 1, sample_rate: 22_050, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
                let mut w = hound::WavWriter::new(&mut cur, spec).unwrap();
                for i in 0..n { w.write_sample((6000.0 * (i as f32 * 0.05).sin()) as i16).unwrap(); }
                w.finalize().unwrap();
            }
            cur.into_inner()
        };
        let out = wavfile(0.5);
        let f = start(Box::new(move |r, _| {
            if r.path.starts_with("/v1/voice-changer/VoiceAAA111") { (200, out.clone()) }
            else if r.path.starts_with("/v2/voices") { (200, voices_json()) } else { (404, b"{}".to_vec()) }
        }));
        let a = adapter(&f);
        let rec = crate::clone::build_performance_record(&crate::clone::Attestation { subject: "Ama Okafor".into(), subject_is_self: true, adult: true, alive: true, not_public_figure: true, agrees: true, purpose: "Voice acting".into(), commercial: false, expires: None }, "consent-v").unwrap();
        let proof = crate::consent::proof(&rec, &crate::consent::Use::PerformanceReference, false).unwrap();
        assert!(a.can_convert_voice());
        let input = wavfile(1.0);
        let (samples, rate) = a.convert_voice(&input, "elevenlabs:VoiceAAA111", &proof).unwrap();
        assert!(samples.len() > 1000 && rate > 0);
        let hit = f.hits.lock().unwrap().iter().find(|h| h.path.starts_with("/v1/voice-changer/")).cloned().unwrap();
        assert!(hit.body.contains("name=\"audio\"") && hit.body.contains("eleven_multilingual_sts_v2"));
        for private in ["Okafor", "consent", "Voice acting"] {
            assert!(!hit.body.contains(private), "{private} must never be uploaded");
        }
        assert!(matches!(a.convert_voice(&input, "elevenlabs:NotListed", &proof), Err(AdapterError::UnknownVoice(_))));
        assert!(matches!(a.convert_voice(&input, "elevenlabs:../x", &proof), Err(AdapterError::UnknownVoice(_))));
        assert!(a.convert_voice(b"nope", "elevenlabs:VoiceAAA111", &proof).is_err());
    }

    #[test]
    fn voice_design_refuses_bad_input_before_any_request() {
        let f = start(happy());
        let a = adapter(&f);
        let d = |t: &str, s: Option<&str>| DesignRequest { description: t.into(), sample_text: s.map(String::from) };
        for bad in [d("short", None), d("sounds like Morgan Freeman and speaks slowly", None), d(&"x".repeat(1001), None), d("a calm older woman with a warm accent", Some("too short"))] {
            assert!(a.design_voice(&bad).is_err());
        }
        for (id, name, desc) in [("../x", "Ok", "a calm older woman with a warm accent"), ("GenAAA111", "", "a calm older woman with a warm accent"), ("GenAAA111", "Ok", "short")] {
            assert!(a.save_designed_voice(id, name, desc).is_err());
        }
        assert!(f.hits.lock().unwrap().iter().all(|h| !h.path.contains("text-to-voice")), "nothing was sent");
    }

    #[test]
    fn url_encoding_and_voice_id_rules() {
        assert_eq!(url_encode("a b/c+d=="), "a%20b%2Fc%2Bd%3D%3D");
        assert!(valid_voice_id("21m00Tcm4TlvDq8ikWAM") && !valid_voice_id("a/b") && !valid_voice_id(""));
        assert_eq!(native_tag("sneeze"), None);
        assert_eq!(native_tag("laugh"), Some("[laughs]"));
    }
}
