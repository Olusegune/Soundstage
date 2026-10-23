//! The shared machinery for job-style audio services (fal, Kie including Suno, and ComfyUI): fill a JSON template,
//! submit a job, wait for it, and fetch the audio. The user supplies the model and its input template, because model
//! names and input fields differ per model and change often; Soundstage claims nothing about what any model can do.
//!
//! Templates are filled by walking the parsed JSON, never by pasting text into JSON, so a prompt can never break out
//! of its string. `{{seconds}}`, `{{seed}}` and `{{instrumental}}` become real numbers and booleans when they stand
//! alone as a value.

use crate::audio::resample;
use crate::decode::decode_bytes;
use crate::engines::VoiceDef;
use crate::generate::{clamp_seconds, GenKind, GenLimits, GenOutput, GenRequest, Generator};
use crate::licence::{Licence, LicenceInfo};
use crate::providers::*;
use serde_json::Value;

pub const PLACEHOLDERS: [&str; 6] = ["text", "prompt", "voice", "seconds", "seed", "instrumental"];
pub const MAX_TEMPLATE_BYTES: usize = 64 * 1024;
pub const MAX_WORKFLOW_BYTES: usize = 1024 * 1024;
pub const MAX_RESULT_AUDIO: usize = 80 * 1024 * 1024;

#[derive(Clone, Debug, Default)]
pub struct Vars {
    pub text: Option<String>,
    pub prompt: Option<String>,
    pub voice: Option<String>,
    pub seconds: Option<f64>,
    pub seed: Option<u64>,
    pub instrumental: Option<bool>,
}

fn names_in(s: &str) -> Vec<String> {
    let mut out = vec![];
    let mut rest = s;
    while let Some(i) = rest.find("{{") {
        let after = &rest[i + 2..];
        match after.find("}}") {
            Some(j) => {
                out.push(after[..j].trim().to_string());
                rest = &after[j + 2..];
            }
            None => break,
        }
    }
    out
}

fn walk_strings(v: &Value, f: &mut dyn FnMut(&str)) {
    match v {
        Value::String(s) => f(s),
        Value::Array(a) => a.iter().for_each(|x| walk_strings(x, f)),
        Value::Object(o) => o.values().for_each(|x| walk_strings(x, f)),
        _ => {}
    }
}

pub fn template_uses(tpl: &Value, name: &str) -> bool {
    let mut found = false;
    walk_strings(tpl, &mut |s| found |= names_in(s).iter().any(|n| n == name));
    found
}

/// Parse and vet a template the user typed. `needs` is the placeholder that must appear (`text` or `prompt`).
pub fn parse_template(text: &str, needs: &str, max_bytes: usize) -> Result<Value, String> {
    if text.len() > max_bytes {
        return Err(format!("The template is too large (limit {} KB).", max_bytes / 1024));
    }
    let v: Value = serde_json::from_str(text).map_err(|e| format!("The template is not valid JSON: {e}"))?;
    if !v.is_object() {
        return Err("The template must be a JSON object, written with { and }.".into());
    }
    let mut unknown: Option<String> = None;
    walk_strings(&v, &mut |s| {
        for n in names_in(s) {
            if !PLACEHOLDERS.contains(&n.as_str()) && unknown.is_none() {
                unknown = Some(n);
            }
        }
    });
    if let Some(u) = unknown {
        return Err(format!("{{{{{u}}}}} is not a placeholder Soundstage knows. Use {}.", PLACEHOLDERS.iter().map(|p| format!("{{{{{p}}}}}")).collect::<Vec<_>>().join(", ")));
    }
    if !template_uses(&v, needs) {
        return Err(format!("The template needs {{{{{needs}}}}} somewhere, so Soundstage knows where to put your {}.", if needs == "text" { "words" } else { "description" }));
    }
    Ok(v)
}

fn fill_string(s: &str, v: &Vars) -> Value {
    match s.trim() {
        "{{seconds}}" => return v.seconds.and_then(|x| serde_json::Number::from_f64(x).map(Value::Number)).unwrap_or(Value::Null),
        "{{seed}}" => return v.seed.map(|x| Value::Number(x.into())).unwrap_or(Value::Null),
        "{{instrumental}}" => return v.instrumental.map(Value::Bool).unwrap_or(Value::Null),
        _ => {}
    }
    let mut out = s.to_string();
    for (name, val) in [("text", &v.text), ("prompt", &v.prompt), ("voice", &v.voice)] {
        let rep = val.clone().unwrap_or_default();
        // Both spacings are accepted: {{text}} and {{ text }}.
        out = out.replace(&format!("{{{{{name}}}}}"), &rep).replace(&format!("{{{{ {name} }}}}"), &rep);
    }
    for (name, val) in [("seconds", v.seconds.map(|x| x.to_string())), ("seed", v.seed.map(|x| x.to_string())), ("instrumental", v.instrumental.map(|x| x.to_string()))] {
        let rep = val.unwrap_or_default();
        out = out.replace(&format!("{{{{{name}}}}}"), &rep);
    }
    Value::String(out)
}

