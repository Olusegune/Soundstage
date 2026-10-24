//! Back up a project as one file, and open one.
//!
//! A `.soundstage` file is a zip holding `project.json` and the audio the project refers to (`takes/` and `assets/`). It is
//! for backups, moving to another computer, and handing a scene to someone. It does not carry consent records or keys:
//! those belong to a person and a computer, so cloned voices need their consent set up again where the project is opened.
//!
//! Opening a file is treated as opening something from a stranger. Only the expected names are accepted, nothing is written
//! outside the project's own new folder, sizes are capped as the data is read (not as the file claims), and anything odd
//! stops the whole open and leaves no trace.

use crate::project::{migrate, safe_relative, Project, Store};
use crate::util::{new_id, slugify};
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub const EXTENSION: &str = "soundstage";
const MAX_ENTRIES: usize = 20_000;
const MAX_TOTAL: u64 = 4 * 1024 * 1024 * 1024;
const MAX_AUDIO: u64 = 1024 * 1024 * 1024;
const MAX_JSON: u64 = 20 * 1024 * 1024;

fn wav_name_ok(n: &str) -> bool {
    n.len() <= 120 && n.to_ascii_lowercase().ends_with(".wav") && n.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b' ')) && !n.starts_with('.') && !n.contains("..")
}

/// Is this an entry name a Soundstage project file may contain?
fn entry_ok(name: &str) -> bool {
    if name.is_empty() || name.len() > 140 || name.chars().any(|c| c.is_control() || c == '\\' || c == ':') {
        return false;
    }
    match name {
        "project.json" | "takes/" | "assets/" => true,
        _ => name.strip_prefix("takes/").or_else(|| name.strip_prefix("assets/")).is_some_and(wav_name_ok),
    }
}

/// Write the project at `id` to `dest` (a `.soundstage` file). Returns the size in bytes.
pub fn pack(store: &Store, id: &str, dest: &Path) -> Result<u64, String> {
    let project = store.load(id)?;
    let dir = store.dir(id)?;
    let mut files: Vec<String> = project.takes.iter().map(|t| t.file.clone()).chain(project.assets.iter().map(|a| a.file.clone())).collect();
    files.sort();
    files.dedup();
    let tmp = dest.with_extension("part");
    let result = (|| {
        let mut zip = ZipWriter::new(std::fs::File::create(&tmp).map_err(|e| format!("Could not create the file: {e}"))?);
        let deflate = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("project.json", deflate).map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?.as_bytes()).map_err(|e| e.to_string())?;
        for f in &files {
            let ok = safe_relative(f) && (f.starts_with("takes/") || f.starts_with("assets/")) && entry_ok(f);
            if !ok {
                return Err(format!("The project refers to a file Soundstage will not pack ({f})."));
            }
            let path = dir.join(f);
            // A missing file is skipped: the project still opens, and that line shows as needing a render.
            let Ok(mut src) = std::fs::File::open(&path) else { continue };
            zip.start_file(f.as_str(), SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)).map_err(|e| e.to_string())?;
            std::io::copy(&mut src, &mut zip).map_err(|e| format!("Could not read {f}: {e}"))?;
        }
        zip.finish().map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, dest).map_err(|e| format!("Could not save the file: {e}"))?;
        std::fs::metadata(dest).map(|m| m.len()).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Open a `.soundstage` file as a new project. Returns the new project's id.
