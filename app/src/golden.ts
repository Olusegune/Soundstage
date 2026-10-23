// The Golden Path, computed from real project state. Nothing here is stored: it is derived every time,
// so it can never disagree with the project. See docs/design/02-DESIGN-DIRECTION.md section 3.
import type { Analysis, Project, Voice } from "./types";

export type StepId = "script" | "cast" | "direct" | "render" | "mix" | "export";
export type StepStatus = "done" | "current" | "ready" | "blocked";
export type Cost = "free" | "paid" | "mixed";

export interface GoldenStep {
  id: StepId;
  index: number;
  label: string;
  optional: boolean;
  status: StepStatus;
  progress: { done: number; of: number } | null;
  /** One line on where this step stands right now. */
  summary: string;
  why: string;
  ifSkipped: string;
  cost: Cost;
  costNote: string;
  blockedReason: string | null;
  tasks: { label: string; done: boolean }[];
  helpId: string;
}

export interface NextAction {
  stepId: StepId;
  label: string;
  /** What pressing it does: open a step, or run something. */
  kind: "goto" | "render" | "preview-mix" | "export" | "sample";
  paid: boolean;
  count?: number;
  reason: string;
}

export interface GoldenInput {
  project: Project;
  analysis: Analysis | null;
  voices: Voice[];
  /** Set by the UI once a preview mix has been built this session. */
  mixPreviewed?: boolean;
  /** Set by the UI once an export finished this session. */
  exported?: boolean;
}

const ORDER: StepId[] = ["script", "cast", "direct", "render", "mix", "export"];

export const STEP_COPY: Record<StepId, { label: string; optional: boolean; why: string; ifSkipped: string; helpId: string }> = {
  script: {
    label: "Script", optional: false, helpId: "writing-script",
    why: "Everything is made from your words. Each line starts with a speaker name, so Soundstage knows who says what.",
    ifSkipped: "There is nothing to cast, render or mix.",
  },
  cast: {
    label: "Cast", optional: false, helpId: "casting-voices",
    why: "A voice gives each character their sound. You can try a sample before you commit.",
    ifSkipped: "Lines cannot be rendered without a voice.",
  },
  direct: {
    label: "Direct", optional: true, helpId: "directing-performance",
    why: "Emotion, pauses and sounds like sighs turn reading into acting. It is also where Soundstage tells you if a voice cannot do something you asked for.",
    ifSkipped: "Lines are spoken in a neutral, even tone, and any unsupported tag will block rendering until you fix it.",
  },
  render: {
    label: "Render", optional: false, helpId: "rendering",
    why: "Rendering turns each line into audio you can hear and judge. Each version is kept as a take, so you can compare.",
    ifSkipped: "There is no audio to mix or export.",
  },
  mix: {
    label: "Mix", optional: true, helpId: "mixing",
    why: "The mix balances voice with music and effects and brings the whole thing to the loudness your destination expects.",
    ifSkipped: "The export is the voice track alone, set to your loudness target.",
  },
  export: {
    label: "Export", optional: false, helpId: "exporting",
    why: "Export checks that every voice, music track and effect may be used the way you plan, then writes a master, stems and a record of what was used.",
    ifSkipped: "Nothing leaves Soundstage.",
  },
};

const plural = (n: number, one: string, many = one + "s") => `${n} ${n === 1 ? one : many}`;

/** Lines that would need a network round trip to render. */
function cloudLines(analysis: Analysis, voices: Voice[], keys: string[]): number {
  return analysis.lines.filter((l) => keys.includes(l.line.key) && voices.find((v) => v.id === l.voice_id)?.sends_data).length;
}

