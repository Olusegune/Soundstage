//! Thin desktop shell. All decisions (licence gate, consent, render, mix) live in
//! `soundstage-core`; this file only exposes them to the UI and handles files and keys.

use serde::{Deserialize, Serialize};
use soundstage_core::audio::probe_duration_ms;
use soundstage_core::exporter::{self, ExportError, ExportResult};
use soundstage_core::layout::{self, Layout};
use soundstage_core::licence::{GateReport, LicenceInfo};
use soundstage_core::loudness::{Destination, DESTINATIONS};
use soundstage_core::project::{Asset, AssetKind, Project, Store, Summary, Take};
use soundstage_core::adapters::elevenlabs::KeyProvider;
use soundstage_core::providers::{Registry, Voice};
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
    Arc::new(|| keyring::Entry::new("Soundstage", "elevenlabs").ok()?.get_password().ok())
}

/// Builds the registry on first use and again after a key is saved or removed.
struct RegistryCell {
    inner: Mutex<Option<Arc<Registry>>>,
}

impl RegistryCell {
    fn new() -> Self {
        RegistryCell { inner: Mutex::new(None) }
    }
    fn get(&self) -> Arc<Registry> {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        g.get_or_insert_with(|| Arc::new(Registry::standard(key_provider()))).clone()
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

#[tauri::command]
async fn analyze_project(state: State<'_, AppState>, project: Project) -> Result<Analysis, String> {
    let (_, reg) = shared(&state);
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
async fn render_lines(app: AppHandle, state: State<'_, AppState>, project: Project, keys: Vec<String>) -> Result<RenderOutcome, String> {
    let (store, reg) = shared(&state);
    let dir = store.dir(&project.id)?;
    blocking(move || {
        let reg = reg.get();
        let reg = &*reg;
        let mut p = project;
        let before = p.takes.len();
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
                    results.push(LineOutcome { key, ok: true, cached: r.cached, error_kind: None, message: None, result: Some(r) });
                }
                Err(e) => {
                    let _ = app.emit("render-progress", serde_json::json!({ "key": key, "status": "failed" }));
                    results.push(LineOutcome { key, ok: false, cached: false, error_kind: Some(e.kind), message: Some(e.message), result: None });
                }
            }
        }
        RenderOutcome { new_takes: p.takes[before..].to_vec(), selected, results }
    })
    .await
}

#[tauri::command]
fn compute_layout(project: Project) -> Layout {
    layout::compute(&project, &soundstage_core::script::parse(&project.script))
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

#[tauri::command]
async fn export_project(state: State<'_, AppState>, project: Project, dest: String, acknowledged: bool) -> Result<ExportResult, ExportError> {
    let (store, reg) = shared(&state);
    let dir = store.dir(&project.id).map_err(|m| ExportError::Failed { message: m })?;
    blocking(move || exporter::export(&project, &reg.get(), &dir, &PathBuf::from(dest), acknowledged))
        .await
        .map_err(|m| ExportError::Failed { message: m })?
}

const MAX_ASSET_BYTES: u64 = 500 * 1024 * 1024;

/// Copy a WAV into the project. The licence starts as `unknown`, so it cannot reach a
/// commercial export until the user records where the terms were checked.
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
        if src.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("wav")) != Some(true) {
            return Err("Only WAV files can be imported in this version.".to_string());
        }
        let duration_ms = probe_duration_ms(&src)?;
        let id = new_id("asset");
        let rel = format!("assets/{id}.wav");
        std::fs::create_dir_all(dir.join("assets")).map_err(|e| e.to_string())?;
        std::fs::copy(&src, dir.join(&rel)).map_err(|e| format!("Could not copy the file into the project: {e}"))?;
        let name = src.file_stem().map(|s| safe_file_stem(&s.to_string_lossy())).unwrap_or_else(|| "Imported audio".into());
        Ok(Asset { id, name, kind, file: rel, duration_ms, licence: LicenceInfo::default(), uses_prohibited: vec![], source_note: String::new() })
    })
    .await?
}

// Provider keys live in the Windows Credential Manager and are never returned to the UI.
const KEY_PROVIDERS: &[&str] = &["elevenlabs"];

fn entry(provider: &str) -> Result<keyring::Entry, String> {
    if !KEY_PROVIDERS.contains(&provider) {
        return Err("Unknown provider.".into());
    }
    keyring::Entry::new("Soundstage", provider).map_err(|e| format!("Could not reach the Windows Credential Manager: {e}"))
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

/// Cheap authenticated call so Settings can say whether a saved key works.
#[tauri::command]
async fn test_connection(state: State<'_, AppState>, provider: String) -> Result<String, String> {
    if !KEY_PROVIDERS.contains(&provider.as_str()) {
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
            let registry: Arc<RegistryCell> = Arc::new(RegistryCell::new());
            // Find the installed voices in the background so the first screen opens at once.
            let warm = registry.clone();
            std::thread::spawn(move || {
                warm.get();
            });
            app.manage(AppState { store: Arc::new(Store::new(root)), registry });
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
            compute_layout,
            destinations,
            check_export,
            export_project,
            import_asset,
            test_connection,
            set_api_key,
            has_api_key,
            delete_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Soundstage");
}
