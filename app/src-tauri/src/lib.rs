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
    matches!(name, "elevenlabs" | "fal" | "kie" | "assistant") || name.strip_prefix("engine:").is_some_and(valid_engine_id)
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
        g.get_or_insert_with(|| Arc::new(Registry::standard(key_provider(), &EnginesConfig::load(&self.config_path).unwrap_or_default()))).clone()
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
}

#[tauri::command]
async fn list_designers(state: State<'_, AppState>) -> Result<Vec<DesignerView>, String> {
    let (_, reg) = shared(&state);
    blocking(move || {
        reg.get().adapters.iter().filter(|a| a.capabilities().voice_design).map(|a| DesignerView { id: a.id().into(), label: a.label().into(), ready: a.ready() }).collect()
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
    for p in ["elevenlabs", "fal", "kie"] {
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
            waveform,
            destinations,
            check_export,
            export_capabilities,
            export_project,
            import_asset,
            test_connection,
            improve_prompt,
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
