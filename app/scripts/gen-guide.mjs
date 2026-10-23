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
const shots = [
  ["dark-welcome.png", "The welcome screen on first launch."],
  ["dark-1280x720-home.png", "Projects: start a new scene, or pick up where you left off."],
  ["dark-1280x720-script.png", "Step 1, Script: write or import dialogue with speaker names."],
  ["dark-1280x720-cast.png", "Step 2, Cast: choose a voice for each speaker."],
  ["dark-1280x720-direct.png", "Step 3, Direct: set the feeling and pauses for each line."],
  ["dark-1280x720-render.png", "Step 4, Render: make the audio. Cloud voices ask first."],
  ["dark-1280x720-mix.png", "Step 5, Mix: dialogue, effects and music on one timeline."],
  ["dark-1280x720-mix-create.png", "Create music or an effect with AI, with the prompt helper open."],
  ["dark-1280x720-export.png", "Step 6, Export: licence checks, then the files."],
  ["dark-1280x720-export-formats.png", "Export formats, subtitles and the optional FFmpeg setting."],
  ["dark-1280x720-export-blocked.png", "A commercial export blocked until licences are recorded."],
  ["dark-1280x720-library.png", "The voice library, with languages shown by flag."],
  ["dark-1280x720-voice-design.png", "Design a voice from a description, with the prompt helper."],
  ["dark-1280x720-voice-engines.png", "Voice engines: local engines, Piper, music and effects, ComfyUI."],
  ["dark-1280x720-settings.png", "Settings, including the optional AI prompt helper."],
  ["dark-1280x720-palette.png", "The command palette (Ctrl+K)."],
  ["dark-1280x720-shortcuts.png", "The shortcut reference, with rebinding."],
  ["dark-1280x720-help.png", "The built-in help centre."],
  ["dark-1280x720-tour.png", "The guided tour."],
  ["light-1280x720-mix.png", "The light theme."],
];
doc += "---\n\n## Screen gallery\n\n_Captured from the real app by `tools/e2e/run.mjs`._\n\n" + shots.map(([f, c]) => `![${c}](design/screens/${f})\n*${c}*\n`).join("\n") + "\n";
doc += "---\n\n## Glossary\n\n" + GLOSSARY.slice().sort((a, b) => a.term.localeCompare(b.term)).map((e) => `**${e.term}.** ${e.meaning}`).join("\n\n") + "\n";
const target = fileURLToPath(new URL("../../docs/USER_GUIDE.md", import.meta.url));
writeFileSync(target, doc);
console.log(`Wrote ${pathToFileURL(target).pathname} (${ARTICLES.length} articles, ${GLOSSARY.length} glossary terms)`);
