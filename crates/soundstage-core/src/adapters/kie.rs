//! Kie.ai (verified from Kie's docs on 2026-10-23): `POST https://api.kie.ai/api/v1/jobs/createTask` with
//! `{ "model": ..., "input": {...} }` and `Authorization: Bearer ...`, then poll
//! `GET /api/v1/jobs/recordInfo?taskId=...` until `state` is `success` (or `fail`). Kie returns HTTP 200 even for
//! errors, with the real result in a numeric `code`, so that is what is checked. Generated files are kept for 14 days,
//! so they are downloaded straight away. Suno music is available as the model `ai-music-api/generate`.
//!
//! Kie does not publish which hosts serve result files. Downloads therefore use the hardened fetcher with no host
//! allowlist: https only, public addresses only, no credentials, capped size.

use crate::jobs::{fill_template, find_audio_urls, Fetcher, RunOutput, Transport, Vars, MAX_RESULT_AUDIO};
use crate::net::{read_json, url_encode};
use crate::providers::{AdapterError, KeyProvider};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

pub const KIE_HOST: &str = "https://api.kie.ai";

pub fn valid_model(m: &str) -> bool {
    !m.is_empty() && m.len() <= 120 && m.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'/' | b':'))
}

fn code_error(code: i64) -> AdapterError {
    match code {
        401 | 403 => AdapterError::Auth,
        402 => AdapterError::Quota,
        404 => AdapterError::UnknownVoice("model".into()),
        429 => AdapterError::RateLimit,
        c => AdapterError::ProviderError(format!("Kie reported code {c}")),
    }
}

pub struct KieTransport {
    model: String,
    template: Value,
    keys: KeyProvider,
    agent: ureq::Agent,
    base: String,
    fetch: Arc<dyn Fetcher>,
    poll: Duration,
    max_polls: u32,
}

impl KieTransport {
    pub fn new(model: String, template: Value, keys: KeyProvider, fetch: Arc<dyn Fetcher>) -> Self {
        Self::build(model, template, keys, KIE_HOST.into(), fetch, Duration::from_secs(5), 120)
    }
    fn build(model: String, template: Value, keys: KeyProvider, base: String, fetch: Arc<dyn Fetcher>, poll: Duration, max_polls: u32) -> Self {
        let agent = crate::net::cloud_agent_builder().redirects(0).timeout_connect(Duration::from_secs(10)).timeout_write(Duration::from_secs(30)).timeout_read(Duration::from_secs(60)).user_agent("Soundstage").build();
        KieTransport { model, template, keys, agent, base, fetch, poll, max_polls }
    }
    #[cfg(test)]
    fn for_test(model: &str, template: Value, keys: KeyProvider, base: String, fetch: Arc<dyn Fetcher>, max_polls: u32) -> Self {
        Self::build(model.into(), template, keys, base, fetch, Duration::ZERO, max_polls)
    }

    fn key(&self) -> Result<String, AdapterError> {
        match (self.keys)("kie").map(|k| k.trim().to_string()) {
            Some(k) if !k.is_empty() && k.len() <= 512 && k.bytes().all(|b| b.is_ascii_graphic()) => Ok(k),
            _ => Err(AdapterError::Auth),
        }
    }