pub fn fill_template(tpl: &Value, v: &Vars) -> Value {
    match tpl {
        Value::String(s) => fill_string(s, v),
        Value::Array(a) => Value::Array(a.iter().map(|x| fill_template(x, v)).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| (k.clone(), fill_template(x, v))).collect()),
        other => other.clone(),
    }
}

const AUDIO_EXTS: [&str; 7] = ["mp3", "wav", "flac", "ogg", "m4a", "aac", "opus"];

fn path_ext(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next()?;
    let last = path.rsplit('/').next()?;
    last.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase())
}

/// Audio addresses in a result, best guess first: https links that end like an audio file, then any https link found
/// under a key that sounds like audio. The caller still vets each one before fetching.
pub fn find_audio_urls(v: &Value) -> Vec<String> {
    fn walk(v: &Value, key: &str, strong: &mut Vec<String>, weak: &mut Vec<String>) {
        match v {
            Value::String(s) if s.starts_with("https://") && s.len() <= 2000 && !s.contains(char::is_whitespace) => {
                if path_ext(s).is_some_and(|e| AUDIO_EXTS.contains(&e.as_str())) {
                    if !strong.contains(s) { strong.push(s.clone()); }
                } else if ["audio", "song", "track", "music", "sound", "speech"].iter().any(|k| key.to_ascii_lowercase().contains(k)) && !weak.contains(s) {
                    weak.push(s.clone());
                }
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, key, strong, weak)),
            Value::Object(o) => o.iter().for_each(|(k, x)| walk(x, k, strong, weak)),
            _ => {}
        }
    }
    let (mut strong, mut weak) = (vec![], vec![]);
    walk(v, "", &mut strong, &mut weak);
    strong.extend(weak);
    strong
}

/// Downloads a file a provider pointed us to. A seam so tests never touch the network.
pub trait Fetcher: Send + Sync {
    fn fetch(&self, url: &str, suffixes: &[&str], max: usize) -> Result<Vec<u8>, String>;
}

pub struct RealFetcher;
impl Fetcher for RealFetcher {
    fn fetch(&self, url: &str, suffixes: &[&str], max: usize) -> Result<Vec<u8>, String> {
        crate::net::download(url, suffixes, max)
    }
}

pub struct RunOutput {
    pub audio: Vec<u8>,
    pub ext: Option<String>,
}

impl std::fmt::Debug for RunOutput {
    // Never print the audio itself, only how much there is.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RunOutput {{ {} bytes, ext: {:?} }}", self.audio.len(), self.ext)
    }
}

/// One way of running a job: fal, Kie or ComfyUI.
pub trait Transport: Send + Sync {
    fn run(&self, vars: &Vars) -> Result<RunOutput, AdapterError>;
    fn check(&self) -> Result<(), AdapterError>;
    fn sends_data(&self) -> bool;
    fn ready(&self) -> bool;
}

/// What the user configured for a job engine, once validated.
#[derive(Clone, Debug)]
pub struct JobMeta {
    pub id: String,
    pub label: String,
    pub licence_note: String,
    pub voices: Vec<VoiceDef>,
    pub limits: GenLimits,
}

fn job_licence(note: &str) -> LicenceInfo {
    LicenceInfo { licence: Licence::Unknown, source: None, checked: None, notes: if note.is_empty() { "Depends on the model. Read its terms and record your check.".into() } else { note.to_string() } }
}

/// Speech through a job engine. Directions are not available (the user cannot declare tags for a job template),
/// so a tag is "not supported by this voice" and pauses are exact silence.
pub struct JobSpeech {
    pub meta: JobMeta,
    pub id: String,
    pub transport: Box<dyn Transport>,
}

