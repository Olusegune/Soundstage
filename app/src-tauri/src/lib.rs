//! Thin desktop shell. All decisions (licence gate, consent, render, mix) live in
//! `soundstage-core`; this file only exposes them to the UI and handles files and keys.

use serde::{Deserialize, Serialize};
use soundstage_core::exporter::{self, ExportError, ExportResult};
use soundstage_core::output::{Aspect, Codec, ExportOptions};
use soundstage_core::layout::{self, Layout};
use soundstage_core::licence::{GateReport, LicenceInfo};
use soundstage_core::loudness::{Destination, DESTINATIONS};
use soundstage_core::generate::{clean_prompt, GenKind, GenLimits, GenRequest};
use soundstage_core::licence::LicenceInfo as Lic;
use soundstage_core::project::{Asset, AssetKind, GenInfo, Project, Store, Summary, Take};
use soundstage_core::jobrec::{self, JobState, JobStore};
use soundstage_core::director::{self, Estimate, LineChange, Proposal};
use soundstage_core::plan::{LinePlan, PerformancePlan, PlanStore};
use soundstage_core::adapters::openai_speech::OpenAiSpeechAdapter;
use soundstage_core::engines::{valid_engine_id, EnginesConfig, VoiceDef};
use soundstage_core::providers::{KeyProvider, Registry, Voice};
use soundstage_core::render::{self, Analysis, RenderResult};
use soundstage_core::util::{new_id, safe_file_stem};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

struct AppState {
    store: Arc<Store>,
    registry: Arc<RegistryCell>,
}

type Shared = (Arc<Store>, Arc<RegistryCell>);

/// Credentials come from the Windows Credential Manager, read on every call. Adapters never touch the store.
fn key_provider() -> KeyProvider {
    Arc::new(|name| {
        if !valid_key_name(name) {
            return None;
        }
        keyring::Entry::new("Soundstage", name).ok()?.get_password().ok()
    })
}

/// Only names Soundstage itself defines can reach the key store: `elevenlabs`, or `engine:<id>` for an engine the user added.
fn valid_key_name(name: &str) -> bool {
    matches!(name, "elevenlabs" | "fal" | "kie" | "assistant" | "voicekiller" | "voicekiller:userid") || name.strip_prefix("engine:").is_some_and(valid_engine_id)
}

/// Where the offline speech program is. It is installed with Soundstage, beside the app. A developer can point
/// `SOUNDSTAGE_WHISPER_DIR` at another folder.
fn whisper_exe() -> PathBuf {
    if let Some(d) = std::env::var_os("SOUNDSTAGE_WHISPER_DIR") {
        return PathBuf::from(d).join("whisper-cli.exe");
    }
    let dir = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())).unwrap_or_default();
    [dir.join("resources").join("whisper").join("whisper-cli.exe"), dir.join("whisper").join("whisper-cli.exe")].into_iter().find(|p| p.is_file()).unwrap_or_else(|| dir.join("resources").join("whisper").join("whisper-cli.exe"))
}

/// Builds the registry on first use and again after a key is saved or removed.
struct RegistryCell {
    inner: Mutex<Option<Arc<Registry>>>,
    config_path: PathBuf,
}

impl RegistryCell {
    fn new(config_path: PathBuf) -> Self {
        RegistryCell { inner: Mutex::new(None), config_path }
    }
    fn get(&self) -> Arc<Registry> {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        // A damaged engines file must never stop the app from starting; Settings shows what is wrong with it.
        g.get_or_insert_with(|| Arc::new(Registry::standard(key_provider(), &EnginesConfig::load(&self.config_path).unwrap_or_default()).with_clones(self.consent_store().list()).with_pronunciations(self.pronunciation_rules()).with_whisper(whisper_exe(), self.models_dir()))).clone()
    }
    /// Consent records live beside the projects folder's other private data, never inside a project.
    fn consent_store(&self) -> soundstage_core::clone::ConsentStore {
        let root = self.config_path.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf()).unwrap_or_default();
        soundstage_core::clone::ConsentStore::new(root.join(".consent"))
    }
    fn models_dir(&self) -> PathBuf {
        self.config_dir().parent().map(|p| p.join(".models")).unwrap_or_default()
    }
    fn config_dir(&self) -> PathBuf {
        self.config_path.parent().map(|p| p.to_path_buf()).unwrap_or_default()
    }
    fn pronunciation_path(&self) -> PathBuf {
        self.config_dir().join("pronunciation.json")
    }
    /// App-wide pronunciation rules. A damaged file means no rules, never a failure to start.
    fn pronunciation_rules(&self) -> Vec<soundstage_core::pronounce::Rule> {
        std::fs::read(self.pronunciation_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }
    fn history_store(&self) -> soundstage_core::history::HistoryStore {
        soundstage_core::history::HistoryStore::new(self.config_dir().parent().map(|p| p.join(".history")).unwrap_or_default())
    }
    fn reset(&self) {
        *self.inner.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

fn shared(s: &State<'_, AppState>) -> Shared {
    (s.store.clone(), s.registry.clone())
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| format!("The task stopped unexpectedly: {e}"))
}

#[derive(Serialize)]
struct AppInfo {
    version: &'static str,
    projects_root: String,
}

#[tauri::command]
fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo { version: env!("CARGO_PKG_VERSION"), projects_root: state.store.root.to_string_lossy().to_string() }
}

#[tauri::command]
fn list_projects(state: State<'_, AppState>) -> Vec<Summary> {
    state.store.list()
}

#[tauri::command]
fn create_project(state: State<'_, AppState>, name: String, sample: bool) -> Result<Project, String> {
    let p = Project::new(&name, sample);
    state.store.save(&p)?;
    Ok(p)
}

#[tauri::command]
fn load_project(state: State<'_, AppState>, id: String) -> Result<Project, String> {
    state.store.load(&id)
}

#[tauri::command]
fn save_project(state: State<'_, AppState>, project: Project) -> Result<(), String> {
    state.store.save(&project)
}

#[tauri::command]
fn delete_project(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.store.delete(&id)
}

#[tauri::command]
fn project_dir(state: State<'_, AppState>, id: String) -> Result<String, String> {
    Ok(state.store.dir(&id)?.to_string_lossy().to_string())
}

#[derive(Serialize)]
struct VoiceView {
    #[serde(flatten)]
    voice: Voice,
    /// Canonical tags this voice can perform (pauses always work).
    tags: Vec<String>,
    provider_label: String,
    /// True when the line's text is sent over the network to render it.
    sends_data: bool,
}

#[tauri::command]
async fn list_voices(state: State<'_, AppState>) -> Result<Vec<VoiceView>, String> {
    let (_, reg) = shared(&state);
    blocking(move || {
        let reg = reg.get();
        let reg = &*reg;
        reg.voices
            .iter()
            .map(|v| {
                let a = reg.adapter(&v.provider);
                VoiceView {
                    voice: v.clone(),
                    tags: a.map(|a| a.capabilities().tags).unwrap_or_default(),
                    provider_label: a.map(|a| a.label().to_string()).unwrap_or_default(),
                    sends_data: a.map(|a| a.sends_data()).unwrap_or(false),
                }
            })
            .collect()
    })
    .await
}

#[derive(Serialize)]
struct Preview {
    path: String,
}

/// Render a throwaway sample into the preview folder. Nothing is added to any project.
#[tauri::command]
async fn preview_voice(state: State<'_, AppState>, voice_id: String, text: String) -> Result<Preview, String> {
    let (store, reg) = shared(&state);
    let dir = store.root.join(".preview");
    blocking(move || {
        let reg = reg.get();
        let reg = &*reg;
        let mut p = Project::new("preview", false);
        p.script = format!("SAMPLE: {}", text.chars().take(300).collect::<String>().replace(['\n', '\r'], " "));
        p.characters.insert("SAMPLE".into(), soundstage_core::project::Character { voice_id: Some(voice_id), ..Default::default() });
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let r = render::render_line(&mut p, reg, &dir, "s0-l0").map_err(|e| e.message)?;
        let take = p.take(&r.take_id).ok_or("The preview was not created.")?;
        Ok(Preview { path: dir.join(&take.file).to_string_lossy().to_string() })
    })
    .await?
}

#[derive(Serialize)]
struct MixPreview {
    path: String,
    /// 0..1 peak envelope of the whole mix, for drawing.
    peaks: Vec<f32>,
    loudness: soundstage_core::loudness::Loudness,
    target_lufs: f64,
    gain_db: f64,
    target_reached: bool,
    warnings: Vec<String>,
    total_ms: u64,
}

/// Mix the project to a temporary file so it can be heard and metered before export.
#[tauri::command]
async fn mix_preview(state: State<'_, AppState>, project: Project) -> Result<MixPreview, String> {
    let (store, _) = shared(&state);
    let dir = store.dir(&project.id)?;
    let preview_dir = store.root.join(".preview");
    blocking(move || {
        let (out, dest, layout) = exporter::render_mix(&project, &dir).map_err(|e| match e {
            ExportError::Failed { message } => message,
            _ => "The mix could not be built.".to_string(),
        })?;
        std::fs::create_dir_all(&preview_dir).map_err(|e| e.to_string())?;
        let path = preview_dir.join("mix.wav");
        soundstage_core::audio::write_wav_24(&path, &out.master)?;
        Ok(MixPreview {
            path: path.to_string_lossy().to_string(),
            peaks: soundstage_core::audio::peaks_of(&[&out.master.l, &out.master.r], 900),
            loudness: out.loudness,
            target_lufs: dest.target_lufs,
            gain_db: out.gain_db,
            target_reached: out.target_reached,
            warnings: out.warnings,
            total_ms: layout.total_ms,
        })
    })
    .await?
}

