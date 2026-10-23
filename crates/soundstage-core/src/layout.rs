//! Places selected takes on one timeline. The same layout drives the Mix screen and export.

use crate::direction::resolve;
use crate::project::{Project, TakeStatus};
use crate::script::Parsed;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LineSlot {
    pub key: String,
    pub scene: usize,
    pub speaker: String,
    pub take_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneSlot {
    pub index: usize,
    pub title: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Layout {
    pub scenes: Vec<SceneSlot>,
    pub lines: Vec<LineSlot>,
    pub total_ms: u64,
}

const MAX_GAP_MS: u32 = 60_000;

/// Lines with a selected take that is not rejected are placed; others are skipped.
pub fn compute(project: &Project, parsed: &Parsed) -> Layout {
    let mut out = Layout::default();
    let mut cursor = 0u64;
    for scene in &parsed.scenes {
        let start = cursor;
        let scene_dir = project.scene_directions.get(&scene.index.to_string()).cloned().unwrap_or_default();
        for line in parsed.lines.iter().filter(|l| l.scene == scene.index) {
            let Some(take) = project.selected_take(&line.key).filter(|t| t.status != TakeStatus::Rejected) else { continue };
            let char_dir = project.characters.get(&line.speaker).map(|c| c.direction.clone()).unwrap_or_default();
            let line_dir = project.line_directions.get(&line.key).cloned().unwrap_or_default();
            let r = resolve(&project.project_direction, &char_dir, &scene_dir, &line_dir);
            let gap = r.pause_before_ms.value.unwrap_or(project.mix.gap_ms).min(MAX_GAP_MS) as u64;
            let s = cursor + gap;
            let e = s + take.duration_ms;
            out.lines.push(LineSlot { key: line.key.clone(), scene: scene.index, speaker: line.speaker.clone(), take_id: take.id.clone(), start_ms: s, end_ms: e });
            cursor = e;
        }
        out.scenes.push(SceneSlot { index: scene.index, title: scene.title.clone(), start_ms: start, end_ms: cursor });
        cursor += project.mix.scene_gap_ms.min(MAX_GAP_MS) as u64;
    }
    out.total_ms = out.scenes.last().map_or(0, |s| s.end_ms);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::direction::Direction;
    use crate::project::Take;

    fn take(id: &str, key: &str, ms: u64) -> Take {
        Take {
            id: id.into(), line_key: key.into(), speaker: "AMA".into(), text: "x".into(), voice_id: "v".into(), adapter_id: "a".into(),
            provider_version: "1".into(), cache_key: "k".into(), file: format!("takes/{id}.wav"), duration_ms: ms, sample_rate: 22050,
            direction_summary: String::new(), created: String::new(), status: TakeStatus::Pending, approver: None,
        }
    }

    fn fixture() -> (Project, Parsed) {
        let mut p = Project::new("L", false);
        p.script = "# One\nAMA: a\nKOLA: b\n# Two\nAMA: c".into();
        for (k, id, ms) in [("s0-l0", "t1", 1000), ("s0-l1", "t2", 500), ("s1-l0", "t3", 700)] {
            p.takes.push(take(id, k, ms));
            p.selected.insert(k.into(), id.into());
        }
        let parsed = crate::script::parse(&p.script);
        (p, parsed)
    }

    #[test]
    fn lines_follow_each_other_with_gaps_and_scenes_with_a_scene_gap() {
        let (p, parsed) = fixture();
        let l = compute(&p, &parsed);
        assert_eq!((l.lines[0].start_ms, l.lines[0].end_ms), (350, 1350));
        assert_eq!(l.lines[1].start_ms, 1700);
        assert_eq!(l.scenes[0].end_ms, 2200);
        assert_eq!(l.scenes[1].start_ms, 2200 + 1000);
        assert_eq!(l.total_ms, l.scenes[1].end_ms);
    }

    #[test]
    fn line_pause_before_overrides_the_default_gap() {
        let (mut p, parsed) = fixture();
        p.line_directions.insert("s0-l1".into(), Direction { pause_before_ms: Some(0), ..Default::default() });
        let l = compute(&p, &parsed);
        assert_eq!(l.lines[1].start_ms, l.lines[0].end_ms);
    }

    #[test]
    fn rejected_takes_are_left_out() {
        let (mut p, parsed) = fixture();
        p.takes[1].status = TakeStatus::Rejected;
        assert_eq!(compute(&p, &parsed).lines.len(), 2);
    }
}
