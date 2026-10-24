import type { Analysis, Licence, LicenceInfo, Project, StepName } from "./types";
import { STEPS } from "./types";

export const REQUIRED: Record<StepName, boolean> = {
  Script: true, Cast: true, Direct: false, Render: true, Mix: false, Export: true,
};

/** Is a step finished? Optional steps (Direct, Mix) count as finished once reached. */
export function stepComplete(step: StepName, project: Project, a: Analysis | null): boolean {
  if (!a) return false;
  switch (step) {
    case "Script":
      return a.lines.length > 0;
    case "Cast":
      return a.lines.length > 0 && a.speakers_without_voice.length === 0;
    case "Render":
      return a.lines.length > 0 && a.lines.every((l) => l.status === "rendered" || l.status === "skipped") &&
        a.lines.some((l) => l.status === "rendered");
    case "Direct":
    case "Mix":
      return stepComplete("Cast", project, a);
    case "Export":
      return false;
  }
}

/** A user may jump back freely, and forward only past finished required steps. */
export function canEnter(index: number, project: Project, a: Analysis | null): boolean {
  for (let i = 0; i < index; i++) {
    const name = STEPS[i];
    if (name && REQUIRED[name] && !stepComplete(name, project, a)) return false;
  }
  return true;
}

export function firstBlockedReason(index: number, project: Project, a: Analysis | null): string | null {
  for (let i = 0; i < index; i++) {
    const name = STEPS[i];
    if (name && REQUIRED[name] && !stepComplete(name, project, a)) {
      return ({
        Script: "Add at least one line of dialogue first.",
        Cast: "Give every speaker a voice first.",
        Render: "Render every line (or skip it) first.",
        Direct: "", Mix: "", Export: "",
      } as Record<StepName, string>)[name];
    }
  }
  return null;
}

/** Remove the first `[tag]` from a 1-based script row. Nothing else changes. */
export function removeTagFromRow(script: string, row: number, tag: string): string {
  const lines = script.split("\n");
  const i = row - 1;
  const line = lines[i];
  if (line === undefined) return script;
  const re = new RegExp(`\\s*\\[\\s*${tag.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*\\]`, "i");
  lines[i] = line.replace(re, "");
  return lines.join("\n");
}

export function insertAt(text: string, pos: number, insert: string): { text: string; caret: number } {
  const p = Math.max(0, Math.min(pos, text.length));
  return { text: text.slice(0, p) + insert + text.slice(p), caret: p + insert.length };
}

export function fmtMs(ms: number): string {
  const s = Math.round(ms / 100) / 10;
  if (s < 60) return `${s.toFixed(1)} s`;
  return `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, "0")}`;
}

export function licenceText(l: Licence): string {
  return l === "commercial" ? "Commercial" : l === "non-commercial" ? "Non-commercial" : "Licence unknown";
}

export function licenceClass(l: Licence): string {
  return l === "commercial" ? "ok" : l === "non-commercial" ? "bad" : "warn";
}

export function today(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export function daysSince(date: string | null): number | null {
  if (!date) return null;
  const t = Date.parse(`${date}T00:00:00Z`);
  return Number.isNaN(t) ? null : Math.floor((Date.now() - t) / 86_400_000);
}

export function licenceNote(info: LicenceInfo): string {
  const age = daysSince(info.checked);
  if (info.licence === "unknown") return "Not checked";
  if (age === null) return "No check date";
  return age > 90 ? `Checked ${age} days ago: needs re-check` : `Checked ${info.checked}`;
}

/** Split text into plain and tag parts so tags can be highlighted. Unknown tags are flagged. */
const KNOWN = /^(laugh|chuckle|sigh|cough|sneeze|gasp|cry|breath|sniff|whisper|shout|excited|sad|tired|sarcastic|curious|serious|pause:(short|long|[1-9]\d{0,4}))$/i;
export function splitTags(text: string): { text: string; tag: boolean; bad: boolean }[] {
  const out: { text: string; tag: boolean; bad: boolean }[] = [];
  const re = /\[([^\[\]]*)\]/g;
  let last = 0;
  for (let m = re.exec(text); m; m = re.exec(text)) {
    if (m.index > last) out.push({ text: text.slice(last, m.index), tag: false, bad: false });
    out.push({ text: m[0], tag: true, bad: !KNOWN.test((m[1] ?? "").trim()) });
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push({ text: text.slice(last), tag: false, bad: false });
  return out;
}

/** A blank character profile. */
export function emptyBible() {
  return { age: "", background: "", speech_habits: "", emotional_range: "", notes: "" };
}
