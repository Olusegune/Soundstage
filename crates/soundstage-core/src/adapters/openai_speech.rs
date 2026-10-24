//! Any server that speaks the OpenAI speech API: `POST {base}/audio/speech` with `model`, `input` and `voice`.
//! That covers many local engines (Kokoro-FastAPI, LocalAI, Speaches, Chatterbox and Orpheus servers and more)
//! and cloud routers such as OpenRouter, with one adapter. The user declares voices and tags; Soundstage claims nothing
//! about quality, accent or non-verbal ability. Audio comes back as WAV, MP3, FLAC, Opus or AAC and is decoded in-process.
//!
//! Security: the address is vetted by `net::parse_base_url` (https anywhere, http only on this computer or a private
//! network), redirects are never followed, keys travel only in the Authorization header, and response sizes are capped.

use crate::audio::resample;
use crate::decode::decode_bytes;
use crate::engines::{SpeechEngine, VoiceDef};
use crate::licence::{Licence, LicenceInfo};
use crate::net::{error_from_status, read_json, read_limited, BaseUrl};
use crate::providers::*;
use std::time::Duration;

const MAX_AUDIO_BYTES: usize = 64 * 1024 * 1024;
const RETRIES: u32 = 3;

pub struct OpenAiSpeechAdapter {
    id: String,
    label: String,
    cfg: SpeechEngine,
    base: BaseUrl,
    keys: KeyProvider,
    agent: ureq::Agent,
    retry_delay: Duration,
}

impl OpenAiSpeechAdapter {
    pub fn new(cfg: SpeechEngine, keys: KeyProvider) -> Result<Self, String> {
        cfg.validate()?;
        let base = crate::net::parse_base_url(&cfg.base_url)?;
        let agent = ureq::AgentBuilder::new().redirects(0).timeout_connect(Duration::from_secs(10)).timeout_write(Duration::from_secs(30)).timeout_read(Duration::from_secs(180)).user_agent("Soundstage").build();
        Ok(OpenAiSpeechAdapter { id: format!("speech-{}", cfg.id), label: cfg.label.clone(), base, keys, agent, retry_delay: Duration::from_secs(1), cfg })
    }

    #[cfg(test)]
    fn instant(mut self) -> Self {
        self.retry_delay = Duration::ZERO;
        self
    }

    pub fn key_name(&self) -> String {
        format!("engine:{}", self.cfg.id)
    }

    /// The saved key, if it is safe to put in a header. A key that is required but missing is an auth error.
    fn key(&self) -> Result<Option<String>, AdapterError> {
        match (self.keys)(&self.key_name()).map(|k| k.trim().to_string()) {
            Some(k) if !k.is_empty() && k.len() <= 512 && k.bytes().all(|b| b.is_ascii_graphic()) => Ok(Some(k)),
            Some(_) => Err(AdapterError::Auth),
            None if self.cfg.needs_key => Err(AdapterError::Auth),
            None => Ok(None),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base.as_string(), path)
    }

    fn call(&self, method: &str, path: &str, body: Option<&serde_json::Value>) -> Result<ureq::Response, AdapterError> {
        let key = self.key()?;
        let mut attempt = 0;
        loop {
            let mut req = self.agent.request(method, &self.url(path));
            if let Some(k) = &key {
                req = req.set("Authorization", &format!("Bearer {k}"));
            }
            let res = match body {
                Some(b) => req.set("content-type", "application/json").send_string(&b.to_string()),
                None => req.call(),
            };
            match res {
                Ok(r) if r.status() == 200 => return Ok(r),
                Ok(r) => return Err(AdapterError::ProviderError(format!("unexpected HTTP {}", r.status()))),
                Err(ureq::Error::Status(code, r)) => {
                    let e = error_from_status(code, r);
                    if e == AdapterError::RateLimit && attempt < RETRIES {
                        attempt += 1;
                        std::thread::sleep(self.retry_delay * 2u32.pow(attempt - 1));
                        continue;
                    }
                    return Err(e);
                }
                Err(ureq::Error::Transport(t)) => { crate::net::note_transport(&t); return Err(AdapterError::Network); }
            }
        }
    }

    fn native_tag(&self, canonical: &str) -> Option<String> {
        self.cfg.tags.iter().any(|t| t == canonical).then(|| self.cfg.tag_map.get(canonical).cloned().unwrap_or_else(|| format!("[{canonical}]")))
    }

