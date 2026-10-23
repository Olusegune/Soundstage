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
