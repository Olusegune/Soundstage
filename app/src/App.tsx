import { listen } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, messageOf } from "./api";
import type { Ctx } from "./ctx";
import { golden } from "./golden";
import { ARTICLES, GLOSSARY } from "./help/content";
import { HelpCenter } from "./help/HelpCenter";
import * as I from "./icons";
import { applyTheme, loadPrefs, resolveTheme, savePrefs, type Mode, type Prefs, type Theme } from "./prefs";
import { Home } from "./screens/Home";
import { Providers } from "./screens/Providers";
import { Settings } from "./screens/Settings";
import { VoiceLibrary } from "./screens/VoiceLibrary";
import { CommandPalette, type PaletteItem } from "./shell/CommandPalette";
import { GuidePanel } from "./shell/GuidePanel";
import { Splash, Welcome } from "./shell/Onboarding";
import { Rail, type View } from "./shell/Rail";
import { ShortcutsDialog } from "./shell/ShortcutsDialog";
import { Toolbar } from "./shell/Toolbar";
import { Tour } from "./shell/Tour";
import { DEFAULT_SHORTCUTS, comboOf, isTextTarget, resolveBindings, shouldHandle, type ShortcutId } from "./shortcuts";
import { CastStep } from "./steps/CastStep";
import { DirectStep } from "./steps/DirectStep";
import { ExportStep } from "./steps/ExportStep";
import { MixStep } from "./steps/MixStep";
import { RenderStep } from "./steps/RenderStep";
import { SAMPLE, ScriptStep } from "./steps/ScriptStep";
import { Confirm, PlayerProvider, Toast, usePlayer, type ToastMsg } from "./ui";
import type { Analysis, MixPreview, Project, Summary, Voice } from "./types";

const HISTORY_LIMIT = 30;
const STEP_IDS = ["script", "cast", "direct", "render", "mix", "export"] as const;

