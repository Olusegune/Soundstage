import { describe, expect, it } from "vitest";
import { DEFAULT_SHORTCUTS, comboOf, findConflicts, findShortcut, isReserved, ownerOf, resolveBindings, shouldHandle } from "./shortcuts";

const ev = (key: string, mods: Partial<{ ctrlKey: boolean; shiftKey: boolean; altKey: boolean; metaKey: boolean }> = {}) =>
  ({ key, ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, ...mods });

describe("combo naming", () => {
  it("names modifier combos consistently", () => {
    expect(comboOf(ev("k", { ctrlKey: true }))).toBe("Ctrl+K");
    expect(comboOf(ev("Z", { ctrlKey: true, shiftKey: true }))).toBe("Ctrl+Shift+Z");
    expect(comboOf(ev("ArrowLeft", { altKey: true }))).toBe("Alt+Left");
    expect(comboOf(ev(" "))).toBe("Space");
    expect(comboOf(ev("F1"))).toBe("F1");
    expect(comboOf(ev("1", { ctrlKey: true }))).toBe("Ctrl+1");
    expect(comboOf(ev(",", { ctrlKey: true }))).toBe("Ctrl+,");
    expect(comboOf(ev("/", { ctrlKey: true }))).toBe("Ctrl+/");
  });
  it("ignores a bare modifier press", () => {
    expect(comboOf(ev("Control", { ctrlKey: true }))).toBeNull();
    expect(comboOf(ev("Shift", { shiftKey: true }))).toBeNull();
  });
  it("treats the Windows key as Ctrl only for matching, never as its own shortcut", () => {
    expect(comboOf(ev("k", { metaKey: true }))).toBe("Ctrl+K");
  });
});

describe("defaults", () => {
  it("have no conflicts and bind nothing reserved", () => {
    expect(findConflicts(resolveBindings())).toEqual([]);
    for (const d of DEFAULT_SHORTCUTS) for (const k of d.keys) expect(isReserved(k), `${d.id} uses reserved ${k}`).toBe(false);
  });
  it("cover every action the brief requires", () => {
    const ids = DEFAULT_SHORTCUTS.map((d) => d.id);
    for (const need of ["new-project", "open-project", "save", "undo", "redo", "palette", "step-1", "step-6", "step-prev", "step-next", "toggle-rail", "toggle-guide", "preview", "help", "shortcuts"]) {
      expect(ids).toContain(need);
    }
  });
});

describe("rebinding", () => {
  it("applies overrides and detects a clash before saving", () => {
    const b = resolveBindings({ save: ["Ctrl+Shift+S"], voices: ["Ctrl+K"] });
    expect(b.save).toEqual(["Ctrl+Shift+S"]);
    expect(findConflicts(b)).toEqual([{ combo: "Ctrl+K", ids: ["palette", "voices"] }]);
    expect(ownerOf(resolveBindings(), "Ctrl+K")).toBe("palette");
    expect(ownerOf(resolveBindings(), "Ctrl+K", "palette")).toBeNull();
  });
  it("falls back to the default for an empty override", () => {
    expect(resolveBindings({ save: [] }).save).toEqual(["Ctrl+S"]);
  });
  it("refuses keys that keep a native meaning", () => {
    for (const k of ["Ctrl+C", "Ctrl+V", "Ctrl+A", "Tab", "Esc", "Alt+F4"]) expect(isReserved(k)).toBe(true);
  });
});

describe("never steals text editing", () => {
  const def = (id: string) => DEFAULT_SHORTCUTS.find((d) => d.id === id)!;
  it("leaves undo and redo to a text field's own history", () => {
    expect(shouldHandle(def("undo"), "Ctrl+Z", true, false)).toBe(false);
    expect(shouldHandle(def("redo"), "Ctrl+Y", true, false)).toBe(false);
    expect(shouldHandle(def("undo"), "Ctrl+Z", false, false)).toBe(true);
  });
  it("ignores plain keys such as Space while typing", () => {
    expect(shouldHandle(def("preview"), "Space", true, false)).toBe(false);
    expect(shouldHandle(def("preview"), "Space", false, false)).toBe(true);
  });
  it("allows modifier shortcuts while typing", () => {
    expect(shouldHandle(def("palette"), "Ctrl+K", true, false)).toBe(true);
    expect(shouldHandle(def("step-3"), "Ctrl+3", true, false)).toBe(true);
    expect(shouldHandle(def("help"), "F1", true, false)).toBe(true);
  });
  it("lets an open dialog keep its keys, except the ones that open help", () => {
    expect(shouldHandle(def("step-3"), "Ctrl+3", false, true)).toBe(false);
    expect(shouldHandle(def("shortcuts"), "Ctrl+/", false, true)).toBe(true);
  });
  it("finds the shortcut for a combo", () => {
    expect(findShortcut(resolveBindings(), "Ctrl+Y")?.id).toBe("redo");
    expect(findShortcut(resolveBindings(), "Ctrl+Q")).toBeNull();
  });
});
