import { describe, expect, it } from "vitest";
import { ARTICLES, GLOSSARY, TOUR_STEPS } from "./content";
import { STEP_COPY } from "../golden";
import { DEFAULT_SHORTCUTS } from "../shortcuts";
import { articleById, articleText, score, searchHelp } from "./search";

describe("help content", () => {
  it("has unique ids and no empty articles", () => {
    const ids = ARTICLES.map((a) => a.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const a of ARTICLES) {
      expect(a.title.length).toBeGreaterThan(3);
      expect(articleText(a).length, a.id).toBeGreaterThan(80);
    }
  });
  it("covers every Golden Path step with an article that exists", () => {
    for (const s of Object.values(STEP_COPY)) expect(articleById(s.helpId), s.helpId).toBeTruthy();
  });
  it("covers every topic the brief names", () => {
    const ids = ARTICLES.map((a) => a.id);
    for (const need of ["quick-start", "keyboard", "troubleshooting", "free-vs-paid", "simple-advanced", "providers"]) expect(ids).toContain(need);
    expect(GLOSSARY.length).toBeGreaterThanOrEqual(20);
  });
  it("explains every term the interface uses without explanation elsewhere", () => {
    const terms = GLOSSARY.map((g) => g.term.toLowerCase());
    for (const t of ["lufs", "true peak", "ducking", "stem", "take", "bus", "manifest", "consent record", "cloud voice"]) expect(terms).toContain(t);
  });
  it("documents shortcuts that really exist", () => {
    const all = new Set(DEFAULT_SHORTCUTS.flatMap((d) => d.keys));
    const keyboard = articleById("keyboard")!;
    const text = articleText(keyboard);
    for (const k of ["Ctrl+K", "Ctrl+N", "Ctrl+S", "Ctrl+Enter", "Ctrl+B", "Ctrl+J", "Ctrl+Shift+M", "F1", "Ctrl+/"]) {
      expect(all.has(k), `${k} is not a default shortcut`).toBe(true);
      expect(text, k).toContain(k);
    }
  });
  it("never promises what v0.1 does not do", () => {
    const everything = ARTICLES.map(articleText).join(" ").toLowerCase();
    for (const lie of ["generate music", "mp3 export", "voice design is available", "cloning is available"]) expect(everything).not.toContain(lie);
  });
  it("tour targets are unique", () => {
    expect(new Set(TOUR_STEPS.map((t) => t.target)).size).toBe(TOUR_STEPS.length);
  });
});

describe("help search", () => {
  it("finds an article by its topic", () => {
    expect(searchHelp("ducking")[0]?.title.toLowerCase()).toMatch(/ducking|mix/);
    expect(searchHelp("licence")[0]?.id).toBeTruthy();
    expect(searchHelp("sigh").some((h) => h.id === "directing-performance")).toBe(true);
  });
  it("finds glossary terms and aliases", () => {
    expect(searchHelp("lufs").some((h) => h.kind === "glossary" && h.id === "LUFS")).toBe(true);
    expect(searchHelp("stale").some((h) => h.kind === "glossary")).toBe(true);
  });
  it("returns nothing for nonsense", () => {
    expect(searchHelp("zzzzqqqq")).toEqual([]);
  });
  it("ranks exact and prefix matches first", () => {
    expect(score("render", "Render")).toBeGreaterThan(score("render", "Step 4: Render"));
    expect(score("rndr", "render")).toBeGreaterThan(0);
    expect(score("xyz", "render")).toBe(0);
  });
});