/// The project from the screen has no plan in it (the plan lives in `plan.json`), so attach the saved one.
fn with_plan(store: &Store, mut project: Project) -> Project {
    if let Ok(dir) = store.dir(&project.id) {
        project.plan = PlanStore::new(&dir).load().unwrap_or_default();
    }
    project
}

#[tauri::command]
async fn analyze_project(state: State<'_, AppState>, project: Project) -> Result<Analysis, String> {
    let (store, reg) = shared(&state);
    let project = with_plan(&store, project);
    blocking(move || render::analyze(&project, &reg.get())).await
}

#[derive(Serialize)]
struct LineOutcome {
    key: String,
    ok: bool,
    cached: bool,
    error_kind: Option<String>,
    message: Option<String>,
    result: Option<RenderResult>,
}

/// What a render run changed. The UI merges this into its own copy, so edits made
/// while a render was running are never overwritten.
#[derive(Serialize)]
struct RenderOutcome {
    new_takes: Vec<Take>,
    selected: BTreeMap<String, String>,
    results: Vec<LineOutcome>,
}

#[tauri::command]
async fn render_lines(app: AppHandle, state: State<'_, AppState>, project: Project, keys: Vec<String>, job_id: Option<String>) -> Result<RenderOutcome, String> {
    let (store, reg) = shared(&state);
    let dir = store.dir(&project.id)?;
    let project = with_plan(&store, project);
    blocking(move || {
        let reg = reg.get();
        let reg = &*reg;
        let jobs = JobStore::new(&dir);
        // Progress notes must never stop a render, so a missing or damaged record is simply ignored.
        let mut job = job_id.as_deref().and_then(|id| jobs.load(id).ok());
        let mut p = project;
        let before = p.takes.len();
        let mut noted = before;
        let mut selected = BTreeMap::new();
        let mut results = Vec::new();
        for key in keys {
            let _ = app.emit("render-progress", serde_json::json!({ "key": key, "status": "rendering" }));
            match render::render_line(&mut p, reg, &dir, &key) {
                Ok(r) => {
                    if let Some(id) = p.selected.get(&key) {
                        selected.insert(key.clone(), id.clone());
                    }
                    let _ = app.emit("render-progress", serde_json::json!({ "key": key, "status": "rendered" }));
                    if let Some(j) = job.as_mut() {
                        j.takes.extend(p.takes[noted..].iter().cloned());
                        noted = p.takes.len();
                        j.done.push(key.clone());
                        let _ = jobs.save(j);
                    }
                    results.push(LineOutcome { key, ok: true, cached: r.cached, error_kind: None, message: None, result: Some(r) });
                }
                Err(e) => {
                    let _ = app.emit("render-progress", serde_json::json!({ "key": key, "status": "failed" }));
                    if let Some(j) = job.as_mut() {
                        j.failed.push(key.clone());
                        let _ = jobs.save(j);
                    }
                    results.push(LineOutcome { key, ok: false, cached: false, error_kind: Some(e.kind), message: Some(e.message), result: None });
                }
            }
        }
        RenderOutcome { new_takes: p.takes[before..].to_vec(), selected, results }
    })
    .await
}

/// Other voices that could stand in on a line that failed or cannot be done. Nothing is called here.
#[tauri::command]
fn fallback_options(state: State<'_, AppState>, project: Project, line_key: String) -> Vec<render::FallbackOption> {
    let (store, reg) = shared(&state);
    let project = with_plan(&store, project);
    render::fallback_options(&project, &reg.get(), &line_key, soundstage_core::net::local_only())
}

/// Try one line with other voices (at most four). Each voice makes its own take. The cast and the chosen take do not
/// change, so the person decides afterwards. Voices that send text over the internet need `confirmed`.
#[tauri::command]
async fn audition_line(state: State<'_, AppState>, project: Project, line_key: String, voice_ids: Vec<String>, confirmed: bool) -> Result<RenderOutcome, String> {
    let (store, reg) = shared(&state);
    let dir = store.dir(&project.id)?;
    let project = with_plan(&store, project);
    blocking(move || {
        let reg = reg.get();
        let reg = &*reg;
        let sends = voice_ids.iter().any(|v| reg.adapter_for_voice(v).is_some_and(|a| a.sends_data()));
        if sends && !confirmed {
            return Err("Confirm what will be sent before the audition runs.".to_string());
        }
        let mut p = project;
        let before = p.takes.len();
        let results = render::audition_line(&mut p, reg, &dir, &line_key, &voice_ids)
            .into_iter()
            .map(|(voice, r)| match r {
                Ok(r) => LineOutcome { key: voice, ok: true, cached: r.cached, error_kind: None, message: None, result: Some(r) },
                Err(e) => LineOutcome { key: voice, ok: false, cached: false, error_kind: Some(e.kind), message: Some(e.message), result: None },
            })
            .collect();
        Ok(RenderOutcome { new_takes: p.takes[before..].to_vec(), selected: BTreeMap::new(), results })
    })
    .await?
}

/// Start a record for a batch of lines, so a cut-short run can be picked up again.
#[tauri::command]
fn job_begin(state: State<'_, AppState>, project_id: String, keys: Vec<String>) -> Result<String, String> {
    let dir = state.store.dir(&project_id)?;
    Ok(JobStore::new(&dir).create("render", keys)?.id)
}

#[tauri::command]
fn job_finish(state: State<'_, AppState>, project_id: String, job_id: String, outcome: String) -> Result<(), String> {
    let dir = state.store.dir(&project_id)?;
    let st = match outcome.as_str() {
        "succeeded" => JobState::Succeeded,
        "cancelled" => JobState::Cancelled,
        _ => JobState::Failed,
    };
    JobStore::new(&dir).finish(&job_id, st, None)
}

#[derive(Serialize)]
struct PendingJob {
    id: String,
    created: String,
    total: usize,
    done: usize,
    failed: usize,
    remaining: Vec<String>,
    kept_takes: usize,
}

/// Work that was cut short the last time this project was open. Anything still marked running is
/// marked interrupted first, since nothing can be running while a project is being opened.
#[tauri::command]
fn pending_jobs(state: State<'_, AppState>, project_id: String) -> Result<Vec<PendingJob>, String> {
    let dir = state.store.dir(&project_id)?;
    let jobs = JobStore::new(&dir);
    jobs.mark_interrupted();
    Ok(jobs
        .pending()
        .into_iter()
        .map(|j| PendingJob { id: j.id.clone(), created: j.created.clone(), total: j.keys.len(), done: j.done.len(), failed: j.failed.len(), remaining: j.remaining(), kept_takes: j.takes.len() })
        .collect())
}

/// Keep the audio an interrupted run finished, then close the record. The UI merges the result like a render.
#[tauri::command]
fn recover_job(state: State<'_, AppState>, project: Project, job_id: String) -> Result<jobrec::Adopted, String> {
    let dir = state.store.dir(&project.id)?;
    let jobs = JobStore::new(&dir);
    let rec = jobs.load(&job_id)?;
    let mut p = project;
    let adopted = jobrec::adopt(&mut p, &dir, &rec);
    jobs.discard(&job_id)?;
    Ok(adopted)
}

/// Peak envelope of a take or asset inside a project, for drawing waveforms.
#[tauri::command]
async fn waveform(state: State<'_, AppState>, project_id: String, file: String, bins: usize) -> Result<Vec<f32>, String> {
    let dir = state.store.dir(&project_id)?;
    if !soundstage_core::project::safe_relative(&file) {
        return Err("That is not a file inside the project.".into());
    }
    blocking(move || soundstage_core::audio::peaks(&dir.join(file), bins)).await?
}

#[tauri::command]
fn compute_layout(project: Project) -> Layout {
    layout::compute(&project, &project.parsed())
}

#[tauri::command]
fn destinations() -> Vec<Destination> {
    DESTINATIONS.to_vec()
}

#[tauri::command]
async fn check_export(state: State<'_, AppState>, project: Project) -> Result<GateReport, String> {
    let (_, reg) = shared(&state);
    blocking(move || exporter::check(&project, &reg.get())).await
}

#[derive(Serialize)]
struct FormatInfo {
    kind: &'static str,
    label: String,
    needs_ffmpeg: bool,
    available: bool,
}

#[derive(Serialize)]
struct ExportCaps {
    ffmpeg: Option<soundstage_core::output::Ffmpeg>,
    /// True when the user chose a file and it could not be used (so the UI can say so).
    configured_failed: bool,
    formats: Vec<FormatInfo>,
}

