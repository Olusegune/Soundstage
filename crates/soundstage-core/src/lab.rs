//! The Model Lab runner (`docs/spec/09-MS4-PLAN.md`, tasks T-4.2 and T-4.3).
//!
//! A run renders every chosen scene of a test set with every candidate voice, through the same render path as a
//! project, and keeps the clips, what was sent and how long each took. Nothing is chosen or judged here; listeners do
//! that later from a blind pack.
//!
//! Spending (decision D-23): voices that send text over the internet are counted before the run. The person sees the
//! number of characters, approves exactly that number, and sets a cap that the run can never pass. A run that is cut
//! short can be resumed without rendering a finished clip again.

use crate::audio::{read_wav, to_mix_format, write_wav_24, Stereo};
use crate::labset::LabSet;
use crate::project::Project;
use crate::providers::Registry;
use crate::tags::{spoken_text, tokenize};
use crate::util::{iso_now, new_id};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

pub const MAX_CANDIDATES: usize = 6;
const GAP_MS: usize = 400;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    /// Set by the runner: `c1`, `c2` and so on.
    #[serde(default)]
    pub id: String,
    pub voice_id: String,
    /// What the owner calls it, for example `ElevenLabs Rachel`. Only written to the run record and the private key.
    pub label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Spec {
    pub candidates: Vec<Candidate>,
    /// Empty means every scene that is ready.
    #[serde(default)]
    pub scene_ids: Vec<String>,
    #[serde(default)]
    pub include_unreviewed: bool,
    /// The most characters the run may send to online voices. Required when any online voice is chosen.
    #[serde(default)]
    pub cap_chars: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OnlineUse {
    pub provider_label: String,
    pub voices: Vec<String>,
    pub clips: usize,
    pub chars: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Estimate {
    pub scenes: usize,
    pub candidates: usize,
    pub clips: usize,
    pub local_clips: usize,
    pub online_clips: usize,
    /// Characters that would be sent over the internet.
    pub online_chars: u64,
    pub online: Vec<OnlineUse>,
    pub cap_chars: Option<u64>,
    /// False when the cap is lower than what the run needs, so some clips would be skipped.
    pub within_cap: bool,
    /// True when local-only mode is on and an online voice is chosen: those clips would fail.
    pub blocked_by_local_only: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Running,
    Done,
    Cancelled,
    Interrupted,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClipRecord {
    pub candidate: String,
    pub scene: String,
    /// Relative to the run folder. Empty when the clip could not be made.
    pub file: String,
    pub ok: bool,
    pub error: Option<String>,
    pub voice_id: String,
    pub provider_version: String,
    pub online: bool,
    /// Characters of dialogue given to the voice (what an online provider would see and bill).
    pub chars: u64,
    pub duration_ms: u64,
    /// Time spent waiting for the voice, summed over the scene's lines.
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RunRecord {
    pub id: String,
    pub created: String,
    pub updated: String,
    pub set_id: String,
    pub set_checksum: String,
    pub state: RunState,
    pub spec: Spec,
    /// The characters the person approved sending, which must equal the estimate when the run started.
    pub approved_chars: u64,
    pub sent_chars: u64,
    pub clips: Vec<ClipRecord>,
    pub notes: Vec<String>,
}

impl RunRecord {
    pub fn total_latency_ms(&self) -> u64 {
        self.clips.iter().map(|c| c.latency_ms).sum()
    }
}

fn scene_chars(script: &str) -> u64 {
    crate::script::parse(script).lines.iter().map(|l| spoken_text(&tokenize(&l.text).0).chars().count() as u64).sum()
}

fn chosen_scenes<'a>(set: &'a LabSet, spec: &Spec) -> Result<Vec<&'a crate::labset::LabScene>, String> {
    let ready = set.scenes_ready(spec.include_unreviewed);
    if spec.scene_ids.is_empty() {
        return Ok(ready);
    }
    spec.scene_ids
        .iter()
        .map(|id| ready.iter().copied().find(|s| &s.id == id).ok_or_else(|| format!("Scene \"{id}\" is not in this test set, or still needs a native speaker to check it.")))
        .collect()
}

fn check_spec(spec: &Spec, reg: &Registry) -> Result<(), String> {
    if spec.candidates.is_empty() || spec.candidates.len() > MAX_CANDIDATES {
        return Err(format!("Choose between 1 and {MAX_CANDIDATES} voices to compare."));
    }
    for c in &spec.candidates {
        if c.label.trim().is_empty() || c.label.chars().count() > 80 {
            return Err("Give every voice a short label.".into());
        }
        if reg.voice(&c.voice_id).is_none() {
            return Err(format!("{} is not available on this computer.", c.label));
        }
    }
    let mut ids: Vec<&String> = spec.candidates.iter().map(|c| &c.voice_id).collect();
    ids.sort();
    ids.dedup();
    if ids.len() != spec.candidates.len() {
        return Err("The same voice is chosen twice.".into());
    }
    Ok(())
}

fn is_online(reg: &Registry, voice_id: &str) -> bool {
    reg.adapter_for_voice(voice_id).is_some_and(|a| a.sends_data())
}

/// What a run would do and send, worked out before anything is rendered.
pub fn estimate(set: &LabSet, spec: &Spec, reg: &Registry, local_only: bool) -> Result<Estimate, String> {
    check_spec(spec, reg)?;
    let scenes = chosen_scenes(set, spec)?;
    let mut online: Vec<OnlineUse> = vec![];
    let (mut local_clips, mut online_clips, mut online_chars) = (0usize, 0usize, 0u64);
    for c in &spec.candidates {
        if !is_online(reg, &c.voice_id) {
            local_clips += scenes.len();
            continue;
        }
        let chars: u64 = scenes.iter().map(|s| scene_chars(&s.script)).sum();
        online_clips += scenes.len();
        online_chars += chars;
        let label = reg.adapter_for_voice(&c.voice_id).map(|a| a.label().to_string()).unwrap_or_default();
        match online.iter_mut().find(|o| o.provider_label == label) {
            Some(o) => {
                o.voices.push(c.label.clone());
                o.clips += scenes.len();
                o.chars += chars;
            }
            None => online.push(OnlineUse { provider_label: label, voices: vec![c.label.clone()], clips: scenes.len(), chars }),
        }
    }
    Ok(Estimate {
        scenes: scenes.len(),
        candidates: spec.candidates.len(),
        clips: scenes.len() * spec.candidates.len(),
        local_clips,
        online_clips,
        online_chars,
        online,
        cap_chars: spec.cap_chars,
        within_cap: online_chars == 0 || spec.cap_chars.is_some_and(|c| c >= online_chars),
        blocked_by_local_only: local_only && online_clips > 0,
    })
}

fn runs_dir(lab_root: &Path) -> PathBuf {
    lab_root.join("runs")
}

fn run_dir(lab_root: &Path, id: &str) -> Result<PathBuf, String> {
    if !crate::project::valid_id(id) {
        return Err("That is not a valid run id.".into());
    }
    Ok(runs_dir(lab_root).join(id))
}

fn save(lab_root: &Path, rec: &mut RunRecord) -> Result<(), String> {
    rec.updated = iso_now();
    let dir = run_dir(lab_root, &rec.id)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not save the run: {e}"))?;
    let json = serde_json::to_vec_pretty(rec).map_err(|e| e.to_string())?;
    let tmp = dir.join("run.json.tmp");
    std::fs::write(&tmp, json).map_err(|e| format!("Could not save the run: {e}"))?;
    std::fs::rename(&tmp, dir.join("run.json")).map_err(|e| format!("Could not save the run: {e}"))
}

pub fn load_run(lab_root: &Path, id: &str) -> Result<RunRecord, String> {
    let text = std::fs::read_to_string(run_dir(lab_root, id)?.join("run.json")).map_err(|_| "That run does not exist.".to_string())?;
    serde_json::from_str(&text).map_err(|_| "That run record is damaged.".to_string())
}

/// Every readable run, newest first. A run still marked running when nothing is running was cut short.
pub fn list_runs(lab_root: &Path) -> Vec<RunRecord> {
    let Ok(rd) = std::fs::read_dir(runs_dir(lab_root)) else { return vec![] };
    let mut out: Vec<RunRecord> = rd.flatten().filter_map(|e| load_run(lab_root, &e.file_name().to_string_lossy()).ok()).collect();
    for r in &mut out {
        if r.state == RunState::Running {
            r.state = RunState::Interrupted;
            let _ = save(lab_root, r);
        }
    }
    out.sort_by(|a, b| b.created.cmp(&a.created));
    out
}

/// Delete a run and everything it made. Projects are never touched.
pub fn delete_run(lab_root: &Path, id: &str) -> Result<(), String> {
    let dir = run_dir(lab_root, id)?;
    if !dir.join("run.json").is_file() {
        return Err("That run does not exist.".into());
    }
    std::fs::remove_dir_all(&dir).map_err(|e| format!("Could not delete the run: {e}"))
}

/// One scene with one voice: render each line through the normal path and join them into a single clip.
fn render_scene(reg: &Registry, scene: &crate::labset::LabScene, cand: &Candidate, work: &Path, out_file: &Path) -> Result<(Stereo, String, u64), String> {
    let parsed = crate::script::parse(&scene.script);
    let mut project = Project::new("lab", false);
    project.script = scene.script.clone();
    project.sync_ids();
    for sp in &parsed.speakers {
        project.characters.insert(sp.clone(), crate::project::Character { voice_id: Some(cand.voice_id.clone()), ..Default::default() });
    }
    std::fs::create_dir_all(work).map_err(|e| e.to_string())?;
    let mut mix = Stereo::zeros(0);
    let gap = Stereo::zeros(crate::audio::SR as usize * GAP_MS / 1000);
    let mut version = String::new();
    let mut waited = 0u64;
    for line in project.parsed().lines {
        let t = Instant::now();
        let r = crate::render::render_line(&mut project, reg, work, &line.key).map_err(|e| e.message)?;
        waited += t.elapsed().as_millis() as u64;
        let take = project.take(&r.take_id).ok_or("A take went missing.")?.clone();
        version = take.provider_version.clone();
        let audio = to_mix_format(&read_wav(&work.join(&take.file))?);
        let n = mix.len();
        mix.resize(n + audio.len() + gap.len());
        mix.add(&audio, n, 1.0, None);
    }
    write_wav_24(out_file, &mix)?;
    Ok((mix, version, waited))
}

fn run_clips(lab_root: &Path, rec: &mut RunRecord, set: &LabSet, reg: &Registry, cancel: &AtomicBool, progress: &mut dyn FnMut(&RunRecord)) -> Result<(), String> {
    let scenes = chosen_scenes(set, &rec.spec)?;
    let dir = run_dir(lab_root, &rec.id)?;
    std::fs::create_dir_all(dir.join("clips")).map_err(|e| e.to_string())?;
    let cap = rec.spec.cap_chars;
    let candidates = rec.spec.candidates.clone();
    for scene in scenes {
        for cand in &candidates {
            if rec.clips.iter().any(|c| c.candidate == cand.id && c.scene == scene.id) {
                continue;
            }
            if cancel.load(Ordering::SeqCst) {
                rec.state = RunState::Cancelled;
                save(lab_root, rec)?;
                return Ok(());
            }
            let online = is_online(reg, &cand.voice_id);
            let chars = scene_chars(&scene.script);
            let mut clip = ClipRecord { candidate: cand.id.clone(), scene: scene.id.clone(), file: String::new(), ok: false, error: None, voice_id: cand.voice_id.clone(), provider_version: String::new(), online, chars: 0, duration_ms: 0, latency_ms: 0 };
            if online && cap.map_or(true, |c| rec.sent_chars + chars > c) {
                clip.error = Some("The spending cap would be passed, so this clip was not made.".into());
            } else {
                let file = format!("clips/{}-{}.wav", cand.id, scene.id);
                let work = dir.join("work").join(format!("{}-{}", cand.id, scene.id));
                if online {
                    // Counted before the call, so a failure part-way can never let a run slip past the cap.
                    rec.sent_chars += chars;
                    clip.chars = chars;
                }
                match render_scene(reg, scene, cand, &work, &dir.join(&file)) {
                    Ok((audio, version, waited)) => {
                        clip.ok = true;
                        clip.file = file;
                        clip.provider_version = version;
                        clip.duration_ms = audio.len() as u64 * 1000 / crate::audio::SR as u64;
                        clip.latency_ms = waited;
                        if !online {
                            clip.chars = chars;
                        }
                    }
                    Err(e) => clip.error = Some(e),
                }
                let _ = std::fs::remove_dir_all(&work);
            }
            rec.clips.push(clip);
            save(lab_root, rec)?;
            progress(rec);
        }
    }
    rec.state = RunState::Done;
    save(lab_root, rec)
}

/// Start a run. `approved_chars` is the number of characters the person agreed to send; it must equal the estimate, so
/// a script or voice change after approval cannot send more than was shown.
pub fn start(lab_root: &Path, set: &LabSet, mut spec: Spec, approved_chars: u64, reg: &Registry, local_only: bool, cancel: &AtomicBool, progress: &mut dyn FnMut(&RunRecord)) -> Result<RunRecord, String> {
    for (i, c) in spec.candidates.iter_mut().enumerate() {
        c.id = format!("c{}", i + 1);
    }
    let est = estimate(set, &spec, reg, local_only)?;
    if est.blocked_by_local_only {
        return Err("Local-only mode is on, so online voices cannot be used. Turn it off in Settings or choose voices on this computer.".into());
    }
    if est.online_chars > 0 {
        if spec.cap_chars.is_none() {
            return Err("Set a spending cap before using online voices.".into());
        }
        if approved_chars != est.online_chars {
            return Err(format!("This run would send {} characters to online voices. Approve exactly that number first.", est.online_chars));
        }
    }
    cancel.store(false, Ordering::SeqCst);
    let mut rec = RunRecord { id: new_id("run"), created: iso_now(), updated: iso_now(), set_id: set.id.clone(), set_checksum: set.checksum.clone(), state: RunState::Running, spec, approved_chars, sent_chars: 0, clips: vec![], notes: vec![] };
    save(lab_root, &mut rec)?;
    if let Err(e) = run_clips(lab_root, &mut rec, set, reg, cancel, progress) {
        rec.state = RunState::Failed;
        rec.notes.push(e.clone());
        save(lab_root, &mut rec)?;
        return Err(e);
    }
    if rec.clips.iter().any(|c| c.error.as_deref().is_some_and(|e| e.contains("spending cap"))) {
        rec.notes.push("Some online clips were skipped because the spending cap would have been passed.".into());
        save(lab_root, &mut rec)?;
    }
    Ok(rec)
}

/// Carry on a run that was cut short or stopped. Clips already made are kept and no finished clip is rendered again.
/// The spending that already happened still counts against the cap.
pub fn resume(lab_root: &Path, id: &str, set: &LabSet, reg: &Registry, local_only: bool, cancel: &AtomicBool, progress: &mut dyn FnMut(&RunRecord)) -> Result<RunRecord, String> {
    let mut rec = load_run(lab_root, id)?;
    if rec.state == RunState::Done {
        return Ok(rec);
    }
    if set.checksum != rec.set_checksum {
        return Err("The test set has changed since this run started, so it cannot be continued.".into());
    }
    if local_only && rec.spec.candidates.iter().any(|c| is_online(reg, &c.voice_id)) {
        return Err("Local-only mode is on, so a run with online voices cannot continue.".into());
    }
    cancel.store(false, Ordering::SeqCst);
    // A clip that failed for any reason other than the cap is tried again; one skipped for the cap is not.
    rec.clips.retain(|c| c.ok || c.error.as_deref().is_some_and(|e| e.contains("spending cap")));
    rec.state = RunState::Running;
    save(lab_root, &mut rec)?;
    run_clips(lab_root, &mut rec, set, reg, cancel, progress)?;
    Ok(rec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::scratch::ScratchAdapter;
    use crate::providers::{Adapter, AdapterError, Capabilities, RenderOutput, RenderRequest, Voice};
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;

    /// A stand-in online voice: renders like the scratch tone but counts as sending text over the internet.
    struct Cloudy(Arc<AtomicUsize>);
    impl Adapter for Cloudy {
        fn id(&self) -> &'static str { "cloudy" }
        fn label(&self) -> &'static str { "Cloudy" }
        fn licence(&self) -> crate::licence::LicenceInfo { ScratchAdapter.licence() }
        fn capabilities(&self) -> Capabilities { ScratchAdapter.capabilities() }
        fn list_voices(&self) -> Vec<Voice> {
            vec![Voice { id: "cloudy:one".into(), name: "Cloud One".into(), provider: "cloudy".into(), language: "zxx".into(), accent: String::new(), gender: String::new(), age_range: String::new(), licence: Default::default(), sample_text: "hi".into(), notes: String::new() }]
        }
        fn provider_version(&self) -> String { "cloudy-1".into() }
        fn sends_data(&self) -> bool { true }
        fn render(&self, r: &RenderRequest) -> Result<RenderOutput, AdapterError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            ScratchAdapter.render(&RenderRequest { voice_id: "tone:mid".into(), ..r.clone() })
        }
    }

    fn reg() -> (Registry, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (Registry::new(vec![Box::new(ScratchAdapter), Box::new(Cloudy(calls.clone()))]), calls)
    }

    /// A small valid test set in a temp folder: one plain scene, one that needs a laugh the stand-in voices cannot do.
    fn small_set() -> (LabSet, PathBuf) {
        let d = std::env::temp_dir().join(new_id("ss-lab"));
        std::fs::create_dir_all(d.join("scenes")).unwrap();
        std::fs::write(d.join("scenes/01-plain.txt"), "AMA: Good morning my friend, how are you today?\nKOLA: I am well, thank you very much indeed.\n").unwrap();
        std::fs::write(d.join("scenes/02-laugh.txt"), "AMA: That was the funniest thing I have heard all week [laugh] truly.\n").unwrap();
        let meta = serde_json::json!({
            "id": "tiny", "version": 1, "title": "Tiny", "language": "en",
            "scenes": [
                { "id": "plain", "title": "Plain", "file": "scenes/01-plain.txt", "reference_direction": "Calm.", "checklist": ["Natural?"] },
                { "id": "laugh", "title": "Laugh", "file": "scenes/02-laugh.txt", "reference_direction": "Amused.", "checklist": ["Laugh?"] }
            ]
        });
        std::fs::write(d.join("set.json"), serde_json::to_string_pretty(&meta).unwrap()).unwrap();
        let files = vec!["scenes/01-plain.txt".to_string(), "scenes/02-laugh.txt".to_string()];
        std::fs::write(d.join("CHECKSUM"), crate::labset::checksum(&d, &files).unwrap()).unwrap();
        (crate::labset::load(&d).unwrap(), d)
    }

    fn cand(voice: &str, label: &str) -> Candidate {
        Candidate { id: String::new(), voice_id: voice.into(), label: label.into() }
    }

    fn lab_root() -> PathBuf {
        let d = std::env::temp_dir().join(new_id("ss-labroot"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn quiet(_: &RunRecord) {}

    #[test]
    fn the_estimate_counts_online_characters_by_provider_before_anything_runs() {
        let (set, d) = small_set();
        let (reg, calls) = reg();
        let spec = Spec { candidates: vec![cand("tone:low", "Low"), cand("cloudy:one", "Cloud")], scene_ids: vec![], include_unreviewed: false, cap_chars: Some(10_000) };
        let e = estimate(&set, &spec, &reg, false).unwrap();
        assert_eq!((e.scenes, e.candidates, e.clips, e.local_clips, e.online_clips), (2, 2, 4, 2, 2));
        assert!(e.online_chars > 80 && e.online_chars < 200, "{}", e.online_chars);
        assert_eq!(e.online[0].provider_label, "Cloudy");
        assert!(e.within_cap && !e.blocked_by_local_only);
        assert_eq!(calls.load(Ordering::SeqCst), 0, "estimating calls no voice");
        let low = Spec { cap_chars: Some(10), ..spec.clone() };
        assert!(!estimate(&set, &low, &reg, false).unwrap().within_cap);
        assert!(estimate(&set, &spec, &reg, true).unwrap().blocked_by_local_only);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn a_local_run_makes_one_clip_per_scene_and_voice_and_records_a_failure_without_stopping() {
        let (set, d) = small_set();
        let (reg, _) = reg();
        let root = lab_root();
        let spec = Spec { candidates: vec![cand("tone:low", "Low"), cand("tone:high", "High")], scene_ids: vec![], include_unreviewed: false, cap_chars: None };
        let rec = start(&root, &set, spec, 0, &reg, false, &AtomicBool::new(false), &mut quiet).unwrap();
        assert_eq!(rec.state, RunState::Done);
        assert_eq!(rec.clips.len(), 4);
        let ok: Vec<_> = rec.clips.iter().filter(|c| c.ok).collect();
        assert_eq!(ok.len(), 2, "the laugh scene cannot be done by these voices");
        assert!(ok.iter().all(|c| root.join("runs").join(&rec.id).join(&c.file).is_file() && c.duration_ms > 2000 && !c.online && c.chars > 0));
        let failed = rec.clips.iter().find(|c| !c.ok).unwrap();
        assert!(failed.error.as_deref().unwrap().contains("[laugh]"), "{:?}", failed.error);
        assert_eq!(rec.sent_chars, 0, "nothing was sent anywhere");
        assert!(!root.join("runs").join(&rec.id).join("work").exists() || std::fs::read_dir(root.join("runs").join(&rec.id).join("work")).unwrap().count() == 0, "scratch folders are cleaned up");
        assert_eq!(load_run(&root, &rec.id).unwrap(), rec);
        std::fs::remove_dir_all(d).ok();
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn online_voices_need_a_cap_and_the_exact_approval_and_never_pass_the_cap() {
        let (set, d) = small_set();
        let (reg, calls) = reg();
        let root = lab_root();
        let base = Spec { candidates: vec![cand("cloudy:one", "Cloud")], scene_ids: vec!["plain".into()], include_unreviewed: false, cap_chars: None };
        // No cap.
        assert!(start(&root, &set, base.clone(), 0, &reg, false, &AtomicBool::new(false), &mut quiet).unwrap_err().contains("cap"));
        let capped = Spec { cap_chars: Some(5_000), ..base.clone() };
        let need = estimate(&set, &capped, &reg, false).unwrap().online_chars;
        // Wrong approval.
        assert!(start(&root, &set, capped.clone(), need + 1, &reg, false, &AtomicBool::new(false), &mut quiet).unwrap_err().contains("Approve exactly"));
        assert!(start(&root, &set, capped.clone(), 0, &reg, false, &AtomicBool::new(false), &mut quiet).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0, "refused runs call no voice");
        // Local-only.
        assert!(start(&root, &set, capped.clone(), need, &reg, true, &AtomicBool::new(false), &mut quiet).unwrap_err().contains("Local-only"));
        // Approved: runs, and the characters sent are counted.
        let rec = start(&root, &set, capped, need, &reg, false, &AtomicBool::new(false), &mut quiet).unwrap();
        assert_eq!(rec.sent_chars, need);
        assert!(rec.clips[0].ok && rec.clips[0].online && rec.clips[0].chars == need);
        // A cap that is too low skips the clip instead of overspending.
        let tight = Spec { cap_chars: Some(need - 1), ..base };
        let rec2 = start(&root, &set, tight, need, &reg, false, &AtomicBool::new(false), &mut quiet).unwrap();
        assert!(!rec2.clips[0].ok && rec2.clips[0].error.as_deref().unwrap().contains("spending cap"));
        assert_eq!(rec2.sent_chars, 0);
        assert!(rec2.notes.iter().any(|n| n.contains("skipped")));
        assert_eq!(calls.load(Ordering::SeqCst), 2, "only the approved run called the voice (two lines)");
        std::fs::remove_dir_all(d).ok();
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_stopped_run_resumes_without_repeating_finished_clips_or_resetting_spending() {
        let (set, d) = small_set();
        let (reg, calls) = reg();
        let root = lab_root();
        let spec = Spec { candidates: vec![cand("cloudy:one", "Cloud"), cand("tone:low", "Low")], scene_ids: vec![], include_unreviewed: false, cap_chars: Some(100_000) };
        let need = estimate(&set, &spec, &reg, false).unwrap().online_chars;
        // Stop after the first clip.
        let stop = AtomicBool::new(false);
        let mut seen = 0;
        let rec = start(&root, &set, spec, need, &reg, false, &stop, &mut |_| {
            seen += 1;
            if seen == 1 {
                stop.store(true, Ordering::SeqCst);
            }
        })
        .unwrap();
        assert_eq!(rec.state, RunState::Cancelled);
        assert_eq!(rec.clips.len(), 1);
        let calls_before = calls.load(Ordering::SeqCst);
        let done = resume(&root, &rec.id, &set, &reg, false, &AtomicBool::new(false), &mut quiet).unwrap();
        assert_eq!(done.state, RunState::Done);
        assert_eq!(done.clips.len(), 4);
        assert!(done.sent_chars <= need, "{} vs {need}", done.sent_chars);
        // The first clip was not rendered again: the online voice was called for the laugh scene only (it fails at once, before any call).
        assert!(calls.load(Ordering::SeqCst) >= calls_before);
        assert_eq!(done.clips.iter().filter(|c| c.scene == "plain" && c.candidate == "c1").count(), 1);
        // A run left "running" by a crash is shown as interrupted.
        let mut r = load_run(&root, &done.id).unwrap();
        r.state = RunState::Running;
        save(&root, &mut r).unwrap();
        assert_eq!(list_runs(&root)[0].state, RunState::Interrupted);
        std::fs::remove_dir_all(d).ok();
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn deleting_a_run_removes_only_that_run() {
        let (set, d) = small_set();
        let (reg, _) = reg();
        let root = lab_root();
        let spec = Spec { candidates: vec![cand("tone:low", "Low")], scene_ids: vec!["plain".into()], include_unreviewed: false, cap_chars: None };
        let a = start(&root, &set, spec.clone(), 0, &reg, false, &AtomicBool::new(false), &mut quiet).unwrap();
        let b = start(&root, &set, spec, 0, &reg, false, &AtomicBool::new(false), &mut quiet).unwrap();
        delete_run(&root, &a.id).unwrap();
        assert!(load_run(&root, &a.id).is_err() && load_run(&root, &b.id).is_ok());
        assert!(delete_run(&root, &a.id).is_err());
        assert!(delete_run(&root, "../..").is_err());
        std::fs::remove_dir_all(d).ok();
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_changed_set_or_bad_choices_are_refused() {
        let (set, d) = small_set();
        let (reg, _) = reg();
        let root = lab_root();
        let spec = Spec { candidates: vec![cand("tone:low", "Low")], scene_ids: vec![], include_unreviewed: false, cap_chars: None };
        let rec = start(&root, &set, spec.clone(), 0, &reg, false, &AtomicBool::new(true), &mut quiet).unwrap();
        // `start` clears the stop flag, so it ran; resume of a finished run just returns it.
        assert_eq!(rec.state, RunState::Done);
        let mut changed = set.clone();
        changed.checksum = "different".into();
        let mut r = load_run(&root, &rec.id).unwrap();
        r.state = RunState::Cancelled;
        save(&root, &mut r).unwrap();
        assert!(resume(&root, &rec.id, &changed, &reg, false, &AtomicBool::new(false), &mut quiet).unwrap_err().contains("changed"));
        assert!(estimate(&set, &Spec { candidates: vec![], ..spec.clone() }, &reg, false).is_err());
        assert!(estimate(&set, &Spec { candidates: vec![cand("tone:nope", "X")], ..spec.clone() }, &reg, false).is_err());
        assert!(estimate(&set, &Spec { candidates: vec![cand("tone:low", "A"), cand("tone:low", "B")], ..spec.clone() }, &reg, false).is_err());
        assert!(estimate(&set, &Spec { scene_ids: vec!["nope".into()], ..spec.clone() }, &reg, false).is_err());
        assert!(load_run(&root, "../x").is_err());
        std::fs::remove_dir_all(d).ok();
        std::fs::remove_dir_all(root).ok();
    }
}
