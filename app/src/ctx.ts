import type { GoldenStep, NextAction } from "./golden";
import type { Mode } from "./prefs";
import type { Analysis, MixPreview, Project, RenderOutcome, Voice } from "./types";

export interface Ctx {
  project: Project;
  analysis: Analysis | null;
  voices: Voice[];
  /** Absolute project folder. */
  dir: string;
  mode: Mode;
  ffmpegPath: string;
  setFfmpegPath: (p: string) => void;
  /** Apply an edit. Edits with the same `coalesce` key within a second share one undo step. */
  edit: (fn: (p: Project) => Project, coalesce?: string) => void;
  toast: (text: string, bad?: boolean) => void;
  goto: (step: number) => void;
  openLibrary: () => void;
  openPronunciation: () => void;
  newProject: () => void;
  exportMemory: string;
  setExportMemory: (json: string) => void;
  voiceMemory: Record<string, string>;
  rememberVoices: (picks: Record<string, string>) => void;
  openHelp: (articleId?: string) => void;
  /** Render lines into takes. Resolves with the outcome, or null if the call itself failed. */
  render: (keys: string[], opts?: { confirmed?: boolean }) => Promise<RenderOutcome | null>;
  /** Ask before text leaves this computer. Resolves true at once when every voice is local. */
  confirmSend: (keys: string[]) => Promise<boolean>;
  /** Render many lines one at a time with progress, asking once up front. */
  renderBatch: (keys: string[]) => Promise<void>;
  stopBatch: () => void;
  batch: { done: number; total: number } | null;
  errors: Record<string, string>;
  busy: Set<string>;
  steps: GoldenStep[];
  next: NextAction | null;
  mixPreview: MixPreview | null;
  mixBuilding: boolean;
  buildMix: () => Promise<void>;
  markExported: () => void;
  /** A line to bring into view (set by search). */
  focusKey: string | null;
}
