import type { ParsedScene } from "./types";

export interface SceneSuggestion { ambience: string; music: string }

const clean = (s: string) => s.replace(/[\r\n\t]+/g, " ").replace(/\s+/g, " ").trim();

/**
 * A starting description of the background sound and the music for a scene, written from its heading
 * (`# Kitchen, morning`) and its location and time of day when the script gives them. The person edits these before
 * anything is made, so they only need to be a sensible first draft.
 */
export function suggestScene(scene: Pick<ParsedScene, "title" | "location" | "time_of_day">): SceneSuggestion {
  const title = clean(scene.title) || "the scene";
  const place = clean(scene.location) || title;
  const time = clean(scene.time_of_day);
  const where = time && !place.toLowerCase().includes(time.toLowerCase()) ? `${place}, ${time}` : place;
  const dark = /night|evening|dusk|midnight/i.test(`${time} ${title}`);
  return {
    ambience: clean(`Quiet background ambience of ${where}, steady and natural, no voices and no music`).slice(0, 300),
    music: clean(`Soft, understated instrumental underscore for a scene in ${where}, ${dark ? "calm and a little mysterious" : "gentle and warm"}, no vocals`).slice(0, 300),
  };
}
