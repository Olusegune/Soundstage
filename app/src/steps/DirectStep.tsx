import type { Ctx } from "../ctx";
import { joinPath } from "../api";
import { removeTagFromRow, splitTags } from "../logic";
import { Empty, PlayButton } from "../ui";
import type { Direction, LineInfo, Level } from "../types";
import { EMOTIONS } from "../types";

const LEVEL_LABEL: Record<Level, string> = { default: "default", project: "project", character: "character", scene: "scene", line: "this line" };

function EmotionSelect(props: { id: string; value: string | null | undefined; onChange: (v: string | null) => void; inheritLabel?: string }) {
  return (
    <select id={props.id} value={props.value ?? ""} onChange={(e) => props.onChange(e.target.value || null)}>
      <option value="">{props.inheritLabel ?? "Inherit"}</option>
      {EMOTIONS.map((e) => <option key={e} value={e}>{e}</option>)}
    </select>
  );
}

export function DirectStep({ ctx }: { ctx: Ctx }) {
  const { project, analysis, edit } = ctx;
  if (!analysis || analysis.lines.length === 0) {
    return <div className="page"><Empty title="Nothing to direct yet"><button onClick={() => ctx.goto(0)}>Go to Script</button></Empty></div>;
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

  const preview = async (info: LineInfo) => { await ctx.render([info.line.key]); };

  return (
    <div className="page">
      <div className="card">
        <h2>Direction levels</h2>
        <p className="muted small">A lower level overrides the one above it: project, then character, then scene, then line. Every line below shows where its delivery comes from.</p>
        <div className="row" style={{ alignItems: "end" }}>
          <div><label htmlFor="proj-emotion">Project style</label><EmotionSelect id="proj-emotion" value={project.project_direction.emotion} onChange={(v) => setProjectDir({ emotion: v })} inheritLabel="neutral" /></div>
          <div><label htmlFor="proj-pace">Project pace {(project.project_direction.pace ?? 1).toFixed(2)}×</label><input id="proj-pace" type="range" min={0.5} max={2} step={0.05} value={project.project_direction.pace ?? 1} onChange={(e) => setProjectDir({ pace: Number(e.target.value) })} /></div>
          {analysis.parsed.speakers.map((s) => (
            <div key={s}><label htmlFor={`char-${s}`}>{s} baseline</label><EmotionSelect id={`char-${s}`} value={project.characters[s]?.direction.emotion} onChange={(v) => setCharDir(s, { emotion: v })} /></div>
          ))}
        </div>
      </div>

      {analysis.parsed.scenes.map((scene) => {
        const sd = project.scene_directions[String(scene.index)] ?? {};
        const lines = analysis.lines.filter((l) => l.line.scene === scene.index);
        return (
          <div className="card" key={scene.index}>
            <div className="row" style={{ alignItems: "end", marginBottom: 8 }}>
              <div style={{ minWidth: 220 }}>
                <h2 style={{ margin: 0 }}>{scene.index + 1}. {scene.title}</h2>
                <span className="muted small">{[scene.location, scene.time_of_day].filter(Boolean).join(" · ")} Cast: {scene.cast.join(", ")}</span>
              </div>
              <span className="spacer" />
              <div><label htmlFor={`scene-emo-${scene.index}`}>Scene mood</label><EmotionSelect id={`scene-emo-${scene.index}`} value={sd.emotion} onChange={(v) => setSceneDir(scene.index, { emotion: v })} /></div>
              <div style={{ minWidth: 220 }}><label htmlFor={`scene-note-${scene.index}`}>Scene note</label><input id={`scene-note-${scene.index}`} type="text" style={{ width: "100%" }} maxLength={300} value={sd.note ?? ""} placeholder="e.g. rushed, kitchen, morning" onChange={(e) => setSceneDir(scene.index, { note: e.target.value })} /></div>
              <div>
                <label htmlFor={`scene-status-${scene.index}`}>Status</label>
                <select id={`scene-status-${scene.index}`} value={project.scene_status[String(scene.index)] ?? "not_started"} onChange={(e) => edit((p) => ({ ...p, scene_status: { ...p.scene_status, [String(scene.index)]: e.target.value } }))}>
                  <option value="not_started">Not started</option><option value="in_progress">In progress</option><option value="approved">Approved</option>
                </select>
              </div>
            </div>
            {lines.map((info) => {
              const key = info.line.key;
              const ld = project.line_directions[key] ?? {};
              const take = info.selected_take ? project.takes.find((t) => t.id === info.selected_take) : undefined;
              const busy = ctx.busy.has(key);
              return (
                <div className="linecard" key={key}>
                  <div className="row">
                    <span className="speaker">{info.line.speaker}</span>
                    <span className="chip info" title="Where the delivery comes from">{info.resolved.emotion.value} · from {LEVEL_LABEL[info.resolved.emotion.source]}</span>
                    <span className="chip">{info.resolved.pace.value.toFixed(2)}× · from {LEVEL_LABEL[info.resolved.pace.source]}</span>
                    <span className="spacer" />
                    <button className="small" disabled={busy || info.voice_id === null || info.issues.length > 0} title={info.issues.length > 0 ? "Fix the issue below first" : undefined} onClick={() => void preview(info)}>{busy ? "Rendering…" : "Preview line"}</button>
                    <PlayButton path={take ? joinPath(ctx.dir, take.file) : null} label={`${info.line.speaker} line`} bust={take?.id} />
                  </div>
                  <div className="text">{splitTags(info.line.text).map((p, i) => <span key={i} className={p.tag ? (p.bad ? "tag-bad" : "tag") : ""}>{p.text}</span>)}</div>
                  <div className="row" style={{ alignItems: "end" }}>
                    <div><label htmlFor={`e-${key}`}>Emotion</label><EmotionSelect id={`e-${key}`} value={ld.emotion} onChange={(v) => setLineDir(key, { emotion: v })} /></div>
                    <div><label htmlFor={`p-${key}`}>Pace {(ld.pace ?? info.resolved.pace.value).toFixed(2)}×</label><input id={`p-${key}`} type="range" min={0.5} max={2} step={0.05} value={ld.pace ?? info.resolved.pace.value} onChange={(e) => setLineDir(key, { pace: Number(e.target.value) })} />
                      {ld.pace != null && <button className="ghost small" onClick={() => setLineDir(key, { pace: null })}>Reset</button>}</div>
                    <div><label htmlFor={`b-${key}`}>Pause before (ms)</label><input id={`b-${key}`} type="number" min={0} max={10000} step={50} style={{ width: 90 }} value={ld.pause_before_ms ?? ""} placeholder={String(project.mix.gap_ms)} onChange={(e) => setLineDir(key, { pause_before_ms: e.target.value === "" ? null : Math.max(0, Math.min(10000, Number(e.target.value))) })} /></div>
                    <div style={{ flex: 1, minWidth: 160 }}><label htmlFor={`n-${key}`}>Director's note</label><input id={`n-${key}`} type="text" style={{ width: "100%" }} maxLength={300} value={ld.note ?? ""} onChange={(e) => setLineDir(key, { note: e.target.value })} /></div>
                  </div>
                  {info.issues.map((iss, i) => (
                    <div key={i} className={`callout ${iss.kind === "unsupported_tag" ? "warn" : "bad"} small`} style={{ marginTop: 8 }} role="alert">
                      <strong>{iss.kind === "unsupported_tag" ? "⚠ Not supported by this voice" : iss.kind === "unknown_tag" ? "Unknown tag" : iss.kind === "no_voice" ? "No voice yet" : iss.kind === "refused" ? "Soundstage will not render this" : "Needs attention"}</strong>
                      {iss.message}
                      <div className="row" style={{ marginTop: 6 }}>
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
          </div>
        );
      })}
    </div>
  );
}
