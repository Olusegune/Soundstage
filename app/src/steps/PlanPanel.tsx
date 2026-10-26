import { useCallback, useEffect, useMemo, useState } from "react";
import { api, messageOf } from "../api";
import type { Ctx } from "../ctx";
import { Confirm, Help } from "../ui";
import type { DirectorEstimate, DirectorProposal, LineChange, LinePlan, PerformancePlan } from "../types";
import { EMOTIONS } from "../types";

const FIELD_LABEL: Record<string, string> = {
  objective: "goal", subtext: "subtext", relationship: "relationship", emotion: "emotion", pace: "pace", pauses: "pauses",
  emphasis: "emphasis", events: "sounds", languages: "languages", alternatives: "other readings",
};

function hostOf(url: string): string {
  try { return new URL(url).hostname; } catch { return "the chat server"; }
}
const isLocal = (host: string) => host === "localhost" || host === "127.0.0.1" || host === "[::1]" || host === "::1";

const emptyPlan = (): PerformancePlan => ({ schema: 1, source_hash: "", created_by: { kind: "", at: "" }, scenes: {}, lines: {} });

/** The Director: suggests how each line should be played. It only proposes; the person accepts, edits and locks. */
export function PlanPanel({ ctx }: { ctx: Ctx }) {
  const { project, analysis } = ctx;
  const [plan, setPlan] = useState<PerformancePlan>(emptyPlan());
  const [proposal, setProposal] = useState<DirectorProposal | null>(null);
  const [changes, setChanges] = useState<LineChange[]>([]);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [ask, setAsk] = useState<DirectorEstimate | null>(null);
  const [busy, setBusy] = useState(false);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    let live = true;
    api.planLoad(project.id).then((p) => { if (live) setPlan(p); }).catch(() => { if (live) setPlan(emptyPlan()); });
    return () => { live = false; };
  }, [project.id]);

  const host = hostOf(ctx.assistantUrl);
  const local = isLocal(host);
  const haveModel = ctx.assistantModel.trim().length > 0;
  const planned = Object.keys(plan.lines).length;
  const stale = useMemo(() => planned > 0 && plan.lines && analysis !== null && Object.keys(plan.lines).some((id) => !analysis.lines.some((l) => l.line.key === id)), [plan, planned, analysis]);

  const save = useCallback(async (next: PerformancePlan) => {
    await api.planSave(project, next);
    setPlan(next);
  }, [project]);

  const receive = useCallback(async (p: DirectorProposal) => {
    const ch = await api.planChanges(plan, p.plan);
    setProposal(p);
    setChanges(ch);
    setPicked(new Set(ch.map((c) => c.line_id)));
    if (ch.length === 0) ctx.toast("The Director has nothing new to suggest.");
  }, [plan, ctx]);

  const run = async (useModel: boolean) => {
    setBusy(true);
    setAsk(null);
    try { await receive(await api.directorRun(project, ctx.assistantUrl, ctx.assistantModel, useModel, useModel)); }
    catch (e) { ctx.toast(messageOf(e), true); }
    finally { setBusy(false); }
  };

  const begin = async () => {
    try { setAsk(await api.directorEstimate(project)); } catch (e) { ctx.toast(messageOf(e), true); }
  };

  const accept = async () => {
    if (!proposal) return;
    try {
      const merged = await api.planMerge(plan, proposal.plan, [...picked]);
      await save(merged);
      ctx.toast(`Added the Director's suggestions for ${picked.size} line${picked.size === 1 ? "" : "s"}.`);
      setProposal(null); setChanges([]);
    } catch (e) { ctx.toast(messageOf(e), true); }
  };

  const edit = async (id: string, patch: Partial<LinePlan>) => {
    try {
      const next = await api.planEdit(project, plan, id, { ...plan.lines[id], ...patch } as LinePlan);
      await save(next);
    } catch (e) { ctx.toast(messageOf(e), true); }
  };

  const toDirections = async () => {
    try {
      const [dirs, n] = await api.planDirections(project, plan);
      if (n === 0) { ctx.toast("Nothing to carry over. Lines you already directed are left alone."); return; }
      ctx.edit((p) => ({ ...p, line_directions: dirs }));
      ctx.toast(`Carried the plan into the delivery of ${n} line${n === 1 ? "" : "s"}.`);
    } catch (e) { ctx.toast(messageOf(e), true); }
  };

  const lineInfo = (id: string) => analysis?.lines.find((l) => l.line.key === id);

  return (
    <section className="card" aria-labelledby="director-h">
      <div className="row" style={{ alignItems: "flex-start" }}>
        <div style={{ flex: 1, minWidth: 260 }}>
          <h2 id="director-h" style={{ margin: 0 }}>Director <Help text="The Director reads your script and suggests how each line should be played: what the character wants, what they hold back, and where to pause. It only suggests. You accept, change or ignore each suggestion, and what you change is never overwritten." /></h2>
          <p className="small muted" style={{ margin: "4px 0 0" }}>
            {planned === 0 ? "No plan yet. The Director can suggest one for the whole script." : `${planned} line${planned === 1 ? " has" : "s have"} a plan${plan.created_by.model ? `, last suggested by ${plan.created_by.model === "rules" ? "simple rules" : plan.created_by.model}` : ""}.`}
            {stale ? " Some planned lines are no longer in the script." : ""}
          </p>
        </div>
        <div className="row tight">
          <button className="primary" disabled={busy || !haveModel} onClick={() => void begin()} data-tip={haveModel ? "Ask the chat server from Settings" : "Choose a chat model in Settings first"}>
            {busy ? <><span className="spin" aria-hidden="true" /> Thinking</> : "Suggest a plan"}
          </button>
          <button disabled={busy} onClick={() => void run(false)} data-tip="Uses simple rules on the text. Nothing is sent anywhere.">Simple rules only</button>
        </div>
      </div>
      {!haveModel && <p className="small muted" style={{ margin: "8px 0 0" }}>To get the full Director, set a chat model under Settings, AI prompt helper (Ollama on this computer works and sends nothing).</p>}

      {proposal && (
        <div className="callout" style={{ marginTop: 12 }} role="region" aria-label="Director suggestions">
          <strong>{changes.length === 0 ? "Nothing new" : `${changes.length} line${changes.length === 1 ? "" : "s"} with suggestions`}</strong>
          {proposal.notes.map((n, i) => <div key={i} className="small">{n}</div>)}
          <ul className="small" style={{ listStyle: "none", padding: 0, margin: "8px 0" }}>
            {changes.map((c) => {
              const li = lineInfo(c.line_id);
              const locked = plan.lines[c.line_id]?.user_locked ?? [];
              return (
                <li key={c.line_id} style={{ marginBottom: 4 }}>
                  <label className="inline">
                    <input type="checkbox" checked={picked.has(c.line_id)} onChange={(e) => setPicked((s) => { const n = new Set(s); e.target.checked ? n.add(c.line_id) : n.delete(c.line_id); return n; })} />
                    <span><strong>{li?.line.speaker ?? "?"}</strong>: {(li?.line.text ?? "").slice(0, 70)} <span className="muted">changes {c.fields.map((f) => FIELD_LABEL[f] ?? f).join(", ")}{c.fields.some((f) => locked.includes(f)) ? " (your edits are kept)" : ""}</span></span>
                  </label>
                </li>
              );
            })}
          </ul>
          <div className="row">
            <button className="primary small" disabled={picked.size === 0} onClick={() => void accept()}>Accept {picked.size} selected</button>
            <button className="small" onClick={() => { setProposal(null); setChanges([]); }}>Not now</button>
          </div>
        </div>
      )}

      {planned > 0 && (
        <div style={{ marginTop: 12 }}>
          <div className="row">
            <button className="small" aria-expanded={open} onClick={() => setOpen(!open)}>{open ? "Hide the plan" : "Show the plan"}</button>
            <button className="small" onClick={() => void toDirections()} data-tip="Fills the delivery of lines you have not directed yourself">Use the plan for delivery</button>
          </div>
          {open && (
            <div className="col" style={{ marginTop: 8 }}>
              {Object.entries(plan.lines).map(([id, lp]) => {
                const li = lineInfo(id);
                if (!li) return null;
                const top = [...(lp.emotion ?? [])].sort((a, b) => b.intensity - a.intensity);
                return (
                  <div key={id} className="plan-line" style={{ borderTop: "1px solid var(--line)", paddingTop: 8 }}>
                    <div className="small"><strong>{li.line.speaker}</strong>: {li.line.text}</div>
                    <div className="row tight small" style={{ alignItems: "center" }}>
                      <label htmlFor={`pe-${id}`} className="small">Feeling</label>
                      <select id={`pe-${id}`} value={top[0]?.surface === false ? "" : (top[0]?.name ?? "")} onChange={(e) => void edit(id, { emotion: e.target.value ? [{ name: e.target.value, intensity: top[0]?.intensity ?? 0.6, surface: true }] : [] })}>
                        <option value="">Not set</option>
                        {EMOTIONS.map((x) => <option key={x} value={x}>{x}</option>)}
                        {top[0] && !(EMOTIONS as readonly string[]).includes(top[0].name) && top[0].surface && <option value={top[0].name}>{top[0].name}</option>}
                      </select>
                      <label htmlFor={`pp-${id}`} className="small">Pace {(lp.pace ?? 1).toFixed(2)}×</label>
                      <input id={`pp-${id}`} type="range" min={0.5} max={2} step={0.05} value={lp.pace ?? 1} onChange={(e) => void edit(id, { pace: Number(e.target.value) })} />
                      {lp.confidence != null && <span className="chip" data-tip="How sure the Director is">{Math.round(lp.confidence * 100)}% sure</span>}
                      {lp.user_locked.length > 0 && <span className="chip info" data-tip="You changed these, so the Director will not overwrite them">Yours: {lp.user_locked.map((f) => FIELD_LABEL[f] ?? f).join(", ")}</span>}
                    </div>
                    {(lp.objective || lp.subtext) && <div className="small muted">{lp.objective ? `Wants: ${lp.objective}. ` : ""}{lp.subtext ? `Underneath: ${lp.subtext}` : ""}</div>}
                    {top.filter((e) => !e.surface).length > 0 && <div className="small muted">Holding back: {top.filter((e) => !e.surface).map((e) => e.name).join(", ")}</div>}
                    {lp.uncertainty_note && <div className="small muted">{lp.uncertainty_note}</div>}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}

      {ask && (
        <Confirm title="Send the script to the Director?" confirmLabel="Send and suggest" danger={false}
          body={<>
            <p>{local ? `The ${ask.scenes} scene${ask.scenes === 1 ? "" : "s"} (${ask.lines} lines, about ${ask.chars.toLocaleString()} characters) will go to ${host}, which is on this computer, so nothing leaves it.` : `The ${ask.scenes} scene${ask.scenes === 1 ? "" : "s"} (${ask.lines} lines, about ${ask.chars.toLocaleString()} characters) will be sent over the internet to ${host}. Their service follows its own privacy and billing terms.`}</p>
            <p className="small">That is {ask.calls} request{ask.calls === 1 ? "" : "s"}, roughly {ask.approx_input_tokens.toLocaleString()} tokens in and up to {ask.max_output_tokens.toLocaleString()} out. {local ? "It is free." : "You may be billed for these tokens; Soundstage cannot see the price."} The model name is {ctx.assistantModel}.</p>
            <p className="small">The Director only suggests. It cannot render, spend, export or change settings.</p>
          </>}
          onCancel={() => setAsk(null)} onConfirm={() => void run(true)} />
      )}
    </section>
  );
}
