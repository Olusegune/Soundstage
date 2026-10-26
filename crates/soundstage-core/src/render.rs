//! Analysis and rendering. Every line gets a cache key built from its text, voice,
//! direction and provider version. Re-rendering an unchanged line makes zero provider calls,
//! and editing one line marks only that line stale.

use crate::audio::{quality, resample, write_wav_16_mono, QualityReport};
use crate::direction::{resolve, Level, Resolved};
use crate::project::{Project, Take, TakeStatus};
use crate::providers::{split_tokens, unsupported_tags, Adapter, AdapterError, RenderRequest, Registry};
use crate::script::{ParsedLine, Parsed};
use crate::tags::{token_canonical, tokenize, Token};
use crate::util::{iso_now, new_id, sha256_hex};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineStatus {
    Drafted,
    Rendered,
    Stale,
    Skipped,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LineIssue {
    /// `unknown_tag`, `unsupported_tag`, `no_voice`, `refused`, `empty`, `voice_missing`.
    pub kind: String,
    pub message: String,
    pub tag: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LineInfo {
    pub line: ParsedLine,
    pub resolved: Resolved,
    pub status: LineStatus,
    pub issues: Vec<LineIssue>,
    pub cache_key: Option<String>,
    pub voice_id: Option<String>,
    pub selected_take: Option<String>,
    pub take_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Analysis {
    pub parsed: Parsed,
    pub lines: Vec<LineInfo>,
    pub speakers_without_voice: Vec<String>,
}

pub fn cache_key(adapter_id: &str, provider_version: &str, voice_id: &str, tokens: &[Token], emotion: &str, pace: f32, language: &str) -> String {
    let canon: Vec<String> = tokens.iter().map(token_canonical).collect();
    sha256_hex(serde_json::json!([adapter_id, provider_version, voice_id, canon, emotion, format!("{pace:.2}"), language]).to_string().as_bytes())
}

pub fn resolved_for(project: &Project, line: &ParsedLine) -> Resolved {
    let ch = project.characters.get(&line.speaker).map(|c| c.direction.clone()).unwrap_or_default();
    let sc = project.scene_directions.get(&line.scene.to_string()).cloned().unwrap_or_default();
    let ln = project.line_directions.get(&line.key).cloned().unwrap_or_default();
    resolve(&project.project_direction, &ch, &sc, &ln)
}

fn describe(r: &Resolved) -> String {
    let lvl = |l: Level| format!("{l:?}").to_lowercase();
    format!(
        "emotion {} ({}), pace {:.2} ({})",
        r.emotion.value,
        lvl(r.emotion.source),
        r.pace.value,
        lvl(r.pace.source)
    )
}

/// The tokens a voice is actually given for a line: the script's own, with pronunciation rules applied to spoken text.
pub fn spoken_tokens(project: &Project, reg: &Registry, text: &str) -> (Vec<Token>, Vec<crate::tags::TagIssue>) {
    let (tokens, issues) = tokenize(text);
    if project.pronunciations.is_empty() && reg.pronunciations.is_empty() {
        return (tokens, issues);
    }
    let rules: Vec<crate::pronounce::Rule> = project.pronunciations.iter().chain(reg.pronunciations.iter()).cloned().collect();
    (crate::pronounce::apply_tokens(tokens, &rules), issues)
}

pub fn analyze(project: &Project, reg: &Registry) -> Analysis {
    let parsed = project.parsed();
    let mut lines = Vec::new();
    for line in &parsed.lines {
        let resolved = resolved_for(project, line);
        let (tokens, tag_issues) = spoken_tokens(project, reg, &line.text);
        let mut issues: Vec<LineIssue> = tag_issues
            .iter()
            .map(|i| LineIssue { kind: "unknown_tag".into(), message: i.message.clone(), tag: Some(i.tag.trim_matches(['[', ']']).to_string()) })
            .collect();
        if !tokens.iter().any(|t| matches!(t, Token::Text(_))) {
            issues.push(LineIssue { kind: "empty".into(), message: "There is nothing to say on this line.".into(), tag: None });
        }
        if let Some(r) = crate::safety::check(&line.text) {
            issues.push(LineIssue { kind: "refused".into(), message: format!("{} {}", r.message, r.alternative), tag: None });
        }
        let voice_id = project.characters.get(&line.speaker).and_then(|c| c.voice_id.clone());
        let mut key = None;
        match voice_id.as_deref() {
            None => issues.push(LineIssue { kind: "no_voice".into(), message: format!("{} has no voice yet.", line.speaker), tag: None }),
            Some(vid) => match (reg.voice(vid), reg.adapter_for_voice(vid)) {
                (Some(v), Some(a)) => {
                    for t in unsupported_tags(&a.capabilities(), &tokens, &resolved.emotion.value) {
                        issues.push(LineIssue {
                            kind: "unsupported_tag".into(),
                            message: format!("[{t}] is not supported by {}.", v.name),
                            tag: Some(t),
                        });
                    }
                    key = Some(cache_key(a.id(), &a.provider_version(), vid, &tokens, &resolved.emotion.value, resolved.pace.value, &v.language));
                }
                _ => issues.push(LineIssue { kind: "voice_missing".into(), message: "The cast voice is not available on this computer.".into(), tag: None }),
            },
        }
        let selected = project.selected_take(&line.key);
        let status = if project.skipped.contains(&line.key) {
            LineStatus::Skipped
        } else {
            match (selected, &key) {
                (None, _) => LineStatus::Drafted,
                (Some(t), Some(k)) if &t.cache_key == k => LineStatus::Rendered,
                (Some(_), _) => LineStatus::Stale,
            }
        };
        lines.push(LineInfo {
            line: line.clone(),
            resolved,
            status,
            issues,
            cache_key: key,
            voice_id,
            selected_take: selected.map(|t| t.id.clone()),
            take_count: project.takes.iter().filter(|t| t.line_key == line.key).count(),
        });
    }
    let speakers_without_voice = parsed
        .speakers
        .iter()
        .filter(|s| project.characters.get(*s).and_then(|c| c.voice_id.as_ref()).is_none())
        .cloned()
        .collect();
    Analysis { parsed, lines, speakers_without_voice }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RenderError {
    pub kind: String,
    pub message: String,
}

fn err(kind: &str, message: impl Into<String>) -> RenderError {
    RenderError { kind: kind.into(), message: message.into() }
}

impl From<AdapterError> for RenderError {
    fn from(e: AdapterError) -> Self {
        let kind = match &e {
            AdapterError::UnsupportedTag(_) => "unsupported_tag",
            AdapterError::Auth => "auth",
            AdapterError::RateLimit => "rate_limit",
            AdapterError::Quota => "quota",
            AdapterError::Network => "network",
            _ => "provider_error",
        };
        err(kind, e.to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderResult {
    pub take_id: String,
    pub cached: bool,
    pub duration_ms: u64,
    pub quality: Option<QualityReport>,
}

/// Render one line into a take and select it. Reuses an identical earlier take without calling the provider.
pub fn render_line(project: &mut Project, reg: &Registry, project_dir: &Path, line_key: &str) -> Result<RenderResult, RenderError> {
    let parsed = project.parsed();
    let line = parsed.lines.iter().find(|l| l.key == line_key).ok_or_else(|| err("not_found", "That line no longer exists in the script."))?.clone();
    let resolved = resolved_for(project, &line);

    let (tokens, issues) = spoken_tokens(project, reg, &line.text);
    if let Some(i) = issues.first() {
        return Err(err("unknown_tag", format!("{} Fix or remove it before rendering.", i.message)));
    }
    if !tokens.iter().any(|t| matches!(t, Token::Text(_))) {
        return Err(err("empty", "There is nothing to say on this line."));
    }
    if let Some(r) = crate::safety::check(&line.text) {
        return Err(err("refused", format!("{} {}", r.message, r.alternative)));
    }
    let voice_id = project
        .characters
        .get(&line.speaker)
        .and_then(|c| c.voice_id.clone())
        .ok_or_else(|| err("no_voice", format!("{} has no voice yet. Pick one in the Cast step.", line.speaker)))?;
    let voice = reg.voice(&voice_id).ok_or_else(|| err("voice_missing", "The cast voice is not available on this computer."))?.clone();
    let adapter: &dyn Adapter = reg.adapter(&voice.provider).ok_or_else(|| err("voice_missing", "The voice's provider is not available."))?;
    // A cloned voice is used only while its owner's consent is clean. This runs before the cache, so a revoked voice
    // cannot hand back an old take either.
    if let Some(problems) = reg.consent_problems(&voice_id, project.licence_target == crate::licence::LicenceTarget::Commercial) {
        return Err(err("consent", format!("{} cannot be used: {}", voice.name, problems.join(" "))));
    }

    let missing = unsupported_tags(&adapter.capabilities(), &tokens, &resolved.emotion.value);
    if !missing.is_empty() {
        let list = missing.iter().map(|t| format!("[{t}]")).collect::<Vec<_>>().join(", ");
        return Err(err("unsupported_tag", format!("{} is not supported by {}. Pick another voice or remove the tag.", list, voice.name)));
    }

    let key = cache_key(adapter.id(), &adapter.provider_version(), &voice_id, &tokens, &resolved.emotion.value, resolved.pace.value, &voice.language);
    if let Some(existing) = project.takes.iter().find(|t| t.cache_key == key && t.line_key == line_key && project_dir.join(&t.file).exists()) {
        let (id, ms) = (existing.id.clone(), existing.duration_ms);
        project.selected.insert(line_key.to_string(), id.clone());
        return Ok(RenderResult { take_id: id, cached: true, duration_ms: ms, quality: None });
    }

    // The core splits oversize requests and carries the active delivery style across chunks.
    let mut samples: Vec<f32> = Vec::new();
    let mut rate = 0u32;
    let mut version = adapter.provider_version();
    let mut style = resolved.emotion.value.clone();
    for chunk in split_tokens(&tokens, adapter.capabilities().max_chars_per_request) {
        let req = RenderRequest { tokens: chunk.clone(), voice_id: voice_id.clone(), emotion: style.clone(), pace: resolved.pace.value, language: voice.language.clone() };
        let out = adapter.render(&req)?;
        version = out.provider_version.clone();
        let piece = if rate == 0 { out.samples } else { resample(&out.samples, out.sample_rate, rate) };
        if rate == 0 {
            rate = out.sample_rate;
        }
        samples.extend(piece);
        for t in &chunk {
            if let Token::Delivery(d) = t {
                style = d.clone();
            }
        }
    }
    if samples.is_empty() || rate == 0 {
        return Err(err("provider_error", "The voice returned no audio."));
    }
    let q = quality(&samples, rate);
    let id = new_id("take");
    let rel = format!("takes/{id}.wav");
    std::fs::create_dir_all(project_dir.join("takes")).map_err(|e| err("io", e.to_string()))?;
    write_wav_16_mono(&project_dir.join(&rel), &samples, rate).map_err(|e| err("io", e))?;
    let duration_ms = samples.len() as u64 * 1000 / rate as u64;
    project.takes.push(Take {
        id: id.clone(),
        line_key: line_key.to_string(),
        speaker: line.speaker.clone(),
        text: tokens.iter().map(token_canonical).collect::<Vec<_>>().join(" "),
        voice_id,
        adapter_id: adapter.id().to_string(),
        provider_version: version,
        cache_key: key,
        file: rel,
        duration_ms,
        sample_rate: rate,
        direction_summary: describe(&resolved),
        created: iso_now(),
        status: TakeStatus::Pending,
        approver: None,
    });
    project.selected.insert(line_key.to_string(), id.clone());
    Ok(RenderResult { take_id: id, cached: false, duration_ms, quality: Some(q) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::scratch::ScratchAdapter;
    use crate::direction::Direction;
    use crate::project::Character;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Wraps the scratch adapter and counts provider calls.
    struct Counting(Arc<AtomicUsize>);
    impl Adapter for Counting {
        fn id(&self) -> &'static str { ScratchAdapter.id() }
        fn label(&self) -> &'static str { "count" }
        fn licence(&self) -> crate::licence::LicenceInfo { ScratchAdapter.licence() }
        fn capabilities(&self) -> crate::providers::Capabilities { ScratchAdapter.capabilities() }
        fn list_voices(&self) -> Vec<crate::providers::Voice> { ScratchAdapter.list_voices() }
        fn provider_version(&self) -> String { ScratchAdapter.provider_version() }
        fn render(&self, r: &RenderRequest) -> Result<crate::providers::RenderOutput, AdapterError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            ScratchAdapter.render(r)
        }
    }

    fn setup() -> (Project, Registry, Arc<AtomicUsize>, std::path::PathBuf) {
        let calls = Arc::new(AtomicUsize::new(0));
        let reg = Registry::new(vec![Box::new(Counting(calls.clone()))]);
        let mut p = Project::new("R", false);
        p.script = "# S\nAMA: hello there friend\nKOLA: [sneeze] bless you\nAMA: second line".into();
        for s in ["AMA", "KOLA"] {
            p.characters.insert(s.into(), Character { voice_id: Some("tone:mid".into()), ..Default::default() });
        }
        let dir = std::env::temp_dir().join(new_id("ss-render"));
        std::fs::create_dir_all(&dir).unwrap();
        (p, reg, calls, dir)
    }

    #[test]
    fn pronunciation_rules_change_what_is_spoken_and_make_old_takes_stale_only_when_they_apply() {
        let (mut p, mut reg, _calls, dir) = setup();
        render_line(&mut p, &reg, &dir, "s0-l0").unwrap();
        let status = |p: &Project, reg: &Registry| analyze(p, reg).lines.iter().find(|l| l.line.key == "s0-l0").unwrap().status;
        assert_eq!(status(&p, &reg), LineStatus::Rendered);
        // A rule that matches nothing in the line changes nothing, so paid takes are not thrown away.
        p.pronunciations.push(crate::pronounce::Rule { from: "Okafor".into(), to: "Oh-kah-for".into(), ..Default::default() });
        assert_eq!(status(&p, &reg), LineStatus::Rendered);
        // A rule that applies changes what the voice is given, so the take is stale until it is made again.
        p.pronunciations.push(crate::pronounce::Rule { from: "friend".into(), to: "frend".into(), ..Default::default() });
        assert_eq!(status(&p, &reg), LineStatus::Stale);
        render_line(&mut p, &reg, &dir, "s0-l0").unwrap();
        assert_eq!(status(&p, &reg), LineStatus::Rendered);
        // App-wide rules count too, and the script itself keeps its spelling.
        reg = reg.with_pronunciations(vec![crate::pronounce::Rule { from: "hello".into(), to: "hullo".into(), ..Default::default() }]);
        assert_eq!(status(&p, &reg), LineStatus::Stale);
        assert!(p.script.contains("hello there friend"));
        let (t, _) = spoken_tokens(&p, &reg, "hello there friend");
        assert_eq!(t, vec![Token::Text("hullo there frend".into())]);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_cloned_voice_is_usable_only_while_its_consent_is_clean() {
        let (mut p, mut reg, calls, dir) = setup();
        let form = crate::clone::Attestation { subject: "Ama Okafor".into(), subject_is_self: true, adult: true, alive: true, not_public_figure: true, agrees: true, purpose: "Narration".into(), commercial: false, expires: None };
        let mut rec = crate::clone::build_record(&form, "consent-gate").unwrap();
        reg.consents.insert("tone:mid".into(), rec.clone());
        assert!(render_line(&mut p, &reg, &dir, "s0-l0").is_ok(), "clean consent renders");
        assert!(crate::exporter::check(&p, &reg).allowed, "and exports");
        assert_eq!(crate::exporter::consent_ids_used(&p, &reg), vec!["consent-gate".to_string()]);

        // Personal use is fine, but the same record blocks a commercial project.
        p.licence_target = crate::licence::LicenceTarget::Commercial;
        let e = render_line(&mut p, &reg, &dir, "s0-l2").unwrap_err();
        assert_eq!(e.kind, "consent");
        assert!(e.message.contains("commercial"), "{}", e.message);
        p.licence_target = crate::licence::LicenceTarget::Personal;

        // Revoking blocks a new render, a cached render and the export, at once.
        crate::consent::revoke(&mut rec).unwrap();
        reg.consents.insert("tone:mid".into(), rec);
        let before = calls.load(Ordering::SeqCst);
        assert_eq!(render_line(&mut p, &reg, &dir, "s0-l0").unwrap_err().kind, "consent", "cached take is blocked too");
        assert_eq!(render_line(&mut p, &reg, &dir, "s0-l2").unwrap_err().kind, "consent");
        assert_eq!(calls.load(Ordering::SeqCst), before, "no provider call is made for a revoked voice");
        let gate = crate::exporter::check(&p, &reg);
        assert!(!gate.allowed && gate.blocking.iter().any(|b| b.kind == "consent" && b.reason.contains("revoked")));
        std::fs::remove_dir_all(dir).ok();
    }

    /// AC-006: adding a line must not disturb the lines that were already rendered.
    #[test]
    fn inserting_a_line_leaves_every_other_line_rendered_and_costs_no_calls() {
        let (mut p, reg, calls, dir) = setup();
        p.script = "# S
AMA: hello there friend
KOLA: bless you
AMA: second line".into();
        p.sync_ids();
        let keys: Vec<String> = p.parsed().lines.iter().map(|l| l.key.clone()).collect();
        for k in &keys {
            render_line(&mut p, &reg, &dir, k).unwrap();
        }
        let made = calls.load(Ordering::SeqCst);
        p.script = format!("# S
AMA: A brand new opening.
{}", p.script.trim_start_matches("# S
"));
        p.sync_ids();
        let a = analyze(&p, &reg);
        assert_eq!(a.lines.len(), 4);
        for k in &keys {
            let l = a.lines.iter().find(|l| &l.line.key == k).expect("old line keeps its id");
            assert_eq!(l.status, LineStatus::Rendered, "{k} stays rendered");
        }
        let fresh = a.lines.iter().filter(|l| !keys.contains(&l.line.key)).count();
        assert_eq!(fresh, 1);
        for k in &keys {
            render_line(&mut p, &reg, &dir, k).unwrap();
        }
        assert_eq!(calls.load(Ordering::SeqCst), made, "re-rendering old lines makes no provider calls");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rerendering_an_unchanged_line_makes_zero_provider_calls() {
        let (mut p, reg, calls, dir) = setup();
        let a = render_line(&mut p, &reg, &dir, "s0-l0").unwrap();
        assert!(!a.cached);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let b = render_line(&mut p, &reg, &dir, "s0-l0").unwrap();
        assert!(b.cached);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(p.takes.len(), 1);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn changing_a_tag_direction_or_voice_misses_the_cache() {
        let (mut p, reg, calls, dir) = setup();
        render_line(&mut p, &reg, &dir, "s0-l0").unwrap();
        p.script = p.script.replace("hello there friend", "hello [pause:short] there friend");
        assert!(!render_line(&mut p, &reg, &dir, "s0-l0").unwrap().cached);
        p.line_directions.insert("s0-l0".into(), Direction { emotion: Some("sad".into()), ..Default::default() });
        assert!(!render_line(&mut p, &reg, &dir, "s0-l0").unwrap().cached);
        p.characters.get_mut("AMA").unwrap().voice_id = Some("tone:high".into());
        assert!(!render_line(&mut p, &reg, &dir, "s0-l0").unwrap().cached);
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        assert_eq!(p.takes.len(), 4, "earlier takes are kept");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn editing_one_line_marks_only_that_line_stale() {
        let (mut p, reg, _c, dir) = setup();
        render_line(&mut p, &reg, &dir, "s0-l0").unwrap();
        render_line(&mut p, &reg, &dir, "s0-l2").unwrap();
        let before = analyze(&p, &reg);
        assert_eq!(before.lines[0].status, LineStatus::Rendered);
        p.script = p.script.replace("second line", "second line, edited");
        let after = analyze(&p, &reg);
        assert_eq!(after.lines[0].status, LineStatus::Rendered);
        assert_eq!(after.lines[2].status, LineStatus::Stale);
        assert_eq!(after.lines[1].status, LineStatus::Drafted);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn scene_note_change_marks_only_inheriting_lines_stale() {
        let (mut p, reg, _c, dir) = setup();
        p.line_directions.insert("s0-l2".into(), Direction { emotion: Some("whisper".into()), ..Default::default() });
        render_line(&mut p, &reg, &dir, "s0-l0").unwrap();
        render_line(&mut p, &reg, &dir, "s0-l2").unwrap();
        p.scene_directions.insert("0".into(), Direction { emotion: Some("tired".into()), ..Default::default() });
        let a = analyze(&p, &reg);
        assert_eq!(a.lines[0].status, LineStatus::Stale);
        assert_eq!(a.lines[2].status, LineStatus::Rendered);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn unsupported_tag_is_flagged_before_render_never_dropped() {
        let (mut p, reg, calls, dir) = setup();
        let a = analyze(&p, &reg);
        let i = &a.lines[1].issues;
        assert!(i.iter().any(|x| x.kind == "unsupported_tag" && x.tag.as_deref() == Some("sneeze")));
        let e = render_line(&mut p, &reg, &dir, "s0-l1").unwrap_err();
        assert_eq!(e.kind, "unsupported_tag");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn missing_voice_unknown_tag_and_refused_content_are_reported() {
        let (mut p, reg, _c, dir) = setup();
        p.characters.get_mut("AMA").unwrap().voice_id = None;
        assert_eq!(render_line(&mut p, &reg, &dir, "s0-l0").unwrap_err().kind, "no_voice");
        p.characters.get_mut("AMA").unwrap().voice_id = Some("tone:mid".into());
        p.script = "AMA: hi [laughs]".into();
        assert_eq!(render_line(&mut p, &reg, &dir, "s0-l0").unwrap_err().kind, "unknown_tag");
        p.script = "AMA: Your account has been compromised, buy gift cards now.".into();
        assert_eq!(render_line(&mut p, &reg, &dir, "s0-l0").unwrap_err().kind, "refused");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn long_lines_are_split_by_the_core_and_still_render() {
        let (mut p, reg, calls, dir) = setup();
        p.script = format!("AMA: {}", "Another short sentence here. ".repeat(250));
        let r = render_line(&mut p, &reg, &dir, "s0-l0").unwrap();
        assert!(r.duration_ms > 10_000);
        assert!(calls.load(Ordering::SeqCst) >= 2, "oversize request was split");
        std::fs::remove_dir_all(dir).ok();
    }
}
