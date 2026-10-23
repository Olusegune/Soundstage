//! Licence-gated export (FR-7, FR-13): master plus dialogue, effects and music stems of
//! identical length, and a provenance manifest recording every decision.

use crate::audio::{read_wav, resample, to_mix_format, write_wav_24, Stereo, SR};
use crate::layout::{compute, Layout};
use crate::licence::{gate, GateReport, Licence, LicenceInfo, LicenceTarget, Subject};
use crate::loudness::{destination, Loudness};
use crate::mix::{mix, MixParams, Placed};
use crate::project::{safe_relative, AssetKind, Project, TakeStatus};
use crate::providers::Registry;
use crate::script;
use crate::util::{iso_now, safe_file_stem, sha256_hex, today_days};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn effective(project: &Project, key: &str, base: LicenceInfo) -> LicenceInfo {
    project.licence_checks.get(key).cloned().unwrap_or(base)
}

/// Everything the export gate must judge: voices, models, music and effects actually used.
pub fn subjects(project: &Project, reg: &Registry) -> Vec<Subject> {
    let parsed = script::parse(&project.script);
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

pub fn check(project: &Project, reg: &Registry) -> GateReport {
    let mut r = gate(project.licence_target, &subjects(project, reg), &project.mix.destination, today_days());
    let parsed = script::parse(&project.script);
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
    let parsed = script::parse(&project.script);
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

pub fn export(project: &Project, reg: &Registry, project_dir: &Path, dest: &Path, acknowledged: bool) -> Result<ExportResult, ExportError> {
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
    let (out, dest_info, layout) = render_mix(project, project_dir)?;

    let stamp = iso_now().replace([':', '-'], "").replace('T', "-").replace('Z', "");
    let folder = dest.join(format!("{} export {}", safe_file_stem(&project.name), stamp));
    std::fs::create_dir(&folder).map_err(|e| failed(format!("Could not create the export folder: {e}")))?;
    let stem = safe_file_stem(&project.name);
    let files = [
        (format!("{stem} - master.wav"), &out.master),
        (format!("{stem} - stem dialogue.wav"), &out.dialogue),
        (format!("{stem} - stem effects.wav"), &out.effects),
        (format!("{stem} - stem music.wav"), &out.music),
    ];
    let mut hashes = Vec::new();
    for (name, buf) in &files {
        let path: PathBuf = folder.join(name);
        write_wav_24(&path, buf).map_err(failed)?;
        let bytes = std::fs::read(&path).map_err(|e| failed(e.to_string()))?;
        hashes.push(serde_json::json!({ "file": name, "sha256": sha256_hex(&bytes), "samples": buf.len(), "sample_rate": SR }));
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
        "loudness": { "destination": dest_info.id, "target_lufs": dest_info.target_lufs, "true_peak_ceiling_dbtp": dest_info.true_peak_ceiling_dbtp,
                      "measured": out.loudness, "gain_applied_db": out.gain_db, "target_reached": out.target_reached, "warnings": out.warnings },
        "licence_gate": { "decision": "allowed", "report": report },
        "licence_checks_recorded_by_user": project.licence_checks,
        "lines": lines,
        "cues": project.cues.iter().filter_map(|c| project.asset(&c.asset_id).map(|a| serde_json::json!({
            "cue": c.id, "asset": a.id, "name": a.name, "kind": a.kind, "licence": a.licence, "scene": c.scene, "offset_ms": c.offset_ms, "gain_db": c.gain_db, "ducked": c.duck }))).collect::<Vec<_>>(),
        "rejected_takes_excluded": project.takes.iter().filter(|t| t.status == TakeStatus::Rejected).map(|t| t.id.clone()).collect::<Vec<_>>(),
        "files": hashes,
        "consent_references": Vec::<String>::new(),
    });
    let manifest_name = format!("{stem} - manifest.json");
    std::fs::write(folder.join(&manifest_name), serde_json::to_string_pretty(&manifest).map_err(|e| failed(e.to_string()))?).map_err(|e| failed(e.to_string()))?;

    let mut names: Vec<String> = files.iter().map(|(n, _)| n.clone()).collect();
    names.push(manifest_name);
    let _ = Licence::Unknown;
    Ok(ExportResult {
        folder: folder.to_string_lossy().to_string(),
        files: names,
        loudness: out.loudness,
        target_lufs: dest_info.target_lufs,
        gain_db: out.gain_db,
        target_reached: out.target_reached,
        warnings: out.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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
            licence: LicenceInfo { licence: lic, source: Some("https://example.com/licence".into()), checked, notes: String::new() },
            uses_prohibited: vec![], source_note: String::new(),
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
            LicenceInfo { licence: Licence::Commercial, source: Some("https://example.com/licence".into()), checked: Some(date_string(today_days())), notes: "re-checked".into() },
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
}
