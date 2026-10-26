import { removeTagFromRow } from "../logic";
import { Fragment, useState } from "react";
import { joinPath } from "../api";
import type { Ctx } from "../ctx";
import * as I from "../icons";
import { fmtMs } from "../logic";
import { Empty, Help, PlayButton, Waveform } from "../ui";
import type { LineInfo, LineStatus, TakeStatus } from "../types";

const STATUS: Record<LineStatus, { label: string; cls: string; icon: React.ReactNode }> = {
  drafted: { label: "Not rendered", cls: "", icon: null },
  rendered: { label: "Rendered", cls: "ok", icon: <I.Check size={12} /> },
  stale: { label: "Out of date", cls: "warn", icon: <I.Clock size={12} /> },
  skipped: { label: "Skipped", cls: "info", icon: null },
};

export function RenderStep({ ctx }: { ctx: Ctx }) {
  const { project, analysis, edit } = ctx;
  const [open, setOpen] = useState<string | null>(null);
  const advanced = ctx.mode === "advanced";

  if (!analysis || analysis.lines.length === 0) {
    return <Empty title="Nothing to render yet"><p className="muted">Rendering turns each line into audio. Write and cast your script first.</p><button className="primary" onClick={() => ctx.goto(0)}>Go to Script</button></Empty>;
  }

  const ready = (l: LineInfo) => l.issues.length === 0 && (l.status === "drafted" || l.status === "stale");
  const todo = analysis.lines.filter(ready);
  const blocked = analysis.lines.filter((l) => l.issues.length > 0 && l.status !== "skipped");
  const rendered = analysis.lines.filter((l) => l.status === "rendered").length;
  const batch = ctx.batch;

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

  const used = [...new Set(analysis.lines.map((l) => l.voice_id).filter((v): v is string => !!v))].map((id) => ctx.voices.find((v) => v.id === id));
  const providers = [...new Set(used.map((v) => v?.provider_label ?? "Unknown"))];
  const cloud = [...new Set(used.filter((v) => v?.sends_data).map((v) => v?.provider_label ?? "a cloud provider"))];

  return (
    <div>
      <div className="card hero">
        <div className="row">
          <div style={{ flex: 1, minWidth: 260 }}>
            <h2 style={{ margin: 0 }}>{rendered} of {analysis.lines.length} lines ready</h2>
            <p className="small muted" style={{ margin: "4px 0 0" }}>
              Voices: {providers.join(", ") || "none yet"}.{" "}
              {cloud.length > 0 ? `Lines using ${cloud.join(" and ")} are sent over the internet and billed to your account there; you will be asked first.` : "These run on this computer, so nothing is sent and nothing is charged."}{" "}
              Lines that have not changed are reused. <Help text="Soundstage remembers audio it has already made. If a line's words, voice and direction are unchanged, it is reused instead of rendered again." />
            </p>
          </div>
          <button className="primary" disabled={todo.length === 0 || batch !== null} onClick={() => void ctx.renderBatch(todo.map((l) => l.line.key))} style={{ minHeight: 44 }}>
            {batch ? <><span className="spin" aria-hidden="true" /> Rendering {batch.done}/{batch.total}</> : <>{cloud.length > 0 && todo.some((l) => ctx.voices.find((v) => v.id === l.voice_id)?.sends_data) ? <I.Cloud size={16} /> : <I.Zap size={16} />} Render {todo.length} line{todo.length === 1 ? "" : "s"}</>}
          </button>
        </div>
        {batch && <div className="row" style={{ marginTop: 8 }}><button className="small" onClick={ctx.stopBatch}>Stop after this line</button><span className="small muted">Lines already rendered are kept.</span></div>}
        {batch && <div className="bar" style={{ marginTop: 12 }} role="progressbar" aria-label="Rendering progress" aria-valuemin={0} aria-valuemax={batch.total} aria-valuenow={batch.done}><div style={{ width: `${(100 * batch.done) / batch.total}%` }} /></div>}
      </div>

      {blocked.length > 0 && (
        <div className="callout warn">
          <strong>{blocked.length} line{blocked.length === 1 ? " needs" : "s need"} attention before rendering</strong>
          Open Direct to see what to fix.
          <div className="row" style={{ marginTop: 8 }}><button className="small" onClick={() => ctx.goto(2)}>Go to Direct</button></div>
        </div>
      )}

      <div className="takewall" role="list" aria-label="Lines">
        {analysis.lines.map((l) => {
          const key = l.line.key;
          const take = project.takes.find((t) => t.id === l.selected_take);
          const takes = project.takes.filter((t) => t.line_key === key);
          const st = STATUS[l.status];
          const err = ctx.errors[key];
          const busy = ctx.busy.has(key);
          return (
            <Fragment key={key}>
              <div className={`take-row ${l.status}`} role="listitem">
                <div>
                  <span className="speaker">{l.line.speaker}</span>
                  <div className="small faint mono">{key}</div>
                  <span className={`chip ${st.cls}`} style={{ marginTop: 6 }}>{st.icon}{st.label}</span>
                </div>
                <div style={{ minWidth: 0 }}>
                  <div className="words" title={l.line.text}>{l.line.text}</div>
                  {err && <div className="callout bad small" style={{ margin: "6px 0 0" }} role="alert"><strong>Could not render</strong>{err}</div>}
                  {l.issues.length > 0 && l.status !== "skipped" && (
                    <div className="small" style={{ color: "var(--warn)", marginTop: 4 }}>
                      {l.issues[0]?.message}
                      <span className="row tight" style={{ marginTop: 4 }}>
                        {(l.issues[0]?.kind === "unsupported_tag" || l.issues[0]?.kind === "unknown_tag") && l.issues[0]?.tag && (
                          <button className="small" onClick={() => ctx.edit((p) => ({ ...p, script: removeTagFromRow(p.script, l.line.row, l.issues[0]!.tag!) }))}>Remove [{l.issues[0].tag}] from this line</button>
                        )}
                        {["no_voice", "voice_missing", "unsupported_tag"].includes(l.issues[0]?.kind ?? "") && <button className="small" onClick={() => ctx.goto(1)}>Pick another voice</button>}
                      </span>
                    </div>
                  )}
                </div>
                <div>
                  {take ? <><Waveform projectId={project.id} file={take.file} bins={72} height={36} dim={l.status !== "rendered"} label={`Waveform of ${l.line.speaker}, ${key}`} /><div className="small faint num">{fmtMs(take.duration_ms)}</div></>
                    : <div className="faint small" style={{ height: 36, display: "grid", alignItems: "center" }}>{busy ? "Rendering…" : "No audio yet"}</div>}
                </div>
                <div className="actions row tight" style={{ justifyContent: "flex-end" }}>
                  <PlayButton path={take ? joinPath(ctx.dir, take.file) : null} label={`${l.line.speaker} ${key}`} bust={take?.id} />
                  <button className="small" disabled={busy || l.issues.length > 0 || l.status === "skipped" || batch !== null} onClick={() => void ctx.render([key])}>
                    {busy ? <span className="spin" aria-hidden="true" /> : null}{l.status === "drafted" ? "Render" : "Render again"}
                  </button>
                  <button className="small ghost" onClick={() => setSkip(key, l.status !== "skipped")}>{l.status === "skipped" ? "Include" : "Skip"}</button>
                  {takes.length > 0 && <button className="small ghost" aria-expanded={open === key} onClick={() => setOpen(open === key ? null : key)}>Takes ({takes.length}) <I.ChevronDown size={14} /></button>}
                </div>
                {open === key && (
                  <div className="take-detail col">
                    {takes.slice().reverse().map((t) => {
                      const sel = l.selected_take === t.id;
                      return (
                        <div key={t.id} className="linecard" style={{ margin: 0 }}>
                          <div className="row">
                            <label className="inline"><input type="radio" name={`take-${key}`} checked={sel} onChange={() => edit((p) => ({ ...p, selected: { ...p.selected, [key]: t.id } }))} /> Use this take</label>
                            <PlayButton path={joinPath(ctx.dir, t.file)} label={`take ${t.id}`} bust={t.id} />
                            <div style={{ width: 160 }}><Waveform projectId={project.id} file={t.file} bins={48} height={24} dim={!sel} label={`Waveform of take ${t.id}`} /></div>
                            <span className="muted small">{fmtMs(t.duration_ms)} · {t.direction_summary} · {t.provider_version}</span>
                            <span className="spacer" />
                            <span className={`chip ${t.status === "approved" ? "ok" : t.status === "rejected" ? "bad" : t.status === "held" ? "warn" : ""}`}>{t.status === "pending" ? "Not reviewed" : t.status}</span>
                            <button className="small" onClick={() => setTakeStatus(t.id, "approved")}>Approve</button>
                            <button className="small" onClick={() => setTakeStatus(t.id, "held")}>Hold</button>
                            <button className="small danger" onClick={() => setTakeStatus(t.id, "rejected")}>Reject</button>
                          </div>
                          {advanced && (
                            <>
                              <label htmlFor={`tn-${t.id}`} style={{ marginTop: 8 }}>Note on this take</label>
                              <input id={`tn-${t.id}`} type="text" style={{ width: "100%" }} maxLength={300} value={noteFor(`take:${t.id}`)} onChange={(e) => setNote(`take:${t.id}`, e.target.value)} />
                            </>
                          )}
                          {t.status === "rejected" && sel && <p className="small" style={{ color: "var(--warn)", marginBottom: 0, marginTop: 8 }}>This take is rejected, so it is left out of the mix and export. Pick another take or render again.</p>}
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </Fragment>
          );
        })}
      </div>
    </div>
  );
}
