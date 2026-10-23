import { ARTICLES, GLOSSARY, type Article, type Block, type GlossaryEntry } from "./content";

/** Subsequence-friendly score for the command palette and help search. 0 means no match. */
export function score(query: string, text: string): number {
  const q = query.trim().toLowerCase();
  if (!q) return 1;
  const t = text.toLowerCase();
  if (t === q) return 100;
  if (t.startsWith(q)) return 80;
  const i = t.indexOf(q);
  if (i >= 0) return 60 - Math.min(i, 30) / 2;
  // every word of the query must appear somewhere
  const words = q.split(/\s+/).filter(Boolean);
  if (words.length > 1 && words.every((w) => t.includes(w))) return 40;
  // loose subsequence match for typos like "rndr"
  let k = 0;
  for (const ch of t) if (ch === q[k]) k++;
  return k === q.length && q.length >= 3 ? 15 : 0;
}

export function blockText(b: Block): string {
  switch (b.t) {
    case "p": case "h": case "tip": case "paid": return b.text;
    case "ul": case "steps": return b.items.join(" ");
    case "keys": return b.rows.map((r) => r.join(" ")).join(" ");
  }
}

export function articleText(a: Article): string {
  return a.body.map(blockText).join(" ");
}

export interface HelpHit { kind: "article" | "glossary"; id: string; title: string; snippet: string; score: number }

export function searchHelp(query: string, articles: Article[] = ARTICLES, glossary: GlossaryEntry[] = GLOSSARY): HelpHit[] {
  const hits: HelpHit[] = [];
  for (const a of articles) {
    const s = Math.max(score(query, a.title) * 3, score(query, a.tags.join(" ")) * 2, score(query, a.summary) * 1.5, score(query, articleText(a)));
    if (s > 0) hits.push({ kind: "article", id: a.id, title: a.title, snippet: a.summary, score: s });
  }
  for (const g of glossary) {
    const s = Math.max(score(query, g.term) * 3, score(query, (g.aka ?? []).join(" ")) * 2, score(query, g.meaning));
    if (s > 0) hits.push({ kind: "glossary", id: g.term, title: g.term, snippet: g.meaning, score: s });
  }
  return hits.sort((a, b) => b.score - a.score).slice(0, 30);
}

export function articleById(id: string): Article | undefined {
  return ARTICLES.find((a) => a.id === id);
}