/// Which delivery formats this computer can write. FFmpeg is found automatically, or taken from the user's choice.
#[tauri::command]
async fn export_capabilities(ffmpeg_path: Option<String>) -> Result<ExportCaps, String> {
    blocking(move || {
        let configured = ffmpeg_path.as_deref().is_some_and(|p| !p.trim().is_empty());
        let ff = exporter::ffmpeg_status(ffmpeg_path.as_deref());
        let reps: [(&'static str, Codec); 7] = [
            ("wav", Codec::Wav { bits: 24 }), ("flac", Codec::Flac), ("mp3", Codec::Mp3 { kbps: 192 }), ("aac", Codec::Aac { kbps: 192 }),
            ("vorbis", Codec::Vorbis { quality: 5 }), ("opus", Codec::Opus { kbps: 128 }), ("mp4", Codec::Mp4 { aspect: Aspect::Wide, kbps: 192 }),
        ];
        let formats = reps
            .iter()
            .map(|(k, c)| FormatInfo { kind: k, label: c.label(), needs_ffmpeg: c.needs_ffmpeg(), available: !c.needs_ffmpeg() || ff.as_ref().is_some_and(|f| f.supports(c)) })
            .collect();
        ExportCaps { configured_failed: configured && ff.is_none(), ffmpeg: ff, formats }
    })
    .await
}

#[tauri::command]
async fn export_project(state: State<'_, AppState>, project: Project, dest: String, acknowledged: bool, options: Option<ExportOptions>) -> Result<ExportResult, ExportError> {
    let (store, reg) = shared(&state);
    let dir = store.dir(&project.id).map_err(|m| ExportError::Failed { message: m })?;
    let options = options.unwrap_or_default();
    blocking(move || exporter::export_with(&project, &reg.get(), &dir, &PathBuf::from(dest), acknowledged, &options))
        .await
        .map_err(|m| ExportError::Failed { message: m })?
}

const MAX_ASSET_BYTES: u64 = 500 * 1024 * 1024;

/// Bring a file into the project as 48 kHz stereo 24-bit WAV (WAV, MP3, FLAC, Ogg and M4A are read). The original is
/// never touched. The licence starts as `unknown`, so it cannot reach a commercial export until the user records where
/// its terms were checked.
#[tauri::command]
async fn import_asset(state: State<'_, AppState>, project_id: String, path: String, kind: AssetKind) -> Result<Asset, String> {
    let (store, _) = shared(&state);
    let dir = store.dir(&project_id)?;
    blocking(move || {
        let src = PathBuf::from(&path);
        let meta = std::fs::metadata(&src).map_err(|e| format!("Could not open that file: {e}"))?;
        if !meta.is_file() {
            return Err("Choose a file, not a folder.".to_string());
        }
        if meta.len() > MAX_ASSET_BYTES {
            return Err("That file is larger than 500 MB.".to_string());
        }
        let ext = src.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
        if !soundstage_core::decode::IMPORT_EXTENSIONS.contains(&ext.as_str()) {
            return Err("Soundstage can import WAV, MP3, FLAC, Ogg and M4A files.".to_string());
        }
        let stereo = soundstage_core::audio::to_mix_format(&soundstage_core::decode::decode_file(&src)?);
        if stereo.is_empty() {
            return Err("That file has no audio in it.".to_string());
        }
        let id = new_id("asset");
        let rel = format!("assets/{id}.wav");
        std::fs::create_dir_all(dir.join("assets")).map_err(|e| e.to_string())?;
        soundstage_core::output::write_wav(&dir.join(&rel), &stereo, soundstage_core::audio::SR, 24)?;
        let name = src.file_stem().map(|s| safe_file_stem(&s.to_string_lossy())).unwrap_or_else(|| "Imported audio".into());
        let duration_ms = stereo.len() as u64 * 1000 / soundstage_core::audio::SR as u64;
        Ok(Asset { id, name, kind, file: rel, duration_ms, licence: LicenceInfo::default(), uses_prohibited: vec![], source_note: String::new(), generated: None })
    })
    .await?
}

#[derive(Serialize)]
struct GeneratorView {
    id: String,
    label: String,
    kinds: Vec<GenKind>,
    sends_data: bool,
    ready: bool,
    music: GenLimits,
    sfx: GenLimits,
    licence: Lic,
    notes: String,
}

/// Rewrite a rough idea as a clear description, using the chat server the user set up. Returns text only.
#[tauri::command]
async fn improve_prompt(base_url: String, model: String, kind: soundstage_core::assistant::AssistKind, draft: String) -> Result<String, String> {
    blocking(move || {
        let key = key_provider()("assistant");
        soundstage_core::assistant::improve(&base_url, &model, key.as_deref(), kind, &draft)
    })
    .await?
}

#[derive(Serialize)]
struct CloneInfo {
    statement: &'static str,
    performance_statement: &'static str,
    min_seconds: f32,
    good_seconds: f32,
    max_seconds: f32,
}

#[tauri::command]
fn clone_info() -> CloneInfo {
    CloneInfo { statement: soundstage_core::clone::STATEMENT, performance_statement: soundstage_core::clone::PERFORMANCE_STATEMENT, min_seconds: soundstage_core::clone::MIN_REFERENCE_S, good_seconds: soundstage_core::clone::GOOD_REFERENCE_S, max_seconds: soundstage_core::clone::MAX_REFERENCE_S }
}

fn load_reference(path: &str) -> Result<soundstage_core::audio::Stereo, String> {
    let src = PathBuf::from(path);
    let meta = std::fs::metadata(&src).map_err(|e| format!("Could not open that file: {e}"))?;
    if !meta.is_file() {
        return Err("Choose a file, not a folder.".to_string());
    }
    if meta.len() > MAX_ASSET_BYTES {
        return Err("That file is larger than 500 MB.".to_string());
    }
    let ext = src.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
    if !soundstage_core::decode::IMPORT_EXTENSIONS.contains(&ext.as_str()) {
        return Err("Choose a WAV, MP3, FLAC, Ogg or M4A recording.".to_string());
    }
    Ok(soundstage_core::audio::to_mix_format(&soundstage_core::decode::decode_file(&src)?))
}

/// Judge a recording before anything is sent. Nothing leaves this computer here.
#[tauri::command]
async fn clone_check_reference(path: String) -> Result<soundstage_core::clone::RefReport, String> {
    blocking(move || Ok(soundstage_core::clone::check_reference(&load_reference(&path)?))).await?
}

/// Create a voice from a recording, with the owner's recorded consent. The record is saved on this computer first; only
/// the trimmed recording and a display name are sent to the provider.
#[tauri::command]
async fn clone_voice(state: State<'_, AppState>, adapter_id: String, path: String, name: String, attestation: soundstage_core::clone::Attestation) -> Result<soundstage_core::clone::ClonedVoice, String> {
    let (_, reg) = shared(&state);
    let cell = reg.clone();
    let out = blocking(move || {
        let rec = soundstage_core::clone::build_record(&attestation, &soundstage_core::clone::ConsentStore::new_record_id()).map_err(|p| p.join(" "))?;
        let audio = load_reference(&path)?;
        let report = soundstage_core::clone::check_reference(&audio);
        if !report.problems.is_empty() {
            return Err(report.problems.join(" "));
        }
        let wav = soundstage_core::clone::encode_sample(&audio)?;
        let registry = reg.get();
        let adapter = registry.adapter(&adapter_id).ok_or("That provider is not available.")?;
        if !adapter.ready() {
            return Err("Save this provider's key first.".to_string());
        }
        soundstage_core::clone::run_clone(adapter, &reg.consent_store(), rec, &name, wav)
    })
    .await??;
    cell.reset();
    Ok(out)
}

/// Use a voice that was cloned on the provider's own website. Needs the same recorded consent as a clone made here.
#[derive(Serialize)]
struct SpeakOut {
    path: String,
    duration_ms: u64,
    voice_name: String,
    /// Set when a copy was kept in the history.
    history_id: Option<String>,
}

/// Say some text in one voice, outside any project (the Speak screen). Goes through the same render path as a project
/// line, so the consent gate, safety rules and tag checks all apply.
#[tauri::command]
async fn speak_text(state: State<'_, AppState>, voice_id: String, text: String, emotion: Option<String>, pace: Option<f32>, keep: bool) -> Result<SpeakOut, String> {
    let (store, reg) = shared(&state);
    let dir = store.root.join(".preview");
    blocking(move || {
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if text.is_empty() {
            return Err("Type something to say.".to_string());
        }
        if text.chars().count() > 5000 {
            return Err("Keep it under 5000 characters. Use a project for longer scripts.".to_string());
        }
        let registry = reg.get();
        let (voice_name, provider_label) = registry.voice(&voice_id).map(|v| (v.name.clone(), registry.adapter(&v.provider).map(|a| a.label().to_string()).unwrap_or_default())).ok_or("That voice is not available.")?;
        let (style, speed) = (emotion.clone().unwrap_or_else(|| "neutral".into()), pace.unwrap_or(1.0));
        let voice_for_history = voice_id.clone();
        let mut p = Project::new("speak", false);
        p.script = format!("SAMPLE: {text}");
        p.characters.insert("SAMPLE".into(), soundstage_core::project::Character { voice_id: Some(voice_id), ..Default::default() });
        let emotion = emotion.filter(|e| e != "neutral" && soundstage_core::tags::DELIVERY.contains(&e.as_str()));
        let pace = pace.filter(|x| x.is_finite()).map(|x| x.clamp(0.7, 1.4));
        if emotion.is_some() || pace.is_some() {
            p.line_directions.insert("s0-l0".into(), soundstage_core::direction::Direction { emotion, pace, ..Default::default() });
        }
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let r = render::render_line(&mut p, &registry, &dir, "s0-l0").map_err(|e| e.message)?;
        let take = p.take(&r.take_id).ok_or("The audio was not created.")?;
        let made = dir.join(&take.file);
        if keep {
            let item = soundstage_core::history::HistoryItem { id: String::new(), created: String::new(), kind: "speak".into(), voice_id: voice_for_history, voice_name: voice_name.clone(), provider_label, text: text.clone(), style, pace: speed, duration_ms: take.duration_ms };
            // History is a convenience: if keeping a copy fails, the audio is still returned.
            if let Ok((item, stored)) = reg.history_store().add(item, &made) {
                return Ok(SpeakOut { path: stored.to_string_lossy().to_string(), duration_ms: take.duration_ms, voice_name, history_id: Some(item.id) });
            }
        }
        Ok(SpeakOut { path: made.to_string_lossy().to_string(), duration_ms: take.duration_ms, voice_name, history_id: None })
    })
    .await?
}

fn codec_for_extension(ext: &str) -> Option<Codec> {
    Some(match ext {
        "wav" => Codec::Wav { bits: 24 },
        "flac" => Codec::Flac,
        "mp3" => Codec::Mp3 { kbps: 192 },
        "m4a" => Codec::Aac { kbps: 192 },
        "ogg" => Codec::Vorbis { quality: 5 },
        "opus" => Codec::Opus { kbps: 128 },
        "mp4" => Codec::Mp4 { aspect: Aspect::Wide, kbps: 192 },
        _ => return None,
    })
}

/// Save a generated audio file where the user chose, in the format its extension names. Only files Soundstage made in
/// its own preview folder can be copied, so this cannot be used to read other files.
#[tauri::command]
async fn save_audio_copy(state: State<'_, AppState>, src: String, dest: String, ffmpeg_path: Option<String>) -> Result<String, String> {
    let (store, _) = shared(&state);
    let preview = store.root.join(".preview");
    let history = store.root.join(".history");
    blocking(move || {
        let src = std::fs::canonicalize(&src).map_err(|_| "That audio is no longer available. Make it again.".to_string())?;
        std::fs::create_dir_all(&preview).map_err(|e| e.to_string())?;
        let preview = std::fs::canonicalize(&preview).map_err(|e| e.to_string())?;
        let history = std::fs::canonicalize(&history).unwrap_or_else(|_| preview.clone());
        if !src.starts_with(&preview) && !src.starts_with(&history) {
            return Err("Only audio made in Soundstage can be saved from here.".to_string());
        }
        let dest = PathBuf::from(dest);
        let ext = dest.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
        let codec = codec_for_extension(&ext).ok_or("Choose a file name ending in .wav, .mp3, .flac, .m4a, .ogg, .opus or .mp4.")?;
        if dest.parent().map_or(true, |d| !d.is_dir()) {
            return Err("That folder does not exist.".to_string());
        }
        let buf = soundstage_core::audio::to_mix_format(&soundstage_core::decode::decode_file(&src)?);
        let ff = exporter::ffmpeg_status(ffmpeg_path.as_deref());
        let stem = dest.with_extension("");
        let title = dest.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        soundstage_core::output::write_audio(&buf, &stem, &codec, 48_000, ff.as_ref(), &title, &preview).map(|p| p.to_string_lossy().to_string())
    })
    .await?
}

/// Save a transcript or subtitle file where the user chose.
#[tauri::command]
async fn save_text_copy(dest: String, content: String) -> Result<(), String> {
    blocking(move || {
        let dest = PathBuf::from(dest);
        let ext = dest.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
        if !["txt", "srt", "vtt"].contains(&ext.as_str()) {
            return Err("Choose a file name ending in .txt, .srt or .vtt.".to_string());
        }
        if content.len() > 5 * 1024 * 1024 {
            return Err("That text is too large to save.".to_string());
        }
        if dest.parent().map_or(true, |d| !d.is_dir()) {
            return Err("That folder does not exist.".to_string());
        }
        std::fs::write(&dest, content).map_err(|e| format!("Could not save: {e}"))
    })
    .await?
}

#[derive(Serialize)]
struct ToolView {
    id: String,
    label: String,
    ready: bool,
    transcribe: bool,
    isolate: bool,
    convert: bool,
    sends_data: bool,
}

#[tauri::command]
async fn list_audio_tools(state: State<'_, AppState>) -> Result<Vec<ToolView>, String> {
    let (_, reg) = shared(&state);
    blocking(move || {
        reg.get()
            .adapters
            .iter()
            .filter(|a| a.can_transcribe() || a.can_isolate() || a.can_convert_voice())
            .map(|a| ToolView { id: a.id().into(), label: a.label().into(), ready: a.ready(), transcribe: a.can_transcribe(), isolate: a.can_isolate(), convert: a.can_convert_voice(), sends_data: a.sends_data() })
            .collect()
    })
    .await
}

/// Turn a recording into text and subtitles. The audio is converted on this computer and sent to the engine you chose.
#[tauri::command]
async fn transcribe_audio(state: State<'_, AppState>, adapter_id: String, path: String, language: Option<String>) -> Result<soundstage_core::transcribe::TranscriptOut, String> {
    let (_, reg) = shared(&state);
    blocking(move || {
        let audio = load_reference(&path)?;
        let (wav, seconds) = soundstage_core::transcribe::prepare_audio(&audio)?;
        let registry = reg.get();
        let a = registry.adapter(&adapter_id).filter(|a| a.can_transcribe()).ok_or("That engine cannot transcribe speech.")?;
        let t = a.transcribe(&wav, language.as_deref()).map_err(|e| e.to_string())?;
        Ok(soundstage_core::transcribe::finish(t, seconds))
    })
    .await?
}

#[derive(Serialize)]
struct CleanOut {
    path: String,
    duration_ms: u64,
}

/// Keep only the speech in a recording (removes noise and music). The result is a new file; the original is untouched.
#[tauri::command]
async fn clean_audio(state: State<'_, AppState>, adapter_id: String, path: String, keep: bool) -> Result<CleanOut, String> {
    let (store, reg) = shared(&state);
    let dir = store.root.join(".preview");
    blocking(move || {
        let audio = load_reference(&path)?;
        let wav = soundstage_core::transcribe::prepare_isolation(&audio)?;
        let registry = reg.get();
        let a = registry.adapter(&adapter_id).filter(|a| a.can_isolate()).ok_or("That engine cannot clean audio.")?;
        let (mono, rate) = a.isolate_audio(&wav).map_err(|e| e.to_string())?;
        let up = soundstage_core::audio::resample(&mono, rate, soundstage_core::audio::SR);
        let stereo = soundstage_core::audio::Stereo { l: up.clone(), r: up };
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let out = dir.join(format!("{}.wav", new_id("clean")));
        soundstage_core::output::write_wav(&out, &stereo, soundstage_core::audio::SR, 24)?;
        let duration_ms = stereo.len() as u64 * 1000 / soundstage_core::audio::SR as u64;
        if keep {
            let name = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let item = soundstage_core::history::HistoryItem { id: String::new(), created: String::new(), kind: "clean".into(), voice_id: String::new(), voice_name: "Cleaned audio".into(), provider_label: a.label().to_string(), text: name, style: "neutral".into(), pace: 1.0, duration_ms };
            if let Ok((_, stored)) = reg.history_store().add(item, &out) {
                return Ok(CleanOut { path: stored.to_string_lossy().to_string(), duration_ms });
            }
        }
        Ok(CleanOut { path: out.to_string_lossy().to_string(), duration_ms })
    })
    .await?
}

#[derive(Serialize)]
struct HistoryView {
    item: soundstage_core::history::HistoryItem,
    path: String,
}

#[tauri::command]
async fn history_list(state: State<'_, AppState>) -> Result<Vec<HistoryView>, String> {
    let (_, reg) = shared(&state);
    blocking(move || {
        let h = reg.history_store();
        h.list().into_iter().filter_map(|item| h.file_for(&item.id).map(|p| HistoryView { path: p.to_string_lossy().to_string(), item })).collect()
    })
    .await
}

#[tauri::command]
async fn history_delete(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let (_, reg) = shared(&state);
    blocking(move || reg.history_store().delete(&id)).await?
}

#[tauri::command]
async fn history_clear(state: State<'_, AppState>) -> Result<(), String> {
    let (_, reg) = shared(&state);
    blocking(move || reg.history_store().clear()).await?
}

/// App-wide pronunciation rules.
#[tauri::command]
async fn pronunciation_get(state: State<'_, AppState>) -> Result<Vec<soundstage_core::pronounce::Rule>, String> {
    let (_, reg) = shared(&state);
    blocking(move || reg.pronunciation_rules()).await
}

/// What the voice would be given for some text, with the rules shown on screen (not yet saved).
#[tauri::command]
async fn pronunciation_preview(text: String, project_rules: Vec<soundstage_core::pronounce::Rule>, app_rules: Vec<soundstage_core::pronounce::Rule>) -> Result<String, String> {
    blocking(move || {
        let all: Vec<soundstage_core::pronounce::Rule> = project_rules.into_iter().chain(app_rules).collect();
        soundstage_core::pronounce::validate_all(&all)?;
        Ok(soundstage_core::pronounce::apply(&text.chars().take(500).collect::<String>(), &all))
    })
    .await?
}

#[tauri::command]
async fn pronunciation_save(state: State<'_, AppState>, rules: Vec<soundstage_core::pronounce::Rule>) -> Result<(), String> {
    let (_, reg) = shared(&state);
    let cell = reg.clone();
    blocking(move || {
        soundstage_core::pronounce::validate_all(&rules)?;
        let path = reg.pronunciation_path();
        std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new("."))).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&rules).map_err(|e| e.to_string())?).map_err(|e| format!("Could not save: {e}"))?;
        std::fs::rename(&tmp, &path).map_err(|e| format!("Could not save: {e}"))
    })
    .await??;
    cell.reset();
    Ok(())
}

