import type { Voice } from "./types";

/** Is this a real speech voice (not the built-in test tones)? */
const isSpeech = (v: Voice) => v.language !== "zxx";

/**
 * Pick a voice for every speaker that has none. Free voices on this computer come first, English first, and each speaker
 * gets a different voice where there are enough of them, alternating gender when the voices say what it is. Speakers
 * that already have a voice keep it. Returns `null` when there is no speech voice to use at all.
 */
export function suggestCast(speakers: string[], assigned: Record<string, string | null | undefined>, voices: Voice[], remembered: Record<string, string> = {}): Record<string, string> | null {
  const speech = voices.filter(isSpeech);
  const local = speech.filter((v) => !v.sends_data);
  const pool = (local.length > 0 ? local : speech).slice().sort((a, b) => Number(b.language.toLowerCase().startsWith("en")) - Number(a.language.toLowerCase().startsWith("en")));
  if (pool.length === 0) return null;
  const used = new Set(speakers.map((s) => assigned[s]).filter((x): x is string => !!x));
  // Voices remembered for speakers that need one are set aside first, so nobody else takes them.
  const reserved = new Set<string>();
  for (const s of speakers) { const r = remembered[s]; if (!assigned[s] && r && speech.some((v) => v.id === r) && !used.has(r) && !reserved.has(r)) reserved.add(r); }
  const out: Record<string, string> = {};
  let lastGender = "";
  let turn = 0;
  for (const s of speakers) {
    if (assigned[s]) { lastGender = pool.find((v) => v.id === assigned[s])?.gender.toLowerCase() ?? lastGender; continue; }
    // The voice this speaker name had last time, if it is still here and free to use.
    const before = remembered[s] && reserved.has(remembered[s]!) ? speech.find((v) => v.id === remembered[s] && !used.has(v.id)) : undefined;
    if (before) { out[s] = before.id; used.add(before.id); lastGender = before.gender.toLowerCase(); continue; }
    const fresh = pool.filter((v) => !used.has(v.id) && !reserved.has(v.id));
    const pick = fresh.find((v) => lastGender && v.gender.toLowerCase() !== lastGender && /^(male|female)$/.test(v.gender.toLowerCase())) ?? fresh[0] ?? pool.filter((v) => !used.has(v.id))[0] ?? pool[turn % pool.length]!;
    turn += 1;
    out[s] = pick.id;
    used.add(pick.id);
    lastGender = pick.gender.toLowerCase();
  }
  return out;
}
