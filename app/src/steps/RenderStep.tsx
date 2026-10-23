import { Fragment, useState } from "react";
import { joinPath } from "../api";
import type { Ctx } from "../ctx";
import { fmtMs } from "../logic";
import { Empty, PlayButton } from "../ui";
import type { LineInfo, LineStatus, TakeStatus } from "../types";

const STATUS: Record<LineStatus, { label: string; cls: string }> = {
  drafted: { label: "Not rendered", cls: "" },
  rendered: { label: "✓ Rendered", cls: "ok" },
  stale: { label: "↻ Out of date", cls: "warn" },
  skipped: { label: "Skipped", cls: "info" },
};

export function RenderStep({ ctx }: { ctx: Ctx }) {
  const { project, analysis, edit } = ctx;
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [open, setOpen] = useState<string | null>(null);

  if (!analysis || analysis.lines.length === 0) {
    return <div className="page"><Empty title="Nothing to render yet"><button onClick={() => ctx.goto(0)}>Go to Script</button></Empty></div>;
  }

  const ready = (l: LineInfo) => l.issues.length === 0 && (l.status === "drafted" || l.status === "stale");
  const todo = analysis.lines.filter(ready);
  const blocked = analysis.lines.filter((l) => l.issues.length > 0 && l.status !== "skipped");
  const rendered = analysis.lines.filter((l) => l.status === "rendered").length;

  const run = async (keys: string[]) => {
    if (keys.length === 0) return;
    setProgress({ done: 0, total: keys.length });
    setErrors((e) => { const n = { ...e }; keys.forEach((k) => delete n[k]); return n; });
    // One line at a time keeps the progress bar honest and lets the UI stay responsive.
    let done = 0;
    for (const k of keys) {
      const out = await ctx.render([k]);
      done += 1;
      setProgress({ done, total: keys.length });
      const bad = out?.results.find((r) => !r.ok);
      if (bad) setErrors((e) => ({ ...e, [k]: bad.message ?? "Rendering failed." }));
      else if (out === null) setErrors((e) => ({ ...e, [k]: "Rendering failed. Try again." }));
    }
    setProgress(null);
  };

  const setSkip = (key: string, on: boolean) =>
    edit((p) => ({ ...p, skipped: on ? [...new Set([...p.skipped, key])] : p.skipped.filter((k) => k !== key) }));

  const setTakeStatus = (id: string, status: TakeStatus) =>
    edit((p) => ({ ...p, takes: p.takes.map((t) => (t.id === id ? { ...t, status, approver: status === "pending" ? null : "owner" } : t)) }));

  const noteFor = (target: string) => project.notes.find((n) => n.target === target)?.text ?? "";
  const setNote = (target: string, text: string) =>
    edit((p) => {
      const others = p.notes.filter((n) => n.target !== target);
      return { ...p, notes: text.trim() ? [...others, { id: `note-${target}`, target, text, created: new Date().toISOString() }] : others };
    }, `note-${target}`);

  const providers = [...new Set(analysis.lines.map((l) => l.voice_id).filter((v): v is string => !!v).map((id) => ctx.voices.find((v) => v.id === id)?.provider_label ?? "Unknown"))];

  return (
    <div className="page">
      <div className="card">
        <div className="row">
          <h2 style={{ margin: 0 }}>Render</h2>
          <span className="muted">{rendered} of {analysis.lines.length} lines ready</span>
          <span className="spacer" />
          <button className="primary" disabled={todo.length === 0 || progress !== null} onClick={() => void run(todo.map((l) => l.line.key))}>
            {progress ? `Rendering ${progress.done}/${progress.total}…` : `Render ${todo.length} line${todo.length === 1 ? "" : "s"}`}
          </button>
        </div>
        {progress && <div className="bar" style={{ marginTop: 10 }} role="progressbar" aria-valuemin={0} aria-valuemax={progress.total} aria-valuenow={progress.done}><div style={{ width: `${(100 * progress.done) / progress.total}%` }} /></div>}
        <p className="small muted" style={{ marginTop: 10, marginBottom: 0 }}>
          Voices in use: {providers.join(", ") || "none yet"}. These voices run on this computer, so no script text or audio leaves it and nothing is charged. Lines that have not changed are reused, not rendered again.
        </p>
      </div>
      {blocked.length > 0 && <div className="callout warn"><strong>{blocked.length} line{blocked.length === 1 ? " needs" : "s need"} attention before rendering</strong>Open Direct to see what to fix.<div className="row" style={{ marginTop: 6 }}><button className="small" onClick={() => ctx.goto(2)}>Go to Direct</button></div></div>}
      <div className="card">
        <table>
          <thead><tr><th>Line</th><th>Text</th><th>Status</th><th style={{ width: 330 }}>Actions</th></tr></thead>
          <tbody>
            {analysis.lines.map((l) => {
              const key = l.line.key;
              const take = project.takes.find((t) => t.id === l.selected_take);
              const takes = project.takes.filter((t) => t.line_key === key);
              const st = STATUS[l.status];
              const err = errors[key];
              return (
                <Fragment key={key}>
                  <tr>
                    <td><span className="speaker">{l.line.speaker}</span><br /><span className="muted small mono">{key}</span></td>
                    <td className="mono small" style={{ maxWidth: 420 }}>{l.line.text}{err && <div className="callout bad small" style={{ marginTop: 4 }} role="alert"><strong>Could not render</strong>{err}</div>}{l.issues.length > 0 && l.status !== "skipped" && <div className="small" style={{ color: "var(--warning)" }}>⚠ {l.issues[0]?.message}</div>}</td>
                    <td><span className={`chip ${st.cls}`}>{st.label}</span>{take && <div className="small muted">{fmtMs(take.duration_ms)}</div>}</td>
                    <td>
                      <div className="row">
                        <button className="small" disabled={ctx.busy.has(key) || l.issues.length > 0 || l.status === "skipped"} onClick={() => void run([key])}>{ctx.busy.has(key) ? "…" : l.status === "drafted" ? "Render" : "Render again"}</button>
                        <PlayButton path={take ? joinPath(ctx.dir, take.file) : null} label={`${l.line.speaker} ${key}`} bust={take?.id} />
                        <button className="small ghost" onClick={() => setSkip(key, l.status !== "skipped")}>{l.status === "skipped" ? "Include" : "Skip"}</button>
                        {takes.length > 0 && <button className="small ghost" aria-expanded={open === key} onClick={() => setOpen(open === key ? null : key)}>Takes ({takes.length})</button>}
                      </div>
                    </td>
                  </tr>
                  {open === key && (
                    <tr key={`${key}-takes`}><td colSpan={4}>
                      <div className="col">
                        {takes.slice().reverse().map((t) => {
                          const sel = l.selected_take === t.id;
                          return (
                            <div key={t.id} className="linecard" style={{ margin: 0 }}>
                              <div className="row">
                                <label className="inline"><input type="radio" name={`take-${key}`} checked={sel} onChange={() => edit((p) => ({ ...p, selected: { ...p.selected, [key]: t.id } }))} /> Use this take</label>
                                <PlayButton path={joinPath(ctx.dir, t.file)} label={`take ${t.id}`} bust={t.id} />
                                <span className="muted small">{fmtMs(t.duration_ms)} · {t.direction_summary} · {t.provider_version}</span>
                                <span className="spacer" />
                                <span className={`chip ${t.status === "approved" ? "ok" : t.status === "rejected" ? "bad" : t.status === "held" ? "warn" : ""}`}>{t.status === "pending" ? "Not reviewed" : t.status}</span>
                                <button className="small" onClick={() => setTakeStatus(t.id, "approved")}>Approve</button>
                                <button className="small" onClick={() => setTakeStatus(t.id, "held")}>Hold</button>
                                <button className="small danger" onClick={() => setTakeStatus(t.id, "rejected")}>Reject</button>
                              </div>
                              <label htmlFor={`tn-${t.id}`} style={{ marginTop: 6 }}>Note on this take</label>
                              <input id={`tn-${t.id}`} type="text" style={{ width: "100%" }} maxLength={300} value={noteFor(`take:${t.id}`)} onChange={(e) => setNote(`take:${t.id}`, e.target.value)} />
                              {t.status === "rejected" && sel && <p className="small" style={{ color: "var(--warning)", marginBottom: 0 }}>This take is rejected, so it is left out of the mix and export. Pick another take or render again.</p>}
                            </div>
                          );
                        })}
                      </div>
                    </td></tr>
                  )}
                </Fragment>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