/// Re-speak a recording in another voice, keeping its words, timing and feeling. The person speaking in the recording must
/// have agreed (a consent record for that use is saved first), and a cloned target voice must have clean consent too.
#[tauri::command]
async fn change_voice(state: State<'_, AppState>, voice_id: String, path: String, attestation: soundstage_core::clone::Attestation, keep: bool) -> Result<CleanOut, String> {
    let (store, reg) = shared(&state);
    let dir = store.root.join(".preview");
    blocking(move || {
        let registry = reg.get();
        let voice = registry.voice(&voice_id).ok_or("That voice is not available.")?.clone();
        let adapter = registry.adapter(&voice.provider).filter(|a| a.can_convert_voice()).ok_or("That voice cannot be used for changing voices.")?;
        if let Some(problems) = registry.consent_problems(&voice_id, attestation.commercial) {
            return Err(format!("{} cannot be used: {}", voice.name, problems.join(" ")));
        }
        let rec = soundstage_core::clone::build_performance_record(&attestation, &soundstage_core::clone::ConsentStore::new_record_id()).map_err(|p| p.join(" "))?;
        let proof = soundstage_core::consent::proof(&rec, &soundstage_core::consent::Use::PerformanceReference, rec.commercial_use_permitted).map_err(|p| p.join(" "))?;
        let audio = load_reference(&path)?;
        let wav = soundstage_core::transcribe::prepare_isolation(&audio)?;
        // The record is kept before anything is sent, as for a clone.
        reg.consent_store().save_record(&rec)?;
        let (mono, rate) = adapter.convert_voice(&wav, &voice_id, &proof).map_err(|e| e.to_string())?;
        let up = soundstage_core::audio::resample(&mono, rate, soundstage_core::audio::SR);
        let stereo = soundstage_core::audio::Stereo { l: up.clone(), r: up };
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let out = dir.join(format!("{}.wav", new_id("convert")));
        soundstage_core::output::write_wav(&out, &stereo, soundstage_core::audio::SR, 24)?;
        let duration_ms = stereo.len() as u64 * 1000 / soundstage_core::audio::SR as u64;
        if keep {
            let name = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let item = soundstage_core::history::HistoryItem { id: String::new(), created: String::new(), kind: "convert".into(), voice_id: voice_id.clone(), voice_name: voice.name.clone(), provider_label: adapter.label().to_string(), text: format!("{name} (performer consent {})", rec.id), style: "neutral".into(), pace: 1.0, duration_ms };
            if let Ok((_, stored)) = reg.history_store().add(item, &out) {
                return Ok(CleanOut { path: stored.to_string_lossy().to_string(), duration_ms });
            }
        }
        Ok(CleanOut { path: out.to_string_lossy().to_string(), duration_ms })
    })
    .await?
}

