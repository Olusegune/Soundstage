import { describe, expect, it } from "vitest";
import { suggestScene } from "./sceneSounds";

describe("suggestScene", () => {
  it("uses the location and time of day when the script gives them", () => {
    const s = suggestScene({ title: "Kitchen, morning", location: "Kitchen", time_of_day: "morning" });
    expect(s.ambience).toBe("Quiet background ambience of Kitchen, morning, steady and natural, no voices and no music");
    expect(s.music).toContain("gentle and warm");
  });
  it("falls back to the title, and picks a calmer mood at night", () => {
    const s = suggestScene({ title: "Market, later", location: "", time_of_day: "" });
    expect(s.ambience).toContain("Market, later");
    expect(suggestScene({ title: "Roof", location: "Rooftop", time_of_day: "night" }).music).toContain("a little mysterious");
  });
  it("copes with empty or messy headings and keeps descriptions short", () => {
    expect(suggestScene({ title: "", location: "", time_of_day: "" }).ambience).toContain("the scene");
    expect(suggestScene({ title: "A\nB\t  C", location: "", time_of_day: "" }).ambience).toContain("A B C");
    expect(suggestScene({ title: "x".repeat(900), location: "", time_of_day: "" }).music.length).toBeLessThanOrEqual(300);
  });
});
