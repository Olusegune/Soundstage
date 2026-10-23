// Flags for voice locales, as real SVGs (MIT-licensed `flag-icons`). Emoji flags are not used: Windows draws them as
// letters, and the interface uses one icon family.
//
// A flag appears only when the locale names a COUNTRY, such as en-GB or yo-NG. A bare language (`en`, `yo`) has no
// country, so it gets a language chip instead. Guessing a flag for a language is wrong and can offend.

const urls = import.meta.glob("/node_modules/flag-icons/flags/4x3/*.svg", { query: "?url", import: "default", eager: true }) as Record<string, string>;
const byCode = new Map(Object.entries(urls).map(([path, url]) => [path.split("/").pop()!.replace(".svg", ""), url]));

/** The lower-case two-letter region of a BCP 47 locale, if it has one and we have its flag. */
export function regionOf(locale: string): string | null {
  const parts = locale.replace(/_/g, "-").split("-").slice(1);
  for (const p of parts) {
    if (/^[A-Za-z]{2}$/.test(p) && byCode.has(p.toLowerCase())) return p.toLowerCase();
  }
  return null;
}

export function flagUrl(locale: string): string | null {
  const r = regionOf(locale);
  return r ? (byCode.get(r) ?? null) : null;
}

/** A readable name such as "English (United Kingdom)". Falls back to the code itself. */
export function languageName(locale: string): string {
  if (!locale || locale === "zxx") return "No language (tones)";
  try {
    const n = new Intl.DisplayNames(["en"], { type: "language", languageDisplay: "standard" }).of(locale.replace(/_/g, "-"));
    return n && n !== locale ? n : locale;
  } catch {
    return locale;
  }
}

export function languageCode(locale: string): string {
  return (locale.split(/[-_]/)[0] ?? locale).toUpperCase().slice(0, 3);
}

/** Flag when the locale has a country, else a short language chip. Always decorative: the name is next to it. */
export function Flag({ locale, name = false, size = 20 }: { locale: string; name?: boolean; size?: number }) {
  const url = flagUrl(locale);
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
      {url ? (
        <img src={url} alt="" aria-hidden="true" width={size} height={Math.round(size * 0.75)} style={{ borderRadius: 2, boxShadow: "0 0 0 1px var(--line-strong)", display: "block" }} />
      ) : (
        <span className="chip" aria-hidden="true" style={{ minHeight: 18, padding: "0 5px", fontSize: 10, fontWeight: 600, letterSpacing: "0.04em" }}>{languageCode(locale)}</span>
      )}
      {name && <span>{languageName(locale)}</span>}
    </span>
  );
}
