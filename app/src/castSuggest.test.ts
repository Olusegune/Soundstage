import { describe, expect, it } from "vitest";
import { suggestCast } from "./castSuggest";
import type { Voice } from "./types";

const v = (id: string, over: Partial<Voice> = {}): Voice => ({
  id, name: id, provider: "p", provider_label: "P", language: "en-US", accent: "", gender: "male", age_range: "", sample_text: "", notes: "", tags: [], sends_data: false,
  licence: { licence: "unknown", source: null, checked: null, notes: "" }, ...over,
});

describe("suggestCast", () => {
  const voices = [v("david", { gender: "male" }), v("zira", { gender: "female" }), v("tone", { language: "zxx" }), v("cloud", { sends_data: true, gender: "female" }), v("hortense", { language: "fr-FR", gender: "female" })];

  it("gives each speaker a different free voice, alternating gender, never a test tone or a paid voice", () => {
    expect(suggestCast(["AMA", "KOLA"], {}, voices)).toEqual({ AMA: "david", KOLA: "zira" });
  });
  it("keeps voices that are already chosen and does not reuse them", () => {
    expect(suggestCast(["AMA", "KOLA"], { AMA: "zira" }, voices)).toEqual({ KOLA: "david" });
    expect(suggestCast(["AMA"], { AMA: "zira" }, voices)).toEqual({});
  });
  it("reuses voices only when there are not enough, and prefers English", () => {
    const out = suggestCast(["A", "B", "C", "D"], {}, voices)!;
    expect(Object.values(out)).toHaveLength(4);
    expect(new Set(Object.values(out).slice(0, 3)).size).toBe(3);
    expect(out.A).toBe("david");
  });
  it("falls back to paid voices only when nothing is free, and to null when there is nothing to speak with", () => {
    expect(suggestCast(["A"], {}, [v("cloud", { sends_data: true }), v("tone", { language: "zxx" })])).toEqual({ A: "cloud" });
    expect(suggestCast(["A"], {}, [v("tone", { language: "zxx" })])).toBeNull();
    expect(suggestCast(["A"], {}, [])).toBeNull();
  });

  it("brings back the voice a speaker name had before, unless it is gone or already taken", () => {
    expect(suggestCast(["AMA", "KOLA"], {}, voices, { AMA: "zira", KOLA: "david" })).toEqual({ AMA: "zira", KOLA: "david" });
    expect(suggestCast(["AMA", "KOLA"], {}, voices, { AMA: "gone", KOLA: "david" })).toEqual({ AMA: "zira", KOLA: "david" });
    expect(suggestCast(["AMA", "KOLA"], {}, voices, { AMA: "david", KOLA: "david" })).toEqual({ AMA: "david", KOLA: "zira" });
  });
});
