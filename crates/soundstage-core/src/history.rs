//! A record of what the Speak and Audio tools screens made, so a good take is not lost when the window closes.
//!
//! Each item is a WAV copy plus a few words about how it was made. Everything stays on this computer, in a folder outside
//! every project. The list is capped by count and by size; the oldest items go first.

use crate::util::{iso_now, new_id};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MAX_ITEMS: usize = 200;
pub const MAX_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HistoryItem {
    pub id: String,
    pub created: String,
    /// `speak` or `clean`.
    pub kind: String,
    pub voice_id: String,
    pub voice_name: String,
    pub provider_label: String,
    pub text: String,
    pub style: String,
    pub pace: f32,
    pub duration_ms: u64,
}

pub struct HistoryStore {
    dir: PathBuf,
}

fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

impl HistoryStore {
    pub fn new(dir: PathBuf) -> Self {
        HistoryStore { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn index(&self) -> PathBuf {
        self.dir.join("index.json")
    }

    pub fn file_for(&self, id: &str) -> Option<PathBuf> {
        safe_id(id).then(|| self.dir.join(format!("{id}.wav")))
    }

    /// Newest first. An unreadable index is treated as empty rather than stopping the app.
    pub fn list(&self) -> Vec<HistoryItem> {
        let mut v: Vec<HistoryItem> = std::fs::read(self.index()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        v.retain(|i| self.file_for(&i.id).is_some_and(|f| f.is_file()));
        v.sort_by(|a, b| b.created.cmp(&a.created));
        v
    }

    fn save(&self, items: &[HistoryItem]) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("Could not create the history folder: {e}"))?;
        let tmp = self.index().with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(items).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, self.index()).map_err(|e| e.to_string())
    }

    /// Keep a copy of `wav` with its description. Returns the stored item and the path of the stored copy.
    pub fn add(&self, mut item: HistoryItem, wav: &Path) -> Result<(HistoryItem, PathBuf), String> {
        item.id = new_id("h");
        item.created = iso_now();
        item.text = item.text.chars().filter(|c| !c.is_control() || *c == '\n').take(5000).collect();
        item.voice_name = item.voice_name.chars().take(120).collect();
        item.provider_label = item.provider_label.chars().take(80).collect();
        if !["speak", "clean", "convert", "edit"].contains(&item.kind.as_str()) {
            return Err("Unknown history kind.".into());
        }
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("Could not create the history folder: {e}"))?;
        let dest = self.file_for(&item.id).ok_or("Bad id.")?;
        std::fs::copy(wav, &dest).map_err(|e| format!("Could not keep a copy: {e}"))?;
        let mut items = self.list();
        items.insert(0, item.clone());
        self.trim(&mut items);
        self.save(&items)?;
        Ok((item, dest))
    }

    /// Drop the oldest items until the list is within its limits.
    fn trim(&self, items: &mut Vec<HistoryItem>) {
        let size = |i: &HistoryItem| self.file_for(&i.id).and_then(|f| std::fs::metadata(f).ok()).map_or(0, |m| m.len());
        while items.len() > MAX_ITEMS || (items.len() > 1 && items.iter().map(size).sum::<u64>() > MAX_BYTES) {
            if let Some(old) = items.pop() {
                if let Some(f) = self.file_for(&old.id) {
                    let _ = std::fs::remove_file(f);
                }
            }
        }
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let f = self.file_for(id).ok_or("That history item is not valid.")?;
        let mut items = self.list();
        items.retain(|i| i.id != id);
        let _ = std::fs::remove_file(f);
        self.save(&items)
    }

    pub fn clear(&self) -> Result<(), String> {
        for i in self.list() {
            if let Some(f) = self.file_for(&i.id) {
                let _ = std::fs::remove_file(f);
            }
        }
        self.save(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(new_id("hist-test"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn item(text: &str) -> HistoryItem {
        HistoryItem { id: String::new(), created: String::new(), kind: "speak".into(), voice_id: "v".into(), voice_name: "Ada".into(), provider_label: "Windows".into(), text: text.into(), style: "neutral".into(), pace: 1.0, duration_ms: 1000 }
    }

    fn wav(d: &Path, name: &str) -> PathBuf {
        let p = d.join(name);
        std::fs::write(&p, vec![1u8; 2000]).unwrap();
        p
    }

    #[test]
    fn items_are_kept_listed_newest_first_and_deleted() {
        let d = tmp();
        let h = HistoryStore::new(d.join(".history"));
        let (a, pa) = h.add(item("first"), &wav(&d, "a.wav")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let (b, _) = h.add(item("second"), &wav(&d, "b.wav")).unwrap();
        assert!(pa.is_file());
        assert_eq!(h.list().iter().map(|i| i.text.as_str()).collect::<Vec<_>>(), vec!["second", "first"]);
        h.delete(&a.id).unwrap();
        assert!(!pa.exists() && h.list().len() == 1);
        h.clear().unwrap();
        assert!(h.list().is_empty() && !h.file_for(&b.id).unwrap().exists());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn ids_cannot_escape_and_bad_kinds_are_refused() {
        let d = tmp();
        let h = HistoryStore::new(d.join(".history"));
        assert!(h.delete("../../x").is_err() && h.file_for("a/b").is_none() && h.file_for("").is_none());
        let mut bad = item("x");
        bad.kind = "other".into();
        assert!(h.add(bad, &wav(&d, "a.wav")).is_err());
        assert!(h.add(item("x"), &d.join("missing.wav")).is_err(), "a missing file is an error, not an empty entry");
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn the_oldest_items_go_first_when_over_the_limit() {
        let d = tmp();
        let h = HistoryStore::new(d.join(".history"));
        let w = wav(&d, "a.wav");
        for i in 0..(MAX_ITEMS + 3) {
            h.add(item(&format!("t{i}")), &w).unwrap();
        }
        let list = h.list();
        assert_eq!(list.len(), MAX_ITEMS);
        let files = std::fs::read_dir(h.dir()).unwrap().filter(|e| e.as_ref().unwrap().path().extension().is_some_and(|x| x == "wav")).count();
        assert_eq!(files, MAX_ITEMS, "files of dropped items are deleted too");
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn damaged_or_orphaned_entries_are_ignored() {
        let d = tmp();
        let h = HistoryStore::new(d.join(".history"));
        std::fs::create_dir_all(h.dir()).unwrap();
        std::fs::write(h.dir().join("index.json"), b"not json").unwrap();
        assert!(h.list().is_empty());
        let (it, p) = h.add(item("ok"), &wav(&d, "a.wav")).unwrap();
        std::fs::remove_file(p).unwrap();
        assert!(h.list().iter().all(|i| i.id != it.id), "an entry whose audio is gone is not listed");
        std::fs::remove_dir_all(d).ok();
    }
}
