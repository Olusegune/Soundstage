import { useCallback, useEffect, useRef, useState } from "react";
import { api, messageOf } from "./api";
import type { Ctx } from "./ctx";
import { canEnter, firstBlockedReason, stepComplete } from "./logic";
import { Home } from "./screens/Home";
import { Settings } from "./screens/Settings";
import { VoiceLibrary } from "./screens/VoiceLibrary";
import { CastStep } from "./steps/CastStep";
import { DirectStep } from "./steps/DirectStep";
import { ExportStep } from "./steps/ExportStep";
import { MixStep } from "./steps/MixStep";
import { RenderStep } from "./steps/RenderStep";
import { ScriptStep } from "./steps/ScriptStep";
import { Confirm, Logo, PlayerProvider, Toast, type ToastMsg } from "./ui";
import { STEPS, type Analysis, type Project, type Voice } from "./types";

type View = "home" | "project" | "library" | "settings";
const HISTORY_LIMIT = 30;

export default function App() {
  const [view, setView] = useState<View>("home");
  const [returnTo, setReturnTo] = useState<View>("home");
  const [project, setProject] = useState<Project | null>(null);
  const [dir, setDir] = useState("");
  const [step, setStep] = useState(0);
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [voices, setVoices] = useState<Voice[]>([]);
  const [info, setInfo] = useState<{ version: string; projects_root: string } | null>(null);
  const [toast, setToast] = useState<ToastMsg | null>(null);
  const [busy, setBusy] = useState<Set<string>>(new Set());
  const [saved, setSaved] = useState<"saved" | "saving" | "error">("saved");
  const [sendPrompt, setSendPrompt] = useState<{ provider: string; lines: number; chars: number; resolve: (ok: boolean) => void } | null>(null);

  const projectRef = useRef<Project | null>(null);
  const history = useRef<string[]>([]);
  const lastCoalesce = useRef<{ key: string; at: number } | null>(null);
  const saveTimer = useRef<number | undefined>(undefined);

  const say = useCallback((text: string, bad?: boolean) => setToast({ text, bad }), []);

  useEffect(() => {
    void api.info().then(setInfo).catch(() => undefined);
    void api.voices().then(setVoices).catch((e) => say(messageOf(e), true));
  }, [say]);

  const analysisRef = useRef<Analysis | null>(null);
  const voicesRef = useRef<Voice[]>([]);
  useEffect(() => { projectRef.current = project; }, [project]);
  useEffect(() => { analysisRef.current = analysis; }, [analysis]);
  useEffect(() => { voicesRef.current = voices; }, [voices]);

  // Re-check the script, voices and render status whenever the project changes.
  useEffect(() => {
    if (!project) return;
    let live = true;
    const t = window.setTimeout(() => { void api.analyze(project).then((a) => live && setAnalysis(a)).catch(() => undefined); }, 200);
    return () => { live = false; window.clearTimeout(t); };
  }, [project, voices]);

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
    lastCoalesce.current = coalesce ? { key: coalesce, at: now } : null;
    const next = fn(cur);
    projectRef.current = next;
    setProject(next);
    scheduleSave();
  }, [scheduleSave]);

  const undo = useCallback(() => {
    const snap = history.current.pop();
    const cur = projectRef.current;
    if (!snap || !cur) return;
    const old = JSON.parse(snap) as Project;
    // Undo reverts authoring changes but never discards rendered takes or imported audio.
    const ids = new Set(cur.takes.map((t) => t.id));
    const next: Project = { ...old, takes: cur.takes, assets: cur.assets, selected: Object.fromEntries(Object.entries(old.selected).filter(([, id]) => ids.has(id))) };
    projectRef.current = next;
    setProject(next);
    scheduleSave();
    say("Undone.");
  }, [scheduleSave, say]);

  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.shiftKey && e.key.toLowerCase() === "z" && view === "project") {
        const tag = (document.activeElement?.tagName ?? "").toLowerCase();
        if (tag === "input" || tag === "textarea") return; // let text fields use their own undo
        e.preventDefault();
        undo();
      }
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [undo, view]);

  useEffect(() => {
    const h = () => { void flush(); };
    window.addEventListener("beforeunload", h);
    return () => window.removeEventListener("beforeunload", h);
  }, [flush]);

  const openProject = async (id: string) => {
    try {
      const p = await api.loadProject(id);
      setDir(await api.projectDir(id));
      history.current = [];
      projectRef.current = p;
      setProject(p);
      setAnalysis(null);
      setStep(0);
      setSaved("saved");
      setView("project");
    } catch (e) { say(messageOf(e), true); }
  };

  const goHome = async () => { await flush(); setProject(null); projectRef.current = null; setAnalysis(null); setView("home"); };

  /** Ask once before any text is sent over the network. Local voices resolve at once. */
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

  const openAux = (v: "library" | "settings") => { setReturnTo(view === "library" || view === "settings" ? returnTo : view); setView(v); };
  const back = () => setView(returnTo === "project" && project ? "project" : "home");

  const ctx: Ctx | null = project ? {
    project, analysis, voices, dir, edit, toast: say, render, confirmSend, busy,
    goto: (n) => setStep(n),
    openLibrary: () => openAux("library"),
  } : null;

  return (
    <PlayerProvider>
      <div className="app">
        <a className="skip" href="#main">Skip to content</a>
        <header className="topbar">
          <button className="brand" onClick={() => void goHome()} aria-label="Soundstage home"><Logo /> Soundstage</button>
          {project && view === "project" && (
            <>
              <input aria-label="Project name" type="text" value={project.name} maxLength={80} style={{ width: 240, fontWeight: 600 }} onChange={(e) => edit((p) => ({ ...p, name: e.target.value }), "name")} />
              <label className="inline small" style={{ margin: 0 }} title="Commercial projects can only use cleared voices, music and effects">
                Target
                <select aria-label="Licence target" value={project.licence_target} onChange={(e) => edit((p) => ({ ...p, licence_target: e.target.value as Project["licence_target"] }))}>
                  <option value="personal">Personal</option><option value="commercial">Commercial</option>
                </select>
              </label>
              <label className="inline small" style={{ margin: 0 }}>
                Rating
                <select aria-label="Content rating" value={project.rating} onChange={(e) => edit((p) => ({ ...p, rating: e.target.value as Project["rating"] }))}>
                  <option value="unrated">Unrated</option><option value="all_ages">All ages</option><option value="teen">Teen</option><option value="adult">Adult</option>
                </select>
              </label>
              <span className="small muted" aria-live="polite">{saved === "saved" ? "Saved" : saved === "saving" ? "Saving…" : "Not saved"}</span>
            </>
          )}
          <span className="spacer" />
          <button className="ghost" onClick={() => openAux("library")}>Voices</button>
          <button className="ghost" onClick={() => openAux("settings")}>Settings</button>
        </header>

        {view === "project" && project && ctx && (
          <nav className="stepbar" aria-label="Steps">
            {STEPS.map((name, i) => {
              const ok = canEnter(i, project, analysis);
              const done = stepComplete(name, project, analysis) && i < step;
              return (
                <button key={name} className={`step ${i === step ? "current" : ""} ${done ? "done" : ""}`} disabled={!ok} aria-current={i === step ? "step" : undefined}
                  title={ok ? undefined : firstBlockedReason(i, project, analysis) ?? undefined} onClick={() => setStep(i)}>
                  <span className="n"><span>{i + 1}</span></span>{name}
                </button>
              );
            })}
            <span className="spacer" />
            <button className="ghost small" onClick={undo} title="Ctrl+Z">Undo</button>
          </nav>
        )}

        <main className="main" id="main">
          {view === "home" && <Home onOpen={(id) => void openProject(id)} toast={say} />}
          {view === "library" && <VoiceLibrary voices={voices} toast={say} onBack={back} inProject={returnTo === "project" && !!project} />}
          {view === "settings" && <Settings info={info} toast={say} onKeysChanged={() => { void api.voices().then(setVoices).catch((e) => say(messageOf(e), true)); }} onBack={back} inProject={returnTo === "project" && !!project} />}
          {view === "project" && ctx && (
            <>
              {step === 0 && <ScriptStep ctx={ctx} />}
              {step === 1 && <CastStep ctx={ctx} />}
              {step === 2 && <DirectStep ctx={ctx} />}
              {step === 3 && <RenderStep ctx={ctx} />}
              {step === 4 && <MixStep ctx={ctx} />}
              {step === 5 && <ExportStep ctx={ctx} />}
              <div className="page row" style={{ justifyContent: "space-between", marginTop: 8 }}>
                <button disabled={step === 0} onClick={() => setStep(step - 1)}>← {STEPS[step - 1] ?? ""}</button>
                <button className="primary" disabled={step === STEPS.length - 1 || !canEnter(step + 1, ctx.project, analysis)}
                  title={firstBlockedReason(step + 1, ctx.project, analysis) ?? undefined} onClick={() => setStep(step + 1)}>{STEPS[step + 1] ?? ""} →</button>
              </div>
              {step < STEPS.length - 1 && !canEnter(step + 1, ctx.project, analysis) && <p className="page small muted" style={{ textAlign: "right" }}>{firstBlockedReason(step + 1, ctx.project, analysis)}</p>}
            </>
          )}
        </main>
        <Toast toast={toast} onClose={() => setToast(null)} />
        {sendPrompt && (
          <Confirm title={`Send text to ${sendPrompt.provider}?`} confirmLabel="Send and render"
            body={<p>{sendPrompt.lines} line{sendPrompt.lines === 1 ? "" : "s"} ({sendPrompt.chars} characters of dialogue) will be sent over the internet to {sendPrompt.provider} to be spoken. It is billed to your account there, and Soundstage cannot show the cost in advance. Their copy follows their own retention policy.</p>}
            onCancel={() => { sendPrompt.resolve(false); setSendPrompt(null); }}
            onConfirm={() => { sendPrompt.resolve(true); setSendPrompt(null); }} />
        )}
      </div>
    </PlayerProvider>
  );
}
