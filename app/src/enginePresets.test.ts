import { describe, expect, it } from "vitest";
import { PRESETS, formatVoices, parseVoices, reachOf, slug, uniqueId } from "./enginePresets";

describe("engine presets", () => {
  it("make no quality, accent or tag claims and start with an unknown licence", () => {
    for (const p of PRESETS) {
      expect(p.engine.tags, p.key).toEqual([]);
      expect(p.engine.tag_map, p.key).toEqual({});
      expect(p.engine.licence_note.length, p.key).toBeLessThanOrEqual(300);
    }
  });
  it("send nothing over the internet unless the preset says so", () => {
    for (const p of PRESETS) {
      const reach = reachOf(p.engine.base_url);
      if (p.where === "this computer") expect(reach, p.key).toBe("this computer");
      if (p.where === "the internet") { expect(reach, p.key).toBe("the internet"); expect(p.engine.base_url.startsWith("https://"), p.key).toBe(true); expect(p.engine.needs_key, p.key).toBe(true); }
    }
  });
  it("only ever offers http for this computer", () => {
    for (const p of PRESETS) if (p.engine.base_url.startsWith("http://")) expect(reachOf(p.engine.base_url), p.key).not.toBe("the internet");
  });
});

describe("voice lists", () => {
  it("parse one voice per line with optional columns", () => {
    expect(parseVoices("af_bella | Bella | en-US | American | female\n\nbare\n  x | Name | yo-NG ")).toEqual([
      { id: "af_bella", name: "Bella", language: "en-US", accent: "American", gender: "female" },
      { id: "bare", name: "bare", language: "", accent: "", gender: "" },
      { id: "x", name: "Name", language: "yo-NG", accent: "", gender: "" },
    ]);
  });
  it("round-trip through text", () => {
    const defs = parseVoices("a | A | en-US | American | female\nb");
    expect(parseVoices(formatVoices(defs))).toEqual(defs);
  });
});

describe("ids and reach", () => {
  it("make short safe ids and avoid clashes", () => {
    expect(slug("My Engine (Local)!")).toBe("my-engine-local");
    expect(slug("***")).toBe("engine");
    expect(uniqueId("Kokoro", ["kokoro"])).toBe("kokoro-2");
    expect(uniqueId("Kokoro", ["kokoro", "kokoro-2"])).toBe("kokoro-3");
  });
  it("classify where text goes", () => {
    expect(reachOf("http://127.0.0.1:8880/v1")).toBe("this computer");
    expect(reachOf("http://localhost:8000")).toBe("this computer");
    expect(reachOf("http://[::1]:8000/v1")).toBe("this computer");
    expect(reachOf("http://192.168.1.20:8000/v1")).toBe("your network");
    expect(reachOf("http://172.20.0.5/v1")).toBe("your network");
    expect(reachOf("https://openrouter.ai/api/v1")).toBe("the internet");
    expect(reachOf("not a url")).toBe("the internet");
  });
});