/// Translate subtitle lines with the chat server set in Settings. The result is shown for the person to edit.
#[tauri::command]
async fn translate_cues(base_url: String, model: String, target: String, lines: Vec<String>) -> Result<Vec<String>, String> {
    blocking(move || {
        let key = key_provider()("assistant");
        soundstage_core::assistant::translate_lines(&base_url, &model, key.as_deref(), &target, &lines)
    })
    .await?
}

/// Subtitle text from cues (original or translated, possibly edited).
#[tauri::command]
fn cues_subtitles(cues: Vec<soundstage_core::output::SubCue>, kind: String) -> Result<String, String> {
    soundstage_core::transcribe::subtitles(&cues, &kind)
}

#[derive(Serialize)]
struct AudioInfo {
    duration_ms: u64,
    peaks: Vec<f32>,
}

/// Length and a rough waveform for a file chosen in the Audio editor.
#[tauri::command]
async fn audio_info(path: String) -> Result<AudioInfo, String> {
    blocking(move || {
        let a = load_reference(&path)?;
        let bins = 160usize;
        let step = (a.len() / bins).max(1);
        let peaks = (0..bins).map(|b| (b * step..((b + 1) * step).min(a.len())).fold(0f32, |m, i| m.max(a.l[i].abs().max(a.r[i].abs())))).collect();
        Ok(AudioInfo { duration_ms: a.len() as u64 * 1000 / soundstage_core::audio::SR as u64, peaks })
    })
    .await?
}

#[derive(Deserialize)]
struct EditClipIn {
    path: String,
    #[serde(flatten)]
    params: soundstage_core::editor::ClipParams,
}

/// Trim, place and layer clips into one new file. Sources may be any audio file the user chooses or anything in the
/// history; the originals are never changed.
#[tauri::command]
async fn edit_audio(state: State<'_, AppState>, clips: Vec<EditClipIn>, normalize: bool, keep: bool) -> Result<CleanOut, String> {
    let (store, reg) = shared(&state);
    let dir = store.root.join(".preview");
    blocking(move || {
        if clips.is_empty() || clips.len() > soundstage_core::editor::MAX_CLIPS {
            return Err(format!("Use between 1 and {} clips.", soundstage_core::editor::MAX_CLIPS));
        }
        let mut loaded = Vec::with_capacity(clips.len());
        for c in &clips {
            loaded.push((load_reference(&c.path)?, c.params.clone()));
        }
        let r = soundstage_core::editor::render(&loaded, normalize)?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let out = dir.join(format!("{}.wav", new_id("edit")));
        soundstage_core::output::write_wav(&out, &r.audio, soundstage_core::audio::SR, 24)?;
        let duration_ms = r.audio.len() as u64 * 1000 / soundstage_core::audio::SR as u64;
        if keep {
            let names: Vec<String> = clips.iter().take(4).map(|c| std::path::Path::new(&c.path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()).collect();
            let item = soundstage_core::history::HistoryItem { id: String::new(), created: String::new(), kind: "edit".into(), voice_id: String::new(), voice_name: format!("Edit of {} clip{}", clips.len(), if clips.len() == 1 { "" } else { "s" }), provider_label: "On this computer".into(), text: names.join(", "), style: "neutral".into(), pace: 1.0, duration_ms };
            if let Ok((_, stored)) = reg.history_store().add(item, &out) {
                return Ok(CleanOut { path: stored.to_string_lossy().to_string(), duration_ms });
            }
        }
        Ok(CleanOut { path: out.to_string_lossy().to_string(), duration_ms })
    })
    .await?
}

#[derive(Serialize)]
struct ModelView {
    id: &'static str,
    label: &'static str,
    blurb: &'static str,
    mb: u64,
    installed: bool,
    english_only: bool,
}

#[derive(Serialize)]
struct WhisperStatus {
    program: bool,
    models: Vec<ModelView>,
}

#[tauri::command]
async fn whisper_status(state: State<'_, AppState>) -> Result<WhisperStatus, String> {
    let (_, reg) = shared(&state);
    blocking(move || {
        let dir = reg.models_dir();
        WhisperStatus {
            program: whisper_exe().is_file(),
            models: soundstage_core::adapters::whisper::MODELS.iter().map(|m| ModelView { id: m.id, label: m.label, blurb: m.blurb, mb: m.bytes / 1_000_000, installed: soundstage_core::adapters::whisper::is_installed(&dir, m), english_only: m.english_only }).collect(),
        }
    })
    .await
}

static DOWNLOADING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(Clone, Serialize)]
struct Progress {
    id: String,
    done: u64,
    total: u64,
}

/// Download one offline speech model. Only the fixed list can be fetched, from its fixed host, and the file is kept only if
/// its size and SHA-256 match.
#[tauri::command]
async fn whisper_download(app: AppHandle, state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    let spec = soundstage_core::adapters::whisper::model(&model_id).ok_or("That model is not on the list.")?;
    if DOWNLOADING.swap(true, Ordering::SeqCst) {
        return Err("A download is already running. Wait for it to finish.".into());
    }
    let (_, reg) = shared(&state);
    let id = model_id.clone();
    let res = blocking(move || {
        let dest = soundstage_core::adapters::whisper::model_path(&reg.models_dir(), spec);
        soundstage_core::net::download_to_file(spec.url, soundstage_core::adapters::whisper::MODEL_HOSTS, spec.bytes, spec.sha256, &dest, &mut |done, total| {
            let _ = app.emit("whisper-progress", Progress { id: id.clone(), done, total });
        })
    })
    .await;
    DOWNLOADING.store(false, Ordering::SeqCst);
    res?
}

#[tauri::command]
async fn whisper_remove(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    let spec = soundstage_core::adapters::whisper::model(&model_id).ok_or("That model is not on the list.")?;
    let (_, reg) = shared(&state);
    blocking(move || {
        let p = soundstage_core::adapters::whisper::model_path(&reg.models_dir(), spec);
        if p.exists() { std::fs::remove_file(&p).map_err(|e| format!("Could not remove it: {e}")) } else { Ok(()) }
    })
    .await?
}

/// Save a project, with its audio, as one `.soundstage` file.
#[tauri::command]
async fn backup_project(state: State<'_, AppState>, id: String, dest: String) -> Result<u64, String> {
    let (store, _) = shared(&state);
    blocking(move || {
        let dest = PathBuf::from(dest);
        if dest.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() != Some(soundstage_core::projectpack::EXTENSION) {
            return Err("Choose a file name ending in .soundstage.".to_string());
        }
        if dest.parent().map_or(true, |d| !d.is_dir()) {
            return Err("That folder does not exist.".to_string());
        }
        // Never write inside the projects folder itself, where it could be mistaken for a project.
        if let (Ok(root), Some(parent)) = (std::fs::canonicalize(&store.root), dest.parent().and_then(|p| std::fs::canonicalize(p).ok())) {
            if parent.starts_with(&root) {
                return Err("Save the backup outside the Soundstage projects folder.".to_string());
            }
        }
        soundstage_core::projectpack::pack(&store, &id, &dest)
    })
    .await?
}

/// Open a `.soundstage` file as a new project. The original file is not changed.
#[tauri::command]
async fn open_project_file(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let (store, _) = shared(&state);
    blocking(move || {
        let src = PathBuf::from(path);
        if src.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() != Some(soundstage_core::projectpack::EXTENSION) {
            return Err("Choose a .soundstage project file.".to_string());
        }
        soundstage_core::projectpack::unpack(&store, &src)
    })
    .await?
}

#[tauri::command]
async fn register_clone(state: State<'_, AppState>, adapter_id: String, name: String, attestation: soundstage_core::clone::Attestation) -> Result<soundstage_core::clone::ClonedVoice, String> {
    let (_, reg) = shared(&state);
    let cell = reg.clone();
    let out = blocking(move || {
        let rec = soundstage_core::clone::build_record(&attestation, &soundstage_core::clone::ConsentStore::new_record_id()).map_err(|p| p.join(" "))?;
        let registry = reg.get();
        let adapter = registry.adapter(&adapter_id).ok_or("That provider is not available.")?;
        soundstage_core::clone::register_clone(adapter, &reg.consent_store(), rec, &name)
    })
    .await??;
    cell.reset();
    Ok(out)
}

