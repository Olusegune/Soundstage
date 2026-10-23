// The guided prompt builder: plain choices in, a clear description out. It works offline and needs no account.
// The optional "Improve with AI" button (see PromptHelper) sits on top of this.

export type HelperKind = "music" | "sfx" | "voice";

export interface Choice { id: string; label: string }
export interface Field { key: string; label: string; help: string; options: Choice[] }

const c = (...labels: string[]): Choice[] => labels.map((l) => ({ id: l.toLowerCase(), label: l }));

export const FIELDS: Record<HelperKind, Field[]> = {
  music: [
    { key: "mood", label: "Mood", help: "How should the listener feel?", options: c("Hopeful", "Sad", "Tense", "Joyful", "Mysterious", "Romantic", "Epic", "Calm") },
    { key: "genre", label: "Style", help: "The musical style.", options: c("Cinematic orchestral", "Solo piano", "Ambient", "Acoustic guitar", "Lo-fi", "Electronic", "Jazz", "Afrobeats") },
    { key: "tempo", label: "Speed", help: "How fast it moves.", options: c("Very slow", "Slow", "Medium", "Fast") },
    { key: "use", label: "Where it plays", help: "The moment in the film.", options: c("Opening titles", "Quiet emotional scene", "Chase or action", "Background under dialogue", "Closing credits") },
  ],
  sfx: [
    { key: "sound", label: "The sound", help: "What makes it?", options: c("Door creaking", "Footsteps", "Rain", "Thunder", "Crowd murmur", "Car passing", "Glass breaking", "Phone ringing") },
    { key: "place", label: "Where", help: "The space changes how it sounds.", options: c("Small room", "Large empty hall", "Outdoors", "Forest", "City street", "Underwater") },
    { key: "distance", label: "How close", help: "Close sounds are clearer and louder.", options: c("Very close", "Nearby", "Far away") },
    { key: "feel", label: "Character", help: "Optional flavour.", options: c("Gentle", "Heavy", "Sudden", "Slow and drawn out", "Eerie") },
  ],
  voice: [
    { key: "age", label: "Age", help: "Roughly how old.", options: c("Young adult", "Middle-aged", "Older", "Child-like") },
    { key: "gender", label: "Voice type", help: "The kind of voice.", options: c("Woman", "Man", "Neutral") },
    { key: "accent", label: "Accent", help: "Where the speaker is from.", options: c("Nigerian English", "British", "American", "Indian English", "Filipino English", "Australian", "Neutral") },
    { key: "tone", label: "Tone", help: "How they sound.", options: c("Warm and kind", "Gruff", "Bright and energetic", "Calm and slow", "Mysterious", "Serious") },
  ],
};

export const EXAMPLES: Record<HelperKind, string[]> = {
  music: ["Slow solo piano with soft strings, gentle and hopeful, for a quiet goodbye scene", "Tense low synth pulse building slowly, dark and mysterious, under dialogue"],
  sfx: ["Heavy wooden door creaking slowly open in a large empty hall, close to the microphone", "Steady rain on a tin roof with distant thunder"],
  voice: ["A calm older woman with a warm Nigerian English accent, slow and kind", "A gruff pirate captain, slow and gravelly, with a dry sense of humour"],
};

export const TIPS: Record<HelperKind, string[]> = {
  music: ["Say the feeling first, then the instruments.", "Add the speed (slow, medium, fast).", "Name the moment in the film it will sit under.", "Do not name real artists or songs."],
  sfx: ["Say what makes the sound and what it touches.", "Add the space (small room, outdoors).", "Say how close it is.", "Keep it short. One sound per effect."],
  voice: ["Describe a character, never a real person.", "Give age, accent, pitch and pace.", "Add one personality word.", "Use at least 20 characters."],
};

const PLACES: Record<string, string> = { "small room": "in a small room", "large empty hall": "in a large empty hall", outdoors: "outdoors", forest: "in a forest", "city street": "on a city street", underwater: "underwater" };

export type Picked = Record<string, string>;

/** Join the picked choices into one sentence. Empty fields are skipped; the person's own words are kept at the end. */
export function buildPrompt(kind: HelperKind, picked: Picked, extra: string): string {
  const v = (k: string) => (picked[k] ?? "").trim();
  const parts: string[] = [];
  if (kind === "music") {
    parts.push([v("tempo"), v("genre")].filter(Boolean).join(" ").toLowerCase());
    if (v("mood")) parts.push(`${v("mood").toLowerCase()} mood`);
    if (v("use")) parts.push(`for ${/^(opening|closing)/i.test(v("use")) ? "the " : "a "}${v("use").toLowerCase()}`);
  } else if (kind === "sfx") {
    parts.push([v("feel"), v("sound")].filter(Boolean).join(" ").toLowerCase());
    const place = PLACES[v("place").toLowerCase()];
    if (place) parts.push(place);
    if (v("distance")) parts.push(v("distance").toLowerCase());
  } else {
    const who = [v("age"), v("gender")].filter(Boolean).join(" ").toLowerCase();
    parts.push(who ? `${/^[aeiou]/.test(who) ? "An" : "A"} ${who}` : "A voice");
    if (v("accent")) parts.push(`with a${/^[AEIOU]/.test(v("accent")) ? "n" : ""} ${v("accent")} accent`);
    if (v("tone")) parts.push(v("tone").toLowerCase());
  }
  const base = parts.filter(Boolean).join(", ");
  const tail = extra.trim();
  const out = [base, tail].filter(Boolean).join(base && tail ? ". " : "");
  return out.length ? out[0]!.toUpperCase() + out.slice(1) : "";
}

export interface Strength { level: "empty" | "short" | "good"; message: string }

/** A plain check of how much a description tells the generator. Never blocks; only guides. */
export function strength(kind: HelperKind, text: string): Strength {
  const n = text.trim().length;
  const min = kind === "voice" ? 20 : 3;
  if (n === 0) return { level: "empty", message: "Pick a few options below or write it in your own words." };
  if (n < min) return { level: "short", message: `Add a little more (at least ${min} characters).` };
  const words = text.trim().split(/\s+/).length;
  if (words < (kind === "sfx" ? 4 : 8)) return { level: "short", message: "Good start. Add the feeling, the place or the speed for a better result." };
  return { level: "good", message: "That is a clear description." };
}
