// Writes docs/USER_GUIDE.md from the same content the in-app Help centre shows, so the two can never disagree.
// Usage (from app/): npm run docs
import { writeFileSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";

// Node 22.18+ and 24 import TypeScript directly (type annotations are stripped), so there is no build step here.
const { ARTICLES, GLOSSARY } = await import("../src/help/content.ts");

const md = (b) => {
  switch (b.t) {
    case "p": return b.text + "\n";
    case "h": return `\n### ${b.text}\n`;
    case "ul": return b.items.map((x) => `- ${x}`).join("\n") + "\n";
    case "steps": return b.items.map((x, i) => `${i + 1}. ${x}`).join("\n") + "\n";
    case "tip": return `> **Tip.** ${b.text}\n`;
    case "paid": return `> **Uses your provider account.** ${b.text}\n`;
    case "keys": return "| Keys | Does |\n|---|---|\n" + b.rows.map(([k, d]) => `| ${k} | ${d} |`).join("\n") + "\n";
    default: return "";
  }
};

const groups = ["Start here", "The steps", "Understanding", "Fix a problem"];
let doc = "# Soundstage user guide\n\n_Generated from the in-app Help centre (`app/src/help/content.ts`). Edit that file, then run `npm run docs` in `app/`._\n\n";
doc += "## Contents\n\n" + groups.map((g) => `**${g}**\n` + ARTICLES.filter((a) => a.group === g).map((a) => `- [${a.title}](#${a.title.toLowerCase().replace(/[^a-z0-9 -]/g, "").replace(/ /g, "-")})`).join("\n")).join("\n\n") + "\n\n- [Glossary](#glossary)\n\n";
for (const g of groups) {
  doc += `---\n\n# ${g}\n\n`;
  for (const a of ARTICLES.filter((x) => x.group === g)) doc += `## ${a.title}\n\n_${a.summary}_\n\n${a.body.map(md).join("\n")}\n`;
}
doc += "---\n\n## Glossary\n\n" + GLOSSARY.slice().sort((a, b) => a.term.localeCompare(b.term)).map((e) => `**${e.term}.** ${e.meaning}`).join("\n\n") + "\n";
const target = fileURLToPath(new URL("../../docs/USER_GUIDE.md", import.meta.url));
writeFileSync(target, doc);
console.log(`Wrote ${pathToFileURL(target).pathname} (${ARTICLES.length} articles, ${GLOSSARY.length} glossary terms)`);