#[tauri::command]
async fn consent_list(state: State<'_, AppState>) -> Result<Vec<soundstage_core::clone::CloneEntry>, String> {
    let (_, reg) = shared(&state);
    blocking(move || reg.consent_store().list()).await
}

/// Withdraw consent. New renders and exports with the voice stop at once. When asked, the voice is also deleted from the
/// provider; if that fails, consent is still revoked and the message says so.
#[tauri::command]
async fn consent_revoke(state: State<'_, AppState>, consent_id: String, delete_remote: bool) -> Result<String, String> {
    let (_, reg) = shared(&state);
    let cell = reg.clone();
    let msg = blocking(move || {
        let voice = reg.consent_store().revoke(&consent_id)?;
        let mut msg = format!("Consent revoked. {} can no longer be used in Soundstage.", voice.name);
        if delete_remote {
            let registry = reg.get();
            match registry.adapter(&voice.adapter_id).map(|a| a.delete_voice(&voice.voice_id)) {
                Some(Ok(())) => msg.push_str(" The voice was also deleted from the provider."),
                Some(Err(e)) => msg.push_str(&format!(" It could not be deleted from the provider ({e}); delete it there yourself.")),
                None => msg.push_str(" The provider is not available, so delete the voice there yourself."),
            }
        }
        Ok::<String, String>(msg)
    })
    .await??;
    cell.reset();
    Ok(msg)
}

#[tauri::command]
async fn list_generators(state: State<'_, AppState>) -> Result<Vec<GeneratorView>, String> {
    let (_, reg) = shared(&state);
    blocking(move || {
        reg.get()
            .generators
            .iter()
            .map(|g| GeneratorView { id: g.id().into(), label: g.label().into(), kinds: g.kinds(), sends_data: g.sends_data(), ready: g.ready(), music: g.limits(GenKind::Music), sfx: g.limits(GenKind::Sfx), licence: g.licence(), notes: g.notes() })
            .collect()
    })
    .await
}

/// Make music or an effect and add it to the project as an asset. It starts as `licence unknown`, carries a record of
/// exactly how it was made, and is marked AI-generated in every export manifest.
#[tauri::command]
async fn generate_audio(state: State<'_, AppState>, project_id: String, generator_id: String, kind: GenKind, prompt: String, seconds: Option<f32>, instrumental: bool, name: Option<String>) -> Result<Asset, String> {
    let (store, reg) = shared(&state);
    let dir = store.dir(&project_id)?;
    blocking(move || {
        let prompt = clean_prompt(&prompt)?;
        let reg = reg.get();
        let g = reg.generator(&generator_id).ok_or("That generator is not available.")?;
        if !g.kinds().contains(&kind) {
            return Err("That generator cannot make this kind of sound.".to_string());
        }
        if !g.ready() {
            return Err("Set this generator up first (save its key or start its server).".to_string());
        }
        let out = g.generate(&GenRequest { kind, prompt: prompt.clone(), seconds, instrumental }).map_err(|e| e.to_string())?;
        let id = new_id("asset");
        let rel = format!("assets/{id}.wav");
        std::fs::create_dir_all(dir.join("assets")).map_err(|e| e.to_string())?;
        soundstage_core::output::write_wav(&dir.join(&rel), &out.audio, soundstage_core::audio::SR, 24)?;
        let title = name.map(|n| safe_file_stem(&n)).filter(|n| !n.is_empty() && n != "untitled").unwrap_or_else(|| prompt.chars().take(40).collect::<String>().trim().to_string());
        Ok(Asset {
            id,
            name: title,
            kind: if kind == GenKind::Music { AssetKind::Music } else { AssetKind::Sfx },
            file: rel,
            duration_ms: out.audio.len() as u64 * 1000 / soundstage_core::audio::SR as u64,
            licence: g.licence(),
            uses_prohibited: vec![],
            source_note: format!("Generated by {} ({}).", g.label(), out.provider_version),
            generated: Some(GenInfo { generator: g.id().to_string(), provider_version: out.provider_version, prompt: prompt.chars().take(300).collect(), created: soundstage_core::util::iso_now() }),
        })
    })
    .await?
}

#[tauri::command]
async fn generator_test(state: State<'_, AppState>, generator_id: String) -> Result<String, String> {
    let (_, reg) = shared(&state);
    blocking(move || {
        let reg = reg.get();
        let g = reg.generator(&generator_id).ok_or("Save the engine first, then test it.")?;
        g.check_connection().map(|_| if g.sends_data() { "A key is saved. The first creation will confirm it works.".to_string() } else { "Connected. The server answered.".to_string() }).map_err(|e| e.to_string())
    })
    .await?
}

#[derive(Serialize)]
struct DesignerView {
    id: String,
    label: String,
    ready: bool,
    design: bool,
    clone: bool,
    register: bool,
}

#[tauri::command]
async fn list_designers(state: State<'_, AppState>) -> Result<Vec<DesignerView>, String> {
    let (_, reg) = shared(&state);
    blocking(move || {
        reg.get().adapters.iter().filter(|a| a.capabilities().voice_design || a.capabilities().cloning || a.can_register_clone()).map(|a| DesignerView { id: a.id().into(), label: a.label().into(), ready: a.ready(), design: a.capabilities().voice_design, clone: a.capabilities().cloning, register: a.can_register_clone() }).collect()
    })
    .await
}

#[derive(Serialize)]
struct PreviewOut {
    generated_voice_id: String,
    path: String,
}

/// Three candidate voices from a description. They are written to the preview folder, not to any project.
#[tauri::command]
async fn design_voice(state: State<'_, AppState>, adapter_id: String, description: String, sample_text: Option<String>) -> Result<Vec<PreviewOut>, String> {
    let (store, reg) = shared(&state);
    let dir = store.root.join(".preview");
    blocking(move || {
        let reg = reg.get();
        let a = reg.adapter(&adapter_id).ok_or("That provider is not available.")?;
        let prev = a.design_voice(&soundstage_core::providers::DesignRequest { description, sample_text }).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let tag = new_id("design");
        prev.into_iter()
            .enumerate()
            .map(|(i, p)| {
                let path = dir.join(format!("{tag}-{i}.wav"));
                soundstage_core::audio::write_wav_16_mono(&path, &p.samples, p.sample_rate)?;
                Ok(PreviewOut { generated_voice_id: p.generated_voice_id, path: path.to_string_lossy().to_string() })
            })
            .collect::<Result<Vec<_>, String>>()
    })
    .await?
}

#[tauri::command]
async fn save_designed_voice(state: State<'_, AppState>, adapter_id: String, generated_voice_id: String, name: String, description: String) -> Result<String, String> {
    let (_, reg) = shared(&state);
    let cell = reg.clone();
    let id = blocking(move || {
        let reg = reg.get();
        let a = reg.adapter(&adapter_id).ok_or("That provider is not available.")?;
        a.save_designed_voice(&generated_voice_id, &name, &description).map(|v| v.id).map_err(|e| e.to_string())
    })
    .await??;
    // The new voice has to appear in the library, so the next lookup rebuilds the registry.
    cell.reset();
    Ok(id)
}

// Provider keys live in the Windows Credential Manager and are never returned to the UI.
fn entry(name: &str) -> Result<keyring::Entry, String> {
    if !valid_key_name(name) {
        return Err("Unknown provider.".into());
    }
    keyring::Entry::new("Soundstage", name).map_err(|e| format!("Could not reach the Windows Credential Manager: {e}"))
}

#[derive(Deserialize)]
struct KeyArgs {
    provider: String,
    key: String,
}

#[tauri::command]
fn set_api_key(state: State<'_, AppState>, args: KeyArgs) -> Result<(), String> {
    let k = args.key.trim();
    if k.is_empty() || k.len() > 512 || k.chars().any(|c| c.is_control()) {
        return Err("That does not look like a valid key.".into());
    }
    entry(&args.provider)?.set_password(k).map_err(|e| format!("Could not store the key: {e}"))?;
    state.registry.reset();
    Ok(())
}

#[tauri::command]
fn has_api_key(provider: String) -> Result<bool, String> {
    match entry(&provider)?.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(format!("Could not read the Windows Credential Manager: {e}")),
    }
}

#[tauri::command]
fn delete_api_key(state: State<'_, AppState>, provider: String) -> Result<(), String> {
    match entry(&provider)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => {
            state.registry.reset();
            Ok(())
        }
        Err(e) => Err(format!("Could not remove the key: {e}")),
    }
}

#[derive(Serialize)]
struct EnginesView {
    config: EnginesConfig,
    /// Set when the saved file could not be read. The app keeps working without it.
    load_error: Option<String>,
    /// Whether a key is saved for each engine, by engine id. Keys themselves are never sent here.
    keys: BTreeMap<String, bool>,
}

fn config_path(state: &AppState) -> PathBuf {
    state.store.root.join(".config").join("engines.json")
}

#[tauri::command]
fn engines_get(state: State<'_, AppState>) -> EnginesView {
    let (config, load_error) = match EnginesConfig::load(&config_path(&state)) {
        Ok(c) => (c, None),
        Err(e) => (EnginesConfig::default(), Some(e)),
    };
    let mut keys: BTreeMap<String, bool> = config.speech.iter().map(|e| (e.id.clone(), matches!(entry(&format!("engine:{}", e.id)).map(|k| k.get_password()), Ok(Ok(_))))).collect();
    // Provider-level keys, shared by every engine of that provider.
    for p in ["elevenlabs", "fal", "kie", "voicekiller", "voicekiller:userid"] {
        keys.insert(format!("provider:{p}"), matches!(entry(p).map(|k| k.get_password()), Ok(Ok(_))));
    }
    EnginesView { config, load_error, keys }
}