export function golden(input: GoldenInput): { steps: GoldenStep[]; next: NextAction; current: StepId } {
  const { project, analysis, voices } = input;
  const lines = analysis?.lines ?? [];
  const speakers = analysis?.parsed.speakers ?? [];
  const unvoiced = analysis?.speakers_without_voice ?? [];
  const noteCount = lines.filter((l) => l.issues.some((i) => i.kind !== "no_voice" && i.kind !== "voice_missing")).length;
  const rendered = lines.filter((l) => l.status === "rendered").length;
  const skipped = lines.filter((l) => l.status === "skipped").length;
  const needRender = lines.filter((l) => (l.status === "drafted" || l.status === "stale") && l.issues.length === 0);
  const hasScript = lines.length > 0;
  const castDone = hasScript && unvoiced.length === 0;
  const renderDone = hasScript && rendered > 0 && rendered + skipped === lines.length;
  const mixDone = renderDone && (!!input.mixPreviewed || project.cues.length > 0);

  const done: Record<StepId, boolean> = {
    script: hasScript,
    cast: castDone,
    direct: castDone && noteCount === 0,
    render: renderDone,
    mix: mixDone,
    export: !!input.exported,
  };

  const blockedReason = (id: StepId): string | null => {
    const i = ORDER.indexOf(id);
    for (const prev of ORDER.slice(0, i)) {
      if (STEP_COPY[prev].optional || done[prev]) continue;
      return ({
        script: "Add at least one line of dialogue first.",
        cast: `Choose a voice for ${unvoiced.slice(0, 2).join(" and ") || "every speaker"} first.`,
        render: "Render every line (or skip it) first.",
        export: "",
        direct: "", mix: "",
      } as Record<StepId, string>)[prev];
    }
    return null;
  };

  const progress: Record<StepId, { done: number; of: number } | null> = {
    script: hasScript ? { done: lines.length, of: lines.length } : null,
    cast: speakers.length ? { done: speakers.length - unvoiced.length, of: speakers.length } : null,
    direct: hasScript ? { done: lines.length - noteCount, of: lines.length } : null,
    render: hasScript ? { done: rendered + skipped, of: lines.length } : null,
    mix: null,
    export: null,
  };

  const summary: Record<StepId, string> = {
    script: hasScript ? `${plural(lines.length, "line")}, ${plural(speakers.length, "speaker")}` : "Nothing written yet",
    cast: !speakers.length ? "No speakers yet" : unvoiced.length ? `${plural(unvoiced.length, "speaker")} still need a voice` : "Every speaker has a voice",
    direct: noteCount ? `${plural(noteCount, "note")} to fix` : hasScript ? "No open notes" : "Nothing to direct yet",
    render: !hasScript ? "Nothing to render yet" : `${rendered} of ${lines.length} rendered${skipped ? `, ${skipped} skipped` : ""}`,
    mix: mixDone ? (project.cues.length ? `${plural(project.cues.length, "cue")} placed` : "Preview built") : "Voice track only",
    export: input.exported ? "Exported this session" : "Not exported yet",
  };

  const tasks: Record<StepId, { label: string; done: boolean }[]> = {
    script: [
      { label: "Write or paste dialogue", done: hasScript },
      { label: "Fix any script notes", done: hasScript && !(analysis?.parsed.issues.length) && !lines.some((l) => l.line.tag_issues.length) },
    ],
    cast: speakers.length
      ? speakers.map((s) => ({ label: `Voice for ${s}`, done: !unvoiced.includes(s) }))
      : [{ label: "Add dialogue first", done: false }],
    direct: [
      { label: "Fix notes (unsupported or unknown tags)", done: castDone && noteCount === 0 },
      { label: "Set emotion and pauses where you want them (optional)", done: Object.keys(project.line_directions).length + Object.keys(project.scene_directions).length > 0 },
    ],
    render: [
      { label: "Render every line, or skip it", done: renderDone },
      { label: "Listen and choose your favourite takes (optional)", done: project.takes.some((t) => t.status !== "pending") },
    ],
    mix: [
      { label: "Add music or effects (optional)", done: project.cues.length > 0 },
      { label: "Build a preview mix and check the loudness", done: !!input.mixPreviewed },
    ],
    export: [
      { label: "Clear every licence", done: false },
      { label: "Choose a folder and export", done: !!input.exported },
    ],
  };

  const cost: Record<StepId, { cost: Cost; note: string }> = {
    script: { cost: "free", note: "Always free." },
    cast: { cost: "free", note: "Choosing is free. A sample from a cloud voice uses your provider account, and asks first." },
    direct: { cost: "free", note: "Always free. Previews of cloud voices ask first." },
    render: { cost: "mixed", note: "Free with Windows voices. Cloud voices are billed by the provider and always ask first." },
    mix: { cost: "free", note: "Always free. Runs on this computer." },
    export: { cost: "free", note: "Always free. Runs on this computer." },
  };

  let next: NextAction;
  if (!hasScript) {
    next = { stepId: "script", kind: project.script.trim() ? "goto" : "sample", paid: false, label: project.script.trim() ? "Fix your script" : "Use the sample script", reason: "Soundstage needs at least one line that starts with a speaker name, like AMA: Hello." };
  } else if (!castDone) {
    next = { stepId: "cast", kind: "goto", paid: false, label: `Choose a voice for ${unvoiced[0]}`, count: unvoiced.length, reason: `${plural(unvoiced.length, "speaker")} still ${unvoiced.length === 1 ? "has" : "have"} no voice.` };
  } else if (noteCount > 0) {
    next = { stepId: "direct", kind: "goto", paid: false, label: `Fix ${plural(noteCount, "note")}`, count: noteCount, reason: "These lines use something their voice cannot do, or a tag Soundstage does not know. They cannot be rendered yet." };
  } else if (needRender.length > 0) {
    const cloud = cloudLines(analysis as Analysis, voices, needRender.map((l) => l.line.key));
    next = {
      stepId: "render", kind: "render", paid: cloud > 0, count: needRender.length,
      label: `Render ${plural(needRender.length, "line")}${cloud > 0 ? " (paid)" : ""}`,
      reason: cloud > 0 ? `${plural(cloud, "line")} will be sent to a cloud provider and billed to your account there. You will be asked first.` : "Free. These voices run on this computer.",
    };
  } else if (!input.mixPreviewed) {
    next = { stepId: "mix", kind: "preview-mix", paid: false, label: "Build a preview mix", reason: "Hear the whole scene and check its loudness before you export." };
  } else {
    next = { stepId: "export", kind: "export", paid: false, label: input.exported ? "Open the Export screen" : "Export your scene", reason: "Check the licences, then write the master and stems." };
  }
  // The recommended step is always the step the Next action belongs to, so the panel never contradicts itself.
  const current: StepId = next.stepId;

  const steps: GoldenStep[] = ORDER.map((id, index) => {
    const blocked = blockedReason(id);
    const status: StepStatus = id === current ? "current" : blocked ? "blocked" : done[id] ? "done" : "ready";
    return {
      id, index, label: STEP_COPY[id].label, optional: STEP_COPY[id].optional,
      status, progress: progress[id], summary: summary[id], why: STEP_COPY[id].why, ifSkipped: STEP_COPY[id].ifSkipped,
      cost: cost[id].cost, costNote: cost[id].note, blockedReason: status === "blocked" ? blocked : null,
      tasks: tasks[id], helpId: STEP_COPY[id].helpId,
    };
  });

  return { steps, next, current };
}
