import { describe, expect, it } from "vitest";
import { flagUrl, languageCode, languageName, regionOf } from "./flags";

describe("flags", () => {
  it("uses the country in the locale, for accents that have one", () => {
    expect(regionOf("en-GB")).toBe("gb");
    expect(regionOf("en-US")).toBe("us");
    expect(regionOf("yo-NG")).toBe("ng");
    expect(regionOf("hi-IN")).toBe("in");
    expect(regionOf("pt_BR")).toBe("br");
    expect(regionOf("zh-Hant-TW")).toBe("tw");
  });
  it("never guesses a flag for a bare language", () => {
    for (const l of ["en", "yo", "hi", "fr", "ar", "zxx", "", "es-419", "und"]) expect(regionOf(l), l).toBeNull();
    expect(flagUrl("en")).toBeNull();
  });
  it("ignores regions it has no flag for", () => {
    expect(regionOf("en-ZZ")).toBeNull();
  });
  it("gives readable names and short codes", () => {
    expect(languageName("en-GB")).toMatch(/English/);
    expect(languageName("zxx")).toMatch(/tones/i);
    expect(languageName("nonsense-xx")).toBeTruthy();
    expect(languageCode("yo-NG")).toBe("YO");
  });
  it("finds a real flag file for known countries", () => {
    expect(flagUrl("en-NG")).toMatch(/ng.*\.svg$/);
  });
});
