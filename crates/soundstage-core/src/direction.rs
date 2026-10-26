//! Direction resolves at four levels: project, character, scene, line.
//! A lower level overrides the one above it, field by field (SOUND_DESIGN.md 3.1).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Direction {
    pub emotion: Option<String>,
    pub pace: Option<f32>,
    pub pause_before_ms: Option<u32>,
    pub note: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Default,
    /// A suggestion from the Director or the person's plan. Below everything the person set.
    Plan,
    Project,
    Character,
    Scene,
    Line,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Field<T> {
    pub value: T,
    pub source: Level,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Resolved {
    pub emotion: Field<String>,
    pub pace: Field<f32>,
    /// `None` means "use the project's default gap".
    pub pause_before_ms: Field<Option<u32>>,
    pub notes: Vec<(Level, String)>,
}

pub const MIN_PACE: f32 = 0.5;
pub const MAX_PACE: f32 = 2.0;

pub fn clamp_pace(p: f32) -> f32 {
    if p.is_finite() { p.clamp(MIN_PACE, MAX_PACE) } else { 1.0 }
}

pub fn resolve(project: &Direction, character: &Direction, scene: &Direction, line: &Direction) -> Resolved {
    resolve_with_plan(&Direction::default(), project, character, scene, line)
}

/// Like `resolve`, with the line's plan underneath the four levels the person controls.
pub fn resolve_with_plan(plan: &Direction, project: &Direction, character: &Direction, scene: &Direction, line: &Direction) -> Resolved {
    let levels = [
        (Level::Plan, plan),
        (Level::Project, project),
        (Level::Character, character),
        (Level::Scene, scene),
        (Level::Line, line),
    ];
    let mut emotion = Field { value: "neutral".to_string(), source: Level::Default };
    let mut pace = Field { value: 1.0f32, source: Level::Default };
    let mut pause = Field { value: None, source: Level::Default };
    let mut notes = Vec::new();
    for (lvl, d) in levels {
        if let Some(e) = d.emotion.as_ref().filter(|e| !e.trim().is_empty()) {
            emotion = Field { value: e.trim().to_lowercase(), source: lvl };
        }
        if let Some(p) = d.pace {
            pace = Field { value: clamp_pace(p), source: lvl };
        }
        if let Some(ms) = d.pause_before_ms {
            pause = Field { value: Some(ms.min(10_000)), source: lvl };
        }
        if let Some(n) = d.note.as_ref().filter(|n| !n.trim().is_empty()) {
            notes.push((lvl, n.trim().to_string()));
        }
    }
    Resolved { emotion, pace, pause_before_ms: pause, notes }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(emotion: Option<&str>, pace: Option<f32>) -> Direction {
        Direction { emotion: emotion.map(String::from), pace, ..Default::default() }
    }

    #[test]
    fn line_beats_scene_beats_character_beats_project() {
        let r = resolve(&d(Some("excited"), Some(1.1)), &d(Some("serious"), None), &d(Some("tired"), None), &d(Some("whisper"), None));
        assert_eq!((r.emotion.value.as_str(), r.emotion.source), ("whisper", Level::Line));
        assert_eq!((r.pace.value, r.pace.source), (1.1, Level::Project));
    }

    #[test]
    fn changing_one_level_changes_only_inheriting_lines() {
        let none = Direction::default();
        let line_a = d(Some("sad"), None);
        let before_a = resolve(&none, &none, &d(Some("tired"), None), &line_a);
        let before_b = resolve(&none, &none, &d(Some("tired"), None), &none);
        let after_a = resolve(&none, &none, &d(Some("excited"), None), &line_a);
        let after_b = resolve(&none, &none, &d(Some("excited"), None), &none);
        assert_eq!(before_a, after_a, "line with its own emotion is unaffected");
        assert_ne!(before_b, after_b, "line that inherits from the scene changes");
    }

    #[test]
    fn the_plan_beats_defaults_but_every_level_the_person_sets_beats_the_plan() {
        let none = Direction::default();
        let plan = d(Some("sad"), Some(0.8));
        let r = resolve_with_plan(&plan, &none, &none, &none, &none);
        assert_eq!((r.emotion.value.as_str(), r.emotion.source, r.pace.source), ("sad", Level::Plan, Level::Plan));
        for (p, c, s, l, who) in [
            (d(Some("shout"), None), none.clone(), none.clone(), none.clone(), Level::Project),
            (none.clone(), d(Some("shout"), None), none.clone(), none.clone(), Level::Character),
            (none.clone(), none.clone(), d(Some("shout"), None), none.clone(), Level::Scene),
            (none.clone(), none.clone(), none.clone(), d(Some("shout"), None), Level::Line),
        ] {
            let r = resolve_with_plan(&plan, &p, &c, &s, &l);
            assert_eq!((r.emotion.value.as_str(), r.emotion.source), ("shout", who));
            assert_eq!(r.pace.value, 0.8, "pace the person did not set still comes from the plan");
        }
    }

    #[test]
    fn defaults_and_clamping() {
        let none = Direction::default();
        let r = resolve(&none, &none, &none, &d(None, Some(9.0)));
        assert_eq!(r.emotion.source, Level::Default);
        assert_eq!(r.pace.value, MAX_PACE);
        assert_eq!(clamp_pace(f32::NAN), 1.0);
    }
}
