// WCAG 2.2 AA check of the real design tokens, for both themes. Reads src/styles/tokens.css so the test
// can never drift from what ships.
import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";

const raw = readFileSync(new URL("./styles/tokens.css", import.meta.url), "utf8");

const css = raw.replace(/\/\*[\s\S]*?\*\//g, "");

/** Declarations of every rule whose selector matches (whitespace ignored). Works on pretty or minified CSS. */
function block(selector: string): Record<string, string> {
  const want = selector.replace(/\s+/g, "");
  const out: Record<string, string> = {};
  let found = false;
  for (const rule of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    if (rule[1]!.replace(/\s+/g, "").endsWith(want) && rule[1]!.replace(/\s+/g, "") === want) {
      found = true;
      for (const x of rule[2]!.matchAll(/--([a-z0-9-]+)\s*:\s*([^;]+);?/g)) out[x[1]!] = x[2]!.trim();
    }
  }
  if (!found) throw new Error(`block not found: ${selector}`);
  return out;
}

const LIGHT = ':root[data-theme="light"]';
const dark = block(':root,:root[data-theme="dark"]');
const light = { ...dark, ...block(LIGHT) };

type RGBA = [number, number, number, number];
function parse(c: string): RGBA {
  const hex = c.match(/^#([0-9a-f]{6})$/i);
  if (hex) { const n = parseInt(hex[1]!, 16); return [(n >> 16) & 255, (n >> 8) & 255, n & 255, 1]; }
  const rgba = c.match(/^rgba?\(([^)]+)\)$/);
  if (rgba) { const p = rgba[1]!.split(",").map((x) => parseFloat(x)); return [p[0]!, p[1]!, p[2]!, p[3] ?? 1]; }
  throw new Error(`cannot parse colour ${c}`);
}
function over(fg: RGBA, bg: RGBA): RGBA {
  const a = fg[3];
  return [fg[0] * a + bg[0] * (1 - a), fg[1] * a + bg[1] * (1 - a), fg[2] * a + bg[2] * (1 - a), 1];
}
function lum([r, g, b]: RGBA): number {
  const f = (v: number) => { const s = v / 255; return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4; };
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
}
function ratio(a: RGBA, b: RGBA): number {
  const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}
const col = (t: Record<string, string>, k: string, behind?: string): RGBA => {
  const c = parse(t[k] ?? (() => { throw new Error(`missing token ${k}`); })());
  return behind ? over(c, parse(t[behind]!)) : c;
};

const SURFACES = ["bg", "surface-1", "surface-2", "surface-3"];

for (const [name, t] of [["dark", dark], ["light", light]] as const) {
  describe(`${name} theme meets WCAG 2.2 AA`, () => {
    it("body and secondary text (4.5:1) on every surface", () => {
      for (const s of SURFACES) for (const f of ["text", "text-2", "text-3"]) {
        expect(ratio(col(t, f), col(t, s)), `${f} on ${s}`).toBeGreaterThanOrEqual(4.5);
      }
      expect(ratio(col(t, "text"), col(t, "surface-4"))).toBeGreaterThanOrEqual(4.5);
      expect(ratio(col(t, "text-2"), col(t, "surface-4"))).toBeGreaterThanOrEqual(4.5);
    });
    it("accent text, links and status text (4.5:1) on surfaces", () => {
      for (const s of ["surface-1", "surface-2", "surface-3"]) for (const f of ["accent-text", "teal-text", "ok", "warn", "err"]) {
        expect(ratio(col(t, f), col(t, s)), `${f} on ${s}`).toBeGreaterThanOrEqual(4.5);
      }
    });
    it("status text stays readable on its own tinted background", () => {
      for (const [f, w] of [["ok", "ok-wash"], ["warn", "warn-wash"], ["err", "err-wash"], ["teal-text", "teal-wash"], ["accent-text", "gold-wash"]] as const) {
        const bg = over(parse(t[w]!), col(t, "surface-2"));
        expect(ratio(col(t, f), bg), `${f} on ${w}`).toBeGreaterThanOrEqual(4.5);
      }
    });
    it("the primary button label is readable on gold, including hover", () => {
      expect(ratio(col(t, "on-gold"), col(t, "gold"))).toBeGreaterThanOrEqual(4.5);
      expect(ratio(col(t, "on-gold"), col(t, "gold-hi"))).toBeGreaterThanOrEqual(4.5);
    });
    it("form field borders and the focus ring are visible (3:1)", () => {
      for (const s of ["surface-1", "surface-2", "bg"]) {
        expect(ratio(col(t, "control-line"), col(t, s)), `field border on ${s}`).toBeGreaterThanOrEqual(3);
        expect(ratio(col(t, "focus"), col(t, s)), `focus on ${s}`).toBeGreaterThanOrEqual(3);
      }
    });
    it("the gold action colour is distinguishable from the surface it sits on (3:1)", () => {
      for (const s of ["surface-1", "surface-2"]) expect(ratio(col(t, "gold"), col(t, s))).toBeGreaterThanOrEqual(3);
    });
  });
}

describe("token hygiene", () => {
  it("defines the same set of colour tokens in both themes", () => {
    const need = ["bg", "surface-1", "surface-2", "surface-3", "surface-4", "text", "text-2", "text-3", "gold", "on-gold", "accent-text", "teal", "teal-text", "ok", "warn", "err", "focus", "control-line"];
    for (const k of need) {
      expect(dark[k], `dark ${k}`).toBeTruthy();
      expect(block(LIGHT)[k], `light ${k}`).toBeTruthy();
    }
  });
  it("keeps controls at or above the WCAG 2.2 minimum target size", () => {
    expect(css).toMatch(/--ctl:\s*36px/);
    expect(css).toMatch(/--ctl-sm:\s*28px/);
  });
  it("respects reduced motion", () => {
    expect(css).toMatch(/prefers-reduced-motion:\s*reduce/);
  });
});