impl Adapter for JobSpeech {
    fn id(&self) -> &str {
        &self.id
    }
    fn label(&self) -> &str {
        &self.meta.label
    }
    fn licence(&self) -> LicenceInfo {
        job_licence(&self.meta.licence_note)
    }
    fn capabilities(&self) -> Capabilities {
        let mut languages: Vec<String> = self.meta.voices.iter().map(|v| if v.language.is_empty() { "en".to_string() } else { v.language.clone() }).collect();
        languages.sort();
        languages.dedup();
        if languages.is_empty() {
            languages.push("en".into());
        }
        Capabilities { languages, tags: vec![], voice_design: false, cloning: false, max_chars_per_request: 2500, streaming: false }
    }
    fn list_voices(&self) -> Vec<Voice> {
        let defs = if self.meta.voices.is_empty() { vec![VoiceDef { id: "default".into(), name: self.meta.label.clone(), language: "en".into(), ..Default::default() }] } else { self.meta.voices.clone() };
        defs.into_iter()
            .map(|v| Voice {
                id: format!("{}:{}", self.id, v.id),
                name: if v.name.is_empty() { v.id.clone() } else { v.name.clone() },
                provider: self.id.clone(),
                language: if v.language.is_empty() { "en".into() } else { v.language },
                accent: if v.accent.is_empty() { "Not stated".into() } else { v.accent },
                gender: if v.gender.is_empty() { "Not stated".into() } else { v.gender },
                age_range: "Not stated".into(),
                licence: self.licence(),
                sample_text: "Good morning. This is how I sound.".into(),
                notes: if self.transport.sends_data() { "Script text is sent to this service. No directions are available.".into() } else { "Runs on this computer or your network. No directions are available.".into() },
            })
            .collect()
    }
    fn provider_version(&self) -> String {
        format!("job/{}", self.meta.id)
    }
    fn sends_data(&self) -> bool {
        self.transport.sends_data()
    }
    fn check_connection(&self) -> Result<(), AdapterError> {
        self.transport.check()
    }
    fn render(&self, req: &RenderRequest) -> Result<RenderOutput, AdapterError> {
        precheck(&self.capabilities(), req)?;
        let prefix = format!("{}:", self.id);
        let local = req.voice_id.strip_prefix(&prefix).ok_or_else(|| AdapterError::UnknownVoice(req.voice_id.chars().take(60).collect()))?;
        if !self.list_voices().iter().any(|v| v.id == req.voice_id) {
            return Err(AdapterError::UnknownVoice(req.voice_id.chars().take(60).collect()));
        }
        let segs = compose_segments(&req.tokens, &req.emotion, &|_| None)?;
        let mut pieces: Vec<Result<(Vec<f32>, u32), u32>> = vec![];
        for s in segs {
            match s {
                Err(ms) => pieces.push(Err(ms)),
                Ok(text) => {
                    let out = self.transport.run(&Vars { text: Some(text), voice: Some(local.to_string()), ..Default::default() })?;
                    let d = decode_bytes(&out.audio, out.ext.as_deref()).map_err(|_| AdapterError::ProviderError("the service did not return audio Soundstage could read".into()))?;
                    let n = d.channels[0].len();
                    let mono = (0..n).map(|i| d.channels.iter().map(|c| c.get(i).copied().unwrap_or(0.0)).sum::<f32>() / d.channels.len() as f32).collect();
                    pieces.push(Ok((mono, d.rate)));
                }
            }
        }
        let rate = pieces.iter().find_map(|p| p.as_ref().ok().map(|(_, r)| *r)).unwrap_or(24_000);
        let mut samples = vec![];
        for p in pieces {
            match p {
                Ok((s, r)) => samples.extend(if r == rate { s } else { resample(&s, r, rate) }),
                Err(ms) => samples.extend(std::iter::repeat(0.0).take((ms as usize * rate as usize) / 1000)),
            }
        }
        Ok(RenderOutput { samples, sample_rate: rate, provider_version: self.provider_version() })
    }
}

/// Music or sound effects through a job engine.
pub struct JobGenerator {
    pub meta: JobMeta,
    pub kind: GenKind,
    pub transport: Box<dyn Transport>,
}

