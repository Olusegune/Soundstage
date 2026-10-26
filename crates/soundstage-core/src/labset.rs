//! Model Lab test sets (`docs/spec/09-MS4-PLAN.md`, task T-4.1).
//!
//! A test set is a folder of short original scripts, each with the direction it is meant to be played in and a list of
//! things a listener should notice. Every set carries a checksum so a result can always be tied to the exact words that
//! produced it. Loading refuses a set that has been changed, or that breaks the rules for what a test script may be.

use crate::script::parse;
use crate::tags::{expected_duration_ms, tokenize};
use crate::util::sha256_hex;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MAX_SCENES: usize = 30;
const MAX_SCENE_BYTES: u64 = 4_000;
const MIN_MS: u64 = 2_000;
const MAX_MS: u64 = 40_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LabScene {
    pub id: String,
    pub title: String,
    /// Path inside the set folder.
    pub file: String,
    /// True when the scene uses words from another language. It may not be used for a language or accent result until
    /// native speakers have checked it.
    #[serde(default)]
    pub needs_native_review: bool,
    pub reference_direction: String,
    pub checklist: Vec<String>,
    /// The script text, filled in when the set is loaded.
    #[serde(default, skip_deserializing)]
    pub script: String,
    /// Estimated length at a normal speaking rate.
    #[serde(default, skip_deserializing)]
    pub approx_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LabSet {
    pub id: String,
    pub version: u32,
    pub title: String,
    pub language: String,
    #[serde(default)]
    pub accent_claim: Option<String>,
    #[serde(default)]
    pub note: String,
    pub scenes: Vec<LabScene>,
    /// The checksum of the files, filled in when the set is loaded.
    #[serde(default, skip_deserializing)]
    pub checksum: String,
}

fn norm(bytes: &[u8]) -> Vec<u8> {
    // The same checksum on every machine, whatever line endings a checkout used.
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// SHA-256 over `set.json` and each scene file in the order the set lists them, with their names.
pub fn checksum(dir: &Path, files: &[String]) -> Result<String, String> {
    let mut all: Vec<u8> = vec![];
    for name in std::iter::once("set.json".to_string()).chain(files.iter().cloned()) {
        let bytes = std::fs::read(dir.join(&name)).map_err(|_| format!("The test set is missing {name}."))?;
        all.extend_from_slice(name.as_bytes());
        all.push(b'\n');
        all.extend_from_slice(&norm(&bytes));
        all.push(b'\n');
    }
    Ok(sha256_hex(&all))
}

fn bad(msg: impl Into<String>) -> String {
    msg.into()
}

pub fn load(dir: &Path) -> Result<LabSet, String> {
    let meta_bytes = std::fs::read(dir.join("set.json")).map_err(|_| bad("That folder is not a test set (no set.json)."))?;
    let mut set: LabSet = serde_json::from_slice(&meta_bytes).map_err(|e| format!("The test set's set.json is damaged: {e}"))?;
    if set.id.is_empty() || !crate::project::valid_id(&set.id) {
        return Err(bad("The test set needs a short id of lower-case letters, digits and dashes."));
    }
    if set.scenes.is_empty() || set.scenes.len() > MAX_SCENES {
        return Err(format!("A test set has between 1 and {MAX_SCENES} scenes."));
    }
    if set.language.is_empty() || set.language.len() > 12 || !set.language.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(bad("The test set's language must be a language code such as en."));
    }
    let files: Vec<String> = set.scenes.iter().map(|s| s.file.clone()).collect();
    for f in &files {
        if !crate::project::safe_relative(f) {
            return Err(format!("{f} is not a file inside the test set."));
        }
    }
    let expected = std::fs::read_to_string(dir.join("CHECKSUM")).map_err(|_| bad("The test set has no CHECKSUM file."))?;
    let actual = checksum(dir, &files)?;
    if expected.trim() != actual {
        return Err(bad("The test set has been changed since its checksum was made. A result must belong to exactly one version of the words."));
    }
    let mut seen: Vec<String> = vec![];
    for s in &mut set.scenes {
        if !crate::project::valid_id(&s.id) || seen.contains(&s.id) {
            return Err(format!("Scene id \"{}\" is not valid or is repeated.", s.id));
        }
        seen.push(s.id.clone());
        if s.title.trim().is_empty() || s.title.chars().count() > 80 || s.reference_direction.trim().is_empty() || s.reference_direction.chars().count() > 400 {
            return Err(format!("Scene \"{}\" needs a short title and a reference direction.", s.id));
        }
        if s.checklist.is_empty() || s.checklist.len() > 6 || s.checklist.iter().any(|c| c.trim().is_empty() || c.chars().count() > 200) {
            return Err(format!("Scene \"{}\" needs between 1 and 6 short checklist items.", s.id));
        }
        let path = dir.join(&s.file);
        if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(u64::MAX) > MAX_SCENE_BYTES {
            return Err(format!("Scene \"{}\" is too long for a listening test.", s.id));
        }
        let raw = std::fs::read(&path).map_err(|_| format!("The test set is missing {}.", s.file))?;
        s.script = String::from_utf8(norm(&raw)).map_err(|_| format!("Scene \"{}\" is not text.", s.id))?;
        let parsed = parse(&s.script);
        if !parsed.issues.is_empty() || parsed.lines.is_empty() {
            return Err(format!("Scene \"{}\" is not a valid script: every line needs a speaker.", s.id));
        }
        if parsed.lines.iter().any(|l| !l.tag_issues.is_empty()) {
            return Err(format!("Scene \"{}\" uses a tag Soundstage does not know.", s.id));
        }
        if let Some(r) = crate::safety::check(&s.script) {
            return Err(format!("Scene \"{}\" breaks a safety rule: {}", s.id, r.message));
        }
        let ms: u64 = parsed.lines.iter().map(|l| expected_duration_ms(&tokenize(&l.text).0)).sum();
        if !(MIN_MS..=MAX_MS).contains(&ms) {
            return Err(format!("Scene \"{}\" should run between 2 and 40 seconds; this one is about {} seconds.", s.id, ms / 1000));
        }
        s.approx_ms = ms;
    }
    set.checksum = actual;
    Ok(set)
}

impl LabSet {
    /// Scenes that can be used for a general result. Scenes that need a native check are left out unless asked for.
    pub fn scenes_ready(&self, include_unreviewed: bool) -> Vec<&LabScene> {
        self.scenes.iter().filter(|s| include_unreviewed || !s.needs_native_review).collect()
    }
}

/// The English set that ships with the app, so the Lab works without any download.
const BUILTIN_ENGLISH: &[(&str, &str)] = &[
    ("set.json", include_str!("../../../fixtures/lab/english-v1/set.json")),
    ("CHECKSUM", include_str!("../../../fixtures/lab/english-v1/CHECKSUM")),
    ("scenes/01-narration.txt", include_str!("../../../fixtures/lab/english-v1/scenes/01-narration.txt")),
    ("scenes/02-calm-conversation.txt", include_str!("../../../fixtures/lab/english-v1/scenes/02-calm-conversation.txt")),
    ("scenes/03-argument.txt", include_str!("../../../fixtures/lab/english-v1/scenes/03-argument.txt")),
    ("scenes/04-grief.txt", include_str!("../../../fixtures/lab/english-v1/scenes/04-grief.txt")),
    ("scenes/05-joy.txt", include_str!("../../../fixtures/lab/english-v1/scenes/05-joy.txt")),
    ("scenes/06-whisper.txt", include_str!("../../../fixtures/lab/english-v1/scenes/06-whisper.txt")),
    ("scenes/07-shout.txt", include_str!("../../../fixtures/lab/english-v1/scenes/07-shout.txt")),
    ("scenes/08-laugh-in-line.txt", include_str!("../../../fixtures/lab/english-v1/scenes/08-laugh-in-line.txt")),
    ("scenes/09-cough.txt", include_str!("../../../fixtures/lab/english-v1/scenes/09-cough.txt")),
    ("scenes/10-gasp.txt", include_str!("../../../fixtures/lab/english-v1/scenes/10-gasp.txt")),
    ("scenes/11-names-and-long-sentence.txt", include_str!("../../../fixtures/lab/english-v1/scenes/11-names-and-long-sentence.txt")),
    ("scenes/12-code-switch.txt", include_str!("../../../fixtures/lab/english-v1/scenes/12-code-switch.txt")),
    ("scenes/13-multi-speaker.txt", include_str!("../../../fixtures/lab/english-v1/scenes/13-multi-speaker.txt")),
];

/// Write the built-in English set under `lab_root/sets/english-v1` if it is not there or has been changed, and return
/// its folder. The copy is checked like any other set when it is loaded.
pub fn install_builtin(lab_root: &Path) -> Result<std::path::PathBuf, String> {
    let dir = lab_root.join("sets").join("english-v1");
    let intact = load(&dir).is_ok();
    if !intact {
        let _ = std::fs::remove_dir_all(&dir);
        for (name, text) in BUILTIN_ENGLISH {
            let p = dir.join(name);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("Could not set up the test set: {e}"))?;
            }
            std::fs::write(&p, text).map_err(|e| format!("Could not set up the test set: {e}"))?;
        }
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::new_id;

    fn fixture() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/lab/english-v1")
    }

    fn copy_set() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(new_id("ss-labset"));
        std::fs::create_dir_all(d.join("scenes")).unwrap();
        for e in std::fs::read_dir(fixture()).unwrap().flatten() {
            if e.path().is_file() {
                std::fs::copy(e.path(), d.join(e.file_name())).unwrap();
            }
        }
        for e in std::fs::read_dir(fixture().join("scenes")).unwrap().flatten() {
            std::fs::copy(e.path(), d.join("scenes").join(e.file_name())).unwrap();
        }
        d
    }

    #[test]
    fn the_shipped_english_set_loads_and_every_scene_is_a_valid_short_script() {
        let set = load(&fixture()).unwrap();
        assert_eq!(set.scenes.len(), 13);
        assert_eq!(set.language, "en");
        for s in &set.scenes {
            assert!(s.approx_ms >= 2_000 && s.approx_ms <= 40_000, "{}: {}", s.id, s.approx_ms);
            assert!(!s.checklist.is_empty());
        }
        assert_eq!(set.scenes_ready(false).len(), 12, "the code-switching scene waits for native review");
        assert_eq!(set.scenes_ready(true).len(), 13);
        assert!(set.scenes.iter().any(|s| s.script.contains("[laugh]")) && set.scenes.iter().any(|s| s.script.contains("[cough]")) && set.scenes.iter().any(|s| s.script.contains("[gasp]")));
        assert_eq!(set.checksum.len(), 64);
    }

    #[test]
    fn a_changed_word_a_missing_file_or_windows_line_endings() {
        let d = copy_set();
        assert!(load(&d).is_ok());
        // Windows line endings do not change the checksum.
        let p = d.join("scenes/02-calm-conversation.txt");
        let t = std::fs::read_to_string(&p).unwrap().replace('\n', "\r\n");
        std::fs::write(&p, t).unwrap();
        assert!(load(&d).is_ok(), "line endings are normalised");
        // A changed word does.
        let t = std::fs::read_to_string(&p).unwrap().replace("tea", "coffee");
        std::fs::write(&p, t).unwrap();
        assert!(load(&d).unwrap_err().contains("has been changed"));
        std::fs::remove_file(&p).unwrap();
        assert!(load(&d).unwrap_err().contains("missing"));
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn rules_for_what_a_test_script_may_be_are_enforced() {
        let d = copy_set();
        let meta = std::fs::read_to_string(d.join("set.json")).unwrap();
        let reseal = |d: &Path, meta: &str, files: &[String]| {
            std::fs::write(d.join("set.json"), meta).unwrap();
            let sum = checksum(d, files).unwrap();
            std::fs::write(d.join("CHECKSUM"), sum).unwrap();
        };
        let files: Vec<String> = serde_json::from_str::<serde_json::Value>(&meta).unwrap()["scenes"].as_array().unwrap().iter().map(|s| s["file"].as_str().unwrap().to_string()).collect();
        // A line with no speaker.
        std::fs::write(d.join("scenes/01-narration.txt"), "This line has no speaker.\n").unwrap();
        reseal(&d, &meta, &files);
        assert!(load(&d).unwrap_err().contains("every line needs a speaker"));
        // An unknown tag.
        std::fs::write(d.join("scenes/01-narration.txt"), "AMA: Hello [giggle] there my friend, this is a test.\n").unwrap();
        reseal(&d, &meta, &files);
        assert!(load(&d).unwrap_err().contains("tag"));
        // A script that breaks a safety rule.
        std::fs::write(d.join("scenes/01-narration.txt"), "AMA: Please send me your password so I can help you today, friend.\n").unwrap();
        reseal(&d, &meta, &files);
        assert!(load(&d).unwrap_err().contains("safety"));
        // Too short.
        std::fs::write(d.join("scenes/01-narration.txt"), "AMA: Hi.\n").unwrap();
        reseal(&d, &meta, &files);
        assert!(load(&d).unwrap_err().contains("between 2 and 40"));
        // A path that leaves the folder.
        let evil = meta.replace("scenes/01-narration.txt", "../../outside.txt");
        std::fs::write(d.join("set.json"), &evil).unwrap();
        assert!(load(&d).unwrap_err().contains("not a file inside"));
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn the_built_in_set_installs_repairs_itself_and_matches_the_files_in_the_repo() {
        let root = std::env::temp_dir().join(new_id("ss-labroot2"));
        std::fs::create_dir_all(&root).unwrap();
        let dir = install_builtin(&root).unwrap();
        let set = load(&dir).unwrap();
        assert_eq!(set.checksum, load(&fixture()).unwrap().checksum, "the embedded copy is the repo copy");
        std::fs::write(dir.join("scenes/01-narration.txt"), "AMA: tampered words for a test run here today.
").unwrap();
        assert!(load(&dir).is_err());
        install_builtin(&root).unwrap();
        assert!(load(&dir).is_ok(), "a changed copy is replaced");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn damaged_or_wrong_folders_give_plain_errors() {
        let d = std::env::temp_dir().join(new_id("ss-labset-x"));
        std::fs::create_dir_all(&d).unwrap();
        assert!(load(&d).unwrap_err().contains("not a test set"));
        std::fs::write(d.join("set.json"), "{nope").unwrap();
        assert!(load(&d).unwrap_err().contains("damaged"));
        std::fs::remove_dir_all(d).ok();
    }
}