    /// Speak one piece of text. Returns mono samples and their rate.
    fn speak(&self, text: &str, voice: &str, pace: f32) -> Result<(Vec<f32>, u32), AdapterError> {
        let mut body = serde_json::json!({ "model": self.cfg.model, "input": text, "voice": voice, "response_format": self.cfg.response_format });
        if (pace - 1.0).abs() > 0.01 {
            body["speed"] = serde_json::json!(pace.clamp(0.25, 4.0));
        }
        let bytes = read_limited(self.call("POST", "/audio/speech", Some(&body))?, MAX_AUDIO_BYTES)?;
        let d = decode_bytes(&bytes, Some(&self.cfg.response_format)).map_err(|_| AdapterError::ProviderError("the server did not return audio Soundstage could read".into()))?;
        // Mix any extra channels down to one, so every engine's output looks the same to the rest of the pipeline.
        let n = d.channels[0].len();
        let mono: Vec<f32> = (0..n).map(|i| d.channels.iter().map(|c| c.get(i).copied().unwrap_or(0.0)).sum::<f32>() / d.channels.len() as f32).collect();
        Ok((mono, d.rate))
    }

    pub fn discover_voices(&self) -> Result<Vec<VoiceDef>, AdapterError> {
        let lang = self.cfg.voices.first().map(|v| v.language.clone()).filter(|l| !l.is_empty()).unwrap_or_else(|| "en".into());
        for path in ["/audio/voices", "/voices"] {
            let Ok(r) = self.call("GET", path, None) else { continue };
            let v = read_json(r, 2 * 1024 * 1024)?;
            let items = v.get("voices").cloned().unwrap_or(v);
            let list = items.as_array().cloned().unwrap_or_default();
            let found: Vec<VoiceDef> = list
                .iter()
                .filter_map(|x| {
                    let id = x.as_str().map(String::from).or_else(|| ["id", "voice_id", "name", "voice"].iter().find_map(|k| x[*k].as_str().map(String::from)))?;
                    if id.is_empty() || id.len() > 80 || id.chars().any(|c| c.is_control()) {
                        return None;
                    }
                    let name = x["name"].as_str().unwrap_or(&id).chars().take(80).collect::<String>();
                    Some(VoiceDef { id, name, language: lang.clone(), accent: String::new(), gender: String::new() })
                })
                .take(crate::engines::MAX_VOICES)
                .collect();
            if !found.is_empty() {
                return Ok(found);
            }
        }
        Err(AdapterError::ProviderError("this server does not list its voices. Type them in by hand.".into()))
    }
}

fn licence_for(cfg: &SpeechEngine) -> LicenceInfo {
    LicenceInfo {
        licence: Licence::Unknown,
        source: None,
        checked: None,
        notes: if cfg.licence_note.is_empty() { "Depends on the model this engine runs. Read its licence and record your check.".into() } else { cfg.licence_note.clone() },
    }
}

