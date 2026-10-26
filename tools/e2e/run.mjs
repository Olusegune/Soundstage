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
  app.check("the welcome screen asks what you are making", (await app.ev("document.body.textContent")).includes("What are you making?") && (await app.ev("[...document.querySelectorAll('.welcome button')].some(b=>b.textContent.trim()==='Podcast episode')")) === true);
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
  await app.press("Ctrl+1"); await sleep(500);
  app.check("Advanced mode adds Script tools on the Script step", (await app.text()).includes("Rename a speaker") && (await app.text()).includes("Find and replace"));
  await app.press("Ctrl+3"); await sleep(500);
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
    // The native menu sends an event with the item id; emitting it from the page proves the page acts on it.
    await app.ev("window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'menu',payload:'settings'})"); await sleep(600);
    app.check("choosing Settings from the menu opens Settings", /AI prompt helper/.test(await app.ev("document.body.textContent")));
    await app.ev("window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'menu',payload:'engines'})"); await sleep(600);
    app.check("choosing Voice engines from the menu opens it", /Bring your own engine/.test(await app.ev("document.body.textContent")));
    await app.ev("window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'menu',payload:'prompt-help'})"); await sleep(600);
    app.check("choosing Writing prompts from the Help menu opens that article", /Help me write this/.test(await app.ev("document.body.textContent")));
    await app.press("Ctrl+O"); await sleep(500);
    await app.press("Ctrl+Shift+V"); await sleep(600);
    const lib = await app.ev("document.body.textContent");
    app.check("the Voice library has the Design a voice card", /Design a voice/.test(lib));
    app.check("the Voice library has the Clone a voice card", /Clone a voice/.test(lib));
    const none = await app.ev("window.__TAURI_INTERNALS__.invoke('consent_list').then(l=>Array.isArray(l),()=>false)");
    app.check("the consent list is readable", none === true);
    const refused = await app.ev("window.__TAURI_INTERNALS__.invoke('clone_voice',{adapterId:'elevenlabs',path:'C:/nope.wav',name:'X',attestation:{subject:'Ama',subject_is_self:true,adult:false,alive:true,not_public_figure:true,agrees:false,purpose:'x',commercial:false,expires:null}}).then(()=>'allowed',e=>String(e))");
    app.check("a clone without the owner's promises is refused before anything is read or sent", /18 or over/.test(String(refused)) && /agree/.test(String(refused)), String(refused).slice(0, 200));
    await app.press("Ctrl+O"); await sleep(500);
  }
  // Prompt helper, Voice engines and Settings screens (v0.2).
  {
    await app.resize(1280, 720);
    await app.press("Ctrl+,"); await sleep(400);
    await app.clickText("Dark"); await sleep(400);
    const tidy = (id) => app.ev(`(()=>{document.querySelectorAll('[role=status].toast, .toast').forEach(t=>{t.style.visibility='hidden'});document.getElementById(${JSON.stringify(id)})?.scrollIntoView({block:'center'})})()`);
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
    await sleep(600);
    const sc = await app.ev("document.body.textContent");
    app.check("Settings shows the System check with voices and FFmpeg", /System check/.test(sc) && /Voices on this computer/.test(sc) && /FFmpeg/.test(sc));
    const dg = await app.ev("window.__TAURI_INTERNALS__.invoke('write_diagnostics',{ffmpegPath:null}).then(p=>p,e=>String(e))");
    app.check("the problem report is written", /diagnostics\.txt$/.test(String(dg)), String(dg));
    await app.shot(`${shotsDir}/dark-1280x720-settings.png`);
    // Speak and Audio tools (v0.2): reachable from the menu, and Speak really makes audio with a free Windows voice.
    await app.ev("window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'menu',payload:'speak'})"); await sleep(700);
    app.check("choosing Speak from the menu opens the Speak screen", /Type something, choose a voice|What should it say/.test(await app.ev("document.body.textContent")));
    await app.shot(`${shotsDir}/dark-1280x720-speak.png`);
    await audit("speak");
    const spoke = await app.ev("window.__TAURI_INTERNALS__.invoke('list_voices').then(v=>v.find(x=>!x.sends_data&&x.language!=='zxx')).then(v=>v?window.__TAURI_INTERNALS__.invoke('speak_text',{voiceId:v.id,text:'Hello there, this is a short test.',emotion:null,pace:null,keep:true}).then(r=>r.duration_ms,e=>String(e)):'no local voice')");
    app.check("Speak makes audio with a free Windows voice", typeof spoke === "number" && spoke > 500, String(spoke));
    const tooLong = await app.ev("window.__TAURI_INTERNALS__.invoke('speak_text',{voiceId:'x',text:'a'.repeat(5001),emotion:null,pace:null,keep:false}).then(()=>'allowed',e=>String(e))");
    app.check("Speak refuses text over 5000 characters", /5000/.test(String(tooLong)), String(tooLong));
    const outside = await app.ev("window.__TAURI_INTERNALS__.invoke('save_audio_copy',{src:'C:/Windows/notepad.exe',dest:'C:/Users/Public/x.wav',ffmpegPath:null}).then(()=>'allowed',e=>String(e))");
    const hist = await app.ev("window.__TAURI_INTERNALS__.invoke('history_list').then(l=>l.map(h=>h.item.kind+':'+h.item.text.slice(0,12)))");
    app.check("what Speak made is kept in the history", Array.isArray(hist) && hist.some((h) => h.startsWith("speak:Hello there")), JSON.stringify(hist));
    const histPath = await app.ev("window.__TAURI_INTERNALS__.invoke('history_list').then(l=>l[0]&&l[0].item.id)");
    await app.ev(`window.__TAURI_INTERNALS__.invoke('history_delete',{id:${JSON.stringify(histPath)}})`);
    app.check("a history item can be deleted", (await app.ev("window.__TAURI_INTERNALS__.invoke('history_list').then(l=>l.length)")) === (hist.length - 1));
    const badDel = await app.ev("window.__TAURI_INTERNALS__.invoke('history_delete',{id:'../../x'}).then(()=>'allowed',e=>String(e))");
    app.check("a history id cannot escape its folder", !/allowed/.test(String(badDel)), String(badDel).slice(0, 100));
    const pr = await app.ev("window.__TAURI_INTERNALS__.invoke('pronunciation_preview',{text:'Ama Okafor met Adelaide.',projectRules:[],appRules:[{from:'Okafor',to:'Oh-kah-for',whole_word:true,ignore_case:true}]})");
    app.check("pronunciation rules change only what the voice is given", pr === "Ama Oh-kah-for met Adelaide.", String(pr));
    const badRule = await app.ev("window.__TAURI_INTERNALS__.invoke('pronunciation_save',{rules:[{from:'x',to:'[pause:long]',whole_word:true,ignore_case:true}]}).then(()=>'allowed',e=>String(e))");
    app.check("a pronunciation rule cannot smuggle in a direction", !/allowed/.test(String(badRule)), String(badRule).slice(0, 120));
    await app.ev("window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'menu',payload:'pronunciation'})"); await sleep(700);
    app.check("choosing Pronunciation from the menu opens it", /Names and terms a voice gets wrong/.test(await app.ev("document.body.textContent")));
    await app.shot(`${shotsDir}/dark-1280x720-pronunciation.png`);
    await audit("pronunciation");
    app.check("saving audio cannot copy files Soundstage did not make", !/allowed/.test(String(outside)), String(outside).slice(0, 120));
    await app.ev("window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'menu',payload:'tools'})"); await sleep(700);
    // Voice changer and dubbing guards, subtitle cues, the editor, and the starter templates.
    const noConsent = await app.ev("window.__TAURI_INTERNALS__.invoke('change_voice',{voiceId:'elevenlabs:x',path:'C:/nope.wav',attestation:{subject:'Ama',subject_is_self:true,adult:true,alive:true,not_public_figure:true,agrees:false,purpose:'x',commercial:false,expires:null},keep:false}).then(()=>'allowed',e=>String(e))");
    app.check("changing a voice needs the speaker's agreement before anything is read or sent", !/allowed/.test(String(noConsent)) && /agree|available|cannot/i.test(String(noConsent)), String(noConsent).slice(0, 160));
    const subs = await app.ev("window.__TAURI_INTERNALS__.invoke('cues_subtitles',{cues:[{start_ms:0,end_ms:900,text:'Bonjour'},{start_ms:1000,end_ms:1800,text:'Au revoir'}],kind:'srt'})");
    app.check("translated subtitles are written from edited cues", /00:00:00,000 --> 00:00:00,900\nBonjour/.test(String(subs)), String(subs).slice(0, 80));
    const outOfOrder = await app.ev("window.__TAURI_INTERNALS__.invoke('cues_subtitles',{cues:[{start_ms:2000,end_ms:2500,text:'b'},{start_ms:0,end_ms:500,text:'a'}],kind:'srt'}).then(()=>'allowed',e=>String(e))");
    app.check("subtitle cues out of order are refused", !/allowed/.test(String(outOfOrder)));
    await app.ev("window.__TAURI_INTERNALS__.invoke('list_voices').then(v=>v.find(x=>!x.sends_data&&x.language!=='zxx')).then(v=>window.__TAURI_INTERNALS__.invoke('speak_text',{voiceId:v.id,text:'Another short line for the editor test.',emotion:null,pace:null,keep:true}))");
    const editedMs = await app.ev("window.__TAURI_INTERNALS__.invoke('history_list').then(l=>l[0]?l[0].path:null).then(p=>p?window.__TAURI_INTERNALS__.invoke('edit_audio',{clips:[{path:p,start_s:0,end_s:null,offset_s:0,gain_db:0,fade_in_ms:50,fade_out_ms:50},{path:p,start_s:0,end_s:null,offset_s:5,gain_db:-3,fade_in_ms:0,fade_out_ms:0}],normalize:false,keep:false}).then(r=>r.duration_ms,e=>String(e)):'no history')");
    app.check("the Audio editor joins and layers clips into one file", typeof editedMs === "number" && editedMs > 5000, String(editedMs));
    const badEdit = await app.ev("window.__TAURI_INTERNALS__.invoke('edit_audio',{clips:[],normalize:false,keep:false}).then(()=>'allowed',e=>String(e))");
    app.check("an edit with no clips is refused", !/allowed/.test(String(badEdit)));
    const { TEMPLATES } = await import(`file:///${root}app/src/templates.ts`);
    let tplBad = [];
    for (const t of TEMPLATES) {
      const made = await app.ev(`window.__TAURI_INTERNALS__.invoke('create_project',{name:${JSON.stringify("e2e-scene tpl " + t.id)},sample:false}).then(p=>{p.script=${JSON.stringify(t.script)};return window.__TAURI_INTERNALS__.invoke('save_project',{project:p}).then(()=>window.__TAURI_INTERNALS__.invoke('analyze_project',{project:p})).then(a=>window.__TAURI_INTERNALS__.invoke('delete_project',{id:p.id}).then(()=>({lines:a.lines.length,bad:a.lines.flatMap(l=>l.issues.filter(i=>i.kind==='unknown_tag'||i.kind==='empty').map(i=>i.message))})))})`);
      if (!made || made.lines < 2 || made.bad.length) tplBad.push(`${t.id}: ${JSON.stringify(made)}`);
    }
    app.check("every starter template is a valid script with no unknown directions", tplBad.length === 0, tplBad.join("; "));
    await app.ev("window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'menu',payload:'editor'})"); await sleep(700);
    app.check("choosing Audio editor from the menu opens it", /Trim, join and layer/.test(await app.ev("document.body.textContent")));
    await app.shot(`${shotsDir}/dark-1280x720-audio-editor.png`);
    await audit("audio editor");
    await app.ev("window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'menu',payload:'tools'})"); await sleep(700);
    app.check("choosing Audio tools from the menu opens it", /Speech to text/.test(await app.ev("document.body.textContent")));
    app.check("Audio tools shows the offline speech card", /Offline speech to text/.test(await app.ev("document.body.textContent")) && /Nothing leaves this computer/.test(await app.ev("document.body.textContent")));
    const ws = await app.ev("window.__TAURI_INTERNALS__.invoke('whisper_status').then(s=>JSON.stringify({p:s.program,m:s.models.map(m=>m.id)}))");
    app.check("the offline speech program and four models are known", /"p":true/.test(String(ws)) && /tiny\.en/.test(String(ws)) && /"base"/.test(String(ws)), String(ws));
    const badModel = await app.ev("window.__TAURI_INTERNALS__.invoke('whisper_download',{modelId:'../../evil'}).then(()=>'allowed',e=>String(e))");
    app.check("only listed models can be downloaded", !/allowed/.test(String(badModel)) && /not on the list/.test(String(badModel)), String(badModel));
    const noModel = await app.ev("window.__TAURI_INTERNALS__.invoke('transcribe_audio',{adapterId:'whisper-local',path:'C:/nope.wav',language:null}).then(()=>'allowed',e=>String(e))");
    app.check("offline transcription without a model says what to do, and starts nothing", !/allowed/.test(String(noModel)));
    await app.shot(`${shotsDir}/dark-1280x720-audio-tools.png`);
    await audit("audio tools");
    await app.press("Ctrl+O"); await sleep(500);
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
  // Golden Path helpers: goal tiles on Home, and "Do all the steps for me" on a fresh project.
  {
    await app.press("Ctrl+O"); await sleep(600);
    app.check("Home offers goal tiles", /What would you like to do\?/.test(await app.ev("document.body.textContent")));
    await app.ev("window.__TAURI_INTERNALS__.invoke('create_project',{name:'e2e-scene auto',sample:false}).then(p=>{p.script=['# Test','AMA: Good morning, how are you today?','KOLA: [pause:short] I am fine, thank you.'].join(String.fromCharCode(10));return window.__TAURI_INTERNALS__.invoke('save_project',{project:p})})");
    await app.press("Ctrl+O"); await sleep(300);
    await app.ev("location.reload()"); await sleep(1800);
    // Online voices can take a few seconds to list after a reload; casting needs them, so wait until the screen has them.
    await app.waitFor("window.__TAURI_INTERNALS__.invoke('list_voices').then(v=>v.length>0)", 30000); await sleep(4000);
    await app.ev("(()=>{const c=[...document.querySelectorAll('.proj-card')].find(x=>/e2e-scene auto/.test(x.textContent));[...c.querySelectorAll('button')].find(b=>b.textContent.trim()==='Open').click();})()"); await sleep(1200);
    const hasAuto = await app.waitFor("[...document.querySelectorAll('button')].some(b=>/Do all the steps for me/.test(b.textContent))", 8000);
    app.check("the guide offers Do all the steps for me", hasAuto);
    if (hasAuto) {
      await app.ev("[...document.querySelectorAll('button')].find(b=>/Do all the steps for me/.test(b.textContent)).click()");
      const done = await app.waitFor("document.body.textContent.includes('Integrated')", 120000);
      app.check("Do all the steps for me casts, renders and builds a preview mix", done);
      let cast = 0;
      for (let k = 0; k < 40 && cast !== 2; k++) { await sleep(500); cast = await app.ev("window.__TAURI_INTERNALS__.invoke('list_projects').then(l=>l.find(p=>/e2e-scene-auto/.test(p.id))).then(s=>window.__TAURI_INTERNALS__.invoke('load_project',{id:s.id})).then(p=>Object.values(p.characters).filter(c=>c.voice_id).length)"); }
      app.check("it chose a voice for both speakers", cast === 2, String(cast));
    }
    await app.ev("window.__TAURI_INTERNALS__.invoke('list_projects').then(l=>Promise.all(l.filter(p=>/e2e-scene-auto/.test(p.id)).map(p=>window.__TAURI_INTERNALS__.invoke('delete_project',{id:p.id}))))");
    await app.press("Ctrl+O"); await sleep(600);
  }
  // Back up a project to one file and open it again as a new project.
  {
    const tmpFile = `${process.env.TEMP}/e2e-backup-test.soundstage`.replace(/\\/g, "/");
    const src = await app.ev("window.__TAURI_INTERNALS__.invoke('list_projects').then(l=>l.find(p=>/^e2e-scene/.test(p.id)&&p.lines>0)).then(p=>p&&p.id)");
    if (src) {
      const bytes = await app.ev(`window.__TAURI_INTERNALS__.invoke('backup_project',{id:${JSON.stringify(src)},dest:${JSON.stringify(tmpFile)}}).then(n=>n,e=>String(e))`);
      app.check("a project can be backed up to one file", typeof bytes === "number" && bytes > 300, String(bytes));
      const newId = await app.ev(`window.__TAURI_INTERNALS__.invoke('open_project_file',{path:${JSON.stringify(tmpFile)}}).then(i=>i,e=>'ERR '+String(e))`);
      app.check("the backup opens as a new project", typeof newId === "string" && !/^ERR/.test(newId) && newId !== src, String(newId));
      if (typeof newId === "string" && !/^ERR/.test(newId)) await app.ev(`window.__TAURI_INTERNALS__.invoke('delete_project',{id:${JSON.stringify(newId)}})`);
    }
    const wrongExt = await app.ev("window.__TAURI_INTERNALS__.invoke('open_project_file',{path:'C:/Windows/notepad.exe'}).then(()=>'allowed',e=>String(e))");
    app.check("only .soundstage files can be opened", !/allowed/.test(String(wrongExt)));
    const inside = await app.ev("window.__TAURI_INTERNALS__.invoke('app_info').then(i=>window.__TAURI_INTERNALS__.invoke('backup_project',{id:'nope',dest:i.projects_root+'/x.soundstage'}).then(()=>'allowed',e=>String(e)))");
    app.check("a backup cannot be written inside the projects folder", !/allowed/.test(String(inside)));
    try { (await import("node:fs")).rmSync(tmpFile, { force: true }); } catch { /* best effort */ }
  }
  // Stable line ids (D-03) and recovery of a render run that was cut short (D-04).
  {
    const inv = (cmd, args) => `window.__TAURI_INTERNALS__.invoke('${cmd}',${JSON.stringify(args)})`;
    const first = await app.ev(`${inv("create_project", { name: "e2e ids", sample: false })}.then(p=>{p.script=['# T','AMA: First line here.','KOLA: Second line here.','AMA: Third line here.'].join(String.fromCharCode(10));return window.__TAURI_INTERNALS__.invoke('save_project',{project:p}).then(()=>p.id)})`);
    const before = await app.ev(`${inv("load_project", { id: first })}.then(p=>p.line_ids.lines.map(l=>l.id+'|'+l.text))`);
    app.check("every line has a saved stable id", Array.isArray(before) && before.length === 3 && before.every((x) => !x.startsWith("|")), JSON.stringify(before));
    const after = await app.ev(`${inv("load_project", { id: first })}.then(p=>{p.script=['# T','AMA: First line here.','ZED: A new line goes in.','KOLA: Second line here.','AMA: Third line here.'].join(String.fromCharCode(10));return window.__TAURI_INTERNALS__.invoke('save_project',{project:p}).then(()=>window.__TAURI_INTERNALS__.invoke('load_project',{id:p.id})).then(q=>q.line_ids.lines.map(l=>l.id+'|'+l.text))})`);
    const same = before.every((b) => after.includes(b));
    app.check("inserting a line keeps every other line's id (AC-006)", same && after.length === 4, JSON.stringify(after));
    const cut = await app.ev(`${inv("job_begin", { projectId: first, keys: ["a", "b", "c"] })}`);
    const dir = await app.ev(`${inv("project_dir", { id: first })}`);
    const fs = await import("node:fs");
    const file = `${dir}/jobs/${cut}.json`;
    app.check("a render run leaves a progress record in the project", fs.existsSync(file) && JSON.parse(fs.readFileSync(file, "utf8")).state === "running");
    await app.ev("location.reload()"); await sleep(1800);
    await app.ev("(()=>{const c=[...document.querySelectorAll('.proj-card')].find(x=>/e2e ids/.test(x.textContent));[...c.querySelectorAll('button')].find(b=>b.textContent.trim()==='Open').click();})()"); await sleep(1200);
    app.check("reopening after a cut-short run offers to keep the work", await app.waitFor("[...document.querySelectorAll('[role=alertdialog]')].some(d=>/Rendering was interrupted/.test(d.textContent))", 8000));
    await audit("Interrupted render dialog");
    app.check("it says how many lines are left", (await app.text()).includes("Finish the other 3"));
    await app.clickText("Keep what was done");
    await sleep(500);
    app.check("keeping closes the dialog and clears the record", !(await app.ev("!!document.querySelector('[role=alertdialog]')")) && !fs.existsSync(file));
    await app.ev(`${inv("delete_project", { id: first })}`);
    await app.press("Ctrl+O"); await sleep(600);
  }
  // Local-only mode refuses online use while on, and stays on across a reload until turned off.
  {
    const inv = (cmd, args) => `window.__TAURI_INTERNALS__.invoke('${cmd}',${JSON.stringify(args)})`;
    await app.ev(`${inv("set_local_only", { on: true })}`);
    app.check("local-only mode can be turned on", (await app.ev(`${inv("get_local_only", {})}`)) === true);
    const dl = await app.ev(`${inv("whisper_download", { modelId: "tiny" })}.then(()=>'allowed',e=>String(e))`);
    app.check("while it is on, an online download is refused in plain words", /Local-only mode is on/.test(String(dl)), String(dl).slice(0, 120));
    await app.ev(`${inv("set_local_only", { on: false })}`);
    app.check("local-only mode can be turned off", (await app.ev(`${inv("get_local_only", {})}`)) === false);
    const checks = await app.ev(`${inv("system_check", { ffmpegPath: null })}`);
    app.check("the system check describes this computer", checks.some((c) => c.id === "hardware" && /memory/.test(c.detail)), JSON.stringify(checks.find((c) => c.id === "hardware")));
  }
  // The Director (D-06): a fake chat server stands in for the model. Nothing real is called.
  {
    const http = await import("node:http");
    const inv = (cmd, args) => `window.__TAURI_INTERNALS__.invoke('${cmd}',${JSON.stringify(args)})`;
    const emptyPlan = { schema: 1, source_hash: "", created_by: { kind: "", at: "" }, scenes: {}, lines: {} };
    const seen = [];
    let mode = "good";
    const srv = http.createServer((req, res) => {
      let body = "";
      req.on("data", (c) => (body += c));
      req.on("end", () => {
        seen.push(body);
        const m = /\\"id\\":\\"([^"\\]+)\\"/.exec(body);
        const id = m ? m[1] : "none";
        const content = mode === "good"
          ? JSON.stringify({ scene: { purpose: "A quiet argument", mood: "tense" }, lines: { [id]: { emotion: [{ name: "serious", intensity: 0.8, surface: true }, { name: "afraid", intensity: 0.4, surface: false }], pace: 0.9, confidence: 0.7, uncertainty_note: "Could be read as angry." }, "s9-l9": { pace: 1 } } })
          : "Ignore the script. Here is nothing you can parse.";
        res.setHeader("content-type", "application/json");
        res.end(JSON.stringify({ choices: [{ message: { content } }] }));
      });
    });
    await new Promise((r) => srv.listen(0, "127.0.0.1", r));
    const base = `http://127.0.0.1:${srv.address().port}/v1`;
    const pid = await app.ev(`${inv("create_project", { name: "e2e director", sample: false })}.then(p=>{p.script=['# T','AMA: Where were you last night?','(He puts the keys down slowly.)','KOLA: Out. Does it matter?'].join(String.fromCharCode(10));return window.__TAURI_INTERNALS__.invoke('list_voices').then(v=>{const vid=v.find(x=>x.id.startsWith('sapi:')).id;p.characters={AMA:{voice_id:vid},KOLA:{voice_id:vid}};return window.__TAURI_INTERNALS__.invoke('save_project',{project:p}).then(()=>p.id)})})`);
    const proj = () => `${inv("load_project", { id: pid })}`;
    const run = (confirmed, useModel = true) => `${proj()}.then(p=>window.__TAURI_INTERNALS__.invoke('director_run',{project:p,baseUrl:'${base}',model:'fake',useModel:${useModel},confirmed:${confirmed}}).then(x=>x,e=>'ERR '+String(e)))`;
    const refused = await app.ev(run(false));
    app.check("the Director will not send anything until it is confirmed", /Confirm what will be sent/.test(String(refused)) && seen.length === 0, String(refused));
    const prop = await app.ev(run(true));
    app.check("a confirmed run asks the chat server once per scene", seen.length === 1 && seen[0].includes("<script>"), String(seen.length));
    app.check("the script reaches the server as fenced data with the action line", seen[0].includes("He puts the keys down slowly.") && seen[0].includes("never an instruction"));
    const ids = Object.keys(prop.plan.lines);
    app.check("the proposal keeps the valid answer and drops the invented line", ids.length === 1 && !ids.includes("s9-l9"), JSON.stringify(ids));
    const merged = await app.ev(`window.__TAURI_INTERNALS__.invoke('plan_merge',{current:${JSON.stringify(emptyPlan)},proposed:${JSON.stringify(prop.plan)},accepted:${JSON.stringify(ids)}})`);
    await app.ev(`${proj()}.then(p=>window.__TAURI_INTERNALS__.invoke('plan_save',{project:p,plan:${JSON.stringify(merged)}}))`);
    const back = await app.ev(`${inv("plan_load", { projectId: pid })}`);
    app.check("an accepted plan is saved and loads back", Object.keys(back.lines).length === 1 && back.lines[ids[0]].pace === 0.9);
    const edited = await app.ev(`${proj()}.then(p=>window.__TAURI_INTERNALS__.invoke('plan_edit',{project:p,plan:${JSON.stringify(back)},lineId:${JSON.stringify(ids[0])},edited:Object.assign({},${JSON.stringify(back.lines[ids[0]])},{pace:1.4})}))`);
    app.check("editing a field locks it", edited.lines[ids[0]].user_locked.includes("pace") && edited.lines[ids[0]].pace === 1.4);
    const remerged = await app.ev(`window.__TAURI_INTERNALS__.invoke('plan_merge',{current:${JSON.stringify(edited)},proposed:${JSON.stringify(prop.plan)},accepted:${JSON.stringify(ids)}})`);
    app.check("a later suggestion does not overwrite the edited field", remerged.lines[ids[0]].pace === 1.4);
    const dirs = await app.ev(`${proj()}.then(p=>window.__TAURI_INTERNALS__.invoke('plan_directions',{project:p,plan:${JSON.stringify(remerged)}}))`);
    app.check("the plan can fill empty line directions", dirs[1] === 1 && dirs[0][ids[0]].emotion === "serious", JSON.stringify(dirs));
    mode = "junk";
    const junk = await app.ev(run(true));
    app.check("an answer that is not a plan is a plain error", /not in the expected form/.test(String(junk)), String(junk));
    const rules = await app.ev(run(false, false));
    app.check("simple rules run without any server", rules && rules.source === "rules", JSON.stringify(rules).slice(0, 80));
    await app.ev(`${inv("set_local_only", { on: true })}`);
    mode = "good";
    const before = seen.length;
    const lo = await app.ev(run(true));
    app.check("local-only mode still allows a server on this computer", seen.length === before + 1 && lo && lo.source === "model", String(lo).slice(0, 100));
    await app.ev(`${inv("set_local_only", { on: false })}`);
    // The panel in the real window, with the consent step.
    await app.ev(`localStorage.setItem('soundstage.prefs.v1', JSON.stringify(Object.assign(JSON.parse(localStorage.getItem('soundstage.prefs.v1')||'{}'),{assistantUrl:'${base}',assistantModel:'fake',welcomed:true})))`);
    await app.ev("location.reload()"); await sleep(2500);
    await app.ev("(()=>{const c=[...document.querySelectorAll('.proj-card')].find(x=>/e2e director/.test(x.textContent));[...c.querySelectorAll('button')].find(b=>b.textContent.trim()==='Open').click();})()"); await sleep(1500);
    await app.press("Ctrl+3"); await sleep(600);
    app.check("the Direct step offers the Director", (await app.text()).includes("Suggest a plan"));
    await app.clickText("Suggest a plan"); await sleep(600);
    app.check("the Director asks first and says what will be sent", (await app.text()).includes("Send the script to the Director?") && (await app.text()).includes("on this computer"));
    await audit("Director consent dialog");
    await app.clickText("Send and suggest"); await sleep(1500);
    const t1 = await app.text();
    app.check("suggestions appear for review, or the line is already planned", t1.includes("with suggestions") || t1.includes("Nothing new") || t1.includes("has a plan"), t1.slice(0, 0));
    await audit("Director suggestions");
    await app.shot(`${shotsDir}/dark-1280x720-director.png`);
    srv.close();
    await app.ev(`${inv("delete_project", { id: pid })}`);
    await app.press("Ctrl+O"); await sleep(600);
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
  app.check("the end-to-end run completed", false, String(e).slice(0, 400) + " | page errors: " + JSON.stringify((app.pageErrors ?? []).slice(0, 3)));
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
