//! The performance plan (PRD-002 section 5, `docs/spec/03` section 4): what each line should sound like,
//! as intent rather than provider settings. One versioned JSON file per project, `plan.json`.
//!
//! Nothing here calls a model. This is the schema, the checks that every plan must pass before it is
//! kept, and the save and load rules. A plan that fails a check is refused and the previous plan stays.
//! Fields a newer version added are kept when the file is read and written back.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const PLAN_SCHEMA: u32 = 1;
const MAX_LINES: usize = 5000;
const MAX_TEXT: usize = 500;
const MAX_LIST: usize = 50;
const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Source {
    /// `director` (a model or the rules) or `user`.
    pub kind: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EmotionPart {
    pub name: String,
    /// 0.0 to 1.0.
    pub intensity: f32,
    /// True when this is shown on the surface; false when it is held back underneath.
    pub surface: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Pause {
    /// Character offset in the line's text.
    pub at: usize,
    pub ms: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Emphasis {
    pub start: usize,
    pub end: usize,
    pub level: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct VocalEvent {
    /// For example `laugh`, `cough`, `breath`. Not limited here; the capability registry says what a model can do.
    pub kind: String,
    pub at: usize,
    pub duration_ms: Option<u32>,
    pub intensity: f32,
    /// `provider-tag`, `generated` or `library`.
    pub source: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LanguageSpan {
    pub start: usize,
    pub end: usize,
    pub language: String,
    pub accent: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Alternative {
    pub label: String,
    /// What this alternative changes, as a partial plan for the line.
    pub change: Value,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LinePlan {
    pub objective: Option<String>,
    pub subtext: Option<String>,
    pub relationship: Option<String>,
    pub emotion: Vec<EmotionPart>,
    pub pace: Option<f32>,
    pub pauses: Vec<Pause>,
    pub emphasis: Vec<Emphasis>,
    pub events: Vec<VocalEvent>,
    pub languages: Vec<LanguageSpan>,
    pub alternatives: Vec<Alternative>,
    /// 0.0 to 1.0: how sure the author of the plan is. A plan the user wrote is 1.0.
    pub confidence: Option<f32>,
    pub uncertainty_note: Option<String>,
    /// Names of fields the user changed by hand. A later run of the Director must not touch them.
    pub user_locked: Vec<String>,
    /// Fields from a newer version, kept so they are not lost.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SceneIntent {
    pub purpose: Option<String>,
    pub mood: Option<String>,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PerformancePlan {
    pub schema: u32,
    /// Hash of the script the plan was made for (see `ids::script_hash`), to say when it may be out of date.
    pub source_hash: String,
    pub created_by: Source,
    /// Keyed by scene index as text.
    pub scenes: BTreeMap<String, SceneIntent>,
    /// Keyed by stable line id.
    pub lines: BTreeMap<String, LinePlan>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for PerformancePlan {
    fn default() -> Self {
        PerformancePlan { schema: PLAN_SCHEMA, source_hash: String::new(), created_by: Source::default(), scenes: BTreeMap::new(), lines: BTreeMap::new(), extra: BTreeMap::new() }
    }
}

fn unit(x: f32) -> bool {
    x.is_finite() && (0.0..=1.0).contains(&x)
}

fn short(s: &Option<String>) -> bool {
    s.as_ref().is_none_or(|t| t.chars().count() <= MAX_TEXT && !t.chars().any(|c| c.is_control() && c != '\n'))
}

impl LinePlan {
    pub fn validate(&self, text_len: Option<usize>) -> Result<(), String> {
        let in_text = |n: usize| text_len.is_none_or(|l| n <= l);
        if !(short(&self.objective) && short(&self.subtext) && short(&self.relationship) && short(&self.uncertainty_note)) {
            return Err("A note in the plan is too long or has control characters.".into());
        }
        if self.emotion.len() > 8 || self.pauses.len() > MAX_LIST || self.emphasis.len() > MAX_LIST || self.events.len() > MAX_LIST || self.languages.len() > MAX_LIST || self.alternatives.len() > 5 || self.user_locked.len() > 20 {
            return Err("A line has more entries than the plan allows.".into());
        }
        if self.emotion.iter().any(|e| e.name.is_empty() || e.name.chars().count() > 40 || !unit(e.intensity)) {
            return Err("An emotion needs a short name and an intensity from 0 to 1.".into());
        }
        if self.pace.is_some_and(|p| !p.is_finite() || !(0.5..=2.0).contains(&p)) {
            return Err("Pace must be between 0.5 and 2.".into());
        }
        if self.pauses.iter().any(|p| p.ms > 10_000 || !in_text(p.at)) {
            return Err("A pause is too long or sits outside the line.".into());
        }
        if self.emphasis.iter().any(|e| e.start >= e.end || !in_text(e.end) || !unit(e.level)) {
            return Err("An emphasis range is outside the line.".into());
        }
        if self.events.iter().any(|e| e.kind.is_empty() || e.kind.chars().count() > 40 || !unit(e.intensity) || !in_text(e.at) || e.duration_ms.is_some_and(|d| d > 30_000) || !matches!(e.source.as_str(), "provider-tag" | "generated" | "library")) {
            return Err("A vocal event is not valid: check its name, position, length, intensity and source.".into());
        }
        if self.languages.iter().any(|l| l.start >= l.end || !in_text(l.end) || l.language.is_empty() || l.language.chars().count() > 20) {
            return Err("A language range is outside the line.".into());
        }
        if self.confidence.is_some_and(|c| !unit(c)) {
            return Err("Confidence must be between 0 and 1.".into());
        }
        if self.alternatives.iter().any(|a| a.label.chars().count() > 60 || a.change.to_string().len() > 4000) {
            return Err("An alternative is too large.".into());
        }
        Ok(())
    }
}

impl PerformancePlan {
    /// `line_lengths` gives the text length of each known line id so positions can be checked. Lines
    /// missing from it (the plan may be older than the script) are checked for everything else.
    pub fn validate(&self, line_lengths: &BTreeMap<String, usize>) -> Result<(), String> {
        if self.schema == 0 || self.schema > PLAN_SCHEMA {
            return Err(format!("This plan was made by a newer version of Soundstage (plan format {}).", self.schema));
        }
        if self.lines.len() > MAX_LINES || self.scenes.len() > MAX_LINES {
            return Err("The plan has too many lines.".into());
        }
        for (id, lp) in &self.lines {
            if !crate::project::valid_id(id) {
                return Err("A line id in the plan is not valid.".into());
            }
            lp.validate(line_lengths.get(id).copied()).map_err(|e| format!("Line {id}: {e}"))?;
        }
        for s in self.scenes.values() {
            if !(short(&s.purpose) && short(&s.mood) && short(&s.notes)) {
                return Err("A scene note is too long.".into());
            }
        }
        if self.created_by.kind.chars().count() > 20 {
            return Err("The plan source is not valid.".into());
        }
        Ok(())
    }
}

/// `plan.json` in a project folder. The old file is kept as `plan.json.bak`.
pub struct PlanStore {
    file: PathBuf,
}

impl PlanStore {
    pub fn new(project_dir: &Path) -> PlanStore {
        PlanStore { file: project_dir.join("plan.json") }
    }

    /// A project with no plan yet has the empty plan.
    pub fn load(&self) -> Result<PerformancePlan, String> {
        if !self.file.exists() {
            return Ok(PerformancePlan::default());
        }
        let read = |p: &Path| -> Result<PerformancePlan, String> {
            if std::fs::metadata(p).map(|m| m.len()).unwrap_or(0) > MAX_FILE_BYTES {
                return Err("The plan file is too large to open.".into());
            }
            let text = std::fs::read_to_string(p).map_err(|e| format!("Could not read the plan: {e}"))?;
            let plan: PerformancePlan = serde_json::from_str(&text).map_err(|_| "The plan file is damaged.".to_string())?;
            if plan.schema > PLAN_SCHEMA {
                return Err(format!("This plan was made by a newer version of Soundstage (plan format {}).", plan.schema));
            }
            Ok(plan)
        };
        read(&self.file).or_else(|first| read(&self.file.with_extension("json.bak")).map_err(|_| first))
    }

    /// Check, then write through a temporary file. A plan that fails the checks is refused and nothing changes.
    pub fn save(&self, plan: &PerformancePlan, line_lengths: &BTreeMap<String, usize>) -> Result<(), String> {
        plan.validate(line_lengths)?;
        let json = serde_json::to_vec_pretty(plan).map_err(|e| e.to_string())?;
        if json.len() as u64 > MAX_FILE_BYTES {
            return Err("The plan is too large to save.".into());
        }
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("Could not save the plan: {e}"))?;
        }
        let tmp = self.file.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(|e| format!("Could not save the plan: {e}"))?;
        if self.file.exists() {
            let _ = std::fs::copy(&self.file, self.file.with_extension("json.bak"));
        }
        std::fs::rename(&tmp, &self.file).map_err(|e| format!("Could not save the plan: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::new_id;

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(new_id("ss-plan"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn good() -> PerformancePlan {
        let mut p = PerformancePlan::default();
        p.lines.insert(
            "s0-l0".into(),
            LinePlan {
                emotion: vec![EmotionPart { name: "tired".into(), intensity: 0.6, surface: true }, EmotionPart { name: "hopeful".into(), intensity: 0.3, surface: false }],
                pace: Some(0.9),
                pauses: vec![Pause { at: 5, ms: 400 }],
                events: vec![VocalEvent { kind: "sigh".into(), at: 0, duration_ms: Some(600), intensity: 0.5, source: "provider-tag".into() }],
                confidence: Some(0.7),
                ..Default::default()
            },
        );
        p
    }

    fn lens() -> BTreeMap<String, usize> {
        BTreeMap::from([("s0-l0".to_string(), 20usize)])
    }

    #[test]
    fn a_good_plan_round_trips_and_an_empty_project_has_the_empty_plan() {
        let d = tmp();
        let s = PlanStore::new(&d);
        assert_eq!(s.load().unwrap(), PerformancePlan::default());
        s.save(&good(), &lens()).unwrap();
        assert_eq!(s.load().unwrap(), good());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn a_bad_plan_is_refused_and_the_old_one_stays() {
        let d = tmp();
        let s = PlanStore::new(&d);
        s.save(&good(), &lens()).unwrap();
        let mut bad = good();
        bad.lines.get_mut("s0-l0").unwrap().emotion[0].intensity = 7.0;
        assert!(s.save(&bad, &lens()).is_err());
        let mut far = good();
        far.lines.get_mut("s0-l0").unwrap().pauses[0].at = 999;
        assert!(s.save(&far, &lens()).is_err(), "a pause past the end of the line");
        assert_eq!(s.load().unwrap(), good());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn each_kind_of_nonsense_is_caught() {
        let try_it = |f: &dyn Fn(&mut LinePlan)| {
            let mut p = good();
            f(p.lines.get_mut("s0-l0").unwrap());
            p.validate(&lens()).is_err()
        };
        assert!(try_it(&|l| l.pace = Some(f32::NAN)));
        assert!(try_it(&|l| l.pace = Some(5.0)));
        assert!(try_it(&|l| l.confidence = Some(-0.1)));
        assert!(try_it(&|l| l.events[0].source = "somewhere".into()));
        assert!(try_it(&|l| l.events[0].kind = String::new()));
        assert!(try_it(&|l| l.emphasis.push(Emphasis { start: 4, end: 2, level: 0.5 })));
        assert!(try_it(&|l| l.subtext = Some("x".repeat(600))));
        assert!(try_it(&|l| l.objective = Some("bell\u{7}".into())));
        assert!(try_it(&|l| l.events = (0..60).map(|_| l.events[0].clone()).collect()));
        let mut p = good();
        p.lines.insert("../etc".into(), LinePlan::default());
        assert!(p.validate(&lens()).is_err(), "line ids must be plain ids");
    }

    #[test]
    fn a_newer_plan_is_refused_but_unknown_fields_in_the_same_version_survive() {
        let d = tmp();
        let s = PlanStore::new(&d);
        std::fs::write(d.join("plan.json"), r#"{"schema":99}"#).unwrap();
        assert!(s.load().unwrap_err().contains("newer version"));
        std::fs::write(d.join("plan.json"), r#"{"schema":1,"future_thing":{"a":1},"lines":{"s0-l0":{"pace":1.0,"mood_ring":"blue"}}}"#).unwrap();
        let p = s.load().unwrap();
        assert_eq!(p.extra["future_thing"]["a"], 1);
        assert_eq!(p.lines["s0-l0"].extra["mood_ring"], "blue");
        s.save(&p, &lens()).unwrap();
        let again = std::fs::read_to_string(d.join("plan.json")).unwrap();
        assert!(again.contains("future_thing") && again.contains("mood_ring"));
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn a_damaged_plan_falls_back_to_the_backup_and_otherwise_errors_plainly() {
        let d = tmp();
        let s = PlanStore::new(&d);
        s.save(&good(), &lens()).unwrap();
        s.save(&good(), &lens()).unwrap();
        std::fs::write(d.join("plan.json"), "{broken").unwrap();
        assert_eq!(s.load().unwrap(), good());
        std::fs::remove_file(d.join("plan.json.bak")).unwrap();
        assert_eq!(s.load().unwrap_err(), "The plan file is damaged.");
        std::fs::remove_dir_all(d).ok();
    }
}
