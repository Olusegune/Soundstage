import type { Analysis, Project, RenderOutcome, Voice } from "./types";

export interface Ctx {
  project: Project;
  analysis: Analysis | null;
  voices: Voice[];
  /** Absolute project folder. */
  dir: string;
  /** Apply an edit. Edits with the same `coalesce` key within a second share one undo step. */
  edit: (fn: (p: Project) => Project, coalesce?: string) => void;
  toast: (text: string, bad?: boolean) => void;
  goto: (step: number) => void;
  openLibrary: () => void;
  /** Render lines into takes. Resolves with the outcome, or null if the call itself failed. */
  render: (keys: string[]) => Promise<RenderOutcome | null>;
  busy: Set<string>;
}
