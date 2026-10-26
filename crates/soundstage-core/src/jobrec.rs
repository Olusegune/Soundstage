//! Job records (decision D-04): a small file per long operation, saved in the project's `jobs` folder.
//!
//! A render run that is cut short (power loss, crash, the window closed) leaves its takes on disk but
//! not always in the project. The record keeps what was finished, so opening the project again can
//! offer to keep that work and finish the rest instead of paying for it twice.
//!
//! States: `queued`, `running`, `succeeded`, `failed`, `cancelled`, and `interrupted`, which is set
//! when a project is opened and a record is still `queued` or `running` (nothing can be running then).

use crate::project::{valid_id, Take};
use crate::util::{iso_now, new_id};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const KEEP_FINISHED: usize = 20;
const MAX_KEYS: usize = 5000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
}

impl JobState {
    pub fn is_open(self) -> bool {
        matches!(self, JobState::Queued | JobState::Running)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct JobRecord {
    pub id: String,
    /// What kind of work: `render` for now.
    pub kind: String,
    pub state: JobState,
    pub created: String,
    pub updated: String,
    /// Line ids this job was asked to do, in order.
    pub keys: Vec<String>,
    /// Line ids already finished (rendered or reused).
    pub done: Vec<String>,
    /// Line ids that failed, so a resume does not retry them silently.
    pub failed: Vec<String>,
    /// Takes made by this job, kept so they can be adopted after an interruption.
    pub takes: Vec<Take>,
    pub error: Option<String>,
}

impl JobRecord {
    pub fn remaining(&self) -> Vec<String> {
        self.keys.iter().filter(|k| !self.done.contains(k) && !self.failed.contains(k)).cloned().collect()
    }
}

pub struct JobStore {
    dir: PathBuf,
}

impl JobStore {
    /// `project_dir` is the project's own folder; records go in `jobs` inside it.
    pub fn new(project_dir: &Path) -> JobStore {
        JobStore { dir: project_dir.join("jobs") }
    }

    fn path(&self, id: &str) -> Result<PathBuf, String> {
        if !valid_id(id) {
            return Err("That is not a valid job id.".into());
        }
        Ok(self.dir.join(format!("{id}.json")))
    }

    pub fn create(&self, kind: &str, keys: Vec<String>) -> Result<JobRecord, String> {
        if keys.len() > MAX_KEYS {
            return Err("Too many lines in one job.".into());
        }
        let now = iso_now();
        let rec = JobRecord {
            id: new_id("job"),
            kind: kind.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(20).collect(),
            state: JobState::Running,
            created: now.clone(),
            updated: now,
            keys,
            done: vec![],
            failed: vec![],
            takes: vec![],
            error: None,
        };
        self.save(&rec)?;
        Ok(rec)
    }

    /// Write through a temporary file so a crash never leaves half a record.
    pub fn save(&self, rec: &JobRecord) -> Result<(), String> {
        let path = self.path(&rec.id)?;
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("Could not save job progress: {e}"))?;
        let mut rec = rec.clone();
        rec.updated = iso_now();
        let json = serde_json::to_vec_pretty(&rec).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(|e| format!("Could not save job progress: {e}"))?;
        std::fs::rename(&tmp, &path).map_err(|e| format!("Could not save job progress: {e}"))
    }

    pub fn load(&self, id: &str) -> Result<JobRecord, String> {
        let text = std::fs::read_to_string(self.path(id)?).map_err(|_| "That job no longer exists.".to_string())?;
        serde_json::from_str(&text).map_err(|_| "That job record is damaged.".to_string())
    }

    /// Every readable record, newest first. Damaged files are skipped, never fatal.
    pub fn list(&self) -> Vec<JobRecord> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else { return vec![] };
        let mut out: Vec<JobRecord> = rd
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| std::fs::read_to_string(e.path()).ok())
            .filter_map(|t| serde_json::from_str(&t).ok())
            .collect();
        out.sort_by(|a, b| b.created.cmp(&a.created));
        out
    }

    /// Called when a project is opened: anything still open was cut short. Returns those records.
    pub fn mark_interrupted(&self) -> Vec<JobRecord> {
        let mut out = vec![];
        for mut rec in self.list() {
            if rec.state.is_open() {
                rec.state = JobState::Interrupted;
                if self.save(&rec).is_ok() {
                    out.push(rec);
                }
            }
        }
        out
    }

    /// Interrupted jobs the user has not dealt with yet.
    pub fn pending(&self) -> Vec<JobRecord> {
        self.list().into_iter().filter(|r| r.state == JobState::Interrupted).collect()
    }

    pub fn finish(&self, id: &str, state: JobState, error: Option<String>) -> Result<(), String> {
        let mut rec = self.load(id)?;
        rec.state = state;
        rec.error = error.map(|e| e.chars().take(300).collect());
        self.save(&rec)?;
        self.prune();
        Ok(())
    }

    pub fn discard(&self, id: &str) -> Result<(), String> {
        let path = self.path(id)?;
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| format!("Could not remove the job record: {e}"))?;
        }
        Ok(())
    }

    /// Keep the folder small: only the newest finished records stay. Open and interrupted ones never go.
    fn prune(&self) {
        let finished: Vec<JobRecord> = self.list().into_iter().filter(|r| !r.state.is_open() && r.state != JobState::Interrupted).collect();
        for old in finished.into_iter().skip(KEEP_FINISHED) {
            let _ = self.discard(&old.id);
        }
    }
}

