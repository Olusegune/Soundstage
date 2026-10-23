// Per-viewer preferences. Stored in localStorage when it works and ignored when it does not (private window,
// blocked storage). The app renders correctly either way. Preferences never touch project files.
import type { ShortcutId } from "./shortcuts";

export type Theme = "dark" | "light" | "system";
export type Mode = "simple" | "advanced";

export interface Prefs {
  theme: Theme;
  mode: Mode;
  railCollapsed: boolean;
  guideOpen: boolean;
  guideWidth: number;
  welcomed: boolean;
  toured: boolean;
  shortcuts: Partial<Record<ShortcutId, string[]>>;
  /** A specific ffmpeg.exe the user chose. Empty means look for one automatically. */
  ffmpegPath: string;
}

export const DEFAULT_PREFS: Prefs = {
  theme: "dark", mode: "simple", railCollapsed: false, guideOpen: true, guideWidth: 352, welcomed: false, toured: false, shortcuts: {}, ffmpegPath: "",
};

const KEY = "soundstage.prefs.v1";

export function sanitize(raw: unknown): Prefs {
  const r = (raw && typeof raw === "object" ? raw : {}) as Record<string, unknown>;
  const shortcuts: Prefs["shortcuts"] = {};
  if (r.shortcuts && typeof r.shortcuts === "object") {
    for (const [k, v] of Object.entries(r.shortcuts as Record<string, unknown>)) {
      if (Array.isArray(v) && v.every((x) => typeof x === "string" && x.length < 40)) shortcuts[k as ShortcutId] = v as string[];
    }
  }
  const w = typeof r.guideWidth === "number" && Number.isFinite(r.guideWidth) ? r.guideWidth : DEFAULT_PREFS.guideWidth;
  return {
    theme: r.theme === "light" || r.theme === "system" ? r.theme : "dark",
    mode: r.mode === "advanced" ? "advanced" : "simple",
    railCollapsed: r.railCollapsed === true,
    guideOpen: r.guideOpen !== false,
    guideWidth: Math.max(260, Math.min(520, w)),
    welcomed: r.welcomed === true,
    toured: r.toured === true,
    shortcuts,
    ffmpegPath: typeof r.ffmpegPath === "string" && r.ffmpegPath.length < 400 && !/[\r\n\0]/.test(r.ffmpegPath) ? r.ffmpegPath : "",
  };
}

export function loadPrefs(): Prefs {
  try {
    const t = window.localStorage.getItem(KEY);
    return sanitize(t ? JSON.parse(t) : {});
  } catch {
    return { ...DEFAULT_PREFS };
  }
}

export function savePrefs(p: Prefs): void {
  try {
    window.localStorage.setItem(KEY, JSON.stringify(p));
  } catch {
    /* storage unavailable: the choice lasts for this session only */
  }
}

export function resolveTheme(t: Theme): "dark" | "light" {
  if (t === "system") {
    try { return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark"; } catch { return "dark"; }
  }
  return t;
}

export function applyTheme(t: Theme): void {
  document.documentElement.setAttribute("data-theme", resolveTheme(t));
}
