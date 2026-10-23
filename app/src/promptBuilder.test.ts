import { describe, expect, it } from "vitest";
import { buildPrompt, EXAMPLES, FIELDS, strength } from "./promptBuilder";

describe("prompt builder", () => {
  it("builds a music description from choices and keeps the person's own words", () => {
    const t = buildPrompt("music", { mood: "Hopeful", genre: "Solo piano", tempo: "Slow", use: "Quiet emotional scene" }, "with soft strings");
    expect(t).toBe("Slow solo piano, hopeful mood, for a quiet emotional scene. with soft strings");
  });
  it("builds sound effect and voice descriptions", () => {
    expect(buildPrompt("sfx", { sound: "Door creaking", place: "Large empty hall", distance: "Very close" }, "")).toBe("Door creaking, in a large empty hall, very close");
    expect(buildPrompt("voice", { age: "Older", gender: "Woman", accent: "Nigerian English", tone: "Warm and kind" }, "")).toBe("An older woman, with a Nigerian English accent, warm and kind");
  });
  it("returns nothing when nothing is chosen, and uses only the extra text when that is all there is", () => {
    expect(buildPrompt("music", {}, "")).toBe("");
    expect(buildPrompt("sfx", {}, "a dog barking")).toBe("A dog barking");
  });
  it("rates strength without blocking", () => {
    expect(strength("music", "").level).toBe("empty");
    expect(strength("voice", "old man").level).toBe("short");
    expect(strength("music", "Slow solo piano with soft strings, gentle and hopeful").level).toBe("good");
  });
  it("offers options and examples for every kind, and examples never name real people", () => {
    for (const k of ["music", "sfx", "voice"] as const) {
      expect(FIELDS[k].length).toBeGreaterThan(2);
      for (const f of FIELDS[k]) expect(f.options.length).toBeGreaterThan(1);
      expect(EXAMPLES[k].length).toBeGreaterThan(0);
      for (const e of EXAMPLES[k]) expect(e).not.toMatch(/sounds like|in the voice of/i);
    }
  });
});