#[tauri::command]
fn engines_save(state: State<'_, AppState>, config: EnginesConfig) -> Result<(), String> {
    config.save(&config_path(&state))?;
    state.registry.reset();
    Ok(())
}

/// Remove the saved key of an engine that has just been deleted.
#[tauri::command]
fn engine_forget_key(state: State<'_, AppState>, engine_id: String) -> Result<(), String> {
    match entry(&format!("engine:{engine_id}"))?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => {
            state.registry.reset();
            Ok(())
        }
        Err(e) => Err(format!("Could not remove the key: {e}")),
    }
}

#[tauri::command]
async fn engine_test(state: State<'_, AppState>, engine_id: String) -> Result<String, String> {
    let (_, reg) = shared(&state);
    blocking(move || {
        let reg = reg.get();
        let id = if engine_id == "piper" { "piper".to_string() } else { format!("speech-{engine_id}") };
        let a = reg.adapter(&id).or_else(|| reg.adapter(&format!("job-{engine_id}"))).ok_or(if engine_id == "piper" { "Choose piper.exe and a voices folder, then save." } else { "Save the engine first, then test it." })?;
        a.check_connection().map(|_| "Connected. The server answered.".to_string()).map_err(|e| e.to_string())
    })
    .await?
}

#[tauri::command]
async fn engine_discover_voices(state: State<'_, AppState>, engine_id: String) -> Result<Vec<VoiceDef>, String> {
    let path = config_path(&state);
    blocking(move || {
        let cfg = EnginesConfig::load(&path)?;
        let e = cfg.speech.into_iter().find(|e| e.id == engine_id).ok_or("Save the engine first.")?;
        let a = OpenAiSpeechAdapter::new(e, key_provider())?;
        a.discover_voices().map_err(|e| e.to_string())
    })
    .await?
}

/// Cheap authenticated call so Settings can say whether a saved key works.
#[tauri::command]
async fn test_connection(state: State<'_, AppState>, provider: String) -> Result<String, String> {
    if !valid_key_name(&provider) {
        return Err("Unknown provider.".into());
    }
    let (_, reg) = shared(&state);
    blocking(move || {
        let reg = reg.get();
        let a = reg.adapter(&provider).ok_or("That provider is not available.")?;
        a.check_connection().map(|_| "Connected. The key works.".to_string()).map_err(|e| e.to_string())
    })
    .await?
}

#[derive(Serialize)]
struct Check {
    id: &'static str,
    label: &'static str,
    /// `ok`, `info` or `warn`. Nothing here is fatal: every part of Soundstage that is missing has a free alternative.
    status: &'static str,
    detail: String,
    fix: String,
}

fn run_checks(state: &AppState, config: &std::path::Path, ffmpeg_path: Option<&str>) -> Vec<Check> {
    let reg = state.registry.get();
    let sends = |v: &soundstage_core::providers::Voice| reg.adapter(&v.provider).is_some_and(|a| a.sends_data());
    let local = reg.voices.iter().filter(|v| !sends(v) && v.language != "zxx").count();
    let cloud = reg.voices.iter().filter(|v| sends(v)).count();
    let mut out = vec![];
    out.push(if local > 0 {
        Check { id: "voices", label: "Voices on this computer", status: "ok", detail: format!("{local} voice{} found. Rendering with them is free.", if local == 1 { "" } else { "s" }), fix: String::new() }
    } else {
        Check { id: "voices", label: "Voices on this computer", status: "warn", detail: "No Windows speech voices were found.".into(), fix: "Add one in Windows Settings under Time & language, then Speech. Or connect a local engine or Piper under Voice engines.".into() }
    });
    out.push(match exporter::ffmpeg_status(ffmpeg_path) {
        Some(f) => Check { id: "ffmpeg", label: "FFmpeg (MP3, FLAC, AAC, Opus, MP4 export)", status: "ok", detail: f.version, fix: String::new() },
        None => Check { id: "ffmpeg", label: "FFmpeg (MP3, FLAC, AAC, Opus, MP4 export)", status: "info", detail: "Not found. WAV export works without it.".into(), fix: "To export MP3, FLAC, AAC, Opus or MP4, install FFmpeg (for example run: winget install Gyan.FFmpeg) or choose ffmpeg.exe on the Export screen.".into() },
    });
    let saved: Vec<&str> = ["elevenlabs", "fal", "kie"].into_iter().filter(|n| matches!(entry(n).map(|k| k.get_password()), Ok(Ok(_)))).collect();
    out.push(Check {
        id: "keys",
        label: "Online services",
        status: if saved.is_empty() { "info" } else { "ok" },
        detail: if saved.is_empty() { "No keys saved. That is fine: local voices need none.".into() } else { format!("Keys saved for: {}. {cloud} online voice{} available.", saved.join(", "), if cloud == 1 { "" } else { "s" }) },
        fix: if saved.is_empty() { "Add an ElevenLabs key in Settings for online voices, music and effects, or a Kie or fal key under Voice engines.".into() } else { String::new() },
    });
    let wh = reg.adapter("whisper-local").map(|a| a.ready()).unwrap_or(false);
    out.push(Check {
        id: "offline-stt",
        label: "Offline speech to text (Whisper)",
        status: if wh { "ok" } else { "info" },
        detail: if wh { "Ready. Transcription can run on this computer with nothing sent anywhere.".into() } else { "Not set up. Optional.".into() },
        fix: if wh { String::new() } else { "In Audio tools, download an offline speech model (78 to 148 MB) to transcribe without an account.".into() },
    });
    let (sp, jb) = EnginesConfig::load(config).map(|c| (c.speech.len() + usize::from(c.piper.is_some()), c.jobs.len())).unwrap_or((0, 0));
    out.push(Check { id: "engines", label: "Your own engines", status: "info", detail: format!("{sp} speech engine{} and {jb} music, effects or workflow engine{} set up.", if sp == 1 { "" } else { "s" }, if jb == 1 { "" } else { "s" }), fix: if sp + jb == 0 { "Optional. Add one under Voice engines.".into() } else { String::new() } });
    let hw = soundstage_core::hardware::detect(&state.registry.models_dir());
    let profile = soundstage_core::hardware::profile_of(&hw);
    let gb = |mb: u64| format!("{:.0}", mb as f64 / 1024.0);
    out.push(Check {
        id: "hardware",
        label: "This computer",
        status: "info",
        detail: format!(
            "{} processor threads, {} GB memory, {}. {} GB free for downloaded models. Level: {}.",
            hw.cpu_threads,
            hw.ram_mb.map_or("unknown".into(), gb),
            if hw.gpus.is_empty() { "no NVIDIA graphics card found".to_string() } else { hw.gpus.iter().map(|g| format!("{} ({} GB)", g.name, gb(g.vram_mb))).collect::<Vec<_>>().join(", ") },
            hw.disk_free_mb.map_or("unknown".into(), gb),
            profile.map_or("unknown".to_string(), |p| serde_json::to_string(&p).unwrap_or_default().trim_matches('"').replace('-', " ")),
        ),
        fix: String::new(),
    });
    let probe = state.store.root.join(".write-test");
    let writable = std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(&probe);
    out.push(if writable {
        Check { id: "folder", label: "Projects folder", status: "ok", detail: state.store.root.to_string_lossy().to_string(), fix: String::new() }
    } else {
        Check { id: "folder", label: "Projects folder", status: "warn", detail: "Soundstage cannot write to its projects folder.".into(), fix: "Check that Documents\\Soundstage is not read-only and that your disk has space.".into() }
    });
    out
}

/// What the Director would send, so the screen can say so before anything is sent.
#[tauri::command]
fn director_estimate(project: Project) -> Estimate {
    director::estimate(&project.parsed())
}

/// Ask the Director for a plan proposal. With `use_model` false only the simple text rules run and nothing is sent.
/// Sending needs `confirmed`: the screen shows what will go to which server first.
#[tauri::command]
async fn director_run(project: Project, base_url: String, model: String, use_model: bool, confirmed: bool) -> Result<Proposal, String> {
    director::check_script(&project.script)?;
    if use_model && !confirmed {
        return Err("Confirm what will be sent before the Director runs.".into());
    }
    blocking(move || {
        let parsed = project.parsed();
        if parsed.lines.is_empty() {
            return Err("Write some dialogue first.".to_string());
        }
        let hash = soundstage_core::ids::script_hash(&project.script);
        if use_model {
            let key = key_provider()("assistant");
            director::run_model(&base_url, &model, key.as_deref(), &parsed, &hash)
        } else {
            let mut plan = director::rules_plan(&parsed);
            plan.source_hash = hash;
            Ok(Proposal { plan, source: "rules".into(), dropped: 0, notes: vec![] })
        }
    })
    .await?
}

fn line_lengths(project: &Project) -> BTreeMap<String, usize> {
    project.parsed().lines.iter().map(|l| (l.key.clone(), l.text.chars().count())).collect()
}

#[tauri::command]
fn plan_load(state: State<'_, AppState>, project_id: String) -> Result<PerformancePlan, String> {
    PlanStore::new(&state.store.dir(&project_id)?).load()
}

/// Save the plan after the same checks every plan must pass. A plan that fails them is refused and the old one stays.
#[tauri::command]
fn plan_save(state: State<'_, AppState>, project: Project, plan: PerformancePlan) -> Result<(), String> {
    PlanStore::new(&state.store.dir(&project.id)?).save(&plan, &line_lengths(&project))
}

