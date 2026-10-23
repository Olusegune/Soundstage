import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { api, messageOf } from "../api";
import type { Ctx } from "../ctx";
import { fmtMs, licenceNote } from "../logic";
import { Confirm, LicenceChip, PlayButton } from "../ui";
import type { Asset, Cue, Destination, Layout, MixPreview } from "../types";
import { LicenceEditor } from "./LicenceEditor";

const lufs = (v: number | null) => (v === null ? "n/a" : `${v.toFixed(1)} LUFS`);

export function MixStep({ ctx }: { ctx: Ctx }) {
  const { project, edit } = ctx;
  const [layout, setLayout] = useState<Layout | null>(null);
  const [dests, setDests] = useState<Destination[]>([]);
  const [preview, setPreview] = useState<MixPreview | null>(null);
  const [building, setBuilding] = useState(false);
  const [editing, setEditing] = useState<string | null>(null);
  const [removing, setRemoving] = useState<Asset | null>(null);

  useEffect(() => { void api.destinations().then(setDests); }, []);
  useEffect(() => { void api.layout(project).then(setLayout).catch(() => setLayout(null)); }, [project.takes, project.selected, project.line_directions, project.mix.gap_ms, project.mix.scene_gap_ms, project.script]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => { setPreview(null); }, [project.cues, project.mix, project.selected, project.takes, project.assets]);

  const dest = dests.find((d) => d.id === project.mix.destination);
  const total = Math.max(1, layout?.total_ms ?? 1);

  const importAsset = async (kind: "music" | "sfx") => {
    const picked = await open({ multiple: false, filters: [{ name: "WAV audio", extensions: ["wav"] }], title: kind === "music" ? "Import music" : "Import a sound effect" });
    if (typeof picked !== "string") return;
    try {
      const a = await api.importAsset(project.id, picked, kind);
      edit((p) => ({ ...p, assets: [...p.assets, a] }));
      ctx.toast(`${a.name} imported. Its licence is marked unknown until you record where its terms were checked.`);
      setEditing(a.id);
    } catch (e) { ctx.toast(messageOf(e), true); }
  };

  const addCue = (a: Asset) =>
    edit((p) => ({ ...p, cues: [...p.cues, { id: `cue-${Date.now().toString(36)}`, asset_id: a.id, scene: layout?.scenes[0]?.index ?? null, offset_ms: 0, gain_db: a.kind === "music" ? -14 : -8, loop_to_scene: a.kind === "music", fade_in_ms: a.kind === "music" ? 400 : 10, fade_out_ms: a.kind === "music" ? 800 : 50, duck: a.kind === "music" }] }));

  const setCue = (id: string, patch: Partial<Cue>) => edit((p) => ({ ...p, cues: p.cues.map((c) => (c.id === id ? { ...c, ...patch } : c)) }), `cue-${id}`);

  const build = async () => {
    setBuilding(true);
    try { setPreview(await api.mixPreview(project)); } catch (e) { ctx.toast(messageOf(e), true); } finally { setBuilding(false); }
  };

  const sceneStart = (i: number | null) => (i === null ? 0 : layout?.scenes.find((s) => s.index === i)?.start_ms ?? 0);
  const cueLen = (c: Cue, a: Asset | undefined) => {
    if (!a) return 0;
    if (c.loop_to_scene && c.scene !== null) { const s = layout?.scenes.find((x) => x.index === c.scene); return s ? s.end_ms - s.start_ms - c.offset_ms : a.duration_ms; }
    return a.duration_ms;
  };
  const pct = (ms: number) => `${(100 * ms) / total}%`;

  return (
    <div className="page">
      <div className="card">
        <h2>Timeline</h2>
        {!layout || layout.lines.length === 0 ? <p className="muted">Render some lines and they appear here.</p> : (
          <div className="timeline" aria-label="Timeline of dialogue, effects and music">
            {layout.scenes.map((s) => <div key={s.index} className="scene-mark" style={{ left: pct(s.start_ms) }}>{s.index + 1}. {s.title}</div>)}
            <div className="track-label">Dialogue</div>
            <div className="track">{layout.lines.map((l) => <div key={l.key} className="block" title={`${l.speaker} ${l.key}`} style={{ left: pct(l.start_ms), width: pct(l.end_ms - l.start_ms) }}>{l.speaker}</div>)}</div>
            <div className="track-label">Effects</div>
            <div className="track">{project.cues.filter((c) => project.assets.find((a) => a.id === c.asset_id)?.kind === "sfx").map((c) => { const a = project.assets.find((x) => x.id === c.asset_id); return <div key={c.id} className="block sfx" style={{ left: pct(sceneStart(c.scene) + c.offset_ms), width: pct(cueLen(c, a)) }}>{a?.name}</div>; })}</div>
            <div className="track-label">Music</div>
            <div className="track">{project.cues.filter((c) => project.assets.find((a) => a.id === c.asset_id)?.kind === "music").map((c) => { const a = project.assets.find((x) => x.id === c.asset_id); return <div key={c.id} className="block music" style={{ left: pct(sceneStart(c.scene) + c.offset_ms), width: pct(cueLen(c, a)) }}>{a?.name}</div>; })}</div>
            <div className="small muted">Total {fmtMs(layout.total_ms)}</div>
          </div>
        )}
        <p className="small muted" style={{ marginBottom: 0 }}>Dialogue is laid out in script order with the gaps you set. Without music or effects, the mix is the voice track alone.</p>
      </div>

      <div className="grid2" style={{ gridTemplateColumns: "1fr 1fr" }}>
        <div className="card">
          <h2>Music and effects</h2>
          <div className="row" style={{ marginBottom: 10 }}>
            <button onClick={() => void importAsset("music")}>Import music (WAV)…</button>
            <button onClick={() => void importAsset("sfx")}>Import effect (WAV)…</button>
          </div>
          {project.assets.length === 0 && <p className="muted small">Nothing imported. Soundstage mixes music and effects only from files you bring, and it tracks the licence of each one. It does not generate music or effects in this version.</p>}
          {project.assets.map((a) => (
            <div key={a.id} className="linecard">
              <div className="row">
                <strong>{a.name}</strong><span className="chip">{a.kind === "music" ? "Music" : "Effect"}</span><span className="muted small">{fmtMs(a.duration_ms)}</span>
                <LicenceChip licence={project.licence_checks[`asset:${a.id}`]?.licence ?? a.licence.licence} note={licenceNote(project.licence_checks[`asset:${a.id}`] ?? a.licence)} />
                <span className="spacer" />
                <button className="small" onClick={() => addCue(a)}>Add to timeline</button>
                <button className="small ghost" aria-expanded={editing === a.id} onClick={() => setEditing(editing === a.id ? null : a.id)}>Licence…</button>
                <button className="small danger" onClick={() => setRemoving(a)}>Remove</button>
              </div>
              {editing === a.id && <div style={{ marginTop: 8 }}><LicenceEditor initial={a.licence} onSave={(info) => { edit((p) => ({ ...p, assets: p.assets.map((x) => (x.id === a.id ? { ...x, licence: info } : x)) })); setEditing(null); }} /></div>}
            </div>
          ))}
          {project.cues.length > 0 && <h3 style={{ marginTop: 12 }}>On the timeline</h3>}
          {project.cues.map((c) => {
            const a = project.assets.find((x) => x.id === c.asset_id);
            return (
              <div key={c.id} className="linecard">
                <div className="row"><strong>{a?.name ?? "Missing asset"}</strong><span className="spacer" /><button className="small danger" onClick={() => edit((p) => ({ ...p, cues: p.cues.filter((x) => x.id !== c.id) }))}>Remove from timeline</button></div>
                <div className="row" style={{ alignItems: "end", marginTop: 6 }}>
                  <div><label htmlFor={`c-s-${c.id}`}>Starts in</label><select id={`c-s-${c.id}`} value={c.scene ?? ""} onChange={(e) => setCue(c.id, { scene: e.target.value === "" ? null : Number(e.target.value) })}><option value="">Project start</option>{layout?.scenes.map((s) => <option key={s.index} value={s.index}>Scene {s.index + 1}</option>)}</select></div>
                  <div><label htmlFor={`c-o-${c.id}`}>Offset (ms)</label><input id={`c-o-${c.id}`} type="number" min={0} max={600000} step={100} style={{ width: 90 }} value={c.offset_ms} onChange={(e) => setCue(c.id, { offset_ms: Math.max(0, Number(e.target.value) || 0) })} /></div>
                  <div><label htmlFor={`c-g-${c.id}`}>Level {c.gain_db} dB</label><input id={`c-g-${c.id}`} type="range" min={-40} max={6} step={1} value={c.gain_db} onChange={(e) => setCue(c.id, { gain_db: Number(e.target.value) })} /></div>
                </div>
                <div className="row" style={{ marginTop: 6 }}>
                  <label className="inline"><input type="checkbox" checked={c.loop_to_scene} disabled={c.scene === null} onChange={(e) => setCue(c.id, { loop_to_scene: e.target.checked })} /> Loop to fill the scene</label>
                  <label className="inline" title="Turn off for music that exists inside the scene, like a radio"><input type="checkbox" checked={c.duck} onChange={(e) => setCue(c.id, { duck: e.target.checked })} /> Lower under dialogue</label>
                </div>
              </div>
            );
          })}
        </div>

        <div className="card">
          <h2>Level and loudness</h2>
          <div className="row" style={{ alignItems: "end" }}>
            <div style={{ flex: 1 }}>
              <label htmlFor="dest">Where will this play?</label>
              <select id="dest" value={project.mix.destination} style={{ width: "100%" }} onChange={(e) => edit((p) => ({ ...p, mix: { ...p.mix, destination: e.target.value } }))}>
                {dests.map((d) => <option key={d.id} value={d.id}>{d.label} ({d.target_lufs} LUFS)</option>)}
              </select>
            </div>
          </div>
          {dest && <p className="small muted">Target {dest.target_lufs} LUFS, true peak under {dest.true_peak_ceiling_dbtp} dBTP. {dest.note}. Confirm the current standard for your destination before delivery.</p>}
          <div className="row">
            <div><label htmlFor="duck">Duck depth {project.mix.duck_depth_db} dB</label><input id="duck" type="range" min={0} max={30} value={project.mix.duck_depth_db} onChange={(e) => edit((p) => ({ ...p, mix: { ...p.mix, duck_depth_db: Number(e.target.value) } }), "duck")} /></div>
            <div><label htmlFor="gap">Gap between lines {project.mix.gap_ms} ms</label><input id="gap" type="range" min={0} max={2000} step={50} value={project.mix.gap_ms} onChange={(e) => edit((p) => ({ ...p, mix: { ...p.mix, gap_ms: Number(e.target.value) } }), "gap")} /></div>
          </div>
          <div className="row" style={{ margin: "10px 0" }}>
            <button className="primary" disabled={building || !layout || layout.lines.length === 0} onClick={() => void build()}>{building ? "Mixing…" : "Build a preview mix"}</button>
            <PlayButton path={preview?.path ?? null} label="the preview mix" bust={preview ? String(preview.total_ms) + preview.gain_db : undefined} />
          </div>
          {preview ? (
            <div aria-live="polite">
              <table><tbody>
                <tr><th>Integrated</th><td>{lufs(preview.loudness.integrated_lufs)} <span className="muted">(target {preview.target_lufs})</span></td></tr>
                <tr><th>Short-term max</th><td>{lufs(preview.loudness.short_term_max_lufs)}</td></tr>
                <tr><th>Loudness range</th><td>{preview.loudness.loudness_range_lu === null ? "n/a" : `${preview.loudness.loudness_range_lu.toFixed(1)} LU`}</td></tr>
                <tr><th>True peak (estimated)</th><td>{preview.loudness.true_peak_dbtp.toFixed(1)} dBTP</td></tr>
                <tr><th>Gain applied</th><td>{preview.gain_db.toFixed(1)} dB</td></tr>
              </tbody></table>
              {preview.loudness.integrated_lufs !== null && <div className="meter" style={{ marginTop: 8 }} role="img" aria-label={`Loudness ${lufs(preview.loudness.integrated_lufs)}`}><div style={{ width: `${Math.max(0, Math.min(100, ((preview.loudness.integrated_lufs + 40) / 40) * 100))}%` }} /></div>}
              {preview.target_reached ? <div className="callout ok small" style={{ marginTop: 8 }}><strong>✓ Target reached</strong>The master will land on the target for {dest?.label}.</div> : null}
              {preview.warnings.map((w, i) => <div key={i} className="callout warn small" style={{ marginTop: 8 }} role="alert"><strong>⚠ Flagged</strong>{w}</div>)}
            </div>
          ) : <p className="small muted">Build a preview to hear the mix and see its loudness before you export. Nothing is saved to disk outside the project.</p>}
        </div>
      </div>
      {removing && (
        <Confirm title={`Remove “${removing.name}”?`} confirmLabel="Remove" onCancel={() => setRemoving(null)}
          body={<p>This removes {removing.name} and any places it is used on the timeline from this project. Your original file is not touched.</p>}
          onConfirm={() => { const id = removing.id; edit((p) => ({ ...p, assets: p.assets.filter((x) => x.id !== id), cues: p.cues.filter((c) => c.asset_id !== id) })); setRemoving(null); }} />
      )}
    </div>
  );
}
