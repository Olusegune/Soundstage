// Keyboard shortcuts: one registry for the handler, the palette, the reference dialog and rebinding.
// Rules: shortcuts never steal text editing; users can rebind; conflicts are detected before they are saved.

export type ShortcutId =
  | "palette" | "new-project" | "open-project" | "save" | "undo" | "redo"
  | "step-1" | "step-2" | "step-3" | "step-4" | "step-5" | "step-6" | "step-prev" | "step-next"
  | "do-next" | "toggle-rail" | "toggle-guide" | "toggle-mode" | "voices" | "preview"
  | "settings" | "help" | "shortcuts" | "tour";

export interface ShortcutDef {
  id: ShortcutId;
  label: string;
  group: "General" | "Workflow" | "Panels" | "Help";
  keys: string[];
  /** Works while typing in a text field (modifier shortcuts that do not clash with editing). */
  inText?: boolean;
  /** True for shortcuts that only make sense inside an open project. */
  project?: boolean;
}

export const DEFAULT_SHORTCUTS: ShortcutDef[] = [
  { id: "palette", label: "Command palette and search", group: "General", keys: ["Ctrl+K"], inText: true },
  { id: "new-project", label: "New project", group: "General", keys: ["Ctrl+N"], inText: true },
  { id: "open-project", label: "Open a project (go to Home)", group: "General", keys: ["Ctrl+O"], inText: true },
  { id: "save", label: "Save now", group: "General", keys: ["Ctrl+S"], inText: true, project: true },
  { id: "undo", label: "Undo", group: "General", keys: ["Ctrl+Z"], project: true },
  { id: "redo", label: "Redo", group: "General", keys: ["Ctrl+Shift+Z", "Ctrl+Y"], project: true },
  { id: "step-1", label: "Go to Script", group: "Workflow", keys: ["Ctrl+1"], inText: true, project: true },
  { id: "step-2", label: "Go to Cast", group: "Workflow", keys: ["Ctrl+2"], inText: true, project: true },
  { id: "step-3", label: "Go to Direct", group: "Workflow", keys: ["Ctrl+3"], inText: true, project: true },
  { id: "step-4", label: "Go to Render", group: "Workflow", keys: ["Ctrl+4"], inText: true, project: true },
  { id: "step-5", label: "Go to Mix", group: "Workflow", keys: ["Ctrl+5"], inText: true, project: true },
  { id: "step-6", label: "Go to Export", group: "Workflow", keys: ["Ctrl+6"], inText: true, project: true },
  { id: "step-prev", label: "Previous step", group: "Workflow", keys: ["Alt+Left"], inText: true, project: true },
  { id: "step-next", label: "Next step", group: "Workflow", keys: ["Alt+Right"], inText: true, project: true },
  { id: "do-next", label: "Do the recommended next action", group: "Workflow", keys: ["Ctrl+Enter"], inText: true, project: true },
  { id: "preview", label: "Preview: play or stop what you are looking at", group: "Workflow", keys: ["Space"], project: true },
  { id: "toggle-rail", label: "Show or hide the sidebar", group: "Panels", keys: ["Ctrl+B"], inText: true },
  { id: "toggle-guide", label: "Show or hide the guide panel", group: "Panels", keys: ["Ctrl+J"], inText: true, project: true },
  { id: "toggle-mode", label: "Switch Simple and Advanced", group: "Panels", keys: ["Ctrl+Shift+M"], inText: true },
  { id: "voices", label: "Open the voice library", group: "Panels", keys: ["Ctrl+Shift+V"], inText: true },
  { id: "settings", label: "Settings", group: "Panels", keys: ["Ctrl+,"], inText: true },
  { id: "help", label: "Help and user guide", group: "Help", keys: ["F1"], inText: true },
  { id: "shortcuts", label: "Keyboard shortcut reference", group: "Help", keys: ["Ctrl+/"], inText: true },
  { id: "tour", label: "Take the guided tour", group: "Help", keys: ["Ctrl+Shift+H"], inText: true },
];

