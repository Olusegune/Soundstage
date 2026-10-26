//! Licence-gated export (FR-7, FR-13): master plus dialogue, effects and music stems of
//! identical length, and a provenance manifest recording every decision.

use crate::audio::{read_wav, resample, to_mix_format, Stereo, SR};
use crate::output::{subtitle_cues, to_srt, to_vtt, write_audio, ExportOptions, Ffmpeg, SubtitleFormat};
use crate::layout::{compute, Layout};
use crate::licence::{gate, GateReport, Licence, LicenceInfo, LicenceTarget, Subject};
use crate::loudness::{destination, Loudness};
use crate::mix::{mix, MixParams, Placed};
use crate::project::{safe_relative, AssetKind, Project, TakeStatus};
use crate::providers::Registry;

use crate::util::{iso_now, safe_file_stem, sha256_hex, today_days};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The user's recorded check wins, but conditions the provider declares can never be dropped by it.
fn effective(project: &Project, key: &str, base: LicenceInfo) -> LicenceInfo {
    let Some(mut mine) = project.licence_checks.get(key).cloned() else { return base };
    for c in base.conditions {
        if !mine.conditions.iter().any(|m| m.kind == c.kind && m.text == c.text) {
            mine.conditions.push(c);
        }
    }
    mine
}

/// Everything the export gate must judge: voices, models, music and effects actually used.
pub fn subjects(project: &Project, reg: &Registry) -> Vec<Subject> {
    let parsed = project.parsed();
    let layout = compute(project, &parsed);
    let mut out = Vec::new();
    for slot in &layout.lines {
        let Some(take) = project.take(&slot.take_id) else { continue };
        let voice = reg.voice(&take.voice_id);
        let vinfo = voice.map(|v| v.licence.clone()).unwrap_or_default();
        out.push(Subject {
            kind: "voice".into(),
            id: take.voice_id.clone(),
            name: voice.map_or_else(|| format!("{} (not installed)", take.voice_id), |v| v.name.clone()),
            info: effective(project, &format!("voice:{}", take.voice_id), vinfo),
            uses_prohibited: vec![],
        });
        let (minfo, mname) = reg
            .adapter(&take.adapter_id)
            .map(|a| (a.licence(), a.label().to_string()))
            .unwrap_or_else(|| (LicenceInfo::default(), take.adapter_id.clone()));
        out.push(Subject {
            kind: "model".into(),
            id: take.adapter_id.clone(),
            name: mname,
            info: effective(project, &format!("model:{}", take.adapter_id), minfo),
            uses_prohibited: vec![],
        });
    }
    for cue in &project.cues {
        if let Some(a) = project.asset(&cue.asset_id) {
            out.push(Subject {
                kind: if a.kind == AssetKind::Music { "music".into() } else { "effect".into() },
                id: a.id.clone(),
                name: a.name.clone(),
                info: effective(project, &format!("asset:{}", a.id), a.licence.clone()),
                uses_prohibited: a.uses_prohibited.clone(),
            });
        }
    }
    out
}

/// Consent record ids of every cloned voice that has a take in the mix.
pub fn consent_ids_used(project: &Project, reg: &Registry) -> Vec<String> {
    let layout = compute(project, &project.parsed());
    let mut ids: Vec<String> = layout.lines.iter().filter_map(|s| project.take(&s.take_id)).filter_map(|t| reg.consents.get(&t.voice_id)).map(|c| c.id.clone()).collect();
    ids.sort();
    ids.dedup();
    ids
}

