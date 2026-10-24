//! ComfyUI, running on the user's own computer or network. The user pastes a workflow saved with "Save (API Format)"
//! and marks where Soundstage's text goes with `{{text}}` or `{{prompt}}`. Soundstage queues it with
//! `POST /prompt`, waits on `GET /history/{id}`, and downloads the audio with `GET /view`.
//!
//! VERIFY: these three routes are ComfyUI's documented server API. Output is read from any node output that lists files
//! (`SaveAudio` writes an `audio` list of `{filename, subfolder, type}`). Soundstage cannot know which custom nodes a
//! given workflow uses, so it claims nothing about them; if the workflow runs in ComfyUI, it can run here.

use crate::jobs::{fill_template, RunOutput, Transport, Vars};
use crate::net::{parse_base_url, read_json, read_limited, url_encode, BaseUrl};
use crate::providers::AdapterError;
use serde_json::Value;
use std::time::Duration;

const MAX_AUDIO: usize = 200 * 1024 * 1024;
const AUDIO_EXTS: [&str; 7] = ["wav", "mp3", "flac", "ogg", "m4a", "aac", "opus"];

/// A workflow must be an object of nodes, each with a `class_type`.
pub fn validate_workflow(v: &Value) -> Result<(), String> {
    let o = v.as_object().ok_or("A workflow is a JSON object of nodes.")?;
    if o.is_empty() || !o.values().all(|n| n.is_object() && n["class_type"].is_string()) {
        return Err("This does not look like a ComfyUI workflow saved with \"Save (API Format)\". Every node needs a class_type.".into());
    }
    Ok(())
}

pub struct ComfyTransport {
    base: BaseUrl,
    workflow: Value,
    agent: ureq::Agent,
    poll: Duration,
    max_polls: u32,
}

impl ComfyTransport {
    pub fn new(base_url: &str, workflow: Value) -> Result<Self, String> {
        validate_workflow(&workflow)?;
        Ok(Self::build(parse_base_url(base_url)?, workflow, Duration::from_secs(1), 600))
    }
    fn build(base: BaseUrl, workflow: Value, poll: Duration, max_polls: u32) -> Self {
        let agent = ureq::AgentBuilder::new().redirects(0).timeout_connect(Duration::from_secs(10)).timeout_write(Duration::from_secs(30)).timeout_read(Duration::from_secs(120)).user_agent("Soundstage").build();
        ComfyTransport { base, workflow, agent, poll, max_polls }
    }
    #[cfg(test)]
    fn for_test(base_url: &str, workflow: Value, max_polls: u32) -> Self {
        Self::build(parse_base_url(base_url).unwrap(), workflow, Duration::ZERO, max_polls)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base.as_string(), path)
    }

    fn get(&self, path: &str) -> Result<ureq::Response, AdapterError> {
        match self.agent.get(&self.url(path)).call() {
            Ok(r) if r.status() == 200 => Ok(r),
            Ok(r) => Err(AdapterError::ProviderError(format!("unexpected HTTP {}", r.status()))),
            Err(ureq::Error::Status(c, _)) => Err(AdapterError::ProviderError(format!("ComfyUI answered with HTTP {c}"))),
            Err(ureq::Error::Transport(t)) => { crate::net::note_transport(&t); Err(AdapterError::Network) }
        }
    }
}

fn safe_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 80 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Files an output node lists that look like audio.
fn audio_files(outputs: &Value) -> Vec<(String, String, String)> {
    let mut found = vec![];
    if let Some(nodes) = outputs.as_object() {
        for out in nodes.values() {
            for list in out.as_object().into_iter().flat_map(|o| o.values()) {
                for f in list.as_array().into_iter().flatten() {
                    let name = f["filename"].as_str().unwrap_or("");
                    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
                    // A filename or folder with a path in it is never followed.
                    let clean = |s: &str| !s.contains("..") && !s.contains('\\') && !s.starts_with('/') && !s.contains(':');
                    let sub = f["subfolder"].as_str().unwrap_or("");
                    if AUDIO_EXTS.contains(&ext.as_str()) && !name.contains('/') && clean(name) && clean(sub) {
                        found.push((name.to_string(), sub.to_string(), f["type"].as_str().unwrap_or("output").to_string()));
                    }
                }
            }
        }
    }
    found
}

