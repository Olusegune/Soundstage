// Starting points for the "Add an engine" buttons. These are typical addresses and names, NOT promises: ports and
// model names differ between installs, and nothing here says an engine's quality, accents or non-verbal ability.
// Licence always starts as unknown, because it depends on the weights the user actually runs.
import type { JobEngine, SpeechEngine, VoiceDef } from "./types";

export interface Preset {
  key: string;
  label: string;
  blurb: string;
  where: "this computer" | "the internet" | "your choice";
  engine: Omit<SpeechEngine, "id">;
  /** Plain advice shown under the preset. */
  help: string;
}

const v = (id: string, name: string, language: string, accent: string, gender: string): VoiceDef => ({ id, name, language, accent, gender });

const base: Omit<SpeechEngine, "id" | "label" | "base_url" | "model"> = {
  voices: [], response_format: "wav", tags: [], tag_map: {}, needs_key: false, licence_note: "", notes: "",
};

export const PRESETS: Preset[] = [
  {
    key: "kokoro", label: "Kokoro (local server)", where: "this computer",
    blurb: "A small, fast open voice model you run yourself. Needs a Kokoro server such as Kokoro-FastAPI.",
    help: "Start your Kokoro server, then press Find voices to list exactly what it has. The default address is the one Kokoro-FastAPI usually uses.",
    engine: {
      ...base, label: "Kokoro (local)", base_url: "http://127.0.0.1:8880/v1", model: "kokoro",
      voices: [v("af_heart", "Heart", "en-US", "American", "female"), v("af_bella", "Bella", "en-US", "American", "female"), v("am_adam", "Adam", "en-US", "American", "male"), v("bf_emma", "Emma", "en-GB", "British", "female"), v("bm_george", "George", "en-GB", "British", "male")],
      licence_note: "Kokoro-82M is reported as Apache-2.0. Check the model card of the weights your server uses.",
    },
  },
  {
    key: "localai", label: "LocalAI", where: "this computer",
    blurb: "A local server that can run several speech models behind one address.",
    help: "Enter the model name your LocalAI install exposes, then add or discover voices.",
    engine: { ...base, label: "LocalAI", base_url: "http://127.0.0.1:8080/v1", model: "", licence_note: "Depends on the model you run through LocalAI." },
  },
  {
    key: "speaches", label: "Speaches", where: "this computer",
    blurb: "A local server for speech models with an OpenAI-style address.",
    help: "Enter the model name your Speaches install uses, then add or discover voices.",
    engine: { ...base, label: "Speaches", base_url: "http://127.0.0.1:8000/v1", model: "", licence_note: "Depends on the model you run through Speaches." },
  },
  {
    key: "openrouter", label: "OpenRouter", where: "the internet",
    blurb: "One key for speech models from several providers. Your text is sent over the internet and billed to your OpenRouter account.",
    help: "Paste your OpenRouter key below. Model names change over time; the one filled in came from OpenRouter's documentation and may need updating.",
    engine: {
      ...base, label: "OpenRouter", base_url: "https://openrouter.ai/api/v1", model: "openai/gpt-4o-mini-tts-2025-12-15", response_format: "mp3", needs_key: true,
      voices: ["alloy", "ash", "ballad", "coral", "echo", "fable", "nova", "onyx", "sage", "shimmer"].map((x) => v(x, x[0]!.toUpperCase() + x.slice(1), "en", "", "")),
      licence_note: "OpenRouter routes to other providers. Commercial rights follow the terms of the provider behind the model.",
    },
  },
  {
    key: "custom", label: "Another OpenAI-compatible server", where: "your choice",
    blurb: "Any server that accepts the OpenAI speech request: Chatterbox, Orpheus, Dia and many others run behind one.",
    help: "Enter its address (it ends in /v1), the model name, and a voice or two.",
    engine: { ...base, label: "My engine", base_url: "http://127.0.0.1:8000/v1", model: "" },
  },
];

export function slug(s: string): string {
  const x = s.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "").slice(0, 32);
  return x || "engine";
}

export function uniqueId(label: string, taken: string[]): string {
  const b = slug(label);
  if (!taken.includes(b)) return b;
  for (let i = 2; i < 100; i++) if (!taken.includes(`${b}-${i}`)) return `${b}-${i}`;
  return `${b}-${Date.now().toString(36)}`;
}

/** One voice per line: `id | name | language | accent | gender`. Everything after the id is optional. */
export function parseVoices(text: string): VoiceDef[] {
  return text.split("\n").map((l) => l.trim()).filter(Boolean).map((l) => {
    const [id = "", name = "", language = "", accent = "", gender = ""] = l.split("|").map((x) => x.trim());
    return { id, name: name || id, language, accent, gender };
  });
}

