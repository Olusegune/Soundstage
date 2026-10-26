import { useEffect } from "react";
import { joinPath } from "../api";
import type { Ctx } from "../ctx";
import * as I from "../icons";
import { removeTagFromRow, splitTags } from "../logic";
import { Empty, Help, PlayButton, Waveform } from "../ui";
import { PlanPanel } from "./PlanPanel";
import type { Direction, Level, LineInfo } from "../types";
import { EMOTIONS } from "../types";

const LEVEL_LABEL: Record<Level, string> = { default: "default", project: "project", character: "character", scene: "scene", line: "this line" };
const EMOTION_HINT: Record<string, string> = {
  neutral: "Even and natural", whisper: "Quiet and close", shout: "Loud and urgent", excited: "Fast and bright", sad: "Slow and heavy",
  tired: "Slow and low energy", sarcastic: "Dry, with an edge", curious: "Questioning and light", serious: "Measured and firm",
};

function EmotionChips(props: { label: string; value: string | null | undefined; onChange: (v: string | null) => void }) {
  return (
    <div role="radiogroup" aria-label={props.label} className="row tight">
      <button className="small" role="radio" aria-checked={!props.value} onClick={() => props.onChange(null)} data-tip="Use what the scene, character or project says">Inherit</button>
      {EMOTIONS.map((e) => (
        <button key={e} className="small" role="radio" aria-checked={props.value === e} onClick={() => props.onChange(e)} data-tip={EMOTION_HINT[e]}
          style={props.value === e ? { borderColor: "var(--gold)", background: "var(--gold-wash)", color: "var(--accent-text)" } : undefined}>{e}</button>
      ))}
    </div>
  );
}

