import { describe, expect, it } from "vitest";
import { golden, type GoldenInput } from "./golden";
import type { Analysis, LineInfo, LineStatus, Project, Voice } from "./types";

const project = (over: Partial<Project> = {}): Project => ({ script: "A: x", cues: [], takes: [], line_directions: {}, scene_directions: {}, ...over } as Project);
const line = (key: string, status: LineStatus, speaker = "AMA", issues: LineInfo["issues"] = [], voice: string | null = "v1"): LineInfo =>
  ({ line: { key, scene: 0, speaker, text: "hi", row: 1, tag_issues: [] }, status, issues, voice_id: voice } as unknown as LineInfo);
const analysis = (lines: LineInfo[], unvoiced: string[] = []): Analysis => ({
  parsed: { scenes: [], speakers: ["AMA", "KOLA"], lines: [], issues: [] }, lines, speakers_without_voice: unvoiced,
});
const voice = (id: string, sends: boolean): Voice => ({ id, sends_data: sends } as Voice);
const run = (over: Partial<GoldenInput>) => golden({ project: project(), analysis: null, voices: [], ...over });

describe("Golden Path", () => {
  it("starts at the script and offers the sample when nothing is written", () => {
    const g = run({ project: project({ script: "" }), analysis: analysis([]) });
    expect(g.current).toBe("script");
    expect(g.next).toMatchObject({ stepId: "script", kind: "sample" });
    expect(g.steps.find((s) => s.id === "cast")?.status).toBe("blocked");
    expect(g.steps.find((s) => s.id === "cast")?.blockedReason).toMatch(/dialogue/);
  });

  it("points at the first speaker without a voice, and says how many", () => {
    const g = run({ analysis: analysis([line("s0-l0", "drafted", "AMA", [], null), line("s0-l1", "drafted", "KOLA", [], null)], ["AMA", "KOLA"]) });
    expect(g.current).toBe("cast");
    expect(g.next).toMatchObject({ stepId: "cast", count: 2, label: "Choose a voice for AMA" });
    expect(g.steps.find((s) => s.id === "render")?.status).toBe("blocked");
  });

  it("pulls the optional Direct step forward only when notes would block rendering", () => {
    const issue = [{ kind: "unsupported_tag", message: "x", tag: "sneeze" }];
    const g = run({ analysis: analysis([line("s0-l0", "drafted", "AMA", issue), line("s0-l1", "drafted")]) });
    expect(g.current).toBe("direct");
    expect(g.next).toMatchObject({ stepId: "direct", count: 1, label: "Fix 1 note" });
    const clean = run({ analysis: analysis([line("s0-l0", "drafted"), line("s0-l1", "drafted")]) });
    expect(clean.current).toBe("render");
    expect(clean.steps.find((s) => s.id === "direct")?.optional).toBe(true);
  });

  it("marks a render as paid only when a cloud voice is involved, and says so on the button", () => {
    const free = run({ analysis: analysis([line("a", "drafted", "AMA", [], "w")]), voices: [voice("w", false)] });
    expect(free.next).toMatchObject({ kind: "render", paid: false });
    expect(free.next?.label).not.toMatch(/paid/);
    const paid = run({ analysis: analysis([line("a", "drafted", "AMA", [], "c"), line("b", "rendered", "AMA", [], "w")]), voices: [voice("c", true), voice("w", false)] });
    expect(paid.next).toMatchObject({ kind: "render", paid: true, count: 1 });
    expect(paid.next?.label).toMatch(/paid/);
  });

  it("treats stale lines as work to do, and skipped lines as finished", () => {
    const g = run({ analysis: analysis([line("a", "stale"), line("b", "skipped"), line("c", "rendered")]) });
    expect(g.next).toMatchObject({ kind: "render", count: 1 });
    const done = run({ analysis: analysis([line("a", "skipped"), line("b", "rendered")]) });
    expect(done.steps.find((s) => s.id === "render")?.status).toBe("done");
    expect(done.steps.find((s) => s.id === "render")?.progress).toEqual({ done: 2, of: 2 });
  });

  it("recommends the preview mix, then export, and reports progress honestly", () => {
    const all = analysis([line("a", "rendered"), line("b", "rendered")]);
    expect(run({ analysis: all }).next).toMatchObject({ kind: "preview-mix" });
    expect(run({ analysis: all }).current, "the recommended step matches the Next action").toBe("mix");
    const g = run({ analysis: all, mixPreviewed: true });
    expect(g.next).toMatchObject({ kind: "export", stepId: "export" });
    expect(g.steps.find((s) => s.id === "mix")?.status).toBe("done");
    expect(g.current).toBe("export");
  });

  it("never blocks going back: every earlier step stays reachable and done steps say so", () => {
    const g = run({ analysis: analysis([line("a", "rendered")]), mixPreviewed: true });
    for (const s of g.steps.filter((x) => x.index < 5)) expect(s.status === "blocked").toBe(false);
  });

  it("every step explains why it matters and what skipping does", () => {
    const g = run({ analysis: analysis([line("a", "drafted")]) });
    for (const s of g.steps) {
      expect(s.why.length).toBeGreaterThan(30);
      expect(s.ifSkipped.length).toBeGreaterThan(10);
      expect(s.helpId).toBeTruthy();
      expect(["free", "paid", "mixed"]).toContain(s.cost);
    }
  });

  it("only one step is current", () => {
    const g = run({ analysis: analysis([line("a", "drafted")]) });
    expect(g.steps.filter((s) => s.status === "current")).toHaveLength(1);
  });
});
