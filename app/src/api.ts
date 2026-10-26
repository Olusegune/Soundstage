import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Analysis, Asset, Destination, Attestation, AudioInfo, WhisperStatus, CleanOut, Cue2, HistoryView, PronRule, SpeakOut, ToolInfo, TranscriptOut, CloneEntry, CloneInfo, ClonedVoice, DesignPreview, DesignerInfo, RefReport, EnginesConfig, GeneratorInfo, SystemCheck, EnginesView, ExportCaps, ExportOptions, ExportResult, GateReport, Layout, MixPreview, Project, RenderOutcome, Summary, Take, Voice, VoiceDef,
} from "./types";
import type { Interrupted } from "./shell/InterruptedDialog";

/** Tauri rejects with a string for plain errors and with an object for structured ones. */
export function messageOf(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object") {
    const o = e as { message?: unknown };
    if (typeof o.message === "string") return o.message;
  }
  return "Something went wrong.";
}

export const api = {
  info: () => invoke<{ version: string; projects_root: string }>("app_info"),
  listProjects: () => invoke<Summary[]>("list_projects"),
  createProject: (name: string, sample: boolean) => invoke<Project>("create_project", { name, sample }),
  loadProject: (id: string) => invoke<Project>("load_project", { id }),
  saveProject: (project: Project) => invoke<void>("save_project", { project }),
  deleteProject: (id: string) => invoke<void>("delete_project", { id }),
  projectDir: (id: string) => invoke<string>("project_dir", { id }),
  voices: () => invoke<Voice[]>("list_voices"),
  previewVoice: (voiceId: string, text: string) => invoke<{ path: string }>("preview_voice", { voiceId, text }),
  analyze: (project: Project) => invoke<Analysis>("analyze_project", { project }),
  renderLines: (project: Project, keys: string[], jobId?: string | null) => invoke<RenderOutcome>("render_lines", { project, keys, jobId: jobId ?? null }),
  jobBegin: (projectId: string, keys: string[]) => invoke<string>("job_begin", { projectId, keys }),
  jobFinish: (projectId: string, jobId: string, outcome: "succeeded" | "cancelled" | "failed") => invoke<void>("job_finish", { projectId, jobId, outcome }),
  pendingJobs: (projectId: string) => invoke<Interrupted[]>("pending_jobs", { projectId }),
  recoverJob: (project: Project, jobId: string) => invoke<{ new_takes: Take[]; selected: Record<string, string> }>("recover_job", { project, jobId }),
  waveform: (projectId: string, file: string, bins: number) => invoke<number[]>("waveform", { projectId, file, bins }),
  layout: (project: Project) => invoke<Layout>("compute_layout", { project }),
  destinations: () => invoke<Destination[]>("destinations"),
  mixPreview: (project: Project) => invoke<MixPreview>("mix_preview", { project }),
  checkExport: (project: Project) => invoke<GateReport>("check_export", { project }),
  exportCapabilities: (ffmpegPath: string | null) => invoke<ExportCaps>("export_capabilities", { ffmpegPath }),
  exportProject: (project: Project, dest: string, acknowledged: boolean, options: ExportOptions) =>
    invoke<ExportResult>("export_project", { project, dest, acknowledged, options }),
  importAsset: (projectId: string, path: string, kind: "music" | "sfx") =>
    invoke<Asset>("import_asset", { projectId, path, kind }),
  listGenerators: () => invoke<GeneratorInfo[]>("list_generators"),
  generateAudio: (projectId: string, generatorId: string, kind: "music" | "sfx", prompt: string, seconds: number | null, instrumental: boolean, name: string | null) =>
    invoke<Asset>("generate_audio", { projectId, generatorId, kind, prompt, seconds, instrumental, name }),
  generatorTest: (generatorId: string) => invoke<string>("generator_test", { generatorId }),
  listDesigners: () => invoke<DesignerInfo[]>("list_designers"),
  designVoice: (adapterId: string, description: string, sampleText: string | null) => invoke<DesignPreview[]>("design_voice", { adapterId, description, sampleText }),
  saveDesignedVoice: (adapterId: string, generatedVoiceId: string, name: string, description: string) => invoke<string>("save_designed_voice", { adapterId, generatedVoiceId, name, description }),
  improvePrompt: (baseUrl: string, model: string, kind: "music" | "sfx" | "voice", draft: string) => invoke<string>("improve_prompt", { baseUrl, model, kind, draft }),
  quit: () => invoke<void>("quit_app"),
  systemCheck: (ffmpegPath: string | null) => invoke<SystemCheck[]>("system_check", { ffmpegPath }),
  writeDiagnostics: (ffmpegPath: string | null) => invoke<string>("write_diagnostics", { ffmpegPath }),
  cloneInfo: () => invoke<CloneInfo>("clone_info"),
  cloneCheckReference: (path: string) => invoke<RefReport>("clone_check_reference", { path }),
  cloneVoice: (adapterId: string, path: string, name: string, attestation: Attestation) => invoke<ClonedVoice>("clone_voice", { adapterId, path, name, attestation }),
  registerClone: (adapterId: string, name: string, attestation: Attestation) => invoke<ClonedVoice>("register_clone", { adapterId, name, attestation }),
  speakText: (voiceId: string, text: string, emotion: string | null, pace: number | null, keep: boolean) => invoke<SpeakOut>("speak_text", { voiceId, text, emotion, pace, keep }),
  historyList: () => invoke<HistoryView[]>("history_list"),
  historyDelete: (id: string) => invoke<void>("history_delete", { id }),
  historyClear: () => invoke<void>("history_clear"),
  pronunciationGet: () => invoke<PronRule[]>("pronunciation_get"),
  pronunciationSave: (rules: PronRule[]) => invoke<void>("pronunciation_save", { rules }),
  pronunciationPreview: (text: string, projectRules: PronRule[], appRules: PronRule[]) => invoke<string>("pronunciation_preview", { text, projectRules, appRules }),
  saveAudioCopy: (src: string, dest: string, ffmpegPath: string | null) => invoke<string>("save_audio_copy", { src, dest, ffmpegPath }),
  saveTextCopy: (dest: string, content: string) => invoke<void>("save_text_copy", { dest, content }),
  listAudioTools: () => invoke<ToolInfo[]>("list_audio_tools"),
  transcribeAudio: (adapterId: string, path: string, language: string | null) => invoke<TranscriptOut>("transcribe_audio", { adapterId, path, language }),
  changeVoice: (voiceId: string, path: string, attestation: Attestation, keep: boolean) => invoke<CleanOut>("change_voice", { voiceId, path, attestation, keep }),
  translateCues: (baseUrl: string, model: string, target: string, lines: string[]) => invoke<string[]>("translate_cues", { baseUrl, model, target, lines }),
  cuesSubtitles: (cues: Cue2[], kind: "srt" | "vtt") => invoke<string>("cues_subtitles", { cues, kind }),
  audioInfo: (path: string) => invoke<AudioInfo>("audio_info", { path }),
  editAudio: (clips: { path: string; start_s: number; end_s: number | null; offset_s: number; gain_db: number; fade_in_ms: number; fade_out_ms: number }[], normalize: boolean, keep: boolean) => invoke<CleanOut>("edit_audio", { clips, normalize, keep }),
  whisperStatus: () => invoke<WhisperStatus>("whisper_status"),
  whisperDownload: (modelId: string) => invoke<void>("whisper_download", { modelId }),
  whisperRemove: (modelId: string) => invoke<void>("whisper_remove", { modelId }),
  backupProject: (id: string, dest: string) => invoke<number>("backup_project", { id, dest }),
  openProjectFile: (path: string) => invoke<string>("open_project_file", { path }),
  cleanAudio: (adapterId: string, path: string, keep: boolean) => invoke<CleanOut>("clean_audio", { adapterId, path, keep }),
  consentList: () => invoke<CloneEntry[]>("consent_list"),
  consentRevoke: (consentId: string, deleteRemote: boolean) => invoke<string>("consent_revoke", { consentId, deleteRemote }),
  setApiKey: (provider: string, key: string) => invoke<void>("set_api_key", { args: { provider, key } }),
  enginesGet: () => invoke<EnginesView>("engines_get"),
  enginesSave: (config: EnginesConfig) => invoke<void>("engines_save", { config }),
  engineForgetKey: (engineId: string) => invoke<void>("engine_forget_key", { engineId }),
  engineTest: (engineId: string) => invoke<string>("engine_test", { engineId }),
  engineDiscoverVoices: (engineId: string) => invoke<VoiceDef[]>("engine_discover_voices", { engineId }),
  testConnection: (provider: string) => invoke<string>("test_connection", { provider }),
  hasApiKey: (provider: string) => invoke<boolean>("has_api_key", { provider }),
  deleteApiKey: (provider: string) => invoke<void>("delete_api_key", { provider }),
  onRenderProgress: (fn: (p: { key: string; status: string }) => void) =>
    listen<{ key: string; status: string }>("render-progress", (e) => fn(e.payload)),
};

export function audioUrl(absolutePath: string, bust?: string): string {
  const u = convertFileSrc(absolutePath);
  return bust ? `${u}${u.includes("?") ? "&" : "?"}v=${encodeURIComponent(bust)}` : u;
}

export function joinPath(dir: string, rel: string): string {
  return `${dir.replace(/[\\/]+$/, "")}\\${rel.replace(/\//g, "\\")}`;
}