impl Generator for JobGenerator {
    fn id(&self) -> &str {
        &self.meta.id
    }
    fn label(&self) -> &str {
        &self.meta.label
    }
    fn kinds(&self) -> Vec<GenKind> {
        vec![self.kind]
    }
    fn limits(&self, _kind: GenKind) -> GenLimits {
        self.meta.limits
    }
    fn licence(&self) -> LicenceInfo {
        job_licence(&self.meta.licence_note)
    }
    fn notes(&self) -> String {
        if self.transport.sends_data() { "Your description is sent to this service and may use your account's credits.".into() } else { "Runs on this computer or your network.".into() }
    }
    fn sends_data(&self) -> bool {
        self.transport.sends_data()
    }
    fn ready(&self) -> bool {
        self.transport.ready()
    }
    fn check_connection(&self) -> Result<(), AdapterError> {
        self.transport.check()
    }
    fn generate(&self, req: &GenRequest) -> Result<GenOutput, AdapterError> {
        let secs = clamp_seconds(&self.meta.limits, req.seconds) as f64;
        let vars = Vars { prompt: Some(req.prompt.clone()), seconds: Some(secs), seed: Some(crate::util::unix_now() % 1_000_000), instrumental: Some(req.instrumental), ..Default::default() };
        let out = self.transport.run(&vars)?;
        let audio = crate::decode::to_stereo_48k(&out.audio, out.ext.as_deref()).map_err(|_| AdapterError::ProviderError("the service did not return audio Soundstage could read".into()))?;
        Ok(GenOutput { audio, provider_version: format!("job/{}", self.meta.id) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn templates_are_filled_by_walking_the_json_so_text_can_never_break_out() {
        let t = parse_template(r#"{"input":{"text":"{{text}}","voice":"{{voice}}","note":"Say: {{ text }} now"},"n":["{{prompt}}",3]}"#, "text", 4096).unwrap();
        let v = Vars { text: Some("He said \"hi\"\n}, {\"evil\": true".into()), voice: Some("alloy".into()), prompt: Some("rain".into()), ..Default::default() };
        let f = fill_template(&t, &v);
        assert_eq!(f["input"]["text"], "He said \"hi\"\n}, {\"evil\": true");
        assert_eq!(f["input"]["note"], "Say: He said \"hi\"\n}, {\"evil\": true now");
        assert_eq!(f["n"][0], "rain");
        assert_eq!(f["n"][1], 3);
        assert!(f.get("evil").is_none() && f["input"].get("evil").is_none());
    }

    #[test]
    fn standalone_numbers_and_booleans_keep_their_type() {
        let t = parse_template(r#"{"prompt":"{{prompt}}","duration":"{{seconds}}","seed":"{{seed}}","instrumental":"{{instrumental}}","label":"{{seconds}} s"}"#, "prompt", 4096).unwrap();
        let f = fill_template(&t, &Vars { prompt: Some("x".into()), seconds: Some(12.5), seed: Some(7), instrumental: Some(true), ..Default::default() });
        assert_eq!((f["duration"].as_f64(), f["seed"].as_u64(), f["instrumental"].as_bool()), (Some(12.5), Some(7), Some(true)));
        assert_eq!(f["label"], "12.5 s", "inside text it is text");
        let g = fill_template(&t, &Vars { prompt: Some("x".into()), ..Default::default() });
        assert!(g["duration"].is_null() && g["instrumental"].is_null());
    }

    #[test]
    fn bad_templates_are_refused_with_a_reason() {
        for (bad, needs, why) in [
            ("not json", "text", "not valid JSON"),
            ("[1,2]", "text", "JSON object"),
            (r#"{"a":"{{nope}}","t":"{{text}}"}"#, "text", "not a placeholder"),
            (r#"{"a":"hello"}"#, "text", "needs {{text}}"),
            (r#"{"a":"{{text}}"}"#, "prompt", "needs {{prompt}}"),
        ] {
            let e = parse_template(bad, needs, 4096).unwrap_err();
            assert!(e.contains(why), "{bad}: {e}");
        }
        assert!(parse_template(&format!("{{\"a\":\"{}{{{{text}}}}\"}}", "x".repeat(5000)), "text", 4096).unwrap_err().contains("too large"));
    }

    struct FakeTransport {
        seen: std::sync::Mutex<Vec<Vars>>,
        cloud: bool,
    }
    impl Transport for FakeTransport {
        fn run(&self, vars: &Vars) -> Result<RunOutput, AdapterError> {
            self.seen.lock().unwrap().push(vars.clone());
            let words = vars.text.as_deref().or(vars.prompt.as_deref()).unwrap_or("x").split_whitespace().count().max(1);
            let secs = vars.seconds.map(|s| s as f32).unwrap_or(words as f32 * 0.35);
            let mut cur = std::io::Cursor::new(Vec::new());
            {
                let spec = hound::WavSpec { channels: 1, sample_rate: 24_000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
                let mut w = hound::WavWriter::new(&mut cur, spec).unwrap();
                for i in 0..(secs * 24_000.0) as usize { w.write_sample((8000.0 * (i as f32 * 0.05).sin()) as i16).unwrap(); }
                w.finalize().unwrap();
            }
            Ok(RunOutput { audio: cur.into_inner(), ext: Some("wav".into()) })
        }
        fn check(&self) -> Result<(), AdapterError> { Ok(()) }
        fn sends_data(&self) -> bool { self.cloud }
        fn ready(&self) -> bool { true }
    }
    fn meta(voices: Vec<VoiceDef>) -> JobMeta {
        JobMeta { id: "job-x".into(), label: "Job X".into(), licence_note: String::new(), voices, limits: GenLimits { min_s: 3.0, max_s: 60.0, default_s: 20.0, supports_length: true } }
    }

    #[test]
    fn job_voices_pass_the_adapter_contract_and_start_with_an_unknown_licence() {
        let a = JobSpeech { id: "job-x".into(), meta: meta(vec![VoiceDef { id: "v1".into(), name: "One".into(), language: "en-US".into(), ..Default::default() }]), transport: Box::new(FakeTransport { seen: Default::default(), cloud: true }) };
        let v = a.list_voices();
        assert_eq!((v[0].id.as_str(), v[0].licence.licence), ("job-x:v1", Licence::Unknown));
        assert!(a.sends_data());
        contract::run(&a, &v[0], "sneeze");
    }

    #[test]
    fn a_job_voice_without_a_voice_list_gets_one_default_voice_and_passes_text_and_voice_through() {
        let t = FakeTransport { seen: Default::default(), cloud: false };
        let a = JobSpeech { id: "job-x".into(), meta: meta(vec![]), transport: Box::new(t) };
        let v = a.list_voices();
        assert_eq!(v.len(), 1);
        assert!(!a.sends_data());
        let r = RenderRequest { tokens: crate::tags::tokenize("hello [pause:500] there").0, voice_id: v[0].id.clone(), emotion: "neutral".into(), pace: 1.0, language: "en".into() };
        let out = a.render(&r).unwrap();
        assert_eq!(out.sample_rate, 24_000);
        // two spoken segments of one word (0.35 s) each, plus an exact half-second pause
        assert!((out.samples.len() as i64 - (2 * 8_400 + 12_000)).abs() < 50, "{}", out.samples.len());
        assert_eq!(a.render(&RenderRequest { voice_id: "other:default".into(), ..r.clone() }).unwrap_err(), AdapterError::UnknownVoice("other:default".into()));
        assert_eq!(a.render(&RenderRequest { tokens: crate::tags::tokenize("hi [sigh]").0, ..r }).unwrap_err(), AdapterError::UnsupportedTag("sigh".into()));
    }

    #[test]
    fn job_generators_send_the_prompt_length_and_flags_and_return_48k_stereo() {
        let g = JobGenerator { meta: meta(vec![]), kind: GenKind::Music, transport: Box::new(FakeTransport { seen: Default::default(), cloud: true }) };
        let out = g.generate(&GenRequest { kind: GenKind::Music, prompt: "calm strings".into(), seconds: Some(500.0), instrumental: true }).unwrap();
        assert_eq!(out.audio.l.len(), out.audio.r.len());
        assert!((out.audio.len() as i64 - 60 * 48_000).abs() < 200, "length is clamped to the engine's maximum: {}", out.audio.len());
        assert_eq!(g.kinds(), vec![GenKind::Music]);
        assert!(g.sends_data() && g.ready());
        assert_eq!(g.licence().licence, Licence::Unknown);
    }

    #[test]
    fn audio_links_are_found_best_first() {
        let v = json!({ "audio": { "url": "https://v3.fal.media/files/a/b.mp3?x=1" }, "image": "https://v3.fal.media/files/p.png", "other": ["https://cdn.example.com/s.wav"], "song_link": "https://cdn.example.com/stream/123", "http": "http://insecure.example.com/a.mp3", "x": "https://a b.com/c.mp3" });
        assert_eq!(find_audio_urls(&v), vec!["https://v3.fal.media/files/a/b.mp3?x=1", "https://cdn.example.com/s.wav", "https://cdn.example.com/stream/123"]);
        assert!(find_audio_urls(&json!({"a": 1})).is_empty());
    }
}
