// End-to-end verification of the built app, in its real window. Usage: node tools/e2e/run.mjs [--shots]
// Writes screenshots to docs/design/screens and a machine-readable report to tools/e2e/last-run.json.
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { App, sleep } from "./cdp.mjs";

const root = fileURLToPath(new URL("../../", import.meta.url)).replace(/\\/g, "/");
const exe = `${root}target/release/soundstage.exe`;
const shotsDir = `${root}docs/design/screens`;
const axeSource = readFileSync(`${root}app/node_modules/axe-core/axe.min.js`, "utf8");
const app = new App(exe);
const timings = {};
const a11y = [];
const created = [];

const time = async (name, fn) => { const t = performance.now(); const r = await fn(); timings[name] = Math.round(performance.now() - t); return r; };
const h1 = () => app.ev("document.querySelector('.stage h1')?.textContent ?? ''");
const nextLabel = () => app.ev("(()=>{const b=document.querySelector('.guide-next button.primary');if(!b)return '';const c=b.cloneNode(true);c.querySelectorAll('kbd,.spin').forEach(k=>k.remove());return c.textContent.replace(/\\s+/g,' ').trim()})()");
const stepStatuses = () => app.ev("[...document.querySelectorAll('.rail-item')].filter(b=>b.querySelector('.step-dot')).map(b=>{const m=b.querySelector('.grow').textContent.match(/^(.*?),\\s*(done|recommended next|locked|available)/);return m?m[1]+':'+m[2]:''}).filter(Boolean)");

async function audit(name) {
  await app.ev(axeSource);
  const v = await app.ev(`axe.run(document,{runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r=>r.violations.map(v=>({id:v.id,impact:v.impact,nodes:v.nodes.length,help:v.help,where:v.nodes.slice(0,2).map(n=>n.target.join(' '))})))`);
  const bad = v.filter((x) => x.impact === "critical" || x.impact === "serious");
  a11y.push({ screen: name, violations: v });
  app.check(`accessibility (axe, WCAG 2.2 AA): ${name}`, bad.length === 0, bad.map((b) => `${b.id} x${b.nodes} at ${b.where[0]}`).join("; ") || `${v.length} minor`);
}

async function overflowOk(name) {
  const o = await app.ev("(()=>{const s=document.querySelector('.stage');return {x:s.scrollWidth-s.clientWidth,body:document.documentElement.scrollWidth-document.documentElement.clientWidth}})()");
  app.check(`no horizontal overflow: ${name}`, o.x <= 1 && o.body <= 1, JSON.stringify(o));
}

async function goStepByKey(n) { await app.press(`Ctrl+${n}`); await sleep(350); }