impl Transport for ComfyTransport {
    fn run(&self, vars: &Vars) -> Result<RunOutput, AdapterError> {
        let wf = fill_template(&self.workflow, vars);
        let body = serde_json::json!({ "prompt": wf, "client_id": crate::util::new_id("soundstage") });
        let r = match self.agent.post(&self.url("/prompt")).set("content-type", "application/json").send_string(&body.to_string()) {
            Ok(r) => r,
            Err(ureq::Error::Status(c, _)) => return Err(AdapterError::ProviderError(format!("ComfyUI rejected the workflow (HTTP {c}). Check that it runs in ComfyUI first."))),
            Err(ureq::Error::Transport(t)) => { crate::net::note_transport(&t); return Err(AdapterError::Network); }
        };
        let q = read_json(r, 1 << 20)?;
        let id = q["prompt_id"].as_str().filter(|i| safe_id(i)).ok_or_else(|| AdapterError::ProviderError("ComfyUI did not accept the workflow".into()))?.to_string();
        if q["node_errors"].as_object().is_some_and(|o| !o.is_empty()) {
            return Err(AdapterError::ProviderError("ComfyUI found problems in the workflow. Check that it runs in ComfyUI first.".into()));
        }
        for _ in 0..self.max_polls {
            let h = read_json(self.get(&format!("/history/{id}"))?, 8 << 20)?;
            if let Some(entry) = h.get(&id) {
                if entry["status"]["status_str"] == "error" {
                    return Err(AdapterError::ProviderError("ComfyUI reported an error while running the workflow".into()));
                }
                let files = audio_files(&entry["outputs"]);
                if let Some((name, sub, ty)) = files.into_iter().next() {
                    let bytes = read_limited(self.get(&format!("/view?filename={}&subfolder={}&type={}", url_encode(&name), url_encode(&sub), url_encode(&ty)))?, MAX_AUDIO)?;
                    return Ok(RunOutput { audio: bytes, ext: name.rsplit('.').next().map(|e| e.to_ascii_lowercase()) });
                }
                if entry["status"]["completed"] == true {
                    return Err(AdapterError::ProviderError("the workflow finished but saved no audio. Add a Save Audio node.".into()));
                }
            }
            std::thread::sleep(self.poll);
        }
        Err(AdapterError::ProviderError("ComfyUI took too long".into()))
    }
    fn check(&self) -> Result<(), AdapterError> {
        self.get("/system_stats").map(|_| ())
    }
    fn sends_data(&self) -> bool {
        !self.base.is_local()
    }
    fn ready(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::parse_template;
    use crate::testutil::start;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    fn workflow() -> Value {
        parse_template(r#"{"3":{"class_type":"TextNode","inputs":{"text":"{{text}}","voice":"{{voice}}"}},"9":{"class_type":"SaveAudio","inputs":{"audio":["3",0]}}}"#, "text", 8192).unwrap()
    }

    #[test]
    fn queues_the_filled_workflow_waits_and_downloads_the_audio() {
        let polls = Arc::new(Mutex::new(0u32));
        let p2 = polls.clone();
        let f = start(Box::new(move |r, _| {
            if r.method == "POST" && r.path == "/prompt" {
                (200, json!({"prompt_id":"abc-123","number":1,"node_errors":{}}).to_string().into_bytes())
            } else if r.path == "/history/abc-123" {
                let mut n = p2.lock().unwrap();
                *n += 1;
                if *n < 3 { (200, b"{}".to_vec()) }
                else { (200, json!({"abc-123":{"outputs":{"9":{"audio":[{"filename":"out_00001_.wav","subfolder":"audio","type":"output"}]}},"status":{"status_str":"success","completed":true}}}).to_string().into_bytes()) }
            } else if r.path.starts_with("/view?") {
                (200, b"WAVDATA".to_vec())
            } else { (404, b"{}".to_vec()) }
        }));
        let t = ComfyTransport::for_test(&f.base, workflow(), 10);
        let out = t.run(&Vars { text: Some("Hello \"world\"".into()), voice: Some("af".into()), ..Default::default() }).unwrap();
        assert_eq!((out.audio.as_slice(), out.ext.as_deref()), (&b"WAVDATA"[..], Some("wav")));
        let hits = f.hits.lock().unwrap();
        let sent: Value = serde_json::from_str(&hits[0].body).unwrap();
        assert_eq!(sent["prompt"]["3"]["inputs"]["text"], "Hello \"world\"");
        assert_eq!(sent["prompt"]["3"]["inputs"]["voice"], "af");
        assert!(sent["client_id"].as_str().unwrap().starts_with("soundstage-"));
        let view = hits.iter().find(|h| h.path.starts_with("/view?")).unwrap();
        assert!(view.path.contains("filename=out_00001_.wav") && view.path.contains("subfolder=audio") && view.path.contains("type=output"), "{}", view.path);
    }

    #[test]
    fn hostile_output_names_are_never_followed() {
        let outs = json!({"9":{"audio":[
            {"filename":"../../Windows/win.mp3","subfolder":"","type":"output"},
            {"filename":"a/b.wav","subfolder":"","type":"output"},
            {"filename":"ok.wav","subfolder":"..\\x","type":"output"},
            {"filename":"C:evil.wav","subfolder":"","type":"output"},
            {"filename":"notes.txt","subfolder":"","type":"output"},
            {"filename":"fine.flac","subfolder":"sub","type":"output"}]}});
        assert_eq!(audio_files(&outs), vec![("fine.flac".to_string(), "sub".to_string(), "output".to_string())]);
    }

    #[test]
    fn errors_and_odd_servers_become_plain_messages() {
        let bad = start(Box::new(|_, _| (400, b"{}".to_vec())));
        assert!(matches!(ComfyTransport::for_test(&bad.base, workflow(), 2).run(&Vars::default()), Err(AdapterError::ProviderError(m)) if m.contains("rejected the workflow")));
        let node_err = start(Box::new(|_, _| (200, json!({"prompt_id":"p1","node_errors":{"3":{"errors":["missing"]}}}).to_string().into_bytes())));
        assert!(matches!(ComfyTransport::for_test(&node_err.base, workflow(), 2).run(&Vars::default()), Err(AdapterError::ProviderError(m)) if m.contains("problems in the workflow")));
        let evil_id = start(Box::new(|_, _| (200, json!({"prompt_id":"../../etc"}).to_string().into_bytes())));
        assert!(ComfyTransport::for_test(&evil_id.base, workflow(), 2).run(&Vars::default()).is_err());
        assert_eq!(evil_id.hits.lock().unwrap().len(), 1, "a hostile id is never put in a URL");
        let failed = start(Box::new(|r, _| if r.method == "POST" { (200, json!({"prompt_id":"p2"}).to_string().into_bytes()) } else { (200, json!({"p2":{"outputs":{},"status":{"status_str":"error","completed":false}}}).to_string().into_bytes()) }));
        assert!(matches!(ComfyTransport::for_test(&failed.base, workflow(), 2).run(&Vars::default()), Err(AdapterError::ProviderError(m)) if m.contains("reported an error")));
        let silent = start(Box::new(|r, _| if r.method == "POST" { (200, json!({"prompt_id":"p3"}).to_string().into_bytes()) } else { (200, json!({"p3":{"outputs":{"9":{"images":[]}},"status":{"status_str":"success","completed":true}}}).to_string().into_bytes()) }));
        assert!(matches!(ComfyTransport::for_test(&silent.base, workflow(), 2).run(&Vars::default()), Err(AdapterError::ProviderError(m)) if m.contains("no audio")));
        assert_eq!(ComfyTransport::for_test("http://127.0.0.1:1", workflow(), 1).run(&Vars::default()).unwrap_err(), AdapterError::Network);
    }

    #[test]
    fn only_local_or_private_or_https_servers_are_allowed_and_workflows_must_be_real() {
        assert!(ComfyTransport::new("http://example.com", workflow()).is_err());
        let ok = ComfyTransport::new("http://127.0.0.1:8188", workflow()).unwrap();
        assert!(!ok.sends_data());
        assert!(ComfyTransport::new("https://gpu.example.com", workflow()).unwrap().sends_data());
        assert!(validate_workflow(&json!({"a": 1})).is_err());
        assert!(validate_workflow(&json!({})).is_err());
        assert!(validate_workflow(&json!([1])).is_err());
    }

    #[test]
    fn connection_check_uses_system_stats() {
        let f = start(Box::new(|r, _| if r.path == "/system_stats" { (200, b"{}".to_vec()) } else { (404, b"{}".to_vec()) }));
        ComfyTransport::for_test(&f.base, workflow(), 1).check().unwrap();
        let g = start(Box::new(|_, _| (404, b"{}".to_vec())));
        assert!(ComfyTransport::for_test(&g.base, workflow(), 1).check().is_err());
    }
}