impl Adapter for OpenAiSpeechAdapter {
    fn id(&self) -> &str {
        &self.id
    }
    fn label(&self) -> &str {
        &self.label
    }
    fn licence(&self) -> LicenceInfo {
        licence_for(&self.cfg)
    }
    fn capabilities(&self) -> Capabilities {
        let mut languages: Vec<String> = self.cfg.voices.iter().map(|v| if v.language.is_empty() { "en".to_string() } else { v.language.clone() }).collect();
        languages.sort();
        languages.dedup();
        if languages.is_empty() {
            languages.push("en".into());
        }
        Capabilities { languages, tags: self.cfg.tags.clone(), voice_design: false, cloning: false, max_chars_per_request: 4000, streaming: false }
    }
    fn list_voices(&self) -> Vec<Voice> {
        self.cfg
            .voices
            .iter()
            .map(|v| Voice {
                id: format!("{}:{}", self.id, v.id),
                name: if v.name.is_empty() { v.id.clone() } else { v.name.clone() },
                provider: self.id.clone(),
                language: if v.language.is_empty() { "en".into() } else { v.language.clone() },
                accent: if v.accent.is_empty() { "Not stated".into() } else { v.accent.clone() },
                gender: if v.gender.is_empty() { "Not stated".into() } else { v.gender.clone() },
                age_range: "Not stated".into(),
                licence: licence_for(&self.cfg),
                sample_text: "Good morning. This is how I sound.".into(),
                notes: format!("{} Tags are declared by you and have not been tested.", if self.base.is_local() { "Runs on this computer or your network." } else { "Script text is sent to this server." }),
            })
            .collect()
    }
    fn can_transcribe(&self) -> bool {
        !self.cfg.transcribe_model.trim().is_empty()
    }
    fn transcribe(&self, wav: &[u8], language: Option<&str>) -> Result<crate::transcribe::Transcript, AdapterError> {
        if !self.can_transcribe() {
            return Err(AdapterError::ProviderError("no transcription model is set for this engine".into()));
        }
        if wav.len() < 1000 || wav.len() > MAX_AUDIO_BYTES || !wav.starts_with(b"RIFF") {
            return Err(AdapterError::ProviderError("the recording is not valid".into()));
        }
        let lang = language.map(str::trim).filter(|l| (2..=3).contains(&l.len()) && l.bytes().all(|b| b.is_ascii_lowercase()));
        let mut fields = vec![("model", self.cfg.transcribe_model.as_str()), ("response_format", "verbose_json"), ("timestamp_granularities[]", "word")];
        if let Some(l) = lang {
            fields.push(("language", l));
        }
        let (boundary, body) = crate::net::multipart(&fields, ("file", "audio.wav", "audio/wav", wav));
        let key = self.key()?;
        let mut req = self.agent.post(&self.url("/audio/transcriptions")).set("content-type", &format!("multipart/form-data; boundary={boundary}"));
        if let Some(k) = &key {
            req = req.set("Authorization", &format!("Bearer {k}"));
        }
        // One attempt: a local server can take minutes on long audio, and a second copy would only slow it down.
        let res = match req.send_bytes(&body) {
            Ok(r) if r.status() == 200 => r,
            Ok(r) => return Err(AdapterError::ProviderError(format!("unexpected HTTP {}", r.status()))),
            Err(ureq::Error::Status(code, r)) => return Err(error_from_status(code, r)),
            Err(ureq::Error::Transport(t)) => { crate::net::note_transport(&t); return Err(AdapterError::Network); }
        };
        let v = read_json(res, 32 << 20)?;
        crate::transcribe::parse_openai(&v).map_err(AdapterError::ProviderError)
    }
    fn provider_version(&self) -> String {
        format!("openai-compatible/{}/{}", self.cfg.id, self.cfg.model)
    }
    fn sends_data(&self) -> bool {
        !self.base.is_local()
    }
    fn check_connection(&self) -> Result<(), AdapterError> {
        match self.call("GET", "/models", None) {
            Ok(_) => Ok(()),
            // Many local servers do not implement /models. Any answer other than an auth failure means it is there.
            Err(AdapterError::ProviderError(m)) if m.contains("unexpected HTTP") => Ok(()),
            Err(AdapterError::UnknownVoice(_)) => Ok(()),
            Err(e) => Err(e),
        }
    }
    fn render(&self, req: &RenderRequest) -> Result<RenderOutput, AdapterError> {
        precheck(&self.capabilities(), req)?;
        let local = req.voice_id.strip_prefix(&format!("{}:", self.id)).ok_or_else(|| AdapterError::UnknownVoice(req.voice_id.chars().take(60).collect()))?;
        let def = self.cfg.voices.iter().find(|v| v.id == local).ok_or_else(|| AdapterError::UnknownVoice(req.voice_id.chars().take(60).collect()))?;
        let segs = compose_segments(&req.tokens, &req.emotion, &|c| self.native_tag(c))?;
        let mut pieces: Vec<Result<(Vec<f32>, u32), u32>> = Vec::new();
        for s in segs {
            match s {
                Ok(text) => pieces.push(Ok(self.speak(&text, &def.id, req.pace)?)),
                Err(ms) => pieces.push(Err(ms)),
            }
        }
        let rate = pieces.iter().find_map(|p| p.as_ref().ok().map(|(_, r)| *r)).unwrap_or(24_000);
        let mut samples = Vec::new();
        for p in pieces {
            match p {
                Ok((s, r)) => samples.extend(if r == rate { s } else { resample(&s, r, rate) }),
                Err(ms) => samples.extend(std::iter::repeat(0.0).take((ms as usize * rate as usize) / 1000)),
            }
        }
        if samples.is_empty() {
            return Err(AdapterError::ProviderError("the server returned no audio".into()));
        }
        Ok(RenderOutput { samples, sample_rate: rate, provider_version: self.provider_version() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::VoiceDef;
    use crate::tags::tokenize;
    use crate::testutil::{start, Fake, Handler};
    use std::sync::{Arc, Mutex};

    const KEY: &str = "sk-test-DO-NOT-LEAK-777";

    fn wav_for(text: &str) -> Vec<u8> {
        let words = text.split_whitespace().filter(|w| !w.starts_with('[') && !w.ends_with(']')).count().max(1);
        let n = words * 350 * 22_050 / 1000;
        let mut cur = std::io::Cursor::new(Vec::new());
        {
            let spec = hound::WavSpec { channels: 1, sample_rate: 22_050, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
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
            if r.path == "/v1/audio/speech" {
                let v: serde_json::Value = serde_json::from_str(&r.body).unwrap_or_default();
                (200, wav_for(v["input"].as_str().unwrap_or("")))
            } else if r.path == "/v1/models" {
                (200, br#"{"data":[]}"#.to_vec())
            } else if r.path == "/v1/audio/voices" {
                (200, br#"{"voices":["af_bella","am_adam",{"id":"bf_emma","name":"Emma"}]}"#.to_vec())
            } else {
                (404, b"{}".to_vec())
            }
        })
    }

    fn cfg(f: &Fake) -> SpeechEngine {
        SpeechEngine {
            id: "kokoro".into(), label: "Kokoro (local)".into(), base_url: format!("{}/v1", f.base), model: "kokoro".into(),
            voices: vec![
                VoiceDef { id: "af_bella".into(), name: "Bella".into(), language: "en-US".into(), accent: "American".into(), gender: "female".into() },
                VoiceDef { id: "bf_emma".into(), name: "Emma".into(), language: "en-GB".into(), accent: "British".into(), gender: "female".into() },
            ],
            ..Default::default()
        }
    }

    fn adapter(_f: &Fake, c: SpeechEngine) -> OpenAiSpeechAdapter {
        let keys: KeyProvider = Arc::new(|_| Some(KEY.to_string()));
        OpenAiSpeechAdapter::new(c, keys).unwrap().instant()
    }
    fn req(text: &str, emotion: &str, voice: &str) -> RenderRequest {
        RenderRequest { tokens: tokenize(text).0, voice_id: voice.into(), emotion: emotion.into(), pace: 1.0, language: "en-US".into() }
    }
    fn posts(f: &Fake) -> Vec<serde_json::Value> {
        f.hits.lock().unwrap().iter().filter(|r| r.method == "POST").map(|r| serde_json::from_str(&r.body).unwrap()).collect()
    }

    #[test]
    fn passes_the_adapter_contract() {
        let f = start(happy());
        let a = adapter(&f, cfg(&f));
        let v = a.list_voices().remove(0);
        contract::run(&a, &v, "sneeze");
    }

    #[test]
    fn requests_use_the_openai_shape_and_the_key_stays_in_the_header() {
        let f = start(happy());
        let mut c = cfg(&f);
        c.response_format = "wav".into();
        let a = adapter(&f, c);
        let v = a.list_voices().remove(0);
        let mut r = req("hello there", "neutral", &v.id);
        r.pace = 1.25;
        a.render(&r).unwrap();
        let b = &posts(&f)[0];
        assert_eq!((b["model"].as_str(), b["voice"].as_str(), b["input"].as_str(), b["response_format"].as_str()), (Some("kokoro"), Some("af_bella"), Some("hello there"), Some("wav")));
        assert_eq!(b["speed"], 1.25);
        for h in f.hits.lock().unwrap().iter() {
            assert!(h.headers.iter().any(|(k, v)| k == "authorization" && v == &format!("Bearer {KEY}")));
            assert!(!h.path.contains(KEY) && !h.body.contains(KEY));
        }
    }

    #[test]
    fn declared_tags_are_written_the_way_the_engine_wants_and_pauses_are_exact() {
        let f = start(happy());
        let mut c = cfg(&f);
        c.tags = vec!["laugh".into(), "whisper".into()];
        c.tag_map.insert("laugh".into(), "<laugh>".into());
        let a = adapter(&f, c);
        let v = a.list_voices().remove(0);
        let out = a.render(&req("hi [laugh] there [pause:500] bye [whisper] ok", "neutral", &v.id)).unwrap();
        let bodies = posts(&f);
        assert_eq!(bodies.len(), 2);
        assert_eq!(bodies[0]["input"], "hi <laugh> there");
        assert_eq!(bodies[1]["input"], "bye [whisper] ok");
        let spoken: usize = bodies.iter().map(|b| wav_for(b["input"].as_str().unwrap()).len() / 2 - 22).sum();
        assert!((out.samples.len() as i64 - (spoken as i64 + 11_025)).abs() < 40, "{} vs {}", out.samples.len(), spoken + 11_025);
    }

    #[test]
    fn tags_the_user_did_not_declare_are_refused_before_any_request() {
        let f = start(happy());
        let a = adapter(&f, cfg(&f));
        let v = a.list_voices().remove(0);
        assert_eq!(a.render(&req("hi [sigh]", "neutral", &v.id)).unwrap_err(), AdapterError::UnsupportedTag("sigh".into()));
        assert_eq!(a.render(&req("hi", "sad", &v.id)).unwrap_err(), AdapterError::UnsupportedTag("sad".into()));
        assert!(posts(&f).is_empty());
    }

    #[test]
    fn a_required_key_that_is_missing_is_an_auth_error_with_no_network_call() {
        let f = start(happy());
        let mut c = cfg(&f);
        c.needs_key = true;
        let a = OpenAiSpeechAdapter::new(c, Arc::new(|_| None)).unwrap();
        let v = a.list_voices().remove(0);
        assert_eq!(a.render(&req("hi", "neutral", &v.id)).unwrap_err(), AdapterError::Auth);
        assert!(f.hits.lock().unwrap().is_empty());
        // Local servers that need no key work without one and send no Authorization header.
        let g = start(happy());
        let b = OpenAiSpeechAdapter::new(cfg(&g), Arc::new(|_| None)).unwrap();
        b.render(&req("hi", "neutral", &b.list_voices()[0].id)).unwrap();
        assert!(g.hits.lock().unwrap().iter().all(|h| !h.headers.iter().any(|(k, _)| k == "authorization")));
    }

    #[test]
    fn unsafe_keys_and_foreign_voice_ids_never_reach_the_network() {
        let f = start(happy());
        let a = OpenAiSpeechAdapter::new(cfg(&f), Arc::new(|_| Some("abc\r\nx: 1".into()))).unwrap();
        assert_eq!(a.check_connection().unwrap_err(), AdapterError::Auth);
        let a = adapter(&f, cfg(&f));
        for id in ["speech-kokoro:../../x", "other:af_bella", "speech-kokoro:nope", ""] {
            assert!(matches!(a.render(&req("hi", "neutral", id)), Err(AdapterError::UnknownVoice(_))), "{id}");
        }
        assert!(posts(&f).is_empty());
    }

    #[test]
    fn local_servers_do_not_send_data_and_remote_ones_do() {
        let f = start(happy());
        assert!(!adapter(&f, cfg(&f)).sends_data());
        let mut c = cfg(&f);
        c.base_url = "https://openrouter.ai/api/v1".into();
        c.needs_key = true;
        assert!(OpenAiSpeechAdapter::new(c, Arc::new(|_| Some("k".into()))).unwrap().sends_data());
        let mut c = cfg(&f);
        c.base_url = "http://example.com/v1".into();
        assert!(OpenAiSpeechAdapter::new(c, Arc::new(|_| None)).is_err(), "http to the internet is refused when the engine is created");
    }

    #[test]
    fn errors_map_to_the_common_set_and_rate_limits_are_retried() {
        for (code, want) in [(401, AdapterError::Auth), (402, AdapterError::Quota), (404, AdapterError::UnknownVoice("not found".into()))] {
            let f = start(Box::new(move |_, _| (code, b"{}".to_vec())));
            let a = adapter(&f, cfg(&f));
            let e = a.render(&req("hi", "neutral", &a.list_voices()[0].id)).unwrap_err();
            assert_eq!(e, want, "{code}");
            assert!(!format!("{e:?}").contains(KEY));
        }
        let n = Arc::new(Mutex::new(0usize));
        let n2 = n.clone();
        let f = start(Box::new(move |_, _| { let mut g = n2.lock().unwrap(); *g += 1; if *g <= 2 { (429, b"{}".to_vec()) } else { (200, wav_for("hello")) } }));
        let a = adapter(&f, cfg(&f));
        assert!(a.render(&req("hello", "neutral", &a.list_voices()[0].id)).is_ok());
        assert_eq!(*n.lock().unwrap(), 3);
    }

    #[test]
    fn html_error_pages_and_garbage_are_not_mistaken_for_audio() {
        let f = start(Box::new(|_, _| (200, b"<html><body>Service unavailable</body></html>".to_vec())));
        let a = adapter(&f, cfg(&f));
        assert!(matches!(a.render(&req("hi", "neutral", &a.list_voices()[0].id)), Err(AdapterError::ProviderError(_))));
    }

    #[test]
    fn redirects_are_never_followed() {
        let evil = start(happy());
        let target = evil.base.clone();
        let f = start(Box::new(move |_, _| (307, format!("{target}/v1/audio/speech").into_bytes())));
        let a = adapter(&f, cfg(&f));
        assert!(a.render(&req("hi", "neutral", &a.list_voices()[0].id)).is_err());
        assert!(evil.hits.lock().unwrap().is_empty());
    }

    #[test]
    fn connection_check_and_voice_discovery() {
        let f = start(happy());
        let a = adapter(&f, cfg(&f));
        a.check_connection().unwrap();
        let v = a.discover_voices().unwrap();
        assert_eq!(v.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), vec!["af_bella", "am_adam", "bf_emma"]);
        assert_eq!(v[2].name, "Emma");
        // A server with no /models route still counts as reachable.
        let g = start(Box::new(|r, _| if r.path == "/v1/models" { (404, b"{}".to_vec()) } else { (200, wav_for("x")) }));
        adapter(&g, cfg(&g)).check_connection().unwrap();
        let h = start(Box::new(|_, _| (401, b"{}".to_vec())));
        assert_eq!(adapter(&h, cfg(&h)).check_connection().unwrap_err(), AdapterError::Auth);
        let n = start(Box::new(|_, _| (404, b"{}".to_vec())));
        assert!(adapter(&n, cfg(&n)).discover_voices().is_err());
    }

    #[test]
    fn unreachable_servers_are_a_network_error() {
        let c = SpeechEngine { base_url: "http://127.0.0.1:1/v1".into(), ..cfg(&start(happy())) };
        let a = OpenAiSpeechAdapter::new(c, Arc::new(|_| None)).unwrap();
        assert_eq!(a.check_connection().unwrap_err(), AdapterError::Network);
    }

    #[test]
    fn transcription_goes_to_the_configured_model_and_parses_word_times() {
        let f = start(Box::new(|r, _| {
            if r.path == "/v1/audio/transcriptions" {
                (200, serde_json::json!({"text":"Hello there","language":"english","words":[{"word":"Hello","start":0.0,"end":0.4},{"word":"there","start":0.5,"end":0.9}]}).to_string().into_bytes())
            } else { (404, b"{}".to_vec()) }
        }));
        let mut cfg = SpeechEngine { id: "local".into(), label: "Local".into(), base_url: format!("{}/v1", f.base), model: "kokoro".into(), voices: vec![], response_format: "wav".into(), ..Default::default() };
        let plain = OpenAiSpeechAdapter::new(cfg.clone(), Arc::new(|_| None)).unwrap();
        assert!(!plain.can_transcribe(), "off until a transcription model is named");
        cfg.transcribe_model = "whisper-1".into();
        let a = OpenAiSpeechAdapter::new(cfg, Arc::new(|_| None)).unwrap();
        let mut wav = b"RIFF".to_vec();
        wav.extend(vec![1u8; 3000]);
        let t = a.transcribe(&wav, Some("en")).unwrap();
        assert_eq!((t.text.as_str(), t.words.len()), ("Hello there", 2));
        let hit = f.hits.lock().unwrap().iter().find(|h| h.path == "/v1/audio/transcriptions").cloned().unwrap();
        assert!(hit.body.contains("whisper-1") && hit.body.contains("verbose_json") && hit.body.contains("timestamp_granularities[]"));
        assert!(a.transcribe(b"nope", None).is_err());
    }
}
