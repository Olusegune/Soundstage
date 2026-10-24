//! Project model and storage (FR-8): human-readable JSON plus an assets folder,
//! versioned schema, atomic autosave, and recovery from the last good copy.

use crate::direction::Direction;
use crate::licence::{LicenceInfo, LicenceTarget};
use crate::util::{iso_now, new_id, slugify};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const SCHEMA_VERSION: u32 = 1;

pub const SAMPLE_SCRIPT: &str = "# Kitchen, morning\nAMA: Good morning! [pause:short] You're up early.\nKOLA: [sigh] I couldn't sleep. [pause:long] The rain was loud.\nAMA: [whisper] Then let's not wake the others.\n\n# Market, later\nKOLA: [excited] Look at these mangoes!\nAMA: Careful. [pause:350] Last time you bought the whole stall.\n";

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Bible {
    pub age: String,
    pub background: String,
    pub speech_habits: String,
    pub emotional_range: String,
    pub notes: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Character {
    pub voice_id: Option<String>,
    pub bible: Bible,
    pub direction: Direction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TakeStatus {
    #[default]
    Pending,
    Approved,
    Held,
    Rejected,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Take {
    pub id: String,
    pub line_key: String,
    pub speaker: String,
    /// The line exactly as sent, in canonical tag form.
    pub text: String,
    pub voice_id: String,
    pub adapter_id: String,
    pub provider_version: String,
    pub cache_key: String,
    /// Relative to the project folder, for example `takes/take-1a2b3c4d.wav`.
    pub file: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub direction_summary: String,
    pub created: String,
    #[serde(default)]
    pub status: TakeStatus,
    #[serde(default)]
    pub approver: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Music,
    Sfx,
}

/// How a generated asset was made. Recorded so an export can say so, and so the same sound can be traced and remade.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GenInfo {
    pub generator: String,
    pub provider_version: String,
    pub prompt: String,
    pub created: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub kind: AssetKind,
    /// Relative to the project folder, for example `assets/asset-1a2b3c4d.wav`.
    pub file: String,
    pub duration_ms: u64,
    pub licence: LicenceInfo,
    #[serde(default)]
    pub uses_prohibited: Vec<String>,
    #[serde(default)]
    pub source_note: String,
    /// Set when Soundstage generated this audio. Absent for files the user imported.
    #[serde(default)]
    pub generated: Option<GenInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Cue {
    pub id: String,
    pub asset_id: String,
    /// Scene the offset is measured from. `None` means the project start.
    pub scene: Option<usize>,
    pub offset_ms: u32,
    pub gain_db: f32,
    pub loop_to_scene: bool,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
    /// Source music (a radio in the scene) opts out of ducking.
    pub duck: bool,
}

impl Default for Cue {
    fn default() -> Self {
        Cue { id: String::new(), asset_id: String::new(), scene: None, offset_ms: 0, gain_db: -6.0, loop_to_scene: false, fade_in_ms: 20, fade_out_ms: 200, duck: true }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MixSettings {
    pub destination: String,
    pub duck_depth_db: f32,
    pub attack_ms: u32,
    pub release_ms: u32,
    pub gap_ms: u32,
    pub scene_gap_ms: u32,
}

impl Default for MixSettings {
    fn default() -> Self {
        MixSettings { destination: "streaming".into(), duck_depth_db: 12.0, attack_ms: 40, release_ms: 350, gap_ms: 350, scene_gap_ms: 1000 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub id: String,
    /// `scene:2`, `line:s0-l1` or `take:take-1a2b3c4d`.
    pub target: String,
    pub text: String,
    pub created: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Rating {
    AllAges,
    Teen,
    Adult,
    #[default]
    Unrated,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Project {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub created: String,
    pub modified: String,
    pub script: String,
    pub licence_target: LicenceTarget,
    pub rating: Rating,
    pub project_direction: Direction,
    pub characters: BTreeMap<String, Character>,
    pub scene_directions: BTreeMap<String, Direction>,
    pub line_directions: BTreeMap<String, Direction>,
    pub takes: Vec<Take>,
    /// Line key to the selected take id.
    pub selected: BTreeMap<String, String>,
    /// Lines the user chose to leave without audio.
    pub skipped: Vec<String>,
    pub assets: Vec<Asset>,
    pub cues: Vec<Cue>,
    pub mix: MixSettings,
    /// User-recorded licence checks. Keys are `voice:<id>`, `model:<id>` or `asset:<id>`.
    pub licence_checks: BTreeMap<String, LicenceInfo>,
    pub notes: Vec<Note>,
    /// Scene status by index: `not_started`, `in_progress`, `approved`.
    pub scene_status: BTreeMap<String, String>,
    /// How this project's names and terms should be spoken. Applied before the app-wide rules.
    pub pronunciations: Vec<crate::pronounce::Rule>,
}

impl Default for Project {
    fn default() -> Self {
        Project {
            schema_version: SCHEMA_VERSION,
            id: String::new(),
            name: "Untitled".into(),
            created: String::new(),
            modified: String::new(),
            script: String::new(),
            licence_target: LicenceTarget::Personal,
            rating: Rating::Unrated,
            project_direction: Direction::default(),
            characters: BTreeMap::new(),
            scene_directions: BTreeMap::new(),
            line_directions: BTreeMap::new(),
            takes: vec![],
            selected: BTreeMap::new(),
            skipped: vec![],
            assets: vec![],
            cues: vec![],
            mix: MixSettings::default(),
            licence_checks: BTreeMap::new(),
            notes: vec![],
            scene_status: BTreeMap::new(),
            pronunciations: vec![],
        }
    }
}

impl Project {
    pub fn new(name: &str, sample: bool) -> Project {
        let now = iso_now();
        let name = name.trim();
        let name = if name.is_empty() { "Untitled" } else { name };
        Project {
            id: format!("{}-{}", slugify(name), &new_id("p")[2..]),
            name: name.chars().take(80).collect(),
            created: now.clone(),
            modified: now,
            script: if sample { SAMPLE_SCRIPT.to_string() } else { String::new() },
            ..Default::default()
        }
    }

    pub fn take(&self, id: &str) -> Option<&Take> {
        self.takes.iter().find(|t| t.id == id)
    }

    pub fn selected_take(&self, line_key: &str) -> Option<&Take> {
        self.selected.get(line_key).and_then(|id| self.take(id))
    }

    pub fn asset(&self, id: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.id == id)
    }
}

/// Project ids become folder names, so they are validated rather than trusted.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 80 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// A relative path inside a project folder: no separators that escape, no absolute paths.
pub fn safe_relative(p: &str) -> bool {
    let path = Path::new(p);
    !p.is_empty()
        && path.is_relative()
        && !p.contains(':')
        && path.components().all(|c| matches!(c, std::path::Component::Normal(_)))
}

/// Upgrade an older project to the current schema, or explain why it cannot be opened.
pub fn migrate(mut v: serde_json::Value) -> Result<Project, String> {
    let ver = v.get("schema_version").and_then(|x| x.as_u64()).unwrap_or(1) as u32;
    if ver > SCHEMA_VERSION {
        return Err(format!(
            "This project was saved by a newer version of Soundstage (schema {ver}). Update Soundstage to open it."
        ));
    }
    // Future migrations step `ver` up one at a time here.
    if let Some(o) = v.as_object_mut() {
        o.insert("schema_version".into(), SCHEMA_VERSION.into());
    }
    serde_json::from_value(v).map_err(|e| format!("The project file is damaged: {e}"))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Summary {
    pub id: String,
    pub name: String,
    pub modified: String,
    pub licence_target: LicenceTarget,
    pub lines: usize,
}

pub struct Store {
    pub root: PathBuf,
}

impl Store {
    pub fn new(root: PathBuf) -> Store {
        Store { root }
    }

    pub fn dir(&self, id: &str) -> Result<PathBuf, String> {
        if !valid_id(id) {
            return Err("That is not a valid project id.".into());
        }
        Ok(self.root.join(id))
    }

    fn file(&self, id: &str) -> Result<PathBuf, String> {
        Ok(self.dir(id)?.join("project.json"))
    }

    pub fn save(&self, p: &Project) -> Result<(), String> {
        let dir = self.dir(&p.id)?;
        std::fs::create_dir_all(dir.join("takes")).map_err(|e| format!("Could not create the project folder: {e}"))?;
        std::fs::create_dir_all(dir.join("assets")).map_err(|e| e.to_string())?;
        let mut p = p.clone();
        p.schema_version = SCHEMA_VERSION;
        p.modified = iso_now();
        let json = serde_json::to_string_pretty(&p).map_err(|e| e.to_string())?;
        let (file, tmp, bak) = (dir.join("project.json"), dir.join("project.json.tmp"), dir.join("project.json.bak"));
        std::fs::write(&tmp, json).map_err(|e| format!("Could not save the project: {e}"))?;
        if file.exists() {
            let _ = std::fs::copy(&file, &bak);
        }
        std::fs::rename(&tmp, &file).map_err(|e| format!("Could not save the project: {e}"))
    }

    pub fn load(&self, id: &str) -> Result<Project, String> {
        let file = self.file(id)?;
        let read = |p: &Path| -> Result<Project, String> {
            let text = std::fs::read_to_string(p).map_err(|e| format!("Could not read the project: {e}"))?;
            let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("The project file is damaged: {e}"))?;
            migrate(v)
        };
        match read(&file) {
            Ok(p) if p.id == id => Ok(p),
            Ok(_) => Err("The project file does not match its folder.".into()),
            Err(first) => {
                // Crash recovery: fall back to the last good copy.
                let bak = file.with_extension("json.bak");
                read(&bak).map_err(|_| first)
            }
        }
    }

    pub fn list(&self) -> Vec<Summary> {
        let Ok(rd) = std::fs::read_dir(&self.root) else { return vec![] };
        let mut out: Vec<Summary> = rd
            .flatten()
            .filter_map(|e| {
                let id = e.file_name().to_string_lossy().to_string();
                let p = self.load(&id).ok()?;
                Some(Summary {
                    id,
                    name: p.name.clone(),
                    modified: p.modified.clone(),
                    licence_target: p.licence_target,
                    lines: crate::script::parse(&p.script).lines.len(),
                })
            })
            .collect();
        out.sort_by(|a, b| b.modified.cmp(&a.modified));
        out
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let dir = self.dir(id)?;
        if !dir.join("project.json").exists() {
            return Err("That project does not exist.".into());
        }
        std::fs::remove_dir_all(dir).map_err(|e| format!("Could not delete the project: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        let root = std::env::temp_dir().join(new_id("ss-store"));
        std::fs::create_dir_all(&root).unwrap();
        Store::new(root)
    }

    #[test]
    fn save_load_round_trip_and_list() {
        let s = store();
        let mut p = Project::new("My Film", true);
        p.licence_target = LicenceTarget::Commercial;
        s.save(&p).unwrap();
        let q = s.load(&p.id).unwrap();
        assert_eq!(q.script, p.script);
        assert_eq!(q.licence_target, LicenceTarget::Commercial);
        assert_eq!(s.list().len(), 1);
        assert_eq!(s.list()[0].lines, 5);
        std::fs::remove_dir_all(&s.root).ok();
    }

    #[test]
    fn newer_schema_is_refused_with_a_message() {
        let v = serde_json::json!({"schema_version": SCHEMA_VERSION + 1, "id": "x"});
        let e = migrate(v).unwrap_err();
        assert!(e.contains("newer version"));
    }

    #[test]
    fn older_schema_without_new_fields_migrates() {
        let v = serde_json::json!({"schema_version": 1, "id": "old-1", "name": "Old", "script": "A: hi"});
        let p = migrate(v).unwrap();
        assert_eq!(p.schema_version, SCHEMA_VERSION);
        assert_eq!(p.mix.destination, "streaming");
    }

    #[test]
    fn corrupted_file_recovers_from_backup() {
        let s = store();
        let p = Project::new("Recover", true);
        s.save(&p).unwrap();
        s.save(&p).unwrap(); // second save creates the .bak
        std::fs::write(s.dir(&p.id).unwrap().join("project.json"), "{ not json").unwrap();
        assert_eq!(s.load(&p.id).unwrap().name, "Recover");
        std::fs::remove_dir_all(&s.root).ok();
    }

    #[test]
    fn ids_and_paths_are_validated() {
        assert!(valid_id("my-film-1a2b"));
        assert!(!valid_id("../x") && !valid_id("A") && !valid_id(""));
        assert!(safe_relative("takes/take-1.wav"));
        assert!(!safe_relative("../x.wav") && !safe_relative("C:\\x.wav") && !safe_relative("/etc/passwd"));
        let s = store();
        assert!(s.dir("../escape").is_err());
        assert!(s.delete("..").is_err());
        std::fs::remove_dir_all(&s.root).ok();
    }

    #[test]
    fn delete_removes_the_folder() {
        let s = store();
        let p = Project::new("Gone", false);
        s.save(&p).unwrap();
        s.delete(&p.id).unwrap();
        assert!(s.list().is_empty());
        std::fs::remove_dir_all(&s.root).ok();
    }
}
