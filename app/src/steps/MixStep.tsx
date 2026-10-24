import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { api, messageOf } from "../api";
import type { Ctx } from "../ctx";
import * as I from "../icons";
import { fmtMs, licenceNote } from "../logic";
import { Confirm, Empty, Help, LicenceChip, PeaksView, PlayButton, Waveform } from "../ui";
import type { Asset, Cue, Destination, GeneratorInfo, Layout } from "../types";
import { LicenceEditor } from "./LicenceEditor";
import { suggestScene } from "../sceneSounds";
import { PromptHelper } from "../shell/PromptHelper";

const lufs = (v: number | null) => (v === null ? "n/a" : `${v.toFixed(1)} LUFS`);

export function MixStep({ ctx }: { ctx: Ctx }) {
  const { project, edit } = ctx;
  const advanced = ctx.mode === "advanced";
  const [layout, setLayout] = useState<Layout | null>(null);
  const [dests, setDests] = useState<Destination[]>([]);
  const [editing, setEditing] = useState<string | null>(null);
  const [removing, setRemoving] = useState<Asset | null>(null);
  const preview = ctx.mixPreview;
  const [gens, setGens] = useState<GeneratorInfo[]>([]);
  const [gen, setGen] = useState({ kind: "music" as "music" | "sfx", id: "", prompt: "", seconds: 30, instrumental: true, confirm: false, busy: false });
  useEffect(() => { void api.listGenerators().then(setGens).catch(() => setGens([])); }, []);
  const usable = gens.filter((g) => g.kinds.includes(gen.kind));
  const chosen = usable.find((g) => g.id === gen.id) ?? usable.find((g) => g.ready) ?? usable[0];
  const limits = chosen ? chosen[gen.kind] : null;

  const create = async () => {
    if (!chosen) return;
    setGen((g) => ({ ...g, busy: true, confirm: false }));
    try {
      const a = await api.generateAudio(project.id, chosen.id, gen.kind, gen.prompt, limits?.supports_length ? gen.seconds : null, gen.instrumental, null);
      edit((p) => ({ ...p, assets: [...p.assets, a] }));
      ctx.toast(`${a.name} created. It is marked AI-generated, and its licence is unknown until you record where its terms were checked.`);
      setEditing(a.id);
      setGen((g) => ({ ...g, prompt: "" }));
    } catch (e) { ctx.toast(messageOf(e), true); } finally { setGen((g) => ({ ...g, busy: false })); }
  };

  // Sound for each scene: a drafted description the person edits, then one press to make it and place it.
  const [sceneText, setSceneText] = useState<Record<string, string>>({});
  const [sceneAsk, setSceneAsk] = useState<{ index: number; kind: "music" | "sfx"; prompt: string } | null>(null);
  const [sceneBusy, setSceneBusy] = useState<string | null>(null);
  const sceneDraft = (index: number, kind: "music" | "sfx") => {
    const key = `${index}-${kind}`;
    if (sceneText[key] !== undefined) return sceneText[key]!;
    const ps = ctx.analysis?.parsed.scenes.find((s) => s.index === index);
    const t = layout?.scenes.find((s) => s.index === index)?.title ?? "";
    const sug = suggestScene({ title: ps?.title ?? t, location: ps?.location ?? "", time_of_day: ps?.time_of_day ?? "" });
    return kind === "sfx" ? sug.ambience : sug.music;
  };
  const makeForScene = async (index: number, kind: "music" | "sfx", prompt: string) => {
    const g = gens.filter((x) => x.kinds.includes(kind)).find((x) => x.ready);
    if (!g) { ctx.toast("No generator is set up for this yet. Save an ElevenLabs key in Settings, or add one under Voice engines.", true); return; }
    const sc = layout?.scenes.find((s) => s.index === index);
    const lim = g[kind];
    const secs = lim.supports_length ? Math.min(lim.max_s, Math.max(lim.min_s, Math.round(((sc ? sc.end_ms - sc.start_ms : 20000) / 1000)))) : null;
    setSceneBusy(`${index}-${kind}`);
    try {
      const a = await api.generateAudio(project.id, g.id, kind, prompt, secs, true, null);
      edit((p) => ({ ...p, assets: [...p.assets, a], cues: [...p.cues, { id: `cue-${Date.now().toString(36)}`, asset_id: a.id, scene: index, offset_ms: 0, gain_db: kind === "music" ? -16 : -24, loop_to_scene: true, fade_in_ms: 600, fade_out_ms: 1000, duck: true }] }));
      ctx.toast(`${a.name} was made and placed in scene ${index + 1}. It loops to fill the scene, and sits under the dialogue.`);
    } catch (e) { ctx.toast(messageOf(e), true); } finally { setSceneBusy(null); }
  };

  useEffect(() => { void api.destinations().then(setDests); }, []);
  useEffect(() => { void api.layout(project).then(setLayout).catch(() => setLayout(null)); }, [project.takes, project.selected, project.line_directions, project.mix.gap_ms, project.mix.scene_gap_ms, project.script]); // eslint-disable-line react-hooks/exhaustive-deps

  const dest = dests.find((d) => d.id === project.mix.destination);
  const total = Math.max(1, layout?.total_ms ?? 1);
  const pct = (ms: number) => `${(100 * ms) / total}%`;

  const importAsset = async (kind: "music" | "sfx") => {
    const picked = await open({ multiple: false, filters: [{ name: "Audio", extensions: ["wav", "mp3", "flac", "ogg", "m4a", "aac"] }], title: kind === "music" ? "Import music" : "Import a sound effect" });
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

  const sceneStart = (i: number | null) => (i === null ? 0 : layout?.scenes.find((s) => s.index === i)?.start_ms ?? 0);
  const cueLen = (c: Cue, a: Asset | undefined) => {
    if (!a) return 0;
    if (c.loop_to_scene && c.scene !== null) { const s = layout?.scenes.find((x) => x.index === c.scene); return s ? s.end_ms - s.start_ms - c.offset_ms : a.duration_ms; }
    return a.duration_ms;
  };

  if (!layout || layout.lines.length === 0) {
    return <Empty title="Nothing to mix yet"><p className="muted">Render some lines and they appear here on the timeline.</p><button className="primary" onClick={() => ctx.goto(3)}>Go to Render</button></Empty>;
  }

  const cueBlocks = (kind: "music" | "sfx") => project.cues.filter((c) => project.assets.find((a) => a.id === c.asset_id)?.kind === kind).map((c) => {
    const a = project.assets.find((x) => x.id === c.asset_id);
    return (
      <div key={c.id} className={`block ${kind}`} style={{ left: pct(sceneStart(c.scene) + c.offset_ms), width: pct(cueLen(c, a)) }} title={a?.name}>
        {a && <div className="wave"><Waveform projectId={project.id} file={a.file} bins={80} tone="gold" fill label={`Waveform of ${a.name}`} /></div>}
        <span className="lbl">{a?.name}</span>
      </div>
    );
  });

  return (
    <div>
      <div className="card">
        <div className="row" style={{ marginBottom: 8 }}>
          <h2 style={{ margin: 0 }}>Timeline</h2><span className="muted small">Total {fmtMs(layout.total_ms)}</span>
        </div>
        <div className="timeline" role="group" aria-label="Timeline of dialogue, effects and music">
          {layout.scenes.map((s) => <div key={s.index} className="scene-mark" style={{ left: pct(s.start_ms) }}>{s.index + 1}. {s.title}</div>)}
          <div className="track-label">Dialogue</div>
          <div className="track">{layout.lines.map((l) => {
            const t = project.takes.find((x) => x.id === l.take_id);
            return (
              <div key={l.key} className="block" title={`${l.speaker} ${l.key}`} style={{ left: pct(l.start_ms), width: pct(l.end_ms - l.start_ms) }}>
                {t && <div className="wave"><Waveform projectId={project.id} file={t.file} bins={48} fill label={`Waveform of ${l.speaker}, ${l.key}`} /></div>}
                <span className="lbl">{l.speaker}</span>
              </div>
            );
          })}</div>
          <div className="track-label">Effects</div>
          <div className="track">{cueBlocks("sfx")}</div>
          <div className="track-label">Music</div>
          <div className="track">{cueBlocks("music")}</div>
        </div>
        <p className="small muted" style={{ marginBottom: 0, marginTop: 8 }}>Dialogue follows your script with the gaps you set. With no music or effects, the mix is the voice track alone.</p>
      </div>

      {layout.scenes.length > 0 && gens.some((g) => g.ready) && (
        <div className="card" role="group" aria-label="Sound for your scenes">
          <h2>Sound for your scenes</h2>
          <p className="small muted" style={{ marginTop: 0 }}>Soundstage drafts a description of the background and the music for each scene from its heading. Change the words if you like, then press Make. The sound is placed in that scene, loops to fill it, and sits quietly under the dialogue.</p>
          {layout.scenes.map((s) => (
            <div key={s.index} className="linecard">
              <strong>{s.index + 1}. {s.title}</strong>
              {(["sfx", "music"] as const).map((kind) => (
                <div key={kind} className="row" style={{ alignItems: "end", marginTop: 6 }}>
                  <div style={{ flex: 1, minWidth: 260 }}>
                    <label htmlFor={`sc-${s.index}-${kind}`}>{kind === "sfx" ? "Background sound" : "Music"}</label>
                    <input id={`sc-${s.index}-${kind}`} type="text" style={{ width: "100%" }} maxLength={300} value={sceneDraft(s.index, kind)} onChange={(e) => setSceneText({ ...sceneText, [`${s.index}-${kind}`]: e.target.value })} />
                  </div>
                  <button className="small" disabled={sceneBusy !== null} onClick={() => setSceneAsk({ index: s.index, kind, prompt: sceneDraft(s.index, kind) })}>{sceneBusy === `${s.index}-${kind}` ? "Making…" : "Make"}</button>
                </div>
              ))}
            </div>
          ))}
        </div>
      )}

      <div className="grid2" style={{ gridTemplateColumns: "minmax(0, 1fr) minmax(0, 1fr)" }}>
        <div className="card">
          <h2>Music and effects</h2>
          <div className="row" style={{ marginBottom: 12 }}>
            <button onClick={() => void importAsset("music")}><I.Music size={16} /> Import music</button>
            <button onClick={() => void importAsset("sfx")}><I.Wave size={16} /> Import effect</button>
          </div>
          <div className="linecard" role="group" aria-label="Create music or an effect">
            <strong>Create with AI</strong>
            {gens.length === 0 ? <p className="small muted" style={{ margin: "4px 0 0" }}>No generator is set up. Add ElevenLabs, Kie (Suno music), fal or ComfyUI under Voice engines, and it appears here.</p> : (
              <>
                <div className="row" style={{ alignItems: "end", marginTop: 8 }}>
                  <div><label htmlFor="g-kind">Make</label><select id="g-kind" value={gen.kind} onChange={(e) => setGen({ ...gen, kind: e.target.value as "music" | "sfx", id: "" })}><option value="music">Music</option><option value="sfx">A sound effect</option></select></div>
                  <div><label htmlFor="g-src">With</label><select id="g-src" value={chosen?.id ?? ""} onChange={(e) => setGen({ ...gen, id: e.target.value })}>{usable.map((g) => <option key={g.id} value={g.id}>{g.label}{g.ready ? "" : " (set up first)"}</option>)}</select></div>
                  {limits?.supports_length && <div><label htmlFor="g-len">Length {gen.seconds}s</label><input id="g-len" type="range" min={limits.min_s} max={limits.max_s} step={1} value={Math.min(limits.max_s, Math.max(limits.min_s, gen.seconds))} onChange={(e) => setGen({ ...gen, seconds: Number(e.target.value) })} /></div>}
                </div>
                <label htmlFor="g-prompt" style={{ marginTop: 8 }}>Describe it</label>
                <textarea id="g-prompt" rows={2} maxLength={1000} style={{ width: "100%" }} value={gen.prompt} onChange={(e) => setGen({ ...gen, prompt: e.target.value })} placeholder={gen.kind === "music" ? "Warm piano and soft strings, slow, hopeful" : "Heavy wooden door creaking open"} />
                <PromptHelper kind={gen.kind} value={gen.prompt} onChange={(v) => setGen((g) => ({ ...g, prompt: v }))} toast={ctx.toast} />
                <div className="row" style={{ marginTop: 8 }}>
                  {gen.kind === "music" && <label className="inline"><input type="checkbox" checked={gen.instrumental} onChange={(e) => setGen({ ...gen, instrumental: e.target.checked })} /> No singing</label>}
                  <span className="spacer" />
                  <button className="primary small" disabled={gen.busy || !chosen?.ready || gen.prompt.trim().length < 3} onClick={() => (chosen?.sends_data ? setGen({ ...gen, confirm: true }) : void create())}>{gen.busy ? <><span className="spin" aria-hidden="true" /> Creating</> : <><I.Plus size={14} /> Create</>}</button>
                </div>
                {chosen && <p className="small faint" style={{ margin: "6px 0 0" }}>{chosen.notes} Anything created starts as “licence unknown” and is labelled AI-generated in the export.</p>}
              </>
            )}
          </div>
          {project.assets.length === 0 && <p className="muted small">Nothing in the project yet. Import a file or create something above, and the licence of each one is tracked.</p>}
          {project.assets.map((a) => (
            <div key={a.id} className="linecard">
              <div className="row">
                <strong>{a.name}</strong><span className="chip">{a.kind === "music" ? "Music" : "Effect"}</span>{a.generated && <span className="chip info">AI-generated</span>}<span className="muted small">{fmtMs(a.duration_ms)}</span>
                <LicenceChip licence={project.licence_checks[`asset:${a.id}`]?.licence ?? a.licence.licence} note={licenceNote(project.licence_checks[`asset:${a.id}`] ?? a.licence)} />
                <span className="spacer" />
                <button className="small" onClick={() => addCue(a)}><I.Plus size={14} /> Add to timeline</button>
                <button className="small ghost" aria-expanded={editing === a.id} onClick={() => setEditing(editing === a.id ? null : a.id)}>Licence</button>
                <button className="small danger icon" onClick={() => setRemoving(a)} aria-label={`Remove ${a.name}`}><I.Trash size={14} /></button>
              </div>
              <div style={{ marginTop: 8 }}><Waveform projectId={project.id} file={a.file} bins={120} height={28} tone="gold" label={`Waveform of ${a.name}`} /></div>
              {editing === a.id && <div style={{ marginTop: 8 }}><LicenceEditor initial={a.licence} onSave={(info) => { edit((p) => ({ ...p, assets: p.assets.map((x) => (x.id === a.id ? { ...x, licence: info } : x)) })); setEditing(null); }} /></div>}
            </div>
          ))}
          {project.cues.length > 0 && <h3 style={{ marginTop: 16 }}>On the timeline</h3>}
          {project.cues.map((c) => {
            const a = project.assets.find((x) => x.id === c.asset_id);
            return (
              <div key={c.id} className="linecard">
                <div className="row"><strong>{a?.name ?? "Missing asset"}</strong><span className="spacer" /><button className="small danger" onClick={() => edit((p) => ({ ...p, cues: p.cues.filter((x) => x.id !== c.id) }))}>Remove from timeline</button></div>
                <div className="row" style={{ alignItems: "end", marginTop: 8 }}>
                  <div><label htmlFor={`c-s-${c.id}`}>Starts in</label><select id={`c-s-${c.id}`} value={c.scene ?? ""} onChange={(e) => setCue(c.id, { scene: e.target.value === "" ? null : Number(e.target.value) })}><option value="">Project start</option>{layout.scenes.map((s) => <option key={s.index} value={s.index}>Scene {s.index + 1}</option>)}</select></div>
                  {advanced && <div><label htmlFor={`c-o-${c.id}`}>Offset (ms)</label><input id={`c-o-${c.id}`} type="number" min={0} max={600000} step={100} style={{ width: 96 }} value={c.offset_ms} onChange={(e) => setCue(c.id, { offset_ms: Math.max(0, Number(e.target.value) || 0) })} /></div>}
                  <div><label htmlFor={`c-g-${c.id}`}>Level {c.gain_db} dB</label><input id={`c-g-${c.id}`} type="range" min={-40} max={6} step={1} value={c.gain_db} onChange={(e) => setCue(c.id, { gain_db: Number(e.target.value) })} /></div>
                </div>
                <div className="row" style={{ marginTop: 8 }}>
                  <label className="inline"><input type="checkbox" checked={c.loop_to_scene} disabled={c.scene === null} onChange={(e) => setCue(c.id, { loop_to_scene: e.target.checked })} /> Loop to fill the scene</label>
                  <label className="inline"><input type="checkbox" checked={c.duck} onChange={(e) => setCue(c.id, { duck: e.target.checked })} /> Lower under dialogue <Help text="Turn this off for music that exists inside the scene, like a radio, so it is not turned down when people speak." /></label>
                </div>
              </div>
            );
          })}
        </div>

        <div className="card">
          <h2>Level and loudness</h2>
          <label htmlFor="dest">Where will this play? <Help text="Different places ask for different loudness. Streaming is quieter than it used to be; broadcast is stricter. Soundstage brings the whole mix to the target you pick." /></label>
          <select id="dest" value={project.mix.destination} style={{ width: "100%" }} onChange={(e) => edit((p) => ({ ...p, mix: { ...p.mix, destination: e.target.value } }))}>
            {dests.map((d) => <option key={d.id} value={d.id}>{d.label} ({d.target_lufs} LUFS)</option>)}
          </select>
          {dest && <p className="small muted" style={{ marginTop: 8 }}>Target {dest.target_lufs} LUFS, true peak under {dest.true_peak_ceiling_dbtp} dBTP. {dest.note}. Confirm the current standard for your destination before delivery.</p>}
          {advanced && (
            <div className="row" style={{ marginTop: 8 }}>
              <div><label htmlFor="duck">Ducking depth {project.mix.duck_depth_db} dB <Help text="How far music and effects are turned down while someone speaks." /></label><input id="duck" type="range" min={0} max={30} value={project.mix.duck_depth_db} onChange={(e) => edit((p) => ({ ...p, mix: { ...p.mix, duck_depth_db: Number(e.target.value) } }), "duck")} /></div>
              <div><label htmlFor="gap">Gap between lines {project.mix.gap_ms} ms</label><input id="gap" type="range" min={0} max={2000} step={50} value={project.mix.gap_ms} onChange={(e) => edit((p) => ({ ...p, mix: { ...p.mix, gap_ms: Number(e.target.value) } }), "gap")} /></div>
            </div>
          )}
          <div className="row" style={{ margin: "14px 0" }}>
            <button className="primary" disabled={ctx.mixBuilding} onClick={() => void ctx.buildMix()}>{ctx.mixBuilding ? <><span className="spin" aria-hidden="true" /> Mixing</> : <><I.Mix size={16} /> Build a preview mix</>}</button>
            <PlayButton path={preview?.path ?? null} label="the preview mix" bust={preview ? String(preview.total_ms) + preview.gain_db : undefined} />
          </div>
          {preview ? (
            <div aria-live="polite">
              <div className="mix-wave"><PeaksView peaks={preview.peaks} height={56} tone="gold" label="Waveform of the whole preview mix" /></div>
              {preview.loudness.integrated_lufs !== null && (
                <div style={{ marginTop: 12 }}>
                  <div className="meter" role="img" aria-label={`Loudness ${lufs(preview.loudness.integrated_lufs)}, target ${preview.target_lufs}`}>
                    <div style={{ width: `${Math.max(0, Math.min(100, ((preview.loudness.integrated_lufs + 40) / 40) * 100))}%` }} />
                    <span className="target" style={{ left: `${((preview.target_lufs + 40) / 40) * 100}%` }} title={`Target ${preview.target_lufs} LUFS`} />
                  </div>
                  <div className="row small faint" style={{ justifyContent: "space-between" }}><span>-40</span><span>target {preview.target_lufs}</span><span>0 LUFS</span></div>
                </div>
              )}
              <div className="stat-grid">
                <div className="stat"><div className="k">Integrated</div><div className="v">{lufs(preview.loudness.integrated_lufs)}</div></div>
                <div className="stat"><div className="k">True peak (estimated)</div><div className="v">{preview.loudness.true_peak_dbtp.toFixed(1)} dBTP</div></div>
                <div className="stat"><div className="k">Loudness range</div><div className="v">{preview.loudness.loudness_range_lu === null ? "n/a" : `${preview.loudness.loudness_range_lu.toFixed(1)} LU`}</div></div>
                <div className="stat"><div className="k">Gain applied</div><div className="v">{preview.gain_db.toFixed(1)} dB</div></div>
              </div>
              {preview.target_reached && <div className="callout ok small" style={{ marginTop: 12, marginBottom: 0 }}><strong>Target reached</strong>The master will land on the target for {dest?.label}.</div>}
              {preview.warnings.map((w, i) => <div key={i} className="callout warn small" style={{ marginTop: 12, marginBottom: 0 }} role="alert"><strong>Flagged</strong>{w}</div>)}
            </div>
          ) : <p className="small muted">Build a preview to hear the mix and see its loudness before you export. Nothing is saved outside the project.</p>}
        </div>
      </div>
      {sceneAsk && (
        <Confirm title="Send this description to be made?" confirmLabel="Send and make" onCancel={() => setSceneAsk(null)}
          body={<p>Your description (“{sceneAsk.prompt.slice(0, 120)}”) is sent over the internet to the generator you set up, and may use credits on your account there. No dialogue or audio from your project is sent.</p>}
          onConfirm={() => { const a = sceneAsk; setSceneAsk(null); void makeForScene(a.index, a.kind, a.prompt); }} />
      )}
      {gen.confirm && chosen && (
        <Confirm title={`Send this description to ${chosen.label}?`} confirmLabel="Send and create" onCancel={() => setGen({ ...gen, confirm: false })}
          body={<p>Your description is sent over the internet to {chosen.label} and may use credits on your account there. No script text or audio from your project is sent.</p>}
          onConfirm={() => void create()} />
      )}
      {removing && (
        <Confirm title={`Remove “${removing.name}”?`} confirmLabel="Remove" onCancel={() => setRemoving(null)}
          body={<p>This removes {removing.name} and any places it is used on the timeline from this project. Your original file is not touched.</p>}
          onConfirm={() => { const id = removing.id; edit((p) => ({ ...p, assets: p.assets.filter((x) => x.id !== id), cues: p.cues.filter((c) => c.asset_id !== id) })); setRemoving(null); }} />
      )}
    </div>
  );
}