export function DirectStep({ ctx }: { ctx: Ctx }) {
  const { project, analysis, edit } = ctx;
  const advanced = ctx.mode === "advanced";

  useEffect(() => {
    if (!ctx.focusKey) return;
    const el = document.getElementById(`line-${ctx.focusKey}`);
    el?.scrollIntoView({ block: "center", behavior: "smooth" });
  }, [ctx.focusKey]);

  if (!analysis || analysis.lines.length === 0) {
    return <Empty title="Nothing to direct yet"><p className="muted">Direction shapes how each line is spoken. Write your script first.</p><button className="primary" onClick={() => ctx.goto(0)}>Go to Script</button></Empty>;
  }

  const setProjectDir = (patch: Partial<Direction>) => edit((p) => ({ ...p, project_direction: { ...p.project_direction, ...patch } }), "pdir");
  const setCharDir = (speaker: string, patch: Partial<Direction>) =>
    edit((p) => {
      const c = p.characters[speaker];
      return { ...p, characters: { ...p.characters, [speaker]: { voice_id: c?.voice_id ?? null, bible: c?.bible ?? { age: "", background: "", speech_habits: "", emotional_range: "", notes: "" }, direction: { ...(c?.direction ?? {}), ...patch } } } };
    }, `cdir-${speaker}`);
  const setSceneDir = (i: number, patch: Partial<Direction>) =>
    edit((p) => ({ ...p, scene_directions: { ...p.scene_directions, [String(i)]: { ...(p.scene_directions[String(i)] ?? {}), ...patch } } }), `sdir-${i}`);
  const setLineDir = (key: string, patch: Partial<Direction>) =>
    edit((p) => ({ ...p, line_directions: { ...p.line_directions, [key]: { ...(p.line_directions[key] ?? {}), ...patch } } }), `ldir-${key}`);

  const noteCount = (scene: number) => analysis.lines.filter((l) => l.line.scene === scene && l.issues.length > 0).length;

  return (
    <div className="scene-layout">
      <nav aria-label="Scenes" className="scene-nav">
        <h3>Scenes</h3>
        <div className="col" style={{ gap: 4 }}>
          {analysis.parsed.scenes.map((s) => (
            <button key={s.index} className="ghost" style={{ justifyContent: "space-between", textAlign: "left" }} onClick={() => document.getElementById(`scene-${s.index}`)?.scrollIntoView({ behavior: "smooth", block: "start" })}>
              <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{s.index + 1}. {s.title}</span>
              {noteCount(s.index) > 0 && <span className="chip warn" aria-label={`${noteCount(s.index)} notes`}>{noteCount(s.index)}</span>}
            </button>
          ))}
        </div>
      </nav>

      <div>
        <PlanPanel ctx={ctx} />
        {advanced && (
          <div className="card">
            <h2>Direction levels <Help text="Delivery is set from broad to fine: project, then character, then scene, then line. A finer level overrides a broader one." /></h2>
            <p className="muted small">Each line below shows where its delivery comes from.</p>
            <div className="col">
              <div><label>Project style</label><EmotionChips label="Project style" value={project.project_direction.emotion} onChange={(v) => setProjectDir({ emotion: v })} /></div>
              <div><label htmlFor="proj-pace">Project pace {(project.project_direction.pace ?? 1).toFixed(2)}×</label><input id="proj-pace" type="range" min={0.5} max={2} step={0.05} value={project.project_direction.pace ?? 1} onChange={(e) => setProjectDir({ pace: Number(e.target.value) })} /></div>
              {analysis.parsed.speakers.map((s) => <div key={s}><label>{s} baseline</label><EmotionChips label={`${s} baseline`} value={project.characters[s]?.direction.emotion} onChange={(v) => setCharDir(s, { emotion: v })} /></div>)}
            </div>
          </div>
        )}

        {analysis.parsed.scenes.map((scene) => {
          const sd = project.scene_directions[String(scene.index)] ?? {};
          const lines = analysis.lines.filter((l) => l.line.scene === scene.index);
          return (
            <section className="card" key={scene.index} id={`scene-${scene.index}`} aria-labelledby={`scene-h-${scene.index}`}>
              <div className="row" style={{ marginBottom: 12, alignItems: "flex-start" }}>
                <div style={{ minWidth: 220, flex: 1 }}>
                  <h2 id={`scene-h-${scene.index}`} style={{ margin: 0 }}>{scene.index + 1}. {scene.title}</h2>
                  <span className="muted small">{[scene.location, scene.time_of_day].filter(Boolean).join(" · ")}{scene.location || scene.time_of_day ? " · " : ""}Cast: {scene.cast.join(", ")}</span>
                </div>
                {advanced && (
                  <div>
                    <label htmlFor={`scene-status-${scene.index}`}>Status</label>
                    <select id={`scene-status-${scene.index}`} value={project.scene_status[String(scene.index)] ?? "not_started"} onChange={(e) => edit((p) => ({ ...p, scene_status: { ...p.scene_status, [String(scene.index)]: e.target.value } }))}>
                      <option value="not_started">Not started</option><option value="in_progress">In progress</option><option value="approved">Approved</option>
                    </select>
                  </div>
                )}
              </div>
              <div style={{ marginBottom: 14 }}>
                <label>Scene mood <Help text="Sets the default delivery for every line in this scene that does not set its own." /></label>
                <EmotionChips label={`Scene ${scene.index + 1} mood`} value={sd.emotion} onChange={(v) => setSceneDir(scene.index, { emotion: v })} />
                {advanced && <div style={{ marginTop: 8 }}><label htmlFor={`scene-note-${scene.index}`}>Scene note</label><input id={`scene-note-${scene.index}`} type="text" style={{ width: "100%" }} maxLength={300} value={sd.note ?? ""} placeholder="For example: rushed, kitchen, morning" onChange={(e) => setSceneDir(scene.index, { note: e.target.value })} /></div>}
              </div>

              {lines.map((info: LineInfo) => {
                const key = info.line.key;
                const ld = project.line_directions[key] ?? {};
                const take = info.selected_take ? project.takes.find((t) => t.id === info.selected_take) : undefined;
                const busy = ctx.busy.has(key);
                const focused = ctx.focusKey === key;
                return (
                  <div className="linecard" key={key} id={`line-${key}`} style={focused ? { borderColor: "var(--gold)", boxShadow: "0 0 0 3px var(--gold-wash)" } : undefined}>
                    <div className="row">
                      <span className="speaker">{info.line.speaker}</span>
                      <span className="chip info" title="Where the delivery comes from">{info.resolved.emotion.value} · from {LEVEL_LABEL[info.resolved.emotion.source]}</span>
                      {advanced && <span className="chip">{info.resolved.pace.value.toFixed(2)}× · from {LEVEL_LABEL[info.resolved.pace.source]}</span>}
                      <span className="spacer" />
                      <button className="small" disabled={busy || info.voice_id === null || info.issues.length > 0} onClick={() => void ctx.render([key])} title={info.issues.length > 0 ? "Fix the note below first" : undefined}>
                        {busy ? <><span className="spin" aria-hidden="true" /> Rendering</> : <><I.Play size={12} /> Preview line</>}
                      </button>
                      <PlayButton path={take ? joinPath(ctx.dir, take.file) : null} label={`${info.line.speaker} line`} bust={take?.id} />
                    </div>
                    <div className="text">{splitTags(info.line.text).map((p, i) => <span key={i} className={p.tag ? (p.bad ? "tag-bad" : "tag") : ""}>{p.text}</span>)}</div>
                    {take && <div style={{ marginBottom: 10 }}><Waveform projectId={project.id} file={take.file} bins={120} height={28} label={`Waveform of ${info.line.speaker}'s line`} dim={info.status !== "rendered"} /></div>}
                    <div><label>Emotion for this line</label><EmotionChips label={`Emotion for ${info.line.speaker}'s line`} value={ld.emotion} onChange={(v) => setLineDir(key, { emotion: v })} /></div>
                    {advanced && (
                      <div className="row" style={{ alignItems: "end", marginTop: 10 }}>
                        <div><label htmlFor={`p-${key}`}>Pace {(ld.pace ?? info.resolved.pace.value).toFixed(2)}×</label><input id={`p-${key}`} type="range" min={0.5} max={2} step={0.05} value={ld.pace ?? info.resolved.pace.value} onChange={(e) => setLineDir(key, { pace: Number(e.target.value) })} />
                          {ld.pace != null && <button className="ghost small" onClick={() => setLineDir(key, { pace: null })}>Reset</button>}</div>
                        <div><label htmlFor={`b-${key}`}>Pause before (ms)</label><input id={`b-${key}`} type="number" min={0} max={10000} step={50} style={{ width: 96 }} value={ld.pause_before_ms ?? ""} placeholder={String(project.mix.gap_ms)} onChange={(e) => setLineDir(key, { pause_before_ms: e.target.value === "" ? null : Math.max(0, Math.min(10000, Number(e.target.value))) })} /></div>
                        <div style={{ flex: 1, minWidth: 160 }}><label htmlFor={`n-${key}`}>Director's note</label><input id={`n-${key}`} type="text" style={{ width: "100%" }} maxLength={300} value={ld.note ?? ""} onChange={(e) => setLineDir(key, { note: e.target.value })} /></div>
                      </div>
                    )}
                    {info.issues.map((iss, i) => (
                      <div key={i} className={`callout ${iss.kind === "unsupported_tag" ? "warn" : "bad"} small`} style={{ marginTop: 10, marginBottom: 0 }} role="alert">
                        <strong>{iss.kind === "unsupported_tag" ? "Not supported by this voice" : iss.kind === "unknown_tag" ? "Unknown tag" : iss.kind === "no_voice" ? "No voice yet" : iss.kind === "refused" ? "Soundstage will not render this" : "Needs attention"}</strong>
                        {iss.message}
                        <div className="row" style={{ marginTop: 8 }}>
                          {iss.tag && (iss.kind === "unsupported_tag" || iss.kind === "unknown_tag") && (
                            <button className="small" onClick={() => edit((p) => ({ ...p, script: removeTagFromRow(p.script, info.line.row, iss.tag ?? "") }))}>Remove [{iss.tag}] from this line</button>
                          )}
                          {(iss.kind === "unsupported_tag" || iss.kind === "no_voice" || iss.kind === "voice_missing") && <button className="small" onClick={() => ctx.goto(1)}>Choose another voice</button>}
                        </div>
                      </div>
                    ))}
                  </div>
                );
              })}
            </section>
          );
        })}
      </div>
    </div>
  );
}