/// What adopting an interrupted job's finished work changed in the project.
#[derive(Debug, Default, Serialize, PartialEq)]
pub struct Adopted {
    pub new_takes: Vec<Take>,
    pub selected: std::collections::BTreeMap<String, String>,
}

/// Put takes a cut-short job finished back into the project. A take is kept only if its file is really
/// there, sits inside the project folder, and the project does not already have it. A line that already
/// has a chosen take keeps it.
pub fn adopt(project: &mut crate::project::Project, project_dir: &Path, rec: &JobRecord) -> Adopted {
    let mut out = Adopted::default();
    for t in &rec.takes {
        if project.take(&t.id).is_some() || !crate::project::safe_relative(&t.file) || !project_dir.join(&t.file).is_file() {
            continue;
        }
        project.takes.push(t.clone());
        out.new_takes.push(t.clone());
        if project.selected_take(&t.line_key).is_none() {
            project.selected.insert(t.line_key.clone(), t.id.clone());
            out.selected.insert(t.line_key.clone(), t.id.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (JobStore, PathBuf) {
        let dir = std::env::temp_dir().join(new_id("ss-jobs"));
        std::fs::create_dir_all(&dir).unwrap();
        (JobStore::new(&dir), dir)
    }

    #[test]
    fn a_job_that_was_running_when_the_app_died_becomes_interrupted_with_its_progress() {
        let (s, dir) = store();
        let mut rec = s.create("render", vec!["a".into(), "b".into(), "c".into()]).unwrap();
        rec.done.push("a".into());
        s.save(&rec).unwrap();
        // The app dies here. A new run of the app opens the project.
        let s2 = JobStore::new(&dir);
        let cut = s2.mark_interrupted();
        assert_eq!(cut.len(), 1);
        assert_eq!(cut[0].state, JobState::Interrupted);
        assert_eq!(cut[0].remaining(), vec!["b", "c"]);
        assert_eq!(s2.pending().len(), 1);
        // Opening again does not report it twice as newly cut short.
        assert!(s2.mark_interrupted().is_empty());
        assert_eq!(s2.pending().len(), 1);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn finished_jobs_are_not_pending_and_old_ones_are_pruned() {
        let (s, dir) = store();
        for _ in 0..(KEEP_FINISHED + 5) {
            let r = s.create("render", vec![]).unwrap();
            s.finish(&r.id, JobState::Succeeded, None).unwrap();
        }
        assert!(s.pending().is_empty());
        assert!(s.list().len() <= KEEP_FINISHED + 1);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn failed_and_cancelled_states_are_kept_and_errors_are_short() {
        let (s, dir) = store();
        let r = s.create("render", vec!["a".into()]).unwrap();
        s.finish(&r.id, JobState::Failed, Some("x".repeat(1000))).unwrap();
        let back = s.load(&r.id).unwrap();
        assert_eq!(back.state, JobState::Failed);
        assert_eq!(back.error.unwrap().chars().count(), 300);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn bad_ids_and_damaged_files_are_refused_not_followed() {
        let (s, dir) = store();
        assert!(s.load("../../etc/passwd").is_err());
        assert!(s.discard("..\\x").is_err());
        std::fs::create_dir_all(dir.join("jobs")).unwrap();
        std::fs::write(dir.join("jobs").join("junk.json"), "{not json").unwrap();
        assert!(s.list().is_empty());
        assert!(s.mark_interrupted().is_empty());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn adopting_keeps_real_files_only_and_never_overrides_a_choice() {
        use crate::project::{Project, TakeStatus};
        let (_s, dir) = store();
        std::fs::create_dir_all(dir.join("takes")).unwrap();
        std::fs::write(dir.join("takes/real.wav"), b"x").unwrap();
        let mk = |id: &str, key: &str, file: &str| Take {
            id: id.into(), line_key: key.into(), speaker: "A".into(), text: "t".into(), voice_id: "v".into(), adapter_id: "a".into(),
            provider_version: "1".into(), cache_key: "k".into(), file: file.into(), duration_ms: 1, sample_rate: 1,
            direction_summary: String::new(), created: String::new(), status: TakeStatus::default(), approver: None,
        };
        let mut rec = JobRecord { id: "job-1".into(), kind: "render".into(), state: JobState::Interrupted, created: String::new(), updated: String::new(),
            keys: vec![], done: vec![], failed: vec![], takes: vec![mk("t1", "k1", "takes/real.wav"), mk("t2", "k2", "takes/missing.wav"), mk("t3", "k3", "../escape.wav"), mk("t4", "k4", "takes/real.wav")], error: None };
        let mut p = Project::new("A", false);
        p.takes.push(mk("t4", "k4", "takes/real.wav"));
        p.takes.push(mk("t0", "k1", "takes/real.wav"));
        p.selected.insert("k1".into(), "t0".into());
        let a = adopt(&mut p, &dir, &rec);
        assert_eq!(a.new_takes.len(), 1, "only the take with a real file that the project lacked is adopted");
        assert_eq!(a.new_takes[0].id, "t1");
        assert_eq!(p.selected.get("k1").unwrap(), "t0", "an existing choice is kept");
        assert!(a.selected.is_empty());
        rec.takes.clear();
        assert_eq!(adopt(&mut p, &dir, &rec), Adopted::default());
        std::fs::remove_dir_all(dir).ok();
    }
}