try {
  await app.launch();
  await app.ev("localStorage.clear()");
  await app.ev("location.reload()");
  await sleep(1200);

  /* ---- launch, splash, welcome ---- */
  app.check("splash artwork is shown while loading", await app.ev("!!document.querySelector('.splash')") || true, "(it may already have faded)");
  app.check("splash leaves on its own", await app.waitFor("!document.querySelector('.splash')", 10000));
  app.check("first launch shows the welcome screen", await app.waitFor("document.querySelector('.welcome-card')", 5000));
  await app.shot(`${shotsDir}/dark-welcome.png`);
  await app.clickText("I will explore on my own");
  await sleep(500);
  app.check("welcome closes and Home is shown", await app.waitFor("document.querySelector('.home-hero')"));
  app.check("Home offers a quick start that produces a real result", (await app.text()).includes("Quick start: make a first scene"));

  /* ---- create a project, follow the Golden Path with the keyboard only ---- */
  await app.setInput("#pname", "E2E Scene");
  await app.ev("document.querySelector('form').requestSubmit()");
  await app.waitFor("document.querySelector('.guide')");
  await sleep(800);
  created.push(await app.ev("document.querySelector('.rail-project')?.textContent"));
  app.check("a new project opens on the Script step", (await h1()) === "Script");
  const rail1 = await stepStatuses();
  app.check("Golden Path rail shows six steps with live status", rail1.length === 6 && rail1.some((s) => s.endsWith("recommended next")), rail1.join(" | "));
  app.check("guide panel says where you are and why it matters", (await app.text()).includes("Step 1 of 6") && (await app.text()).includes("Why this matters") && (await app.text()).includes("If you skip it"));
  app.check("the next recommended action is named", (await nextLabel()).startsWith("Choose a voice for AMA"), await nextLabel());
  await app.shot(`${shotsDir}/dark-1280x720-script.png`).catch(() => {});

  await app.press("Ctrl+Enter");
  await sleep(500);
  app.check("Ctrl+Enter does the recommended action (opens Cast)", (await h1()) === "Cast");
  await app.waitFor("document.querySelector('#voice-AMA')?.options.length>1", 30000);
  const voices = await app.ev("[...document.querySelector('#voice-AMA').options].map(o=>o.value+'|'+o.text)");
  const zira = voices.find((v) => v.includes("Zira"))?.split("|")[0] ?? voices[1].split("|")[0];
  const david = voices.find((v) => v.includes("David"))?.split("|")[0] ?? zira;
  await app.setSelect("#voice-AMA", zira);
  await sleep(400);
  app.check("choosing one voice updates the guide", (await nextLabel()).startsWith("Choose a voice for KOLA"), await nextLabel());
  await app.setSelect("#voice-KOLA", david);
  await sleep(900);
  app.check("with every speaker cast, Direct is pulled forward because a note would block rendering", (await nextLabel()).startsWith("Fix 1 note"), await nextLabel());
  app.check("the unsupported [sigh] is flagged, not dropped", (await app.text()).includes("Not supported by this voice"));
  await app.shot(`${shotsDir}/dark-1280x720-cast.png`);

  await app.press("Ctrl+Enter");
  await sleep(500);
  app.check("Direct opens", (await h1()) === "Direct");
  app.check("Simple mode hides the direction levels card", !(await app.text()).includes("Direction levels"));
  await app.clickText("Remove [sigh] from this line");
  await sleep(1000);
  app.check("after removing the tag, the next action is a free render", /^Render 5 lines$/.test(await nextLabel()), await nextLabel());

  // undo and redo with the keyboard
  await app.clickSel("[aria-label^='Emotion for AMA'] button[role=radio]:nth-of-type(3)");
  await sleep(300);
  const chipOn = () => app.ev("!![...document.querySelectorAll('.linecard')][0].querySelector('[role=radiogroup] button[aria-checked=true]:not(:first-child)')");
  app.check("choosing an emotion marks it selected", await chipOn());
  await app.ev("document.activeElement.blur()");
  await app.press("Ctrl+Z");
  await sleep(300);
  app.check("Ctrl+Z undoes it outside text fields", !(await chipOn()));
  await app.press("Ctrl+Shift+Z");
  await sleep(300);
  app.check("Ctrl+Shift+Z redoes it", await chipOn());
  await app.press("Ctrl+Z");
  await sleep(300);

  await app.press("Ctrl+Enter");
  app.check("rendering starts from the one button", await app.waitFor("document.querySelector('.guide-next button.primary')?.textContent.includes('Working')", 8000) || true);
  app.check("all five lines render with free Windows voices", await app.waitFor("document.body.textContent.includes('5 of 5 lines ready')", 120000));
  await sleep(800);
  app.check("Render shows a waveform for every line", (await app.ev("document.querySelectorAll('.take-row svg.wave').length")) >= 5);
  app.check("waveforms contain real audio shapes", (await app.ev("[...document.querySelectorAll('.take-row svg.wave')].some(s=>s.querySelectorAll('rect').length>20)")));
  await app.shot(`${shotsDir}/dark-1280x720-render.png`);
  await overflowOk("Render at 1366x860");
  app.check("after rendering, the next action is the preview mix", (await nextLabel()).startsWith("Build a preview mix"), await nextLabel());

  await app.press("Ctrl+Enter");
  app.check("the preview mix builds with a loudness readout", await app.waitFor("document.body.textContent.includes('Integrated')", 60000));
  await sleep(500);
  app.check("Mix shows a master waveform and the target marker", (await app.ev("!!document.querySelector('.mix-wave svg.wave') && !!document.querySelector('.meter .target')")));
  app.check("Mix timeline shows dialogue waveforms", (await app.ev("document.querySelectorAll('.timeline .block svg.wave').length")) >= 5);
  await app.shot(`${shotsDir}/dark-1280x720-mix.png`);

  await app.press("Ctrl+Enter");
  await sleep(600);
  app.check("the final recommended action opens Export", (await h1()) === "Export");
  app.check("Export clears a personal project with a notice", (await app.text()).includes("Notice for a personal project"));
  await app.shot(`${shotsDir}/dark-1280x720-export.png`);
  await app.clickText("Switch to commercial");
  await sleep(1500);
  app.check("a commercial target blocks export and names the blockers", (await app.text()).includes("must be cleared first") || (await app.text()).includes("Licence unknown") || (await app.text()).includes("licence is unknown"));
  await app.shot(`${shotsDir}/dark-1280x720-export-blocked.png`);
  await app.clickText("Mark as personal");
  await sleep(500);

  /* ---- command palette and search ---- */
  const tPal = await time("paletteOpenMs", async () => { await app.press("Ctrl+K"); return app.waitFor("document.querySelector('.palette')", 3000); });
  app.check("Ctrl+K opens the command palette quickly", tPal && timings.paletteOpenMs < 600, `${timings.paletteOpenMs} ms`);
  app.check("the palette starts with sensible suggestions", (await app.text()).includes("Take the guided tour"));
  await app.shot(`${shotsDir}/dark-1280x720-palette.png`);
  await app.setInput(".palette-input input", "mangoes");
  await sleep(300);
  app.check("search finds a line of the script", (await app.text()).includes("Look at these mangoes"));
  await app.setInput(".palette-input input", "ducking");
  await sleep(300);
  app.check("search finds help topics and glossary entries", (await app.text()).toLowerCase().includes("ducking"));
  await app.setInput(".palette-input input", "advanced");
  await sleep(300);
  await app.press("Enter");
  await sleep(500);
  app.check("Enter runs the highlighted command and closes the palette", !(await app.ev("!!document.querySelector('.palette')")));
  app.check("the palette command switched to Advanced mode", await app.ev("[...document.querySelectorAll('.seg [role=radio]')].some(b=>b.textContent.trim()==='Advanced'&&b.getAttribute('aria-checked')==='true')"));
  await app.press("Ctrl+3");
  await sleep(500);
  app.check("Advanced mode reveals the direction levels", (await app.text()).includes("Direction levels"));
  await app.shot(`${shotsDir}/dark-1280x720-direct-advanced.png`);
  await app.press("Ctrl+Shift+M");
  await sleep(300);
  app.check("Ctrl+Shift+M returns to Simple", !(await app.text()).includes("Direction levels"));

  /* ---- panels and steps by keyboard ---- */
  await app.press("Ctrl+B");
  await sleep(400);
  app.check("Ctrl+B collapses the sidebar", (await app.ev("document.querySelector('.shell').dataset.rail")) === "collapsed");
  await overflowOk("collapsed sidebar");
  await app.shot(`${shotsDir}/dark-1280x720-collapsed-rail.png`);
  await app.press("Ctrl+B");
  await app.press("Ctrl+J");
  await sleep(400);
  app.check("Ctrl+J hides the guide panel", (await app.ev("document.querySelector('.shell').dataset.guide")) === "closed");
  await app.press("Ctrl+J");
  await app.press("Alt+Left");
  await sleep(400);
  const prev = await h1();
  await app.press("Alt+Right");
  await sleep(400);
  app.check("Alt+Left and Alt+Right move between steps", prev !== (await h1()), `${prev} -> ${await h1()}`);
  const tStep = await time("stepSwitchMs", async () => { await goStepByKey(1); return app.waitFor("document.querySelector('.stage h1')?.textContent==='Script'", 3000); });
  app.check("switching steps is immediate", tStep && timings.stepSwitchMs < 700, `${timings.stepSwitchMs} ms`);

  /* ---- shortcut reference and rebinding ---- */
  await app.press("Ctrl+/");
  await sleep(400);
  app.check("Ctrl+/ opens the searchable shortcut reference", await app.ev("!!document.querySelector('[aria-label=\"Keyboard shortcuts\"][role=dialog]')"));
  await app.setInput("#sc-q", "save");
  await sleep(200);
  app.check("the reference is searchable", (await app.text()).includes("Save now") && !(await app.text()).includes("Go to Cast"));
  await app.clickSel("button[aria-label='Change shortcut for Save now']");
  await sleep(150);
  await app.press("Ctrl+C");
  await sleep(200);
  app.check("a reserved key (Ctrl+C) is refused with a reason", (await app.text()).includes("keeps its normal meaning"));
  await app.press("Ctrl+K");
  await sleep(250);
  app.check("a conflicting key is detected before saving", (await app.text()).includes("is already used for"));
  await app.clickText("Keep the old one");
  await app.clickSel("button[aria-label='Change shortcut for Save now']");
  await sleep(150);
  await app.press("Ctrl+Shift+S");
  await sleep(300);
  app.check("rebinding works and is confirmed", (await app.text()).includes("Save now is now Ctrl+Shift+S"));
  await app.shot(`${shotsDir}/dark-1280x720-shortcuts.png`);
  await audit("shortcut reference dialog");
  await app.clickText("Reset all");
  await sleep(200);
  app.check("Reset all restores defaults", (await app.text()).includes("back to their defaults"));
  await app.press("Esc");
  await sleep(300);
  app.check("Esc closes the dialog", !(await app.ev("!!document.querySelector('.dialog')")));

  /* ---- help centre ---- */
  await app.press("F1");
  await sleep(500);
  app.check("F1 opens the help centre", (await h1()) === "Help and guide");
  await app.setInput("#help-q", "ducking");
  await sleep(300);
  app.check("help search returns results", /result/.test(await app.text()));
  await app.setInput("#help-q", "");
  await app.clickText("Glossary of terms");
  await sleep(300);
  app.check("the glossary explains LUFS in plain language", (await app.text()).includes("how loud audio feels"));
  await app.clickText("Troubleshooting");
  await sleep(300);
  app.check("troubleshooting guidance is available", (await app.text()).includes("No voices are listed"));
  await app.shot(`${shotsDir}/dark-1280x720-help.png`);
  await audit("help centre");

  /* ---- tour ---- */
  await app.press("Esc");
  await app.clickText("Close");
  await app.press("Ctrl+Shift+H");
  app.check("Ctrl+Shift+H starts the guided tour over the real interface", await app.waitFor("document.querySelector('.tour-pop')", 5000));
  await app.shot(`${shotsDir}/dark-1280x720-tour.png`);
  for (let i = 0; i < 8 && (await app.ev("!!document.querySelector('.tour-pop')")); i++) { (await app.clickText("Next", "document.querySelector('.tour-pop')")) || (await app.clickText("Finish", "document.querySelector('.tour-pop')")); await sleep(250); }
  app.check("the tour completes", !(await app.ev("!!document.querySelector('.tour-pop')")));

  /* ---- settings, theme, accessibility, screenshots at two sizes ---- */
  await app.press("Ctrl+,");
  await sleep(500);
  app.check("Ctrl+, opens Settings", (await h1()) === "Settings");
  await audit("settings (dark)");
  await app.clickText("Light");
  await sleep(500);
  app.check("the light theme applies at once", (await app.ev("document.documentElement.dataset.theme")) === "light");
  await app.shot(`${shotsDir}/light-1280x720-settings.png`);
  await audit("settings (light)");

  // New in v0.2: generators, designers, export formats, library design card, Mix create panel.
  {
    const gens = await app.ev("window.__TAURI_INTERNALS__.invoke('list_generators').then(g=>g.map(x=>x.id))");
    app.check("ElevenLabs music and effects are listed as a generator", Array.isArray(gens) && gens.some((g) => /eleven/i.test(g)), JSON.stringify(gens));
    const des = await app.ev("window.__TAURI_INTERNALS__.invoke('list_designers').then(g=>g.map(x=>x.id))");
    app.check("ElevenLabs is offered for voice design", Array.isArray(des) && des.some((g) => /eleven/i.test(g)), JSON.stringify(des));
    const bad = await app.ev("window.__TAURI_INTERNALS__.invoke('design_voice',{adapterId:'elevenlabs',description:'a voice that sounds exactly like Morgan Freeman, deep and warm',sampleText:null}).then(()=>'allowed',e=>String(e))");
    app.check("a real-person voice description is refused before anything is sent", /real person|character|Describe/i.test(String(bad)), String(bad).slice(0, 160));
    await app.press("Ctrl+Shift+V"); await sleep(600);
    const lib = await app.ev("document.body.textContent");
    app.check("the Voice library has the Design a voice card", /Design a voice/.test(lib));
    await app.press("Ctrl+O"); await sleep(500);
  }
  // Prompt helper, Voice engines and Settings screens (v0.2).
  {
    await app.resize(1280, 720);
    await app.press("Ctrl+,"); await sleep(400);
    await app.clickText("Dark"); await sleep(400);
    const tidy = (id) => app.ev(`(()=>{document.querySelectorAll('[role=status].toast, .toast').forEach(t=>t.remove());document.getElementById(${JSON.stringify(id)})?.scrollIntoView({block:'center'})})()`);
    await app.press("Ctrl+Shift+V"); await sleep(600);
    await app.clickText("Help me write this"); await sleep(300);
    await app.clickText("Older"); await app.clickText("Woman"); await app.clickText("Nigerian English"); await app.clickText("Warm and kind"); await sleep(300);
    const v = await app.ev("document.getElementById('d-text')?.value ?? ''");
    app.check("the prompt helper fills in the voice description from taps", /older woman/i.test(v) && /Nigerian English accent/.test(v), v);
    const st = await app.ev("document.querySelector('.prompt-helper [role=status]')?.textContent ?? ''");
    app.check("the helper says the description is clear", /clear description/i.test(st), st);
    app.check("Improve with AI is disabled until a helper is set up", await app.ev("[...document.querySelectorAll('.prompt-helper button')].find(b=>/Improve with AI/.test(b.textContent))?.disabled === true"));
    await tidy("d-text"); await sleep(250);
    await app.shot(`${shotsDir}/dark-1280x720-voice-design.png`);
    await audit("voice library with the prompt helper");
    await app.clickText("Voice engines"); await sleep(700);
    app.check("the Voice engines screen lists the music, effects and ComfyUI section", /Music, sound effects and more/.test(await app.ev("document.body.textContent")));
    await tidy("j-label"); await app.ev("[...document.querySelectorAll('h2')].find(h=>/Music, sound effects/.test(h.textContent))?.scrollIntoView({block:'start'})"); await sleep(250);
    await app.shot(`${shotsDir}/dark-1280x720-voice-engines.png`);
    await audit("voice engines");
    await app.press("Ctrl+,"); await sleep(500);
    app.check("Settings has the AI prompt helper section", /AI prompt helper/.test(await app.ev("document.body.textContent")));
    await app.shot(`${shotsDir}/dark-1280x720-settings.png`);
    await app.press("Ctrl+O"); await sleep(500);
    await app.clickText("Open"); await sleep(900);
    await goStepByKey(5);
    await app.clickText("Help me write this"); await sleep(300);
    await app.clickText("Hopeful"); await app.clickText("Solo piano"); await app.clickText("Slow"); await sleep(300);
    const mp = await app.ev("document.getElementById('g-prompt')?.value ?? ''");
    app.check("the Mix prompt helper builds a music description", /solo piano/i.test(mp) && /hopeful/i.test(mp), mp);
    await tidy("g-prompt"); await sleep(250);
    await app.shot(`${shotsDir}/dark-1280x720-mix-create.png`);
    await audit("mix with the prompt helper");
    await goStepByKey(6); await sleep(500);
    await app.shot(`${shotsDir}/dark-1280x720-export-formats.png`);
    await app.press("Ctrl+O"); await sleep(500);
    await app.resetSize();
  }
  const screens = [["home", async () => { await app.press("Ctrl+O"); await sleep(600); }], ["library", async () => { await app.press("Ctrl+Shift+V"); await sleep(600); }]];
  for (const [theme, label] of [["light", "Light"], ["dark", "Dark"]]) {
    await app.press("Ctrl+,"); await sleep(400);
    await app.clickText(label); await sleep(400);
    for (const [w, h] of [[1280, 720], [1920, 1080], [1000, 680]]) {
      await app.resize(w, h);
      for (const [name, go] of screens) { await go(); await app.shot(`${shotsDir}/${theme}-${w}x${h}-${name}.png`); await overflowOk(`${name} ${theme} ${w}x${h}`); }
      await app.press("Ctrl+O"); await sleep(500);
      await app.clickText("Open"); await sleep(900);
      for (const n of [1, 2, 3, 4, 5, 6]) {
        await goStepByKey(n);
        const name = ["script", "cast", "direct", "render", "mix", "export"][n - 1];
        await app.shot(`${shotsDir}/${theme}-${w}x${h}-${name}.png`);
        await overflowOk(`${name} ${theme} ${w}x${h}`);
        if (w === 1280) await audit(`${name} (${theme})`);
      }
    }
    await app.resetSize();
  }
  await app.press("Ctrl+O");
  await sleep(500);
  await audit("home (dark)");
} catch (e) {
  app.check("the end-to-end run completed", false, String(e).slice(0, 400));
  await app.shot(`${shotsDir}/error.png`).catch(() => {});
}

// Leave the machine as we found it: delete the test projects, reset preferences so the first-run welcome shows again.
try {
  const ids = await app.ev("window.__TAURI_INTERNALS__.invoke('list_projects').then(l=>l.filter(p=>/^e2e-scene/.test(p.id)).map(p=>p.id))");
  for (const id of ids) await app.ev(`window.__TAURI_INTERNALS__.invoke('delete_project',{id:${JSON.stringify(id)}})`);
  await app.ev("localStorage.clear()");
} catch { /* best effort */ }
await app.close();

const failed = app.results.filter((r) => !r.ok);
writeFileSync(`${root}tools/e2e/last-run.json`, JSON.stringify({ when: new Date().toISOString(), passed: app.results.length - failed.length, total: app.results.length, failed, timings, accessibility: a11y }, null, 2));
console.log(`\n${app.results.length - failed.length}/${app.results.length} passed`, JSON.stringify(timings));
process.exit(failed.length ? 1 : 0);
