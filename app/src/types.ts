// Mirrors the Rust types in crates/soundstage-core. Keep the two in step.

export type Licence = "commercial" | "non-commercial" | "unknown";
export type LicenceTarget = "commercial" | "personal";
export type TakeStatus = "pending" | "approved" | "held" | "rejected";
export type Level = "default" | "project" | "character" | "scene" | "line";

export interface LicenceInfo {
  licence: Licence;
  source: string | null;
  checked: string | null;
  notes: string;
}

export interface Direction {
  emotion?: string | null;
  pace?: number | null;
  pause_before_ms?: number | null;
  note?: string | null;
}

export interface Bible {
  age: string;
  background: string;
  speech_habits: string;
  emotional_range: string;
  notes: string;
}

export interface Character {
  voice_id: string | null;
  bible: Bible;
  direction: Direction;
}

export interface Take {
  id: string;
  line_key: string;
  speaker: string;
  text: string;
  voice_id: string;
  adapter_id: string;
  provider_version: string;
  cache_key: string;
  file: string;
  duration_ms: number;
  sample_rate: number;
  direction_summary: string;
  created: string;
  status: TakeStatus;
  approver: string | null;
}

export interface Asset {
  id: string;
  name: string;
  kind: "music" | "sfx";
  file: string;
  duration_ms: number;
  licence: LicenceInfo;
  uses_prohibited: string[];
  source_note: string;
  generated?: { generator: string; provider_version: string; prompt: string; created: string } | null;
}

export interface Cue {
  id: string;
  asset_id: string;
  scene: number | null;
  offset_ms: number;
  gain_db: number;
  loop_to_scene: boolean;
  fade_in_ms: number;
  fade_out_ms: number;
  duck: boolean;
}

export interface MixSettings {
  destination: string;
  duck_depth_db: number;
  attack_ms: number;
  release_ms: number;
  gap_ms: number;
  scene_gap_ms: number;
}

export interface Note {
  id: string;
  target: string;
  text: string;
  created: string;
}

export type Rating = "all_ages" | "teen" | "adult" | "unrated";

export interface Project {
  pronunciations: PronRule[];
  schema_version: number;
  id: string;
  name: string;
  created: string;
  modified: string;
  script: string;
  licence_target: LicenceTarget;
  rating: Rating;
  project_direction: Direction;
  characters: Record<string, Character>;
  scene_directions: Record<string, Direction>;
  line_directions: Record<string, Direction>;
  takes: Take[];
  selected: Record<string, string>;
  skipped: string[];
  assets: Asset[];
  cues: Cue[];
  mix: MixSettings;
  licence_checks: Record<string, LicenceInfo>;
  notes: Note[];
  scene_status: Record<string, string>;
}

export interface Summary {
  id: string;
  name: string;
  modified: string;
  licence_target: LicenceTarget;
  lines: number;
}

export interface Voice {
  id: string;
  name: string;
  provider: string;
  provider_label: string;
  language: string;
  accent: string;
  gender: string;
  age_range: string;
  licence: LicenceInfo;
  sample_text: string;
  notes: string;
  tags: string[];
  /** True when script text is sent over the network to render this voice. */
  sends_data: boolean;
}

export interface TagIssue {
  tag: string;
  message: string;
}

export interface ParsedLine {
  key: string;
  scene: number;
  speaker: string;
  text: string;
  row: number;
  tag_issues: TagIssue[];
}

export interface ParsedScene {
  index: number;
  title: string;
  location: string;
  time_of_day: string;
  cast: string[];
}

export interface Field<T> {
  value: T;
  source: Level;
}

export interface Resolved {
  emotion: Field<string>;
  pace: Field<number>;
  pause_before_ms: Field<number | null>;
  notes: [Level, string][];
}

export type LineStatus = "drafted" | "rendered" | "stale" | "skipped";

export interface LineIssue {
  kind: string;
  message: string;
  tag: string | null;
}

export interface LineInfo {
  line: ParsedLine;
  resolved: Resolved;
  status: LineStatus;
  issues: LineIssue[];
  cache_key: string | null;
  voice_id: string | null;
  selected_take: string | null;
  take_count: number;
}

export interface Analysis {
  parsed: {
    scenes: ParsedScene[];
    speakers: string[];
    lines: ParsedLine[];
    issues: { row: number; message: string }[];
  };
  lines: LineInfo[];
  speakers_without_voice: string[];
}

export interface LineOutcome {
  key: string;
  ok: boolean;
  cached: boolean;
  error_kind: string | null;
  message: string | null;
  result: { take_id: string; cached: boolean; duration_ms: number; quality: { clipped_samples: number; longest_silence_ms: number } | null } | null;
}

export interface RenderOutcome {
  new_takes: Take[];
  selected: Record<string, string>;
  results: LineOutcome[];
}

export interface Layout {
  scenes: { index: number; title: string; start_ms: number; end_ms: number }[];
  lines: { key: string; scene: number; speaker: string; take_id: string; start_ms: number; end_ms: number }[];
  total_ms: number;
}

export interface Destination {
  id: string;
  label: string;
  target_lufs: number;
  true_peak_ceiling_dbtp: number;
  note: string;
}

export interface Loudness {
  integrated_lufs: number | null;
  short_term_max_lufs: number | null;
  loudness_range_lu: number | null;
  true_peak_dbtp: number;
}

export interface MixPreview {
  path: string;
  peaks: number[];
  loudness: Loudness;
  target_lufs: number;
  gain_db: number;
  target_reached: boolean;
  warnings: string[];
  total_ms: number;
}