pub fn unpack(store: &Store, src: &Path) -> Result<String, String> {
    let bad = || "That is not a Soundstage project file, or it is damaged.".to_string();
    let file = std::fs::File::open(src).map_err(|e| format!("Could not open the file: {e}"))?;
    let mut zip = ZipArchive::new(file).map_err(|_| bad())?;
    if zip.len() == 0 || zip.len() > MAX_ENTRIES {
        return Err(bad());
    }
    // Look at every name before writing anything.
    let mut seen = std::collections::BTreeSet::new();
    let mut has_json = false;
    for i in 0..zip.len() {
        let e = zip.by_index_raw(i).map_err(|_| bad())?;
        let name = e.name().map(|c| c.into_owned()).map_err(|_| bad())?;
        if !entry_ok(&name) || !seen.insert(name.to_ascii_lowercase()) || e.is_symlink() {
            return Err(bad());
        }
        has_json |= name == "project.json";
        if e.size() > if name == "project.json" { MAX_JSON } else { MAX_AUDIO } {
            return Err("That project file holds something far larger than a project should.".into());
        }
    }
    if !has_json {
        return Err(bad());
    }
    let mut json = Vec::new();
    zip.by_name("project.json").map_err(|_| bad())?.take(MAX_JSON + 1).read_to_end(&mut json).map_err(|_| bad())?;
    if json.len() as u64 > MAX_JSON {
        return Err(bad());
    }
    let mut project: Project = migrate(serde_json::from_slice(&json).map_err(|_| bad())?)?;
    for f in project.takes.iter().map(|t| &t.file).chain(project.assets.iter().map(|a| &a.file)) {
        if !(safe_relative(f) && (f.starts_with("takes/") || f.starts_with("assets/")) && entry_ok(f)) {
            return Err(bad());
        }
    }
    let id = format!("{}-{}", slugify(&project.name), &new_id("p")[2..]);
    project.id = id.clone();
    let dir = store.dir(&id)?;
    if dir.exists() {
        return Err("A project with that name already exists. Try again.".into());
    }
    let result = (|| {
        std::fs::create_dir_all(dir.join("takes")).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(dir.join("assets")).map_err(|e| e.to_string())?;
        let mut total = 0u64;
        for i in 0..zip.len() {
            let entry = zip.by_index(i).map_err(|_| bad())?;
            let name = entry.name().map(|c| c.into_owned()).map_err(|_| bad())?;
            if entry.is_dir() || name == "project.json" {
                continue;
            }
            // Read at most what is allowed, whatever the file claims about itself.
            let mut data = Vec::new();
            entry.take(MAX_AUDIO + 1).read_to_end(&mut data).map_err(|_| bad())?;
            total += data.len() as u64;
            if data.len() as u64 > MAX_AUDIO || total > MAX_TOTAL || !data.starts_with(b"RIFF") {
                return Err(bad());
            }
            std::fs::write(dir.join(&name), &data).map_err(|e| format!("Could not write {name}: {e}"))?;
        }
        store.save(&project)?;
        Ok(id.clone())
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Asset, AssetKind, Take, TakeStatus};

    fn tmp() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(new_id("pack-test"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn riff(n: usize) -> Vec<u8> {
        let mut v = b"RIFF".to_vec();
        v.extend((0..n).map(|i| (i % 251) as u8));
        v
    }

    /// A project with one take and one music asset, written into `store`.
    fn sample(store: &Store) -> String {
        let mut p = Project::new("Pack Test", true);
        let dir = store.dir(&p.id).unwrap();
        std::fs::create_dir_all(dir.join("takes")).unwrap();
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        std::fs::write(dir.join("takes/t1.wav"), riff(5000)).unwrap();
        std::fs::write(dir.join("assets/m1.wav"), riff(3000)).unwrap();
        std::fs::write(dir.join("takes/stray.wav"), riff(100)).unwrap();
        p.takes.push(Take { id: "t1".into(), line_key: "s0-l0".into(), speaker: "AMA".into(), text: "x".into(), voice_id: "v".into(), adapter_id: "a".into(), provider_version: "1".into(), cache_key: "k".into(), file: "takes/t1.wav".into(), duration_ms: 500, sample_rate: 22050, direction_summary: String::new(), created: String::new(), status: TakeStatus::Pending, approver: None });
        p.assets.push(Asset { id: "m1".into(), name: "Theme".into(), kind: AssetKind::Music, file: "assets/m1.wav".into(), duration_ms: 400, licence: Default::default(), uses_prohibited: vec![], source_note: String::new(), generated: None });
        p.pronunciations.push(crate::pronounce::Rule { from: "Okafor".into(), to: "Oh-kah-for".into(), ..Default::default() });
        store.save(&p).unwrap();
        p.id
    }

    fn zip_with(path: &Path, entries: &[(&str, Vec<u8>)]) {
        let mut z = ZipWriter::new(std::fs::File::create(path).unwrap());
        for (n, d) in entries {
            z.start_file(*n, SimpleFileOptions::default()).unwrap();
            z.write_all(d).unwrap();
        }
        z.finish().unwrap();
    }

    fn project_json() -> Vec<u8> {
        serde_json::to_vec(&Project::new("Hostile", false)).unwrap()
    }

    #[test]
    fn a_project_round_trips_with_its_audio_and_rules_under_a_new_id() {
        let d = tmp();
        let a = Store::new(d.join("a"));
        let b = Store::new(d.join("b"));
        let id = sample(&a);
        let file = d.join("pack-test.soundstage");
        assert!(pack(&a, &id, &file).unwrap() > 1000);
        let new_id = unpack(&b, &file).unwrap();
        assert_ne!(new_id, id, "an opened file is always a new project, never an overwrite");
        let p = b.load(&new_id).unwrap();
        assert_eq!((p.name.as_str(), p.takes.len(), p.assets.len(), p.pronunciations.len()), ("Pack Test", 1, 1, 1));
        assert_eq!(std::fs::read(b.dir(&new_id).unwrap().join("takes/t1.wav")).unwrap(), riff(5000));
        assert_eq!(std::fs::read(b.dir(&new_id).unwrap().join("assets/m1.wav")).unwrap(), riff(3000));
        assert!(!b.dir(&new_id).unwrap().join("takes/stray.wav").exists(), "files the project does not use are not packed");
        // Opening it again gives another new project.
        assert_ne!(unpack(&b, &file).unwrap(), new_id);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn hostile_archives_are_refused_and_leave_nothing_behind() {
        let d = tmp();
        let store = Store::new(d.join("store"));
        std::fs::create_dir_all(&store.root).unwrap();
        let cases: Vec<(&str, Vec<(&str, Vec<u8>)>)> = vec![
            ("path climbing", vec![("project.json", project_json()), ("../evil.wav", riff(10))]),
            ("climbing inside takes", vec![("project.json", project_json()), ("takes/../../evil.wav", riff(10))]),
            ("an absolute path", vec![("project.json", project_json()), ("/etc/evil.wav", riff(10))]),
            ("a drive letter", vec![("project.json", project_json()), ("C:/evil.wav", riff(10))]),
            ("backslashes", vec![("project.json", project_json()), ("takes\\evil.wav", riff(10))]),
            ("a nested folder", vec![("project.json", project_json()), ("takes/a/b.wav", riff(10))]),
            ("a program", vec![("project.json", project_json()), ("takes/evil.exe", riff(10))]),
            ("an unexpected top-level file", vec![("project.json", project_json()), ("readme.txt", b"hi".to_vec())]),
            ("audio that is not audio", vec![("project.json", project_json()), ("takes/a.wav", b"MZ not riff".to_vec())]),
            ("no project file", vec![("takes/a.wav", riff(10))]),
            ("a damaged project file", vec![("project.json", b"{not json".to_vec())]),
            ("the same name twice in different case", vec![("project.json", project_json()), ("takes/a.wav", riff(10)), ("takes/A.WAV", riff(10))]),
        ];
        for (what, entries) in cases {
            let f = d.join("bad.soundstage");
            zip_with(&f, &entries);
            assert!(unpack(&store, &f).is_err(), "{what} must be refused");
            assert!(std::fs::read_dir(&store.root).unwrap().next().is_none(), "{what} must leave no project folder");
        }
        assert!(!d.join("evil.wav").exists() && !d.join("store").join("evil.wav").exists());
        // A project that points at a file outside its folders.
        let mut p = Project::new("Points Out", false);
        p.takes.push(Take { id: "t".into(), line_key: "k".into(), speaker: "A".into(), text: "x".into(), voice_id: "v".into(), adapter_id: "a".into(), provider_version: "1".into(), cache_key: "k".into(), file: "../../secret.wav".into(), duration_ms: 1, sample_rate: 1, direction_summary: String::new(), created: String::new(), status: TakeStatus::Pending, approver: None });
        let f = d.join("points.soundstage");
        zip_with(&f, &[("project.json", serde_json::to_vec(&p).unwrap())]);
        assert!(unpack(&store, &f).is_err());
        // Not a zip at all, and an empty file.
        std::fs::write(d.join("junk.soundstage"), b"this is not a zip file").unwrap();
        assert!(unpack(&store, &d.join("junk.soundstage")).is_err());
        std::fs::write(d.join("empty.soundstage"), b"").unwrap();
        assert!(unpack(&store, &d.join("empty.soundstage")).is_err());
        assert!(std::fs::read_dir(&store.root).unwrap().next().is_none());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn packing_refuses_a_project_that_points_outside_its_folders() {
        let d = tmp();
        let a = Store::new(d.join("a"));
        let id = sample(&a);
        let mut p = a.load(&id).unwrap();
        p.takes[0].file = "../outside.wav".into();
        a.save(&p).unwrap();
        assert!(pack(&a, &id, &d.join("x.soundstage")).is_err());
        assert!(!d.join("x.soundstage").exists() && !d.join("x.part").exists());
        std::fs::remove_dir_all(d).ok();
    }
}