pub fn check(project: &Project, reg: &Registry) -> GateReport {
    let mut r = gate(project.licence_target, &subjects(project, reg), &project.mix.destination, today_days());
    let commercial = project.licence_target == LicenceTarget::Commercial;
    let mut seen: Vec<String> = vec![];
    for slot in &compute(project, &project.parsed()).lines {
        let Some(take) = project.take(&slot.take_id) else { continue };
        if seen.contains(&take.voice_id) {
            continue;
        }
        seen.push(take.voice_id.clone());
        if let Some(problems) = reg.consent_problems(&take.voice_id, commercial) {
            r.blocking.push(crate::licence::GateItem {
                kind: "consent".into(),
                id: take.voice_id.clone(),
                name: reg.voice(&take.voice_id).map_or_else(|| take.voice_id.clone(), |v| v.name.clone()),
                reason: problems.join(" "),
                severity: crate::licence::Severity::Blocking,
                source: None,
                conditions: vec![],
            });
            r.allowed = false;
        }
    }
    let parsed = project.parsed();
    let layout = compute(project, &parsed);
    let unplaced = parsed.lines.len().saturating_sub(layout.lines.len());
    if unplaced > 0 && !layout.lines.is_empty() {
        r.notices.push(crate::licence::GateItem {
            kind: "info".into(),
            id: "unplaced".into(),
            name: format!("{unplaced} line(s) without audio"),
            reason: "Lines that are not rendered, or whose take is rejected, are left out of the mix.".into(),
            severity: crate::licence::Severity::Notice,
            source: None,
            conditions: vec![],
        });
    }
    r
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExportError {
    Blocked { report: GateReport },
    NeedsAcknowledgement { report: GateReport },
    Failed { message: String },
}

fn failed(m: impl Into<String>) -> ExportError {
    ExportError::Failed { message: m.into() }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportResult {
    pub folder: String,
    pub files: Vec<String>,
    pub loudness: Loudness,
    pub target_lufs: f64,
    pub gain_db: f64,
    pub target_reached: bool,
    pub warnings: Vec<String>,
}

fn load_take(project_dir: &Path, rel: &str) -> Result<Stereo, ExportError> {
    if !safe_relative(rel) {
        return Err(failed("A take has an unsafe file path."));
    }
    let d = read_wav(&project_dir.join(rel)).map_err(|e| failed(format!("Could not load {rel}: {e}")))?;
    Ok(Stereo::from_mono(&resample(&d.channels[0], d.rate, SR)))
}

/// No cue, gap or loop may reach past an hour, so a damaged or hostile project cannot ask for a huge buffer.
pub const MAX_TIMELINE_MS: u64 = 3_600_000;

fn ms_to_samples(ms: u64) -> usize {
    (ms.min(MAX_TIMELINE_MS) * SR as u64 / 1000) as usize
}

pub struct Built {
    pub dialogue: Vec<Placed>,
    pub effects: Vec<Placed>,
    pub music: Vec<Placed>,
    pub layout: Layout,
}

pub fn build(project: &Project, project_dir: &Path) -> Result<Built, ExportError> {
    let parsed = project.parsed();
    let layout = compute(project, &parsed);
    let mut dialogue = Vec::new();
    for slot in &layout.lines {
        let take = project.take(&slot.take_id).ok_or_else(|| failed("A selected take is missing from the project."))?;
        dialogue.push(Placed { start: ms_to_samples(slot.start_ms), buf: load_take(project_dir, &take.file)?, gain_db: 0.0, fade_in: 48, fade_out: 96, duck: false });
    }
    let (mut effects, mut music) = (Vec::new(), Vec::new());
    for cue in &project.cues {
        let asset = project.asset(&cue.asset_id).ok_or_else(|| failed("A cue refers to an asset that is not in the project."))?;
        if !safe_relative(&asset.file) {
            return Err(failed("An asset has an unsafe file path."));
        }
        let mut buf = to_mix_format(&read_wav(&project_dir.join(&asset.file)).map_err(|e| failed(format!("Could not load {}: {e}", asset.name)))?);
        let scene = cue.scene.and_then(|i| layout.scenes.iter().find(|s| s.index == i));
        let scene_start = scene.map_or(0, |s| s.start_ms);
        if cue.loop_to_scene {
            if let Some(s) = scene {
                let want = ms_to_samples(s.end_ms.saturating_sub(s.start_ms)).saturating_sub(ms_to_samples(cue.offset_ms as u64));
                if buf.is_empty() {
                    return Err(failed(format!("{} is empty.", asset.name)));
                }
                let mut looped = Stereo::default();
                while looped.len() < want {
                    looped.l.extend(&buf.l);
                    looped.r.extend(&buf.r);
                }
                looped.resize(want);
                buf = looped;
            }
        }
        let placed = Placed {
            start: ms_to_samples(scene_start.saturating_add(cue.offset_ms as u64)),
            buf,
            gain_db: cue.gain_db.clamp(-60.0, 12.0),
            fade_in: ms_to_samples(cue.fade_in_ms as u64),
            fade_out: ms_to_samples(cue.fade_out_ms as u64),
            duck: cue.duck,
        };
        if asset.kind == AssetKind::Music { music.push(placed) } else { effects.push(placed) }
    }
    Ok(Built { dialogue, effects, music, layout })
}


/// Build and mix the project without writing anything. Export and the Mix screen's preview share this.
pub fn render_mix(project: &Project, project_dir: &Path) -> Result<(crate::mix::MixOutput, &'static crate::loudness::Destination, Layout), ExportError> {
    let built = build(project, project_dir)?;
    if built.dialogue.is_empty() {
        return Err(failed("There is no rendered dialogue to mix yet. Render at least one line."));
    }
    let dest_info = destination(&project.mix.destination).ok_or_else(|| failed("Unknown loudness destination."))?;
    let params = MixParams {
        duck_depth_db: project.mix.duck_depth_db.clamp(0.0, 40.0),
        attack_ms: project.mix.attack_ms,
        release_ms: project.mix.release_ms,
        target_lufs: dest_info.target_lufs,
        true_peak_ceiling_dbtp: dest_info.true_peak_ceiling_dbtp,
    };
    let out = mix(&built.dialogue, &built.effects, &built.music, &params, SR as usize / 2);
    Ok((out, dest_info, built.layout))
}

/// What FFmpeg-backed formats this computer can write, for the Export screen.
pub fn ffmpeg_status(configured: Option<&str>) -> Option<Ffmpeg> {
    Ffmpeg::detect(configured)
}

pub fn export(project: &Project, reg: &Registry, project_dir: &Path, dest: &Path, acknowledged: bool) -> Result<ExportResult, ExportError> {
    export_with(project, reg, project_dir, dest, acknowledged, &ExportOptions::default())
}

pub fn export_with(project: &Project, reg: &Registry, project_dir: &Path, dest: &Path, acknowledged: bool, opts: &ExportOptions) -> Result<ExportResult, ExportError> {
    let report = check(project, reg);
    if !report.allowed {
        return Err(ExportError::Blocked { report });
    }
    let personal_notices = project.licence_target == LicenceTarget::Personal
        && report.notices.iter().any(|n| n.severity == crate::licence::Severity::Notice && n.kind != "info");
    if personal_notices && !acknowledged {
        return Err(ExportError::NeedsAcknowledgement { report });
    }
    if !dest.is_dir() {
        return Err(failed("Choose a folder that exists to export into."));
    }
    // Everything that can be refused is refused before anything is written.
    opts.validate().map_err(failed)?;
    let needs_ff = opts.master.needs_ffmpeg() || opts.stems.as_ref().is_some_and(|c| c.needs_ffmpeg());
    let ff = if needs_ff {
        let ff = Ffmpeg::detect(opts.ffmpeg_path.as_deref()).ok_or_else(|| {
            failed(if opts.ffmpeg_path.as_deref().is_some_and(|p| !p.trim().is_empty()) {
                "The FFmpeg you chose could not be started. Choose ffmpeg.exe again, or pick WAV."
            } else {
                "FFmpeg is needed for this format and was not found. Install it (it is free), or pick WAV."
            })
        })?;
        for c in std::iter::once(&opts.master).chain(opts.stems.iter()) {
            if !ff.supports(c) {
                return Err(failed(format!("This copy of FFmpeg cannot write {}. Use a full build of FFmpeg, or pick another format.", c.label())));
            }
        }
        Some(ff)
    } else {
        None
    };
    let (out, dest_info, layout) = render_mix(project, project_dir)?;

    let stamp = iso_now().replace([':', '-'], "").replace('T', "-").replace('Z', "");
    // A fresh folder every time, even for two exports in the same second. Nothing is ever overwritten.
    let base = format!("{} export {}", safe_file_stem(&project.name), stamp);
    let mut folder = dest.join(&base);
    let mut n = 1;
    while folder.exists() {
        n += 1;
        folder = dest.join(format!("{base} ({n})"));
    }
    std::fs::create_dir(&folder).map_err(|e| failed(format!("Could not create the export folder: {e}")))?;
    let stem = safe_file_stem(&project.name);
    let tmp = std::env::temp_dir().join(crate::util::new_id("soundstage-export"));
    std::fs::create_dir_all(&tmp).map_err(|e| failed(e.to_string()))?;
    let result = write_everything(project, project_dir, &folder, &stem, &tmp, &out, &layout, dest_info, &report, opts, ff.as_ref(), &consent_ids_used(project, reg));
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

#[allow(clippy::too_many_arguments)]
/// Copy each clip into `media` and write the OTIO file that places them. Returns the files written (relative to the export
/// folder) and a manifest entry for each.
fn write_timeline(project: &Project, project_dir: &Path, folder: &Path, stem: &str, layout: &Layout) -> Result<(Vec<String>, Vec<serde_json::Value>), String> {
    use crate::timeline::{assign_lanes, to_otio, Clip};
    let media_dir_name = format!("{stem} - media");
    let media_dir = folder.join(&media_dir_name);
    std::fs::create_dir_all(&media_dir).map_err(|e| format!("Could not create the media folder: {e}"))?;
    let (mut files, mut entries) = (Vec::new(), Vec::new());
    let copy = |src: &Path, name: String, files: &mut Vec<String>, entries: &mut Vec<serde_json::Value>| -> Result<String, String> {
        let bytes = std::fs::read(src).map_err(|e| format!("Could not read {}: {e}", src.display()))?;
        std::fs::write(media_dir.join(&name), &bytes).map_err(|e| format!("Could not write {name}: {e}"))?;
        let rel = format!("{media_dir_name}/{name}");
        entries.push(serde_json::json!({ "file": rel, "format": "WAV clip", "sha256": sha256_hex(&bytes), "bytes": bytes.len() }));
        files.push(rel.clone());
        Ok(rel)
    };

    let mut dialogue = Vec::new();
    for (n, slot) in layout.lines.iter().enumerate() {
        let take = project.take(&slot.take_id).ok_or("A selected take is missing from the project.")?;
        if !safe_relative(&take.file) {
            return Err("A take has an unsafe file path.".into());
        }
        let name = format!("{:03} {} {}.wav", n + 1, safe_file_stem(&slot.speaker), safe_file_stem(&slot.key));
        let rel = copy(&project_dir.join(&take.file), name, &mut files, &mut entries)?;
        let dur = slot.end_ms.saturating_sub(slot.start_ms);
        dialogue.push(Clip { name: format!("{}: {}", slot.speaker, slot.key), media: rel, start_ms: slot.start_ms, dur_ms: dur, media_dur_ms: take.duration_ms.max(dur), gain_db: 0.0, fade_in_ms: 1, fade_out_ms: 2 });
    }

    let (mut effects, mut music) = (Vec::new(), Vec::new());
    let mut copied: std::collections::BTreeMap<String, String> = Default::default();
    for cue in &project.cues {
        let Some(asset) = project.asset(&cue.asset_id) else { continue };
        if !safe_relative(&asset.file) {
            return Err("An asset has an unsafe file path.".into());
        }
        let rel = match copied.get(&asset.id) {
            Some(r) => r.clone(),
            None => {
                let r = copy(&project_dir.join(&asset.file), format!("{} {}.wav", if asset.kind == AssetKind::Music { "music" } else { "effect" }, safe_file_stem(&asset.name)), &mut files, &mut entries)?;
                copied.insert(asset.id.clone(), r.clone());
                r
            }
        };
        let scene = cue.scene.and_then(|i| layout.scenes.iter().find(|s| s.index == i));
        let start = scene.map_or(0, |s| s.start_ms) + cue.offset_ms as u64;
        let dur = asset.duration_ms.max(1);
        // A looped sound is written as repeated clips, so an editor sees exactly what the mix used.
        let want = if cue.loop_to_scene { scene.map_or(dur, |s| s.end_ms.saturating_sub(s.start_ms).saturating_sub(cue.offset_ms as u64)) } else { dur };
        let mut placed = 0u64;
        let mut repeats = 0;
        while placed < want.max(1) && repeats < 500 {
            let len = (want - placed).min(dur);
            let c = Clip { name: asset.name.clone(), media: rel.clone(), start_ms: start + placed, dur_ms: len, media_dur_ms: dur, gain_db: cue.gain_db, fade_in_ms: if repeats == 0 { cue.fade_in_ms } else { 0 }, fade_out_ms: if placed + len >= want { cue.fade_out_ms } else { 0 } };
            if asset.kind == AssetKind::Music { music.push(c) } else { effects.push(c) }
            placed += len;
            repeats += 1;
        }
    }

    let mut tracks = assign_lanes("Dialogue", dialogue);
    tracks.extend(assign_lanes("Effects", effects));
    tracks.extend(assign_lanes("Music", music));
    let otio = serde_json::to_string_pretty(&to_otio(&project.name, APP_VERSION, &tracks)).map_err(|e| e.to_string())?;
    let file = format!("{stem} - timeline.otio");
    std::fs::write(folder.join(&file), otio.as_bytes()).map_err(|e| format!("Could not write the timeline: {e}"))?;
    entries.push(serde_json::json!({ "file": file, "format": "OpenTimelineIO", "sha256": sha256_hex(otio.as_bytes()), "bytes": otio.len() }));
    files.push(file);
    Ok((files, entries))
}

fn write_everything(
    project: &Project, project_dir: &Path, folder: &Path, stem: &str, tmp: &Path, out: &crate::mix::MixOutput, layout: &Layout,
    dest_info: &crate::loudness::Destination, report: &GateReport, opts: &ExportOptions, ff: Option<&Ffmpeg>, consents: &[String],
) -> Result<ExportResult, ExportError> {
    let mut jobs: Vec<(String, &Stereo, &crate::output::Codec)> = vec![(format!("{stem} - master"), &out.master, &opts.master)];
    if let Some(sc) = &opts.stems {
        jobs.push((format!("{stem} - stem dialogue"), &out.dialogue, sc));
        jobs.push((format!("{stem} - stem effects"), &out.effects, sc));
        jobs.push((format!("{stem} - stem music"), &out.music, sc));
    }
    let mut names: Vec<String> = Vec::new();
    let mut hashes = Vec::new();
    for (name, buf, codec) in &jobs {
        let path = write_audio(buf, &folder.join(name), codec, opts.sample_rate, ff, &project.name, tmp).map_err(failed)?;
        let bytes = std::fs::read(&path).map_err(|e| failed(e.to_string()))?;
        let file = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        hashes.push(serde_json::json!({ "file": file, "format": codec.label(), "sha256": sha256_hex(&bytes), "samples": buf.len(), "sample_rate": opts.sample_rate, "bytes": bytes.len() }));
        names.push(file);
    }

    if !opts.subtitles.is_empty() {
        let parsed = project.parsed();
        let cues = subtitle_cues(project, &parsed, layout, opts.speaker_labels);
        let mut seen: Vec<SubtitleFormat> = vec![];
        for f in &opts.subtitles {
            if seen.contains(f) {
                continue;
            }
            seen.push(*f);
            let (ext, text) = match f { SubtitleFormat::Srt => ("srt", to_srt(&cues)), SubtitleFormat::Vtt => ("vtt", to_vtt(&cues)) };
            let file = format!("{stem}.{ext}");
            std::fs::write(folder.join(&file), text.as_bytes()).map_err(|e| failed(e.to_string()))?;
            hashes.push(serde_json::json!({ "file": file, "format": format!("{} subtitles", ext.to_uppercase()), "sha256": sha256_hex(text.as_bytes()), "cues": cues.len() }));
            names.push(file);
        }
    }

    if opts.timeline {
        let (files, entries) = write_timeline(project, project_dir, folder, stem, layout).map_err(failed)?;
        names.extend(files);
        hashes.extend(entries);
    }

    let lines: Vec<_> = layout
        .lines
        .iter()
        .filter_map(|s| project.take(&s.take_id).map(|t| (s, t)))
        .map(|(s, t)| {
            let bytes = std::fs::read(project_dir.join(&t.file)).unwrap_or_default();
            serde_json::json!({
                "line": s.key, "speaker": s.speaker, "take": t.id, "voice_id": t.voice_id, "adapter": t.adapter_id,
                "provider_version": t.provider_version, "direction": t.direction_summary, "start_ms": s.start_ms, "end_ms": s.end_ms,
                "approval": t.status, "approver": t.approver, "audio_sha256": sha256_hex(&bytes),
                "ai_generated": true
            })
        })
        .collect();
    let manifest = serde_json::json!({
        "project": { "id": project.id, "name": project.name, "rating": project.rating, "licence_target": project.licence_target },
        "exported_at": iso_now(),
        "app": { "name": "Soundstage", "version": APP_VERSION },
        "disclosure": "Dialogue in this export was generated by AI voices.",
        "output": { "master": opts.master.label(), "stems": opts.stems.as_ref().map(|c| c.label()), "sample_rate": opts.sample_rate,
                    "subtitles": opts.subtitles, "encoder": ff.map(|f| f.version.clone()) },
        "loudness": { "destination": dest_info.id, "target_lufs": dest_info.target_lufs, "true_peak_ceiling_dbtp": dest_info.true_peak_ceiling_dbtp,
                      "measured": out.loudness, "gain_applied_db": out.gain_db, "target_reached": out.target_reached, "warnings": out.warnings,
                      "note": "Measured on the mix before encoding. Lossy formats can shift true peak slightly." },
        "licence_gate": { "decision": "allowed", "report": report },
        "licence_checks_recorded_by_user": project.licence_checks,
        "lines": lines,
        "cues": project.cues.iter().filter_map(|c| project.asset(&c.asset_id).map(|a| serde_json::json!({
            "cue": c.id, "asset": a.id, "name": a.name, "kind": a.kind, "licence": a.licence, "scene": c.scene, "offset_ms": c.offset_ms, "gain_db": c.gain_db, "ducked": c.duck,
            "ai_generated": a.generated.is_some(), "generated": a.generated }))).collect::<Vec<_>>(),
        "rejected_takes_excluded": project.takes.iter().filter(|t| t.status == TakeStatus::Rejected).map(|t| t.id.clone()).collect::<Vec<_>>(),
        "files": hashes,
        "consent_references": consents,
    });
    let manifest_name = format!("{stem} - manifest.json");
    std::fs::write(folder.join(&manifest_name), serde_json::to_string_pretty(&manifest).map_err(|e| failed(e.to_string()))?).map_err(|e| failed(e.to_string()))?;
    names.push(manifest_name);
    let _ = Licence::Unknown;
    Ok(ExportResult {
        folder: folder.to_string_lossy().to_string(),
        files: names,
        loudness: out.loudness.clone(),
        target_lufs: dest_info.target_lufs,
        gain_db: out.gain_db,
        target_reached: out.target_reached,
        warnings: out.warnings.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{Aspect, Codec};
    use std::path::PathBuf;
    use crate::adapters::scratch::ScratchAdapter;
    use crate::audio::write_wav_16_mono;
    use crate::project::{Asset, Character, Cue};
    use crate::providers::Adapter;
    use crate::render::render_line;
    use crate::util::{date_string, new_id};

    fn fixture() -> (Project, Registry, PathBuf) {
        let reg = Registry::new(vec![Box::new(ScratchAdapter)]);
        let mut p = Project::new("Test Film", false);
        p.script = "# One\nAMA: hello there my friend\nKOLA: good morning to you\n# Two\nAMA: the end".into();
        for s in ["AMA", "KOLA"] {
            p.characters.insert(s.into(), Character { voice_id: Some("tone:mid".into()), ..Default::default() });
        }
        let dir = std::env::temp_dir().join(new_id("ss-export"));
        std::fs::create_dir_all(&dir).unwrap();
        for k in ["s0-l0", "s0-l1", "s1-l0"] {
            render_line(&mut p, &reg, &dir, k).unwrap();
        }
        (p, reg, dir)
    }

    fn add_music(p: &mut Project, dir: &Path, lic: Licence, checked: Option<String>) {
        let tone: Vec<f32> = (0..22050 * 2).map(|n| 0.3 * (n as f32 * 0.05).sin()).collect();
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        write_wav_16_mono(&dir.join("assets/m.wav"), &tone, 22050).unwrap();
        p.assets.push(Asset {
            id: "asset-m".into(), name: "Library Theme".into(), kind: AssetKind::Music, file: "assets/m.wav".into(), duration_ms: 2000,
            licence: LicenceInfo { licence: lic, source: Some("https://example.com/licence".into()), checked, notes: String::new(), conditions: vec![] },
            uses_prohibited: vec![], source_note: String::new(), generated: None,
        });
        p.cues.push(Cue { id: "cue-1".into(), asset_id: "asset-m".into(), scene: Some(0), loop_to_scene: true, ..Default::default() });
    }

    #[test]
    fn personal_export_writes_master_three_equal_length_stems_and_a_manifest() {
        let (mut p, reg, dir) = fixture();
        add_music(&mut p, &dir, Licence::Unknown, None);
        let dest = dir.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        assert!(matches!(export(&p, &reg, &dir, &dest, false), Err(ExportError::NeedsAcknowledgement { .. })), "unknown music needs an acknowledgement");
        let r = export(&p, &reg, &dir, &dest, true).unwrap();
        let folder = PathBuf::from(&r.folder);
        let len = |n: &str| read_wav(&folder.join(n)).unwrap().channels[0].len();
        let (m, d, e, mu) = (len("Test Film - master.wav"), len("Test Film - stem dialogue.wav"), len("Test Film - stem effects.wav"), len("Test Film - stem music.wav"));
        assert!(m > 48000 && m == d && m == e && m == mu, "{m} {d} {e} {mu}");
        let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(folder.join("Test Film - manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["loudness"]["destination"], "streaming");
        assert_eq!(manifest["lines"].as_array().unwrap().len(), 3);
        assert_eq!(manifest["files"].as_array().unwrap().len(), 4);
        assert!((r.loudness.integrated_lufs.unwrap() + 14.0).abs() < 0.6, "{:?}", r.loudness);
        // Summing the stems reproduces the master (24-bit quantisation tolerance).
        let (a, b, c, mm) = (read_wav(&folder.join("Test Film - stem dialogue.wav")).unwrap(), read_wav(&folder.join("Test Film - stem effects.wav")).unwrap(), read_wav(&folder.join("Test Film - stem music.wav")).unwrap(), read_wav(&folder.join("Test Film - master.wav")).unwrap());
        let worst = (0..m).map(|i| (a.channels[0][i] + b.channels[0][i] + c.channels[0][i] - mm.channels[0][i]).abs()).fold(0.0f32, f32::max);
        assert!(worst < 1e-5, "stems differ from master by {worst}");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn commercial_export_blocks_unknown_music_and_lists_it_by_name() {
        let (mut p, reg, dir) = fixture();
        add_music(&mut p, &dir, Licence::Unknown, None);
        p.licence_target = LicenceTarget::Commercial;
        let dest = dir.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        match export(&p, &reg, &dir, &dest, true) {
            Err(ExportError::Blocked { report }) => {
                assert!(report.blocking.iter().any(|i| i.name == "Library Theme" && i.kind == "music"));
            }
            other => panic!("expected a block, got {other:?}"),
        }
        assert!(std::fs::read_dir(&dest).unwrap().next().is_none(), "nothing is written when blocked");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn commercial_export_succeeds_once_licences_are_cleared_and_stale_ones_block() {
        let (mut p, reg, dir) = fixture();
        p.licence_target = LicenceTarget::Commercial;
        let today = date_string(today_days());
        add_music(&mut p, &dir, Licence::Commercial, Some(today));
        let dest = dir.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        assert!(export(&p, &reg, &dir, &dest, false).is_ok());
        p.assets[0].licence.checked = Some(date_string(today_days() - 91));
        assert!(matches!(export(&p, &reg, &dir, &dest, false), Err(ExportError::Blocked { .. })));
        // The user records a fresh check, which overrides the stale one.
        p.licence_checks.insert(
            "asset:asset-m".into(),
            LicenceInfo { licence: Licence::Commercial, source: Some("https://example.com/licence".into()), checked: Some(date_string(today_days())), notes: "re-checked".into(), conditions: vec![] },
        );
        assert!(export(&p, &reg, &dir, &dest, false).is_ok());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rejected_take_is_excluded_and_nothing_to_export_is_an_error() {
        let (mut p, reg, dir) = fixture();
        for t in p.takes.iter_mut() {
            t.status = TakeStatus::Rejected;
        }
        let dest = dir.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        assert!(matches!(export(&p, &reg, &dir, &dest, true), Err(ExportError::Failed { .. })));
        std::fs::remove_dir_all(dir).ok();
    }


    #[test]
    fn other_formats_subtitles_and_a_truthful_manifest() {
        let Some(ff) = Ffmpeg::detect(None) else { eprintln!("FFmpeg not found; skipping"); return };
        let (mut p, reg, dir) = fixture();
        p.line_directions.clear();
        let opts = ExportOptions { master: Codec::Mp3 { kbps: 192 }, stems: Some(Codec::Flac), sample_rate: 44_100, subtitles: vec![SubtitleFormat::Srt, SubtitleFormat::Vtt, SubtitleFormat::Srt], speaker_labels: true, ffmpeg_path: None, timeline: false };
        if !ff.supports(&opts.master) || !ff.supports(opts.stems.as_ref().unwrap()) { eprintln!("encoders missing; skipping"); return }
        let dest = dir.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        let r = export_with(&p, &reg, &dir, &dest, true, &opts).unwrap();
        let folder = PathBuf::from(&r.folder);
        for f in ["Test Film - master.mp3", "Test Film - stem dialogue.flac", "Test Film - stem effects.flac", "Test Film - stem music.flac", "Test Film.srt", "Test Film.vtt", "Test Film - manifest.json"] {
            assert!(folder.join(f).is_file(), "{f} missing: {:?}", r.files);
        }
        assert_eq!(r.files.iter().filter(|f| f.ends_with(".srt")).count(), 1, "a repeated subtitle format is written once");
        let srt = std::fs::read_to_string(folder.join("Test Film.srt")).unwrap();
        assert!(srt.contains("AMA: hello there my friend"), "{srt}");
        let m: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(folder.join("Test Film - manifest.json")).unwrap()).unwrap();
        assert_eq!(m["output"]["master"], "MP3 192 kbps");
        assert_eq!(m["output"]["sample_rate"], 44_100);
        assert!(m["output"]["encoder"].as_str().unwrap().to_lowercase().starts_with("ffmpeg version"));
        assert!(m["files"].as_array().unwrap().iter().any(|f| f["file"] == "Test Film - master.mp3" && f["bytes"].as_u64().unwrap() > 1000));
        assert!(!folder.join("Test Film - master.wav").exists(), "no stray WAV is left behind");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_timeline_for_editors_has_every_clip_as_its_own_file() {
        let (p, reg, dir) = fixture();
        let dest = dir.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        let opts = ExportOptions { timeline: true, ..Default::default() };
        let r = export_with(&p, &reg, &dir, &dest, true, &opts).unwrap();
        let folder = PathBuf::from(&r.folder);
        let otio_name = r.files.iter().find(|f| f.ends_with("timeline.otio")).expect("the timeline is listed");
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(folder.join(otio_name)).unwrap()).unwrap();
        assert_eq!(v["OTIO_SCHEMA"], "Timeline.1");
        let mut urls = Vec::new();
        for t in v["tracks"]["children"].as_array().unwrap() {
            for c in t["children"].as_array().unwrap().iter().filter(|c| c["OTIO_SCHEMA"] == "Clip.2") {
                urls.push(c["media_reference"]["target_url"].as_str().unwrap().to_string());
            }
        }
        assert!(!urls.is_empty());
        for u in &urls {
            assert!(folder.join(u).is_file(), "{u} must exist next to the timeline");
            assert!(!u.contains(".."), "{u}");
        }
        let m: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(folder.join("Test Film - manifest.json")).unwrap()).unwrap();
        assert!(m["files"].as_array().unwrap().iter().any(|f| f["format"] == "OpenTimelineIO"), "the manifest lists the timeline");
        // Not asked for, not written.
        let r2 = export_with(&p, &reg, &dir, &dest, true, &ExportOptions::default()).unwrap();
        assert!(!r2.files.iter().any(|f| f.ends_with(".otio")));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_missing_ffmpeg_is_refused_before_anything_is_written() {
        let (p, reg, dir) = fixture();
        let dest = dir.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        let opts = ExportOptions { master: Codec::Mp3 { kbps: 192 }, ffmpeg_path: Some("C:\\nowhere\\ffmpeg.exe".into()), ..Default::default() };
        match export_with(&p, &reg, &dir, &dest, true, &opts) {
            Err(ExportError::Failed { message }) => assert!(message.contains("FFmpeg"), "{message}"),
            other => panic!("expected a refusal, got {other:?}"),
        }
        assert!(std::fs::read_dir(&dest).unwrap().next().is_none(), "nothing may be written when a format cannot be produced");
        let mp4_stems = ExportOptions { stems: Some(Codec::Mp4 { aspect: Aspect::Wide, kbps: 192 }), ..Default::default() };
        assert!(matches!(export_with(&p, &reg, &dir, &dest, true, &mp4_stems), Err(ExportError::Failed { .. })));
        assert!(std::fs::read_dir(&dest).unwrap().next().is_none());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn wav_only_at_16_bit_44k_with_no_stems() {
        let (p, reg, dir) = fixture();
        let dest = dir.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        let opts = ExportOptions { master: Codec::Wav { bits: 16 }, stems: None, sample_rate: 44_100, ..Default::default() };
        let r = export_with(&p, &reg, &dir, &dest, true, &opts).unwrap();
        assert_eq!(r.files, vec!["Test Film - master.wav".to_string(), "Test Film - manifest.json".to_string()]);
        let spec = hound::WavReader::open(PathBuf::from(&r.folder).join("Test Film - master.wav")).unwrap().spec();
        assert_eq!((spec.bits_per_sample, spec.sample_rate), (16, 44_100));
        assert!((r.loudness.integrated_lufs.unwrap() + 14.0).abs() < 1.2, "{:?}", r.loudness);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn exports_in_the_same_second_get_separate_folders() {
        let (p, reg, dir) = fixture();
        let dest = dir.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        let a = export(&p, &reg, &dir, &dest, true).unwrap();
        let b = export(&p, &reg, &dir, &dest, true).unwrap();
        assert_ne!(a.folder, b.folder);
        assert!(PathBuf::from(&a.folder).join("Test Film - master.wav").exists(), "the first export is untouched");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn absurd_offsets_and_gaps_cannot_request_huge_buffers() {
        let (mut p, _reg, dir) = fixture();
        add_music(&mut p, &dir, Licence::Commercial, Some(date_string(today_days())));
        p.cues[0].offset_ms = u32::MAX;
        p.cues[0].loop_to_scene = false;
        p.mix.gap_ms = u32::MAX;
        p.mix.scene_gap_ms = u32::MAX;
        let built = build(&p, &dir).unwrap();
        let max = built.music.iter().map(|m| m.start + m.buf.len()).max().unwrap();
        assert!(max <= ms_to_samples(MAX_TIMELINE_MS) + 2 * SR as usize * 2);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn export_refuses_a_missing_destination_and_unsafe_paths() {
        let (mut p, reg, dir) = fixture();
        assert!(matches!(export(&p, &reg, &dir, &dir.join("nope"), true), Err(ExportError::Failed { .. })));
        p.takes[0].file = "../escape.wav".into();
        let dest = dir.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        assert!(matches!(export(&p, &reg, &dir, &dest, true), Err(ExportError::Failed { .. })));
        let _ = ScratchAdapter.id();
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_users_licence_check_cannot_drop_a_condition_the_provider_declares() {
        use crate::licence::{ConditionKind, LicenceCondition};
        let cond = LicenceCondition { kind: ConditionKind::RevenueCap, text: "Revenue limit".into(), value: None, confirmed: None };
        let base = LicenceInfo { licence: Licence::Commercial, conditions: vec![cond.clone()], ..Default::default() };
        let mut p = Project::new("L", false);
        p.licence_checks.insert("voice:v".into(), LicenceInfo { licence: Licence::Commercial, source: Some("https://example.com/t".into()), checked: Some("2026-10-01".into()), notes: String::new(), conditions: vec![] });
        let e = effective(&p, "voice:v", base.clone());
        assert_eq!(e.conditions, vec![cond.clone()]);
        assert_eq!(e.source.as_deref(), Some("https://example.com/t"));
        // The user's confirmation of the same condition is kept, not duplicated.
        let mut confirmed = cond.clone();
        confirmed.confirmed = Some("2026-10-02".into());
        p.licence_checks.get_mut("voice:v").unwrap().conditions = vec![confirmed.clone()];
        assert_eq!(effective(&p, "voice:v", base).conditions, vec![confirmed]);
    }
}