export interface GateItem {
  kind: string;
  id: string;
  name: string;
  reason: string;
  severity: "blocking" | "notice";
  source: string | null;
}

export interface GateReport {
  target: LicenceTarget;
  checked_on: string;
  blocking: GateItem[];
  notices: GateItem[];
  allowed: boolean;
}

export interface ExportResult {
  folder: string;
  files: string[];
  loudness: Loudness;
  target_lufs: number;
  gain_db: number;
  target_reached: boolean;
  warnings: string[];
}

export type ExportError =
  | { kind: "blocked"; report: GateReport }
  | { kind: "needs_acknowledgement"; report: GateReport }
  | { kind: "failed"; message: string };

export const STEPS = ["Script", "Cast", "Direct", "Render", "Mix", "Export"] as const;
export type StepName = (typeof STEPS)[number];

export const EMOTIONS = ["neutral", "whisper", "shout", "excited", "sad", "tired", "sarcastic", "curious", "serious"] as const;
export const NON_VERBAL = ["laugh", "chuckle", "sigh", "cough", "sneeze", "gasp", "cry", "breath", "sniff"] as const;

export type Aspect = "wide" | "tall" | "square";
export type Codec =
  | { kind: "wav"; bits: 16 | 24 | 32 }
  | { kind: "flac" }
  | { kind: "mp3"; kbps: number }
  | { kind: "aac"; kbps: number }
  | { kind: "vorbis"; quality: number }
  | { kind: "opus"; kbps: number }
  | { kind: "mp4"; aspect: Aspect; kbps: number };

export interface ExportOptions {
  master: Codec;
  stems: Codec | null;
  sample_rate: number;
  subtitles: ("srt" | "vtt")[];
  speaker_labels: boolean;
  ffmpeg_path: string | null;
  timeline: boolean;
}

export interface ExportCaps {
  ffmpeg: { path: string; version: string; encoders: string[] } | null;
  configured_failed: boolean;
  formats: { kind: string; label: string; needs_ffmpeg: boolean; available: boolean }[];
}

export interface VoiceDef { id: string; name: string; language: string; accent: string; gender: string }

export interface SpeechEngine {
  id: string;
  label: string;
  base_url: string;
  model: string;
  voices: VoiceDef[];
  response_format: "wav" | "mp3" | "flac" | "opus" | "aac";
  tags: string[];
  tag_map: Record<string, string>;
  needs_key: boolean;
  licence_note: string;
  notes: string;
  transcribe_model: string;
}

export interface PiperConfig { exe: string; voices_dir: string }

export interface JobEngine {
  id: string;
  label: string;
  provider: "fal" | "kie" | "comfy";
  kind: "speech" | "music" | "sfx";
  model: string;
  base_url: string;
  template: string;
  voices: VoiceDef[];
  licence_note: string;
  min_s: number;
  max_s: number;
  default_s: number;
}

export interface EnginesConfig { version: number; speech: SpeechEngine[]; piper: PiperConfig | null; jobs: JobEngine[] }

export interface GenLimits { min_s: number; max_s: number; default_s: number; supports_length: boolean }
export interface GeneratorInfo {
  id: string;
  label: string;
  kinds: ("music" | "sfx")[];
  sends_data: boolean;
  ready: boolean;
  music: GenLimits;
  sfx: GenLimits;
  licence: LicenceInfo;
  notes: string;
}
export interface DesignerInfo { id: string; label: string; ready: boolean; design: boolean; clone: boolean; register: boolean }
export interface DesignPreview { generated_voice_id: string; path: string }

export interface EnginesView { config: EnginesConfig; load_error: string | null; keys: Record<string, boolean> }

export interface SystemCheck { id: string; label: string; status: "ok" | "info" | "warn"; detail: string; fix: string }

export interface Attestation {
  subject: string;
  subject_is_self: boolean;
  adult: boolean;
  alive: boolean;
  not_public_figure: boolean;
  agrees: boolean;
  purpose: string;
  commercial: boolean;
  expires: string | null;
}
export interface SpeakOut { path: string; duration_ms: number; voice_name: string; history_id: string | null }
export interface ToolInfo { id: string; label: string; ready: boolean; transcribe: boolean; isolate: boolean; convert: boolean; sends_data: boolean }
export interface Cue2 { start_ms: number; end_ms: number; text: string }
export interface TranscriptOut { text: string; language: string; srt: string; vtt: string; seconds: number; has_times: boolean; cues: Cue2[] }
export interface CleanOut { path: string; duration_ms: number }
export interface AudioInfo { duration_ms: number; peaks: number[] }
export interface CloneInfo { statement: string; performance_statement: string; min_seconds: number; good_seconds: number; max_seconds: number }
export interface RefReport { seconds: number; used_seconds: number; peak: number; clipped_pct: number; silence_pct: number; problems: string[]; warnings: string[] }
export interface ConsentRecord { id: string; voice_subject: string; subject_is_self: boolean; purpose: string; commercial_use_permitted: boolean; expires: string | null; signed_at: string | null; status: "active" | "revoked" | "expired" }
export interface ClonedVoice { voice_id: string; adapter_id: string; name: string; consent_id: string; created: string }
export interface CloneEntry { voice: ClonedVoice; consent: ConsentRecord }

export interface PronRule { from: string; to: string; whole_word: boolean; ignore_case: boolean }
export interface HistoryItem { id: string; created: string; kind: "speak" | "clean"; voice_id: string; voice_name: string; provider_label: string; text: string; style: string; pace: number; duration_ms: number }
export interface HistoryView { item: HistoryItem; path: string }
