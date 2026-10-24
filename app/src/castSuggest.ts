import type { Voice } from "./types";

/** Is this a real speech voice (not the built-in test tones)? */
const isSpeech = (v: Voice) => v.language !== "zxx";

/**
 * Pick a voice for every speaker that has none. Free voices on this computer come first, English first, and each speaker
 * gets a different voice where there are enough of them, alternating gender when the voices say what it is. Speakers
 * that already have a voice keep it. Returns `null` when there is no speech voice to use at all.
 */
export function suggestCast(speakers: string[], assigned: Record<string, string | null | undefined>, voices: Voice[]): Record<string, string> | null {
  const speech = voices.filter(isSpeech);
  const local = speech.filter((v) => !v.sends_data);
  const pool = (local.length > 0 ? local : speech).slice().sort((a, b) => Number(b.language.toLowerCase().startsWith("en")) - Number(a.language.toLowerCase().startsWith("en")));
  if (pool.length === 0) return null;
  const used = new Set(speakers.map((s) => assigned[s]).filter((x): x is string => !!x));
  const out: Record<string, string> = {};
  let lastGender = "";
  let turn = 0;
  for (const s of speakers) {
    if (assigned[s]) { lastGender = pool.find((v) => v.id === assigned[s])?.gender.toLowerCase() ?? lastGender; continue; }
    const fresh = pool.filter((v) => !used.has(v.id));
    const pick = fresh.find((v) => lastGender && v.gender.toLowerCase() !== lastGender && /^(male|female)$/.test(v.gender.toLowerCase())) ?? fresh[0] ?? pool[turn % pool.length]!;
    turn += 1;
    out[s] = pick.id;
    used.add(pick.id);
    lastGender = pick.gender.toLowerCase();
  }
  return out;
}
