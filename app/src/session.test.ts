import { beforeEach, describe, expect, it } from "vitest";
import { addSent, resetSent, sentBy, sentTotals } from "./session";

describe("session totals", () => {
  beforeEach(() => resetSent());
  it("adds up per provider and ignores nonsense", () => {
    addSent("ElevenLabs", 100); addSent("ElevenLabs", 50.4); addSent("VoiceKiller", 20);
    addSent("ElevenLabs", -5); addSent("ElevenLabs", NaN);
    expect(sentBy("ElevenLabs")).toBe(150);
    expect(sentTotals()).toEqual([["ElevenLabs", 150], ["VoiceKiller", 20]]);
    expect(sentBy("Nobody")).toBe(0);
  });
});