/** The preview key toggles audio from anywhere outside a control. Lives inside the player provider. */
function PreviewKey({ enabled, combo }: { enabled: boolean; combo: string }) {
  const player = usePlayer();
  useEffect(() => {
    if (!enabled) return;
    const h = (e: KeyboardEvent) => {
      if (comboOf(e) !== combo || isTextTarget(e.target)) return;
      if ((e.target as HTMLElement)?.closest?.("button, a, summary, [role=button], [role=option], input, select, textarea")) return;
      if (document.querySelector('[aria-modal="true"]')) return;
      e.preventDefault();
      player.toggleLast();
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [enabled, combo, player]);
  return null;
}

export default function App() {
  const [prefs, setPrefsState] = useState<Prefs>(loadPrefs);
  const setPrefs = useCallback((patch: Partial<Prefs>) => setPrefsState((p) => { const n = { ...p, ...patch }; savePrefs(n); return n; }), []);

  const [view, setView] = useState<View>("home");
  const [returnTo, setReturnTo] = useState<View>("home");
  const [helpId, setHelpId] = useState("quick-start");
  const [project, setProject] = useState<Project | null>(null);
  const [dir, setDir] = useState("");
  const [step, setStep] = useState(0);
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [voices, setVoices] = useState<Voice[]>([]);
  const [voicesReady, setVoicesReady] = useState(false);
  const [info, setInfo] = useState<{ version: string; projects_root: string } | null>(null);
  const [toast, setToast] = useState<ToastMsg | null>(null);
  const [busy, setBusy] = useState<Set<string>>(new Set());
  const [saved, setSaved] = useState<"saved" | "saving" | "error">("saved");
  const [sendPrompt, setSendPrompt] = useState<{ provider: string; lines: number; chars: number; resolve: (ok: boolean) => void } | null>(null);
  const [batch, setBatch] = useState<{ done: number; total: number } | null>(null);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [mixPreview, setMixPreview] = useState<MixPreview | null>(null);
  const [mixBuilding, setMixBuilding] = useState(false);
  const [mixPreviewed, setMixPreviewed] = useState(false);
  const [exported, setExported] = useState(false);
  const [focusKey, setFocusKey] = useState<string | null>(null);
  const [palette, setPalette] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [tour, setTour] = useState(false);
  const [splash, setSplash] = useState(true);
  const [projectsForPalette, setProjectsForPalette] = useState<Summary[]>([]);
  const [announce, setAnnounce] = useState("");
  const [narrow, setNarrow] = useState(() => window.innerWidth < 1100);
  const [, setHistVersion] = useState(0);

  const projectRef = useRef<Project | null>(null);
  const analysisRef = useRef<Analysis | null>(null);
  const voicesRef = useRef<Voice[]>([]);
  const history = useRef<string[]>([]);
  const future = useRef<string[]>([]);
  const lastCoalesce = useRef<{ key: string; at: number } | null>(null);
  const saveTimer = useRef<number | undefined>(undefined);
  const landed = useRef(true);

  const say = useCallback((text: string, bad?: boolean) => setToast({ text, bad }), []);
  const bindings = useMemo(() => resolveBindings(prefs.shortcuts), [prefs.shortcuts]);
  const keyOf = useCallback((id: ShortcutId) => bindings[id][0] ?? "", [bindings]);

  useEffect(() => {
    const h = () => setNarrow(window.innerWidth < 1100);
    window.addEventListener("resize", h);
    return () => window.removeEventListener("resize", h);
  }, []);
  const railCollapsed = prefs.railCollapsed || narrow;

  /* ---------------------------------------------------------------- theme */
  useEffect(() => {
    applyTheme(prefs.theme);
    if (prefs.theme !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: light)");
    const h = () => applyTheme("system");
    mq.addEventListener("change", h);
    return () => mq.removeEventListener("change", h);
  }, [prefs.theme]);

  /* ---------------------------------------------------------------- startup */
  useEffect(() => {
    void api.info().then(setInfo).catch(() => undefined);
    void api.voices().then((v) => { setVoices(v); setVoicesReady(true); }).catch((e) => { say(messageOf(e), true); setVoicesReady(true); });
  }, [say]);
  useEffect(() => { projectRef.current = project; }, [project]);
  useEffect(() => { analysisRef.current = analysis; }, [analysis]);
  useEffect(() => { voicesRef.current = voices; }, [voices]);

  // Re-check the script, voices and render status whenever the project changes.
  useEffect(() => {
    if (!project) return;
    let live = true;
    const t = window.setTimeout(() => { void api.analyze(project).then((a) => live && setAnalysis(a)).catch(() => undefined); }, 180);
    return () => { live = false; window.clearTimeout(t); };
  }, [project, voices]);

  // A preview mix describes the project as it was when it was built.
  useEffect(() => { setMixPreview(null); }, [project?.cues, project?.mix, project?.selected, project?.takes, project?.assets]);

  /* ---------------------------------------------------------------- saving, undo, redo */
  const flush = useCallback(async () => {
    window.clearTimeout(saveTimer.current);
    const p = projectRef.current;
    if (!p) return;
    try { await api.saveProject(p); setSaved("saved"); } catch (e) { setSaved("error"); say(`Could not save: ${messageOf(e)}`, true); }
  }, [say]);

  const scheduleSave = useCallback(() => {
    setSaved("saving");
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => void flush(), 700);
  }, [flush]);

  const edit = useCallback((fn: (p: Project) => Project, coalesce?: string) => {
    const cur = projectRef.current;
    if (!cur) return;
    const now = Date.now();
    const lc = lastCoalesce.current;
    if (!(coalesce && lc && lc.key === coalesce && now - lc.at < 1000)) {
      history.current.push(JSON.stringify(cur));
      if (history.current.length > HISTORY_LIMIT) history.current.shift();
    }
    future.current = [];
    lastCoalesce.current = coalesce ? { key: coalesce, at: now } : null;
    const next = fn(cur);
    projectRef.current = next;
    setProject(next);
    setHistVersion((v) => v + 1);
    scheduleSave();
  }, [scheduleSave]);

  /** Undo and redo revert authoring changes but never discard rendered takes or imported audio. */
  const restore = useCallback((snap: string) => {
    const cur = projectRef.current;
    if (!cur) return;
    const old = JSON.parse(snap) as Project;
    const ids = new Set(cur.takes.map((t) => t.id));
    const next: Project = { ...old, takes: cur.takes, assets: cur.assets, selected: Object.fromEntries(Object.entries(old.selected).filter(([, id]) => ids.has(id))) };
    projectRef.current = next;
    setProject(next);
    setHistVersion((v) => v + 1);
    scheduleSave();
  }, [scheduleSave]);

  const undo = useCallback(() => {
    const snap = history.current.pop();
    const cur = projectRef.current;
    if (!snap || !cur) return;
    future.current.push(JSON.stringify(cur));
    restore(snap);
    say("Undone.");
  }, [restore, say]);

  const redo = useCallback(() => {
    const snap = future.current.pop();
    const cur = projectRef.current;
    if (!snap || !cur) return;
    history.current.push(JSON.stringify(cur));
    restore(snap);
    say("Redone.");
  }, [restore, say]);

  useEffect(() => {
    const h = () => { void flush(); };
    window.addEventListener("beforeunload", h);
    return () => window.removeEventListener("beforeunload", h);
  }, [flush]);

  /* ---------------------------------------------------------------- Golden Path */
  const g = useMemo(() => (project ? golden({ project, analysis, voices, mixPreviewed, exported }) : null), [project, analysis, voices, mixPreviewed, exported]);
  const currentIdx = g ? STEP_IDS.indexOf(g.current) : 0;

  // Land on the recommended step when a project with progress is opened.
  useEffect(() => {
    if (landed.current || !g || !analysis) return;
    landed.current = true;
    if (project && project.takes.length > 0) setStep(currentIdx);
  }, [g, analysis, project, currentIdx]);

  const goStep = useCallback((i: number) => {
    const s = g?.steps[i];
    if (!s) return;
    if (s.status === "blocked") { say(s.blockedReason ?? "Finish the earlier steps first.", true); return; }
    setView("project");
    setStep(i);
    setAnnounce(`${s.label}. ${s.summary}`);
  }, [g, say]);

  /* ---------------------------------------------------------------- projects */
  const openProject = useCallback(async (id: string) => {
    try {
      await flush();
      const p = await api.loadProject(id);
      setDir(await api.projectDir(id));
      history.current = []; future.current = [];
      projectRef.current = p;
      setProject(p);
      setAnalysis(null);
      setMixPreview(null); setMixPreviewed(false); setExported(false); setErrors({}); setBatch(null);
      landed.current = false;
      setStep(0);
      setSaved("saved");
      setView("project");
    } catch (e) { say(messageOf(e), true); }
  }, [flush, say]);

  const goHome = useCallback(async () => {
    await flush();
    setProject(null); projectRef.current = null; setAnalysis(null);
    setView("home");
  }, [flush]);

  const quickStart = useCallback(async () => {
    try {
      const p = await api.createProject("My first scene", true);
      await openProject(p.id);
      setTour(true);
    } catch (e) { say(messageOf(e), true); }
  }, [openProject, say]);

  const openAux = useCallback((v: "library" | "settings" | "help" | "providers") => {
    setReturnTo((r) => (view === "library" || view === "settings" || view === "help" || view === "providers" ? r : view));
    setView(v);
  }, [view]);
  const back = () => setView(returnTo === "project" && project ? "project" : "home");
  const openHelp = useCallback((id?: string) => { if (id) setHelpId(id); openAux("help"); }, [openAux]);

  /* ---------------------------------------------------------------- rendering */
  const confirmSend = useCallback((keys: string[]): Promise<boolean> => {
    const a = analysisRef.current;
    const cloud = (a?.lines ?? [])
      .filter((l) => keys.includes(l.line.key))
      .map((l) => ({ l, v: voicesRef.current.find((v) => v.id === l.voice_id) }))
      .filter((x) => x.v?.sends_data);
    if (cloud.length === 0) return Promise.resolve(true);
    const chars = cloud.reduce((n, x) => n + x.l.line.text.replace(/\[[^\]]*\]/g, "").length, 0);
    const provider = [...new Set(cloud.map((x) => x.v?.provider_label ?? "a cloud provider"))].join(" and ");
    return new Promise((resolve) => setSendPrompt({ provider, lines: cloud.length, chars, resolve }));
  }, []);

  const render = useCallback(async (keys: string[], opts?: { confirmed?: boolean }) => {
    const cur = projectRef.current;
    if (!cur) return null;
    if (!opts?.confirmed && !(await confirmSend(keys))) return null;
    setBusy((b) => new Set([...b, ...keys]));
    try {
      const out = await api.renderLines(cur, keys);
      // Merge only what the run changed, so edits made meanwhile survive.
      edit((p) => ({ ...p, takes: [...p.takes, ...out.new_takes], selected: { ...p.selected, ...out.selected } }));
      const failed = out.results.filter((r) => !r.ok);
      if (failed.length > 0) say(failed[0]?.message ?? "Rendering failed.", true);
      else if (out.results.some((r) => r.result?.quality && r.result.quality.clipped_samples > 0)) say("Rendered. Some audio is clipping, so check the take.", true);
      else if (out.results.length === 1) say(out.results[0]?.cached ? "That line was already up to date." : "Rendered.");
      return out;
    } catch (e) { say(messageOf(e), true); return null; }
    finally { setBusy((b) => { const n = new Set(b); keys.forEach((k) => n.delete(k)); return n; }); }
  }, [edit, say, confirmSend]);

  const renderBatch = useCallback(async (keys: string[]) => {
    if (keys.length === 0) return;
    if (!(await confirmSend(keys))) return;
    setBatch({ done: 0, total: keys.length });
    setErrors((e) => { const n = { ...e }; keys.forEach((k) => delete n[k]); return n; });
    let done = 0, failed = 0;
    // One line at a time keeps the progress honest and the interface responsive.
    for (const k of keys) {
      const out = await render([k], { confirmed: true });
      done += 1;
      setBatch({ done, total: keys.length });
      const bad = out?.results.find((r) => !r.ok);
      if (bad) { failed += 1; setErrors((e) => ({ ...e, [k]: bad.message ?? "Rendering failed." })); }
      else if (out === null) { failed += 1; setErrors((e) => ({ ...e, [k]: "Rendering failed. Try again." })); }
    }
    setBatch(null);
    say(failed ? `${keys.length - failed} of ${keys.length} lines rendered. ${failed} need attention.` : `Rendered ${keys.length} line${keys.length === 1 ? "" : "s"}.`, failed > 0);
  }, [confirmSend, render, say]);

  const buildMix = useCallback(async () => {
    const p = projectRef.current;
    if (!p) return;
    setMixBuilding(true);
    try { setMixPreview(await api.mixPreview(p)); setMixPreviewed(true); } catch (e) { say(messageOf(e), true); } finally { setMixBuilding(false); }
  }, [say]);

  /* ---------------------------------------------------------------- the one button */
  const doNext = useCallback(() => {
    const n = g?.next;
    if (!g || !n) return;
    const idx = STEP_IDS.indexOf(n.stepId);
    switch (n.kind) {
      case "sample": edit((p) => ({ ...p, script: SAMPLE })); setStep(0); break;
      case "goto": goStep(idx); break;
      case "render": {
        const keys = (analysisRef.current?.lines ?? []).filter((l) => (l.status === "drafted" || l.status === "stale") && l.issues.length === 0).map((l) => l.line.key);
        setStep(3); void renderBatch(keys); break;
      }
      case "preview-mix": setStep(4); void buildMix(); break;
      case "export": setStep(5); break;
    }
  }, [g, edit, goStep, renderBatch, buildMix]);

  const newProject = useCallback(async () => {
    await goHome();
    setTimeout(() => document.getElementById("pname")?.focus(), 80);
  }, [goHome]);

  const startTour = useCallback(async () => {
    if (!projectRef.current) { await quickStart(); return; }
    setView("project");
    setPrefs({ guideOpen: true });
    setTimeout(() => setTour(true), 60);
  }, [quickStart, setPrefs]);

  const setMode = useCallback((m: Mode) => { setPrefs({ mode: m }); say(m === "simple" ? "Simple mode: the essentials." : "Advanced mode: every control."); }, [setPrefs, say]);
  const setTheme = useCallback((t: Theme) => setPrefs({ theme: t }), [setPrefs]);

  /* ---------------------------------------------------------------- keyboard */
  const actions: Record<ShortcutId, () => void> = useMemo(() => ({
    palette: () => { void api.listProjects().then(setProjectsForPalette).catch(() => undefined); setPalette((p) => !p); },
    "new-project": () => void newProject(),
    "open-project": () => void goHome(),
    save: () => { void flush().then(() => say("Saved.")); },
    undo, redo,
    "step-1": () => goStep(0), "step-2": () => goStep(1), "step-3": () => goStep(2), "step-4": () => goStep(3), "step-5": () => goStep(4), "step-6": () => goStep(5),
    "step-prev": () => goStep(Math.max(0, step - 1)),
    "step-next": () => goStep(Math.min(5, step + 1)),
    "do-next": doNext,
    preview: () => undefined,
    "toggle-rail": () => setPrefs({ railCollapsed: !prefs.railCollapsed }),
    "toggle-guide": () => setPrefs({ guideOpen: !prefs.guideOpen }),
    "toggle-mode": () => setMode(prefs.mode === "simple" ? "advanced" : "simple"),
    voices: () => openAux("library"),
    settings: () => openAux("settings"),
    help: () => openHelp(),
    shortcuts: () => setShortcutsOpen((s) => !s),
    tour: () => void startTour(),
  }), [newProject, goHome, flush, say, undo, redo, goStep, step, doNext, setPrefs, prefs.railCollapsed, prefs.guideOpen, prefs.mode, setMode, openAux, openHelp, startTour]);
  const actionsRef = useRef(actions);
  actionsRef.current = actions;

  // The native Windows menu sends the id of the item that was chosen.
  useEffect(() => {
    let off: (() => void) | undefined;
    void listen<string>("menu", (ev) => {
      const id = ev.payload;
      const a = actionsRef.current;
      const direct: Record<string, () => void> = {
        export: () => a["step-6"](),
        engines: () => openAux("providers"),
        "theme-dark": () => setTheme("dark"), "theme-light": () => setTheme("light"), "theme-system": () => setTheme("system"),
        "prompt-help": () => openHelp("prompt-helper"),
        troubleshooting: () => openHelp("troubleshooting"),
        "system-check": () => openAux("settings"),
        "report-problem": () => { void api.writeDiagnostics(prefs.ffmpegPath || null).then((p) => { say("Report saved. Attach it when you ask for help. It holds no keys or script text."); void revealItemInDir(p); }).catch((e) => say(messageOf(e), true)); },
        "show-folder": () => { void api.info().then((i) => revealItemInDir(i.projects_root)).catch((e) => say(messageOf(e), true)); },
        exit: () => { void flush().finally(() => void api.quit()); },
      };
      if (direct[id]) { direct[id](); return; }
      if (id in a) a[id as ShortcutId]();
    }).then((u) => { off = u; });
    return () => off?.();
  }, [openAux, setTheme, openHelp, say, flush, prefs.ffmpegPath]);

  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      const combo = comboOf(e);
      if (!combo) return;
      const def = DEFAULT_SHORTCUTS.find((d) => bindings[d.id].includes(combo));
      if (!def || def.id === "preview") return;
      if (palette && def.id !== "palette") return;
      if (def.project && view !== "project") return;
      const dialogOpen = !!document.querySelector('[aria-modal="true"]') && def.id !== "palette";
      if (!shouldHandle(def, combo, isTextTarget(e.target), dialogOpen)) return;
      e.preventDefault();
      actionsRef.current[def.id]();
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [bindings, view, palette]);

  /* ---------------------------------------------------------------- palette content */
  const paletteItems: PaletteItem[] = useMemo(() => {
    const out: PaletteItem[] = [];
    const k = (id: ShortcutId) => bindings[id][0];
    if (project && g?.next) out.push({ id: "do-next", group: "Do", title: `Next: ${g.next.label}`, sub: g.next.reason, icon: <I.Zap size={16} />, keys: k("do-next"), run: doNext, suggested: true });
    out.push({ id: "new", group: "Do", title: "New project", icon: <I.Plus size={16} />, keys: k("new-project"), run: () => void newProject(), suggested: true });
    out.push({ id: "tour", group: "Do", title: "Take the guided tour", icon: <I.Rocket size={16} />, keys: k("tour"), run: () => void startTour(), suggested: true });
    out.push({ id: "quick", group: "Do", title: "Quick start: make a first scene", icon: <I.Sparkle size={16} />, run: () => void quickStart() });
    out.push({ id: "mode", group: "Do", title: prefs.mode === "simple" ? "Switch to Advanced mode" : "Switch to Simple mode", icon: <I.Direct size={16} />, keys: k("toggle-mode"), run: () => setMode(prefs.mode === "simple" ? "advanced" : "simple") });
    out.push({ id: "theme", group: "Do", title: resolveTheme(prefs.theme) === "dark" ? "Switch to the light theme" : "Switch to the dark theme", icon: resolveTheme(prefs.theme) === "dark" ? <I.Sun size={16} /> : <I.Moon size={16} />, run: () => setTheme(resolveTheme(prefs.theme) === "dark" ? "light" : "dark") });
    out.push({ id: "rail", group: "Do", title: "Show or hide the sidebar", icon: <I.PanelLeft size={16} />, keys: k("toggle-rail"), run: () => setPrefs({ railCollapsed: !prefs.railCollapsed }) });
    if (project) {
      out.push({ id: "save", group: "Do", title: "Save now", icon: <I.Check size={16} />, keys: k("save"), run: () => { void flush().then(() => say("Saved.")); } });
      out.push({ id: "undo", group: "Do", title: "Undo", icon: <I.Undo size={16} />, keys: k("undo"), run: undo });
      out.push({ id: "redo", group: "Do", title: "Redo", icon: <I.Redo size={16} />, keys: k("redo"), run: redo });
      out.push({ id: "guide", group: "Do", title: "Show or hide the guide panel", icon: <I.PanelRight size={16} />, keys: k("toggle-guide"), run: () => setPrefs({ guideOpen: !prefs.guideOpen }) });
      if (!project.script.trim()) out.push({ id: "sample", group: "Do", title: "Use the sample script", icon: <I.Script size={16} />, run: () => edit((p) => ({ ...p, script: SAMPLE })) });
      g?.steps.forEach((s, i) => out.push({ id: `step-${s.id}`, group: "Go to", title: `${s.label}`, sub: s.status === "blocked" ? "locked" : s.summary, icon: <I.Layers size={16} />, keys: k(`step-${i + 1}` as ShortcutId), run: () => goStep(i), suggested: s.status === "current" }));
    }
    out.push({ id: "home", group: "Go to", title: "Projects (Home)", icon: <I.Home size={16} />, keys: k("open-project"), run: () => void goHome() });
    out.push({ id: "voices", group: "Go to", title: "Voice library", icon: <I.Voices size={16} />, keys: k("voices"), run: () => openAux("library") });
    out.push({ id: "engines", group: "Go to", title: "Voice engines (local and custom)", icon: <I.Layers size={16} />, run: () => openAux("providers") });
    out.push({ id: "settings", group: "Go to", title: "Settings", icon: <I.Gear size={16} />, keys: k("settings"), run: () => openAux("settings") });
    out.push({ id: "help", group: "Go to", title: "Help and guide", icon: <I.Book size={16} />, keys: k("help"), run: () => openHelp() });
    out.push({ id: "shortcuts", group: "Go to", title: "Keyboard shortcuts", icon: <I.Keyboard size={16} />, keys: k("shortcuts"), run: () => setShortcutsOpen(true) });
    projectsForPalette.filter((s) => s.id !== project?.id).forEach((s) => out.push({ id: `proj-${s.id}`, group: "Projects", title: s.name, sub: `${s.lines} lines`, icon: <I.Folder size={16} />, run: () => void openProject(s.id) }));
    analysis?.lines.slice(0, 300).forEach((l) => out.push({ id: `line-${l.line.key}`, group: "Script lines", title: `${l.line.speaker}: ${l.line.text}`, sub: l.line.key, icon: <I.Script size={16} />, run: () => { setFocusKey(l.line.key); goStep(2); setTimeout(() => setFocusKey(null), 2500); } }));
    voices.forEach((v) => out.push({ id: `voice-${v.id}`, group: "Voices", title: v.name, sub: `${v.provider_label} · ${v.accent}`, icon: <I.Voices size={16} />, run: () => openAux("library") }));
    ARTICLES.forEach((a) => out.push({ id: `help-${a.id}`, group: "Help", title: a.title, sub: a.summary, icon: <I.Book size={16} />, run: () => openHelp(a.id) }));
    GLOSSARY.forEach((x) => out.push({ id: `gloss-${x.term}`, group: "Help", title: `What is ${x.term}?`, sub: x.meaning, icon: <I.Info size={16} />, run: () => openHelp("glossary") }));
    return out;
  }, [project, g, analysis, voices, projectsForPalette, bindings, prefs.mode, prefs.theme, prefs.railCollapsed, prefs.guideOpen, doNext, newProject, startTour, quickStart, setMode, setTheme, setPrefs, flush, say, undo, redo, edit, goStep, goHome, openAux, openHelp, openProject]);

  /* ---------------------------------------------------------------- render */
  const ctx: Ctx | null = project && g ? {
    project, analysis, voices, dir, mode: prefs.mode, ffmpegPath: prefs.ffmpegPath, setFfmpegPath: (p) => setPrefs({ ffmpegPath: p }), edit, toast: say, goto: (n) => goStep(n),
    openLibrary: () => openAux("library"), openHelp, render, confirmSend, renderBatch, batch, errors, busy,
    steps: g.steps, next: g.next, mixPreview, mixBuilding, buildMix, markExported: () => setExported(true), focusKey,
  } : null;

  const showGuide = view === "project" && !!g;
  const stepDef = g?.steps[step];
  const hereLabel = view === "home" ? "Projects" : view === "library" ? "Voice library" : view === "providers" ? "Voice engines" : view === "settings" ? "Settings" : view === "help" ? "Help and guide" : (stepDef?.label ?? "");
  const nextBlocked = g ? g.steps[Math.min(5, step + 1)]?.status === "blocked" : true;

  return (
    <PlayerProvider>
      <PreviewKey enabled={view === "project"} combo={keyOf("preview")} />
      <a className="skip" href="#stage">Skip to content</a>
      <div className="shell" data-rail={railCollapsed ? "collapsed" : "expanded"} data-guide={showGuide && prefs.guideOpen ? "open" : "closed"} style={{ ["--guide-w" as string]: `${prefs.guideWidth}px` }}>
        <Rail view={view} projectName={project?.name ?? null} steps={project && g ? g.steps : null} step={step} collapsed={railCollapsed}
          onHome={() => void goHome()} onStep={goStep} onVoices={() => openAux("library")} onEngines={() => openAux("providers")} onHelp={() => openHelp()} onShortcuts={() => setShortcutsOpen(true)}
          onSettings={() => openAux("settings")} onToggle={() => setPrefs({ railCollapsed: !prefs.railCollapsed })} shortcutHint={(id) => keyOf(id as ShortcutId)} />
        <Toolbar hereLabel={hereLabel} project={view === "project" ? project : null} saveState={saved} canUndo={history.current.length > 0} canRedo={future.current.length > 0}
          onUndo={undo} onRedo={redo} mode={prefs.mode} onMode={setMode} onPalette={actions.palette} paletteKey={keyOf("palette")}
          guideOpen={prefs.guideOpen} showGuide={showGuide} onToggleGuide={() => setPrefs({ guideOpen: !prefs.guideOpen })} onToggleRail={() => setPrefs({ railCollapsed: !prefs.railCollapsed })}
          railCollapsed={railCollapsed} onEdit={edit} guideKey={keyOf("toggle-guide")} />

        <main className="stage" id="stage" tabIndex={-1}>
          {view === "home" && <Home onOpen={(id) => void openProject(id)} toast={say} onQuickStart={() => void quickStart()} onHelp={() => openHelp("quick-start")} />}
          {view === "library" && <VoiceLibrary onChanged={() => { void api.voices().then(setVoices).catch((e) => say(messageOf(e), true)); }} voices={voices} toast={say} onBack={back} inProject={returnTo === "project" && !!project} onSettings={() => openAux("settings")} onEngines={() => openAux("providers")} />}
          {view === "providers" && <Providers toast={say} onBack={back} inProject={returnTo === "project" && !!project} onChanged={() => { void api.voices().then(setVoices).catch((e) => say(messageOf(e), true)); }} />}
          {view === "settings" && <Settings info={info} toast={say} onBack={back} inProject={returnTo === "project" && !!project} prefs={prefs} onPrefs={setPrefs} onTheme={setTheme}
            onKeysChanged={() => { void api.voices().then(setVoices).catch((e) => say(messageOf(e), true)); }} onShortcuts={() => setShortcutsOpen(true)} onTour={() => void startTour()} onEngines={() => openAux("providers")} />}
          {view === "help" && <HelpCenter articleId={helpId} onSelect={setHelpId} onBack={back} onShortcuts={() => setShortcutsOpen(true)} onTour={() => void startTour()} />}
          {view === "project" && ctx && stepDef && (
            <div className={`page ${step === 3 || step === 4 ? "wide" : ""}`}>
              <div className="stage-title">
                <h1>{stepDef.label}</h1>
                <span className="sub">{stepDef.summary}</span>
                <span className="spacer" />
                <button className="ghost small" onClick={() => openHelp(stepDef.helpId)}><I.Book size={14} /> About this step</button>
              </div>
              {step === 0 && <ScriptStep ctx={ctx} />}
              {step === 1 && <CastStep ctx={ctx} />}
              {step === 2 && <DirectStep ctx={ctx} />}
              {step === 3 && <RenderStep ctx={ctx} />}
              {step === 4 && <MixStep ctx={ctx} />}
              {step === 5 && <ExportStep ctx={ctx} />}
              <div className="stage-nav">
                <button disabled={step === 0} onClick={() => goStep(step - 1)}><I.ChevronLeft size={16} /> <span className="sr-only">Previous step: </span><span>{g?.steps[step - 1]?.label ?? "Back"}</span></button>
                <span className="small faint">{step < 5 && nextBlocked ? g?.steps[step + 1]?.blockedReason : ""}</span>
                <button disabled={step === 5 || nextBlocked} onClick={() => goStep(step + 1)}><span className="sr-only">Next step: </span><span>{g?.steps[step + 1]?.label ?? "Next"}</span> <I.ChevronRight size={16} /></button>
              </div>
            </div>
          )}
        </main>

        {showGuide && prefs.guideOpen && g && (
          <GuidePanel steps={g.steps} next={g.next} current={currentIdx} viewing={step} busy={!!batch || mixBuilding} nextKey={keyOf("do-next")}
            onGoto={goStep} onNext={doNext} onHelp={openHelp} width={prefs.guideWidth} onResize={(w) => setPrefs({ guideWidth: Math.max(260, Math.min(520, Math.round(w))) })} />
        )}
      </div>

      <div className="sr-only" aria-live="polite">{announce}</div>
      {palette && <CommandPalette items={paletteItems} onClose={() => setPalette(false)} />}
      {shortcutsOpen && <ShortcutsDialog overrides={prefs.shortcuts} onChange={(o) => setPrefs({ shortcuts: o })} onClose={() => setShortcutsOpen(false)} />}
      {tour && <Tour onClose={(done) => { setTour(false); if (done) setPrefs({ toured: true }); }} />}
      {!splash && !prefs.welcomed && view === "home" && (
        <Welcome onTour={() => { setPrefs({ welcomed: true }); void quickStart(); }} onExplore={() => setPrefs({ welcomed: true })} />
      )}
      {splash && <Splash ready={voicesReady} onDone={() => setSplash(false)} />}
      <Toast toast={toast} onClose={() => setToast(null)} />
      {sendPrompt && (
        <Confirm title={`Send text to ${sendPrompt.provider}?`} confirmLabel="Send and render" danger={false}
          body={<p>{sendPrompt.lines} line{sendPrompt.lines === 1 ? "" : "s"} ({sendPrompt.chars} characters of dialogue) will be sent over the internet to {sendPrompt.provider} to be spoken. It is billed to your account there, and Soundstage cannot show the cost in advance. Their copy follows their own retention policy.</p>}
          onCancel={() => { sendPrompt.resolve(false); setSendPrompt(null); }}
          onConfirm={() => { sendPrompt.resolve(true); setSendPrompt(null); }} />
      )}
    </PlayerProvider>
  );
}
