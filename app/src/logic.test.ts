import { describe, expect, it } from "vitest";
import { canEnter, insertAt, removeTagFromRow, splitTags, stepComplete, withConditions } from "./logic";
import type { Analysis, Project } from "./types";

const project = {} as Project;
const analysis = (over: Partial<Analysis> = {}, statuses: ("drafted" | "rendered" | "stale" | "skipped")[] = ["drafted"]): Analysis => ({
  parsed: { scenes: [], speakers: ["AMA"], lines: [], issues: [] },
  lines: statuses.map((status, i) => ({ line: { key: `s0-l${i}`, scene: 0, speaker: "AMA", text: "x", row: i + 1, tag_issues: [] }, status } as never)),
  speakers_without_voice: [],
  ...over,
});

describe("step gating", () => {
  it("blocks forward steps until required ones are done, but never blocks going back", () => {
    const a = analysis({ speakers_without_voice: ["AMA"] });
    expect(canEnter(0, project, a)).toBe(true);
    expect(canEnter(1, project, a)).toBe(true); // Script done
    expect(canEnter(2, project, a)).toBe(false); // Cast not done
    expect(canEnter(5, project, a)).toBe(false);
  });
  it("needs every line rendered or skipped before Mix and Export", () => {
    expect(canEnter(4, project, analysis({}, ["rendered", "stale"]))).toBe(false);
    expect(canEnter(4, project, analysis({}, ["rendered", "skipped"]))).toBe(true);
  });
  it("optional steps do not gate later steps", () => {
    const a = analysis({}, ["rendered"]);
    expect(stepComplete("Render", project, a)).toBe(true);
    expect(canEnter(5, project, a)).toBe(true);
  });
  it("a script with no lines blocks everything after Script", () => {
    expect(canEnter(1, project, analysis({ lines: [] }))).toBe(false);
  });
});

describe("script editing helpers", () => {
  it("removes only the named tag on the named row", () => {
    const s = "A: hi [sneeze] there [sneeze]\nB: yo [sneeze]";
    expect(removeTagFromRow(s, 1, "sneeze")).toBe("A: hi there [sneeze]\nB: yo [sneeze]");
    expect(removeTagFromRow(s, 9, "sneeze")).toBe(s);
  });
  it("escapes regex characters in tag names", () => {
    expect(removeTagFromRow("A: x [pause:short] y", 1, "pause:short")).toBe("A: x y");
  });
  it("inserts at the caret", () => {
    expect(insertAt("ab", 1, "[x]")).toEqual({ text: "a[x]b", caret: 4 });
    expect(insertAt("ab", 99, "!")).toEqual({ text: "ab!", caret: 3 });
  });
  it("flags unknown tags and accepts known ones", () => {
    const parts = splitTags("Hi [sigh] [Pause:350] [laughs]");
    expect(parts.filter((p) => p.tag).map((p) => p.bad)).toEqual([false, false, true]);
  });
});


describe("withConditions", () => {
  const cap = { kind: "revenue_cap" as const, text: "Revenue limit", value: null, confirmed: null };
  it("adds declared conditions once and keeps the user's confirmation", () => {
    const base = { licence: "commercial" as const, source: null, checked: null, notes: "" };
    expect(withConditions(base, [cap]).conditions).toEqual([cap]);
    const mine = { ...base, conditions: [{ ...cap, confirmed: "2026-10-01" }] };
    expect(withConditions(mine, [cap]).conditions).toEqual([{ ...cap, confirmed: "2026-10-01" }]);
    expect(withConditions(base, undefined).conditions).toEqual([]);
  });
});