export function formatVoices(defs: VoiceDef[]): string {
  return defs.map((d) => [d.id, d.name === d.id ? "" : d.name, d.language, d.accent, d.gender].join(" | ").replace(/( \| )+$/, "")).join("\n");
}

export type Reach = "this computer" | "your network" | "the internet";
export function reachOf(url: string): Reach {
  try {
    const u = new URL(url);
    const h = u.hostname.replace(/^\[|\]$/g, "");
    if (h === "localhost" || h.endsWith(".localhost") || h === "::1" || /^127\./.test(h)) return "this computer";
    if (/^(10\.|192\.168\.|169\.254\.)/.test(h) || /^172\.(1[6-9]|2\d|3[01])\./.test(h) || /^f[cd][0-9a-f]{2}:/i.test(h)) return "your network";
    return "the internet";
  } catch {
    return "the internet";
  }
}

export interface JobPreset { key: string; label: string; blurb: string; help: string; job: Omit<JobEngine, "id"> }

const jobBase: Omit<JobEngine, "id" | "label" | "provider" | "kind"> = {
  model: "", base_url: "http://127.0.0.1:8188", template: "", voices: [], licence_note: "", min_s: 3, max_s: 300, default_s: 30,
};

/** Templates use {{prompt}}, {{seconds}}, {{instrumental}}, {{seed}}. Each service names its own fields, so these are
 *  starting points to check against the service's documentation, not guarantees. */
export const JOB_PRESETS: JobPreset[] = [
  {
    key: "kie-suno", label: "Suno music through Kie",
    blurb: "Songs and instrumentals from Suno, paid with a Kie key. Your description is sent over the internet.",
    help: "Save your Kie key above. The length is chosen by Suno, so there is no length slider.",
    job: { ...jobBase, label: "Suno (Kie)", provider: "kie", kind: "music", model: "ai-music-api/generate", template: '{"custom_mode":false,"instrumental":"{{instrumental}}","model":"V6","prompt":"{{prompt}}"}', licence_note: "Suno output terms depend on your Suno or Kie plan. Read them before commercial use.", min_s: 30, max_s: 240, default_s: 120 },
  },
  {
    key: "fal-music", label: "Music through fal",
    blurb: "Any fal music model. Enter its model id and match the template to the fields on its fal page.",
    help: "Save your fal key above, then open the model's page on fal and copy its input field names into the template.",
    job: { ...jobBase, label: "Music (fal)", provider: "fal", kind: "music", model: "", template: '{"prompt":"{{prompt}}","seconds_total":"{{seconds}}"}', licence_note: "Depends on the model you choose on fal." },
  },
  {
    key: "fal-sfx", label: "Sound effects through fal",
    blurb: "Any fal sound-effect model. Enter its model id and match the template to its fields.",
    help: "Save your fal key above, then copy the field names from the model's fal page.",
    job: { ...jobBase, label: "Effects (fal)", provider: "fal", kind: "sfx", model: "", template: '{"prompt":"{{prompt}}","duration_seconds":"{{seconds}}"}', licence_note: "Depends on the model you choose on fal.", min_s: 1, max_s: 30, default_s: 5 },
  },
  {
    key: "comfy-music", label: "Music in ComfyUI",
    blurb: "Runs your own ComfyUI workflow on this computer. No account and nothing leaves your computer.",
    help: "In ComfyUI choose Save (API format), then paste the whole file below and replace your prompt text with {{prompt}} (and the length with {{seconds}}).",
    job: { ...jobBase, label: "Music (ComfyUI)", provider: "comfy", kind: "music", licence_note: "Depends on the model files your workflow loads." },
  },
  {
    key: "comfy-sfx", label: "Sound effects in ComfyUI",
    blurb: "Runs your own sound-effect workflow in ComfyUI on this computer.",
    help: "Save your workflow in API format, paste it below, and put {{prompt}} where the description goes.",
    job: { ...jobBase, label: "Effects (ComfyUI)", provider: "comfy", kind: "sfx", licence_note: "Depends on the model files your workflow loads.", min_s: 1, max_s: 30, default_s: 5 },
  },
  {
    key: "comfy-speech", label: "Speech in ComfyUI",
    blurb: "Runs your own text-to-speech workflow in ComfyUI. Voices are whatever your workflow offers.",
    help: "Put {{text}} where the line goes and {{voice}} where the voice name goes, then list the voices below.",
    job: { ...jobBase, label: "Speech (ComfyUI)", provider: "comfy", kind: "speech", licence_note: "Depends on the model files your workflow loads." },
  },
];