    /// Call Kie and unwrap its envelope. The HTTP status is ignored on purpose; `code` is the truth.
    fn call(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value, AdapterError> {
        let req = self.agent.request(method, &format!("{}{}", self.base, path)).set("Authorization", &format!("Bearer {}", self.key()?));
        let res = match body {
            Some(b) => req.set("content-type", "application/json").send_string(&b.to_string()),
            None => req.call(),
        };
        let r = match res {
            Ok(r) => r,
            Err(ureq::Error::Status(code, r)) => {
                let _ = crate::net::error_code(r);
                return Err(code_error(code as i64));
            }
            Err(ureq::Error::Transport(t)) => { crate::net::note_transport(&t); return Err(AdapterError::Network); }
        };
        let v = read_json(r, 4 << 20)?;
        match v["code"].as_i64() {
            Some(200) => Ok(v),
            Some(c) => Err(code_error(c)),
            None => Err(AdapterError::ProviderError("the response was not understood".into())),
        }
    }
}

impl Transport for KieTransport {
    fn run(&self, vars: &Vars) -> Result<RunOutput, AdapterError> {
        let body = serde_json::json!({ "model": self.model, "input": fill_template(&self.template, vars) });
        let created = self.call("POST", "/api/v1/jobs/createTask", Some(&body))?;
        let task = created["data"]["taskId"].as_str().filter(|t| !t.is_empty() && t.len() <= 100 && t.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))).ok_or_else(|| AdapterError::ProviderError("the service did not accept the job".into()))?;
        for _ in 0..self.max_polls {
            let v = self.call("GET", &format!("/api/v1/jobs/recordInfo?taskId={}", url_encode(task)), None)?;
            let d = &v["data"];
            match d["state"].as_str().unwrap_or("") {
                "success" => {
                    // The result arrives as a JSON string in `resultJson`, and parsed in `response`. Look in both.
                    let mut sources = vec![d["response"].clone()];
                    if let Some(s) = d["resultJson"].as_str() {
                        if let Ok(p) = serde_json::from_str::<Value>(s) {
                            sources.push(p);
                        }
                    }
                    let url = sources.iter().flat_map(find_audio_urls).next().ok_or_else(|| AdapterError::ProviderError("the result had no audio file in it".into()))?;
                    let audio = self.fetch.fetch(&url, &[], MAX_RESULT_AUDIO).map_err(AdapterError::ProviderError)?;
                    let ext = url.split(['?', '#']).next().and_then(|p| p.rsplit('.').next()).map(|e| e.to_ascii_lowercase());
                    return Ok(RunOutput { audio, ext });
                }
                "fail" => {
                    let c: String = d["failCode"].as_str().unwrap_or("").chars().filter(|c| c.is_ascii_alphanumeric()).take(12).collect();
                    return Err(AdapterError::ProviderError(format!("the service could not make this audio{}", if c.is_empty() { String::new() } else { format!(" (code {c})") })));
                }
                _ => std::thread::sleep(self.poll),
            }
        }
        Err(AdapterError::ProviderError("the service took too long".into()))
    }
    fn check(&self) -> Result<(), AdapterError> {
        // Kie's documented endpoints all create or read tasks. A saved key is what can be checked without spending credits.
        self.key().map(|_| ())
    }
    fn sends_data(&self) -> bool {
        true
    }
    fn ready(&self) -> bool {
        self.key().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::parse_template;
    use crate::testutil::start;
    use serde_json::json;
    use std::sync::Mutex;

    const KEY: &str = "kie-test-DO-NOT-LEAK-9";

    struct FakeFetch(Mutex<Vec<(String, usize)>>);
    impl Fetcher for FakeFetch {
        fn fetch(&self, url: &str, suffixes: &[&str], _m: usize) -> Result<Vec<u8>, String> {
            self.0.lock().unwrap().push((url.to_string(), suffixes.len()));
            Ok(b"AUDIO".to_vec())
        }
    }
    fn keys() -> KeyProvider {
        Arc::new(|n| (n == "kie").then(|| KEY.to_string()))
    }
    fn suno_template() -> Value {
        parse_template(r#"{"custom_mode":false,"instrumental":"{{instrumental}}","model":"V6","prompt":"{{prompt}}"}"#, "prompt", 4096).unwrap()
    }
    fn vars() -> Vars {
        Vars { prompt: Some("lofi beat".into()), instrumental: Some(true), ..Default::default() }
    }

    #[test]
    fn runs_the_documented_flow_and_reads_suno_style_results() {
        let polls = Arc::new(Mutex::new(0u32));
        let p2 = polls.clone();
        let f = start(Box::new(move |r, _| {
            if r.method == "POST" && r.path == "/api/v1/jobs/createTask" {
                (200, json!({"code":200,"msg":"ok","data":{"taskId":"task_1"}}).to_string().into_bytes())
            } else if r.path.starts_with("/api/v1/jobs/recordInfo") {
                let mut n = p2.lock().unwrap();
                *n += 1;
                if *n < 3 { (200, json!({"code":200,"data":{"state":"generating"}}).to_string().into_bytes()) }
                else { (200, json!({"code":200,"data":{"state":"success","response":{"data":[{"audio_url":"https://cdn.example.com/song.mp3","duration":120}]}}}).to_string().into_bytes()) }
            } else { (404, b"{}".to_vec()) }
        }));
        let fetch = Arc::new(FakeFetch(Mutex::new(vec![])));
        let t = KieTransport::for_test("ai-music-api/generate", suno_template(), keys(), f.base.clone(), fetch.clone(), 10);
        let out = t.run(&vars()).unwrap();
        assert_eq!((out.audio.as_slice(), out.ext.as_deref()), (&b"AUDIO"[..], Some("mp3")));
        let hits = f.hits.lock().unwrap();
        let body: Value = serde_json::from_str(&hits[0].body).unwrap();
        assert_eq!(body["model"], "ai-music-api/generate");
        assert_eq!((body["input"]["prompt"].as_str(), body["input"]["instrumental"].as_bool(), body["input"]["custom_mode"].as_bool()), (Some("lofi beat"), Some(true), Some(false)));
        assert!(hits[1].path.ends_with("taskId=task_1"));
        for h in hits.iter() {
            assert!(h.headers.iter().any(|(k, v)| k == "authorization" && v == &format!("Bearer {KEY}")));
            assert!(!h.path.contains(KEY) && !h.body.contains(KEY));
        }
        assert_eq!(fetch.0.lock().unwrap()[0], ("https://cdn.example.com/song.mp3".to_string(), 0));
    }

    #[test]
    fn results_in_the_resultjson_string_are_read_too() {
        let f = start(Box::new(|r, _| {
            if r.method == "POST" { (200, json!({"code":200,"data":{"taskId":"t2"}}).to_string().into_bytes()) }
            else { (200, json!({"code":200,"data":{"state":"success","resultJson":"{\"resultUrls\":[\"https://files.example.com/a.wav\"]}"}}).to_string().into_bytes()) }
        }));
        let t = KieTransport::for_test("m/x", suno_template(), keys(), f.base.clone(), Arc::new(FakeFetch(Mutex::new(vec![]))), 3);
        assert_eq!(t.run(&vars()).unwrap().ext.as_deref(), Some("wav"));
    }

    #[test]
    fn errors_hide_in_a_200_so_the_code_is_what_counts() {
        for (code, want) in [(401, AdapterError::Auth), (402, AdapterError::Quota), (429, AdapterError::RateLimit), (404, AdapterError::UnknownVoice("model".into()))] {
            let f = start(Box::new(move |_, _| (200, json!({"code":code,"msg":"details with the prompt: lofi beat"}).to_string().into_bytes())));
            let t = KieTransport::for_test("m/x", suno_template(), keys(), f.base.clone(), Arc::new(FakeFetch(Mutex::new(vec![]))), 3);
            let e = t.run(&vars()).unwrap_err();
            assert_eq!(e, want, "code {code}");
            assert!(!format!("{e:?}").contains("lofi"), "provider messages are never echoed");
        }
        let f = start(Box::new(|_, _| (200, json!({"code":500,"msg":"boom"}).to_string().into_bytes())));
        let t = KieTransport::for_test("m/x", suno_template(), keys(), f.base.clone(), Arc::new(FakeFetch(Mutex::new(vec![]))), 3);
        assert!(matches!(t.run(&vars()), Err(AdapterError::ProviderError(m)) if m.contains("500")));
    }

    #[test]
    fn failed_slow_and_keyless_jobs_are_plain_errors() {
        let f = start(Box::new(|r, _| {
            if r.method == "POST" { (200, json!({"code":200,"data":{"taskId":"t3"}}).to_string().into_bytes()) }
            else { (200, json!({"code":200,"data":{"state":"fail","failCode":"500<script>","failMsg":"secret"}}).to_string().into_bytes()) }
        }));
        let t = KieTransport::for_test("m/x", suno_template(), keys(), f.base.clone(), Arc::new(FakeFetch(Mutex::new(vec![]))), 3);
        let e = t.run(&vars()).unwrap_err();
        assert!(matches!(&e, AdapterError::ProviderError(m) if m.contains("code 500script") && !m.contains("secret")), "{e:?}");
        let slow = start(Box::new(|r, _| if r.method == "POST" { (200, json!({"code":200,"data":{"taskId":"t4"}}).to_string().into_bytes()) } else { (200, json!({"code":200,"data":{"state":"waiting"}}).to_string().into_bytes()) }));
        let s = KieTransport::for_test("m/x", suno_template(), keys(), slow.base.clone(), Arc::new(FakeFetch(Mutex::new(vec![]))), 2);
        assert!(matches!(s.run(&vars()), Err(AdapterError::ProviderError(m)) if m.contains("too long")));
        let n = KieTransport::for_test("m/x", suno_template(), Arc::new(|_| None), slow.base.clone(), Arc::new(FakeFetch(Mutex::new(vec![]))), 2);
        assert_eq!(n.run(&vars()).unwrap_err(), AdapterError::Auth);
        assert!(!n.ready());
    }

    #[test]
    fn task_ids_and_models_are_validated_before_use() {
        let f = start(Box::new(|_, _| (200, json!({"code":200,"data":{"taskId":"../../x?y=1"}}).to_string().into_bytes())));
        let t = KieTransport::for_test("m/x", suno_template(), keys(), f.base.clone(), Arc::new(FakeFetch(Mutex::new(vec![]))), 2);
        assert!(t.run(&vars()).is_err());
        assert_eq!(f.hits.lock().unwrap().len(), 1, "a hostile task id is never put in a URL");
        for ok in ["ai-music-api/generate", "elevenlabs/text-to-speech-turbo-2-5"] { assert!(valid_model(ok)); }
        for bad in ["", "a b", "a;b", "x\n"] { assert!(!valid_model(bad)); }
    }
}
