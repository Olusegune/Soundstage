//! fal.ai through its queue API (verified from fal's docs on 2026-10-23): `POST https://queue.fal.run/{model}` with
//! `Authorization: Key ...`, then poll the `status_url` until `COMPLETED`, then read the `response_url`. Audio files
//! are hosted on fal's CDN (`*.fal.media`) and fetched by Soundstage's hardened downloader.
//!
//! The model id and input template are the user's, because input fields differ per model. The key is sent only to
//! addresses on queue.fal.run: the status and result addresses fal returns are checked before use, so a bad response
//! cannot make Soundstage send the key anywhere else.

use crate::jobs::{fill_template, find_audio_urls, Fetcher, RunOutput, Transport, Vars, MAX_RESULT_AUDIO};
use crate::net::{error_from_status, read_json};
use crate::providers::{AdapterError, KeyProvider};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

pub const FAL_HOST: &str = "https://queue.fal.run";
const SUFFIXES: [&str; 2] = [".fal.media", "fal.media"];

pub fn valid_model_id(m: &str) -> bool {
    let parts: Vec<&str> = m.split('/').collect();
    (2..=5).contains(&parts.len()) && parts.iter().all(|p| !p.is_empty() && p.len() <= 60 && p.bytes().next().is_some_and(|b| b.is_ascii_alphanumeric()) && p.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-')))
}

pub struct FalTransport {
    model: String,
    template: Value,
    keys: KeyProvider,
    agent: ureq::Agent,
    base: String,
    fetch: Arc<dyn Fetcher>,
    poll: Duration,
    max_polls: u32,
}

impl FalTransport {
    pub fn new(model: String, template: Value, keys: KeyProvider, fetch: Arc<dyn Fetcher>) -> Self {
        Self::build(model, template, keys, FAL_HOST.into(), fetch, Duration::from_secs(2), 300)
    }
    fn build(model: String, template: Value, keys: KeyProvider, base: String, fetch: Arc<dyn Fetcher>, poll: Duration, max_polls: u32) -> Self {
        let agent = crate::net::cloud_agent_builder().redirects(0).timeout_connect(Duration::from_secs(10)).timeout_write(Duration::from_secs(30)).timeout_read(Duration::from_secs(60)).user_agent("Soundstage").middleware(crate::net::local_guard).build();
        FalTransport { model, template, keys, agent, base, fetch, poll, max_polls }
    }
    #[cfg(test)]
    fn for_test(model: &str, template: Value, keys: KeyProvider, base: String, fetch: Arc<dyn Fetcher>, max_polls: u32) -> Self {
        Self::build(model.into(), template, keys, base, fetch, Duration::ZERO, max_polls)
    }

    fn key(&self) -> Result<String, AdapterError> {
        match (self.keys)("fal").map(|k| k.trim().to_string()) {
            Some(k) if !k.is_empty() && k.len() <= 512 && k.bytes().all(|b| b.is_ascii_graphic()) => Ok(k),
            _ => Err(AdapterError::Auth),
        }
    }

    /// Only addresses under the queue host are ever contacted with the key.
    fn own(&self, url: &str) -> Result<(), AdapterError> {
        if url.starts_with(&format!("{}/", self.base)) && !url.contains(char::is_whitespace) && url.len() < 500 {
            Ok(())
        } else {
            Err(AdapterError::ProviderError("the service returned an address Soundstage will not contact".into()))
        }
    }

    fn call(&self, method: &str, url: &str, body: Option<&Value>) -> Result<ureq::Response, AdapterError> {
        self.own(url)?;
        let req = self.agent.request(method, url).set("Authorization", &format!("Key {}", self.key()?));
        let res = match body {
            Some(b) => req.set("content-type", "application/json").send_string(&b.to_string()),
            None => req.call(),
        };
        match res {
            Ok(r) if r.status() == 200 || r.status() == 201 || r.status() == 202 => Ok(r),
            Ok(r) => Err(AdapterError::ProviderError(format!("unexpected HTTP {}", r.status()))),
            Err(ureq::Error::Status(code, r)) => Err(error_from_status(code, r)),
            Err(ureq::Error::Transport(t)) => { crate::net::note_transport(&t); Err(AdapterError::Network) }
        }
    }
}

impl Transport for FalTransport {
    fn run(&self, vars: &Vars) -> Result<RunOutput, AdapterError> {
        let input = fill_template(&self.template, vars);
        let sub = read_json(self.call("POST", &format!("{}/{}", self.base, self.model), Some(&input))?, 1 << 20)?;
        let (status_url, response_url) = (sub["status_url"].as_str().unwrap_or(""), sub["response_url"].as_str().unwrap_or(""));
        if sub["request_id"].as_str().is_none() {
            return Err(AdapterError::ProviderError("the service did not accept the job".into()));
        }
        self.own(status_url)?;
        self.own(response_url)?;
        let mut done = false;
        for _ in 0..self.max_polls {
            let s = read_json(self.call("GET", status_url, None)?, 1 << 20)?;
            if s["error"].is_string() && s["status"] == "COMPLETED" {
                return Err(AdapterError::ProviderError("the service could not make this audio".into()));
            }
            if s["status"] == "COMPLETED" {
                done = true;
                break;
            }
            std::thread::sleep(self.poll);
        }
        if !done {
            return Err(AdapterError::ProviderError("the service took too long".into()));
        }
        let result = read_json(self.call("GET", response_url, None)?, 4 << 20)?;
        let url = find_audio_urls(&result).into_iter().next().ok_or_else(|| AdapterError::ProviderError("the result had no audio file in it".into()))?;
        let audio = self.fetch.fetch(&url, &SUFFIXES, MAX_RESULT_AUDIO).map_err(AdapterError::ProviderError)?;
        let ext = url.split(['?', '#']).next().and_then(|p| p.rsplit('.').next()).map(|e| e.to_ascii_lowercase());
        Ok(RunOutput { audio, ext })
    }
    fn check(&self) -> Result<(), AdapterError> {
        // fal has no free authenticated test call. A saved key is the most that can be checked without spending credits.
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
    use crate::testutil::{start, Fake, Handler};
    use serde_json::json;
    use std::sync::Mutex;

    const KEY: &str = "fal-test-DO-NOT-LEAK-1";

    struct FakeFetch {
        seen: Mutex<Vec<(String, Vec<String>)>>,
    }
    impl Fetcher for FakeFetch {
        fn fetch(&self, url: &str, suffixes: &[&str], _max: usize) -> Result<Vec<u8>, String> {
            self.seen.lock().unwrap().push((url.to_string(), suffixes.iter().map(|s| s.to_string()).collect()));
            Ok(b"RIFFfake".to_vec())
        }
    }
    fn fetcher() -> Arc<FakeFetch> {
        Arc::new(FakeFetch { seen: Mutex::new(vec![]) })
    }
    fn keys() -> KeyProvider {
        Arc::new(|n| (n == "fal").then(|| KEY.to_string()))
    }
    fn tpl() -> Value {
        parse_template(r#"{"prompt":"{{prompt}}","duration":"{{seconds}}"}"#, "prompt", 4096).unwrap()
    }

    fn happy(base_holder: Arc<Mutex<String>>, polls: Arc<Mutex<u32>>) -> Handler {
        Box::new(move |r, _| {
            let base = base_holder.lock().unwrap().clone();
            if r.method == "POST" && r.path == "/fal-ai/some/model" {
                (200, json!({"request_id":"r1","status_url":format!("{base}/fal-ai/some/requests/r1/status"),"response_url":format!("{base}/fal-ai/some/requests/r1")}).to_string().into_bytes())
            } else if r.path == "/fal-ai/some/requests/r1/status" {
                let mut p = polls.lock().unwrap();
                *p += 1;
                (200, json!({"status": if *p < 3 { "IN_PROGRESS" } else { "COMPLETED" }}).to_string().into_bytes())
            } else if r.path == "/fal-ai/some/requests/r1" {
                (200, json!({"audio":{"url":"https://v3.fal.media/files/a/out.mp3"}}).to_string().into_bytes())
            } else {
                (404, b"{}".to_vec())
            }
        })
    }

    fn rig(max_polls: u32) -> (Fake, FalTransport, Arc<FakeFetch>, Arc<Mutex<u32>>) {
        let holder = Arc::new(Mutex::new(String::new()));
        let polls = Arc::new(Mutex::new(0));
        let f = start(happy(holder.clone(), polls.clone()));
        *holder.lock().unwrap() = f.base.clone();
        let fetch = fetcher();
        let t = FalTransport::for_test("fal-ai/some/model", tpl(), keys(), f.base.clone(), fetch.clone(), max_polls);
        (f, t, fetch, polls)
    }

    #[test]
    fn submits_polls_reads_the_result_and_fetches_the_file_from_fal_media_only() {
        let (f, t, fetch, polls) = rig(20);
        let out = t.run(&Vars { prompt: Some("rain".into()), seconds: Some(8.0), ..Default::default() }).unwrap();
        assert_eq!((out.audio.as_slice(), out.ext.as_deref()), (&b"RIFFfake"[..], Some("mp3")));
        assert_eq!(*polls.lock().unwrap(), 3, "it waited until COMPLETED");
        let hits = f.hits.lock().unwrap();
        let body: Value = serde_json::from_str(&hits[0].body).unwrap();
        assert_eq!((body["prompt"].as_str(), body["duration"].as_f64()), (Some("rain"), Some(8.0)));
        for h in hits.iter() {
            assert!(h.headers.iter().any(|(k, v)| k == "authorization" && v == &format!("Key {KEY}")));
            assert!(!h.path.contains(KEY) && !h.body.contains(KEY));
        }
        let seen = fetch.seen.lock().unwrap();
        assert_eq!(seen[0].0, "https://v3.fal.media/files/a/out.mp3");
        assert_eq!(seen[0].1, vec![".fal.media", "fal.media"], "downloads are limited to fal's file hosts");
        let sent_to_cdn = hits.iter().any(|h| h.path.contains("fal.media"));
        assert!(!sent_to_cdn);
    }

    #[test]
    fn the_key_is_never_sent_to_an_address_the_provider_chose() {
        let evil = start(Box::new(|_, _| (200, b"{}".to_vec())));
        let evil_base = evil.base.clone();
        let f = start(Box::new(move |_, _| (200, json!({"request_id":"r","status_url":format!("{evil_base}/steal"),"response_url":format!("{evil_base}/steal")}).to_string().into_bytes())));
        let t = FalTransport::for_test("fal-ai/some/model", tpl(), keys(), f.base.clone(), fetcher(), 5);
        assert!(matches!(t.run(&Vars { prompt: Some("x".into()), ..Default::default() }), Err(AdapterError::ProviderError(_))));
        assert!(evil.hits.lock().unwrap().is_empty(), "the key must not reach another host");
    }

    #[test]
    fn missing_keys_make_no_request_and_failures_map_to_plain_errors() {
        let f = start(Box::new(|_, _| (401, b"{}".to_vec())));
        let none = FalTransport::for_test("fal-ai/some/model", tpl(), Arc::new(|_| None), f.base.clone(), fetcher(), 3);
        assert_eq!(none.run(&Vars::default()).unwrap_err(), AdapterError::Auth);
        assert!(!none.ready() && f.hits.lock().unwrap().is_empty());
        let t = FalTransport::for_test("fal-ai/some/model", tpl(), keys(), f.base.clone(), fetcher(), 3);
        assert_eq!(t.run(&Vars::default()).unwrap_err(), AdapterError::Auth);
        let (_, slow, _, _) = rig(2);
        assert!(matches!(slow.run(&Vars::default()), Err(AdapterError::ProviderError(m)) if m.contains("too long")));
    }

    #[test]
    fn a_result_with_no_audio_is_a_plain_error() {
        let holder = Arc::new(Mutex::new(String::new()));
        let h2 = holder.clone();
        let f = start(Box::new(move |r, _| {
            let base = h2.lock().unwrap().clone();
            if r.method == "POST" { (200, json!({"request_id":"r","status_url":format!("{base}/s"),"response_url":format!("{base}/r")}).to_string().into_bytes()) }
            else if r.path == "/s" { (200, json!({"status":"COMPLETED"}).to_string().into_bytes()) }
            else { (200, json!({"images":[{"url":"https://v3.fal.media/x.png"}]}).to_string().into_bytes()) }
        }));
        *holder.lock().unwrap() = f.base.clone();
        let t = FalTransport::for_test("fal-ai/some/model", tpl(), keys(), f.base.clone(), fetcher(), 3);
        assert!(matches!(t.run(&Vars::default()), Err(AdapterError::ProviderError(m)) if m.contains("no audio")));
    }

    #[test]
    fn model_ids_are_validated() {
        for ok in ["fal-ai/kokoro/american-english", "fal-ai/lyria2", "owner/app"] { assert!(valid_model_id(ok), "{ok}"); }
        for bad in ["", "fal-ai", "../x/y", "a//b", "a/b c", "fal-ai/x?y=1", "-a/b", &"a/".repeat(6)] { assert!(!valid_model_id(bad), "{bad}"); }
    }
}