const KEY_NAMES: Record<string, string> = {
  " ": "Space", ArrowLeft: "Left", ArrowRight: "Right", ArrowUp: "Up", ArrowDown: "Down", Escape: "Esc",
};
const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta", "AltGraph"]);

/** `Ctrl+Shift+K` style name for a keyboard event, or null for a bare modifier press. */
export function comboOf(e: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null;
  let key = KEY_NAMES[e.key] ?? e.key;
  if (key.length === 1) key = key.toUpperCase();
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  // Shift is part of the character for punctuation (for example "?"), so only keep it for letters, digits and named keys.
  if (e.shiftKey && (/^[A-Z0-9]$/.test(key) || key.length > 1)) parts.push("Shift");
  parts.push(key);
  return parts.join("+");
}

/** Keys that must keep their native meaning. These cannot be bound. */
const RESERVED = new Set(["Ctrl+C", "Ctrl+V", "Ctrl+X", "Ctrl+A", "Ctrl+F", "Ctrl+W", "Ctrl+Q", "Ctrl+R", "Ctrl+P", "Ctrl+Shift+I", "Tab", "Shift+Tab", "Esc", "Enter", "Backspace", "Delete", "Alt+F4", "F5", "F11", "F12"]);
export function isReserved(combo: string): boolean {
  return RESERVED.has(combo);
}

export type Bindings = Record<ShortcutId, string[]>;

export function resolveBindings(overrides: Partial<Record<ShortcutId, string[]>> = {}): Bindings {
  const out = {} as Bindings;
  for (const d of DEFAULT_SHORTCUTS) out[d.id] = overrides[d.id] && overrides[d.id]!.length ? overrides[d.id]! : d.keys;
  return out;
}

export function findConflicts(b: Bindings): { combo: string; ids: ShortcutId[] }[] {
  const seen = new Map<string, ShortcutId[]>();
  for (const [id, combos] of Object.entries(b) as [ShortcutId, string[]][]) {
    for (const c of combos) seen.set(c, [...(seen.get(c) ?? []), id]);
  }
  return [...seen].filter(([, ids]) => ids.length > 1).map(([combo, ids]) => ({ combo, ids }));
}

/** Who already owns a combo (excluding one shortcut), so rebinding can warn before it saves. */
export function ownerOf(b: Bindings, combo: string, except?: ShortcutId): ShortcutId | null {
  for (const [id, combos] of Object.entries(b) as [ShortcutId, string[]][]) {
    if (id !== except && combos.includes(combo)) return id;
  }
  return null;
}

export function isTextTarget(t: EventTarget | null): boolean {
  const el = t as HTMLElement | null;
  if (!el || !el.tagName) return false;
  const tag = el.tagName.toLowerCase();
  if (tag === "textarea" || tag === "select") return true;
  if (tag === "input") {
    const type = (el as HTMLInputElement).type;
    return !["checkbox", "radio", "button", "range", "submit"].includes(type);
  }
  return el.isContentEditable === true;
}

/**
 * Should this key press run the shortcut? Native editing always wins: undo and redo stay with a text field's own
 * history, and plain keys such as Space are ignored while typing.
 */
export function shouldHandle(def: ShortcutDef, combo: string, inTextField: boolean, dialogOpen: boolean): boolean {
  if (dialogOpen && !["palette", "shortcuts", "help"].includes(def.id)) return false;
  if (!inTextField) return true;
  if (def.id === "undo" || def.id === "redo") return false;
  if (!def.inText) return false;
  return /^(Ctrl|Alt)\+|^F\d+$/.test(combo);
}

export function displayKeys(combo: string): string[] {
  return combo.split("+");
}

export function findShortcut(b: Bindings, combo: string): ShortcutDef | null {
  for (const d of DEFAULT_SHORTCUTS) if (b[d.id].includes(combo)) return d;
  return null;
}