#[tauri::command]
fn plan_changes(current: PerformancePlan, proposed: PerformancePlan) -> Vec<LineChange> {
    director::changes(&current, &proposed)
}

#[tauri::command]
fn plan_merge(current: PerformancePlan, proposed: PerformancePlan, accepted: Vec<String>) -> PerformancePlan {
    director::merge(&current, &proposed, &accepted)
}

/// The person edited a line's plan: keep it and lock what they changed.
#[tauri::command]
fn plan_edit(project: Project, mut plan: PerformancePlan, line_id: String, edited: LinePlan) -> Result<PerformancePlan, String> {
    let lens = line_lengths(&project);
    edited.validate(lens.get(&line_id).copied())?;
    director::user_edit(&mut plan, &line_id, edited);
    Ok(plan)
}

/// Carry the plan's emotion and pace into the line directions that are still empty.
#[tauri::command]
fn plan_directions(project: Project, plan: PerformancePlan) -> (BTreeMap<String, soundstage_core::direction::Direction>, usize) {
    let mut d = project.line_directions.clone();
    let n = director::apply_to_directions(&plan, &mut d);
    (d, n)
}

#[tauri::command]
fn get_local_only() -> bool {
    soundstage_core::net::local_only()
}

/// Local-only mode (D-13): nothing is sent to or fetched from the internet while it is on. The choice is
/// kept in a small marker file so it is still on the next time the app starts.
#[tauri::command]
fn set_local_only(state: State<'_, AppState>, on: bool) -> Result<(), String> {
    let marker = state.store.root.join(".config").join("local-only");
    if on {
        if let Some(dir) = marker.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("Could not save the setting: {e}"))?;
        }
        std::fs::write(&marker, b"on").map_err(|e| format!("Could not save the setting: {e}"))?;
    } else if marker.exists() {
        std::fs::remove_file(&marker).map_err(|e| format!("Could not save the setting: {e}"))?;
    }
    soundstage_core::net::set_local_only(on);
    Ok(())
}

/// A quick look at what this computer has, so a new user can see what works and what is optional.
#[tauri::command]
async fn system_check(state: State<'_, AppState>, ffmpeg_path: Option<String>) -> Result<Vec<Check>, String> {
    let config = config_path(&state);
    let st = AppState { store: state.store.clone(), registry: state.registry.clone() };
    blocking(move || run_checks(&st, &config, ffmpeg_path.as_deref())).await
}

/// Write a plain-text report the user can attach to a bug report. It holds versions and the checks above and nothing
/// private: no keys, no script text, no project names, no file paths other than the projects folder.
#[tauri::command]
async fn write_diagnostics(app: tauri::AppHandle, state: State<'_, AppState>, ffmpeg_path: Option<String>) -> Result<String, String> {
    let config = config_path(&state);
    let st = AppState { store: state.store.clone(), registry: state.registry.clone() };
    let version = app.package_info().version.to_string();
    blocking(move || {
        let mut t = format!("Soundstage {version}\nWindows: {} {}\nGenerated: {}\n\n", std::env::consts::OS, std::env::consts::ARCH, soundstage_core::util::iso_now());
        for c in run_checks(&st, &config, ffmpeg_path.as_deref()) {
            t.push_str(&format!("[{}] {}: {}\n", c.status, c.label, c.detail.replace(&st.store.root.to_string_lossy().to_string(), "<projects folder>")));
        }
        t.push_str("\nNo keys, script text or project names are included in this report.\n");
        let path = st.store.root.join("diagnostics.txt");
        std::fs::write(&path, t).map_err(|e| format!("Could not write the report: {e}"))?;
        Ok(path.to_string_lossy().to_string())
    })
    .await?
}

/// The Windows menu bar. Item text carries the shortcut after a tab so it is shown but not handled by the menu, because
/// the app already handles those keys itself (and a second handler would run every action twice). Choosing an item sends
/// its id to the page, which runs the same action as the shortcut or the command palette.
fn build_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{AboutMetadataBuilder, MenuBuilder, SubmenuBuilder};
    let file = SubmenuBuilder::new(app, "&File")
        .text("new-project", "&New project\tCtrl+N")
        .text("open-project", "&Open project…\tCtrl+O")
        .separator()
        .text("save", "&Save\tCtrl+S")
        .text("export", "&Export…\tCtrl+6")
        .separator()
        .text("open-file", "Open a project &file…")
        .text("show-folder", "Show projects &folder")
        .separator()
        .text("exit", "E&xit\tAlt+F4")
        .build()?;
    let edit = SubmenuBuilder::new(app, "&Edit")
        .text("undo", "&Undo\tCtrl+Z")
        .text("redo", "&Redo\tCtrl+Shift+Z")
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .text("palette", "&Search or run a command…\tCtrl+K")
        .build()?;
    let view = SubmenuBuilder::new(app, "&View")
        .text("toggle-mode", "&Simple or Advanced\tCtrl+Shift+M")
        .text("toggle-rail", "Show or hide the &sidebar\tCtrl+B")
        .text("toggle-guide", "Show or hide the &guide panel\tCtrl+J")
        .separator()
        .text("theme-dark", "&Dark theme")
        .text("theme-light", "&Light theme")
        .text("theme-system", "&Match Windows theme")
        .separator()
        .separator()
        .text("zoom-in", "Make everything &larger\tCtrl+=")
        .text("zoom-out", "Make everything &smaller\tCtrl+-")
        .text("zoom-reset", "&Normal size\tCtrl+0")
        .separator()
        .fullscreen()
        .build()?;
    let go = SubmenuBuilder::new(app, "&Go")
        .text("step-1", "Step 1: Script\tCtrl+1")
        .text("step-2", "Step 2: Cast\tCtrl+2")
        .text("step-3", "Step 3: Direct\tCtrl+3")
        .text("step-4", "Step 4: Render\tCtrl+4")
        .text("step-5", "Step 5: Mix\tCtrl+5")
        .text("step-6", "Step 6: Export\tCtrl+6")
        .separator()
        .text("speak", "S&peak")
        .text("editor", "Audio &editor")
        .text("tools", "&Audio tools")
        .text("pronunciation", "&Pronunciation")
        .text("voices", "&Voice library")
        .text("engines", "Voice &engines")
        .text("settings", "Se&ttings\tCtrl+,")
        .build()?;
    let about = AboutMetadataBuilder::new()
        .name(Some("Soundstage"))
        .version(Some(env!("CARGO_PKG_VERSION")))
        .comments(Some("The sound side of your film: voices, music, effects and the final mix. Dialogue is AI-generated and labelled as such in every export."))
        .build();
    let help = SubmenuBuilder::new(app, "&Help")
        .text("help", "Help and &guide\tF1")
        .text("shortcuts", "&Keyboard shortcuts\tCtrl+/")
        .text("tour", "Take the guided &tour")
        .text("prompt-help", "Writing &prompts")
        .text("troubleshooting", "&Troubleshooting")
        .text("system-check", "&System check")
        .text("report-problem", "&Report a problem…")
        .separator()
        .about(Some(about))
        .build()?;
    MenuBuilder::new(app).items(&[&file, &edit, &view, &go, &help]).build()
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let docs = app.path().document_dir().map_err(|e| e.to_string())?;
            let root = docs.join("Soundstage");
            std::fs::create_dir_all(&root)?;
            // Previews are throwaway; start each session clean.
            let _ = std::fs::remove_dir_all(root.join(".preview"));
            let registry: Arc<RegistryCell> = Arc::new(RegistryCell::new(root.join(".config").join("engines.json")));
            // Local-only mode is remembered between runs and applied before anything can connect.
            soundstage_core::net::set_local_only(root.join(".config").join("local-only").exists());
            // Find the installed voices in the background so the first screen opens at once.
            let warm = registry.clone();
            std::thread::spawn(move || {
                warm.get();
            });
            app.manage(AppState { store: Arc::new(Store::new(root)), registry });
            let menu = build_menu(app.handle())?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, event| {
                let _ = app.emit("menu", event.id().as_ref().to_string());
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            list_projects,
            create_project,
            load_project,
            save_project,
            delete_project,
            project_dir,
            list_voices,
            preview_voice,
            mix_preview,
            analyze_project,
            render_lines,
            job_begin,
            audition_line,
            fallback_options,
            director_estimate,
            director_run,
            plan_load,
            plan_save,
            plan_changes,
            plan_merge,
            plan_edit,
            plan_directions,
            get_local_only,
            set_local_only,
            job_finish,
            pending_jobs,
            recover_job,
            compute_layout,
            waveform,
            destinations,
            check_export,
            export_capabilities,
            export_project,
            import_asset,
            test_connection,
            improve_prompt,
            clone_info,
            clone_check_reference,
            clone_voice,
            register_clone,
            backup_project,
            open_project_file,
            whisper_status,
            whisper_download,
            whisper_remove,
            speak_text,
            history_list,
            history_delete,
            history_clear,
            pronunciation_get,
            pronunciation_save,
            pronunciation_preview,
            save_audio_copy,
            save_text_copy,
            list_audio_tools,
            transcribe_audio,
            clean_audio,
            change_voice,
            audio_info,
            edit_audio,
            translate_cues,
            cues_subtitles,
            consent_list,
            consent_revoke,
            quit_app,
            system_check,
            write_diagnostics,
            list_generators,
            generate_audio,
            generator_test,
            list_designers,
            design_voice,
            save_designed_voice,
            engines_get,
            engines_save,
            engine_forget_key,
            engine_test,
            engine_discover_voices,
            set_api_key,
            has_api_key,
            delete_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Soundstage");
}
