import { open, save } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { api, messageOf } from "../api";
import * as I from "../icons";
import { PeaksView, PlayButton } from "../ui";
import type { CleanOut, HistoryView } from "../types";

interface Clip {
  key: number;
  path: string;
  name: string;
  duration: number;
  peaks: number[];
  start: number;
  end: number;
  offset: number;
  gain: number;
  fadeIn: number;
  fadeOut: number;
}

const SAVE_FILTERS = [{ name: "WAV (lossless)", extensions: ["wav"] }, { name: "MP3", extensions: ["mp3"] }, { name: "FLAC (lossless)", extensions: ["flac"] }, { name: "AAC (M4A)", extensions: ["m4a"] }, { name: "Ogg Vorbis", extensions: ["ogg"] }, { name: "Opus", extensions: ["opus"] }];
let counter = 0;
const num = (v: string, min: number, max: number, fallback: number) => { const n = Number(v); return Number.isFinite(n) ? Math.min(max, Math.max(min, n)) : fallback; };

/** Trim, place and layer clips, then save one file. Nothing here changes the original recordings. */
export function AudioEditor(props: { keepHistory: boolean; toast: (t: string, bad?: boolean) => void; onBack: () => void; ffmpegPath: string }) {
  const [clips, setClips] = useState<Clip[]>([]);
  const [normalize, setNormalize] = useState(false);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<CleanOut | null>(null);
  const [history, setHistory] = useState<HistoryView[]>([]);
  const [pick, setPick] = useState("");

  useEffect(() => { void api.historyList().then(setHistory).catch(() => setHistory([])); }, []);

  const add = async (path: string, name?: string) => {
    try {
      const info = await api.audioInfo(path);
      const dur = info.duration_ms / 1000;
      const end = clips.reduce((m, c) => Math.max(m, c.offset + (c.end - c.start)), 0);
      setClips((cs) => [...cs, { key: ++counter, path, name: name ?? (path.split(/[\\/]/).pop() ?? "clip"), duration: dur, peaks: info.peaks, start: 0, end: dur, offset: Math.round(end * 100) / 100, gain: 0, fadeIn: 0, fadeOut: 0 }]);
      setResult(null);
    } catch (e) { props.toast(messageOf(e), true); }
  };

  const choose = async () => {
    const p = await open({ multiple: true, title: "Choose audio files", filters: [{ name: "Audio", extensions: ["wav", "mp3", "flac", "ogg", "m4a", "aac", "mp4"] }] });
    const list = typeof p === "string" ? [p] : Array.isArray(p) ? p : [];
    for (const f of list) await add(f);
  };

  const set = (k: number, patch: Partial<Clip>) => { setClips((cs) => cs.map((c) => (c.key === k ? { ...c, ...patch } : c))); setResult(null); };
  const length = clips.reduce((m, c) => Math.max(m, c.offset + Math.max(0, c.end - c.start)), 0);

  const render = async () => {
    setBusy(true);
    try {
      setResult(await api.editAudio(clips.map((c) => ({ path: c.path, start_s: c.start, end_s: c.end >= c.duration - 0.005 ? null : c.end, offset_s: c.offset, gain_db: c.gain, fade_in_ms: c.fadeIn, fade_out_ms: c.fadeOut })), normalize, props.keepHistory));
    } catch (e) { props.toast(messageOf(e), true); } finally { setBusy(false); }
  };

  const saveAs = async () => {
    if (!result) return;
    const dest = await save({ title: "Save audio", defaultPath: "edit.wav", filters: SAVE_FILTERS });
    if (typeof dest !== "string") return;
    try { const p = await api.saveAudioCopy(result.path, dest, props.ffmpegPath || null); props.toast(`Saved ${p.split(/[\\/]/).pop()}.`); } catch (e) { props.toast(messageOf(e), true); }
  };

  return (
    <div className="page">
      <div className="stage-title"><h1>Audio editor</h1><span className="spacer" /><button onClick={props.onBack}>Back</button></div>
      <p className="muted">Trim, join and layer recordings, set levels and fades, and save one new file. Your originals are never changed. Clips that overlap are layered; to join clips, place each one where the last ends.</p>

      <div className="card">
        <div className="row" style={{ alignItems: "end" }}>
          <button className="primary" onClick={() => void choose()}><I.Plus size={16} /> Add files…</button>
          {history.length > 0 && (
            <>
              <div><label htmlFor="ae-h">Or add something you made</label>
                <select id="ae-h" value={pick} onChange={(e) => setPick(e.target.value)}><option value="">Choose from your history…</option>{history.map((h) => <option key={h.item.id} value={h.item.id}>{h.item.voice_name} · {h.item.text.slice(0, 40)}</option>)}</select></div>
              <button className="small" disabled={!pick} onClick={() => { const h = history.find((x) => x.item.id === pick); if (h) void add(h.path, `${h.item.voice_name}: ${h.item.text.slice(0, 24)}`); setPick(""); }}>Add</button>
            </>
          )}
          <span className="spacer" /><span className="muted small">{clips.length} clip{clips.length === 1 ? "" : "s"} · {length.toFixed(1)} s</span>
        </div>
      </div>

      {clips.length === 0 && <div className="card flat"><p className="muted" style={{ margin: 0 }}>No clips yet. Add one or more recordings to start.</p></div>}
      {clips.map((c, i) => (
        <div key={c.key} className="card" role="group" aria-label={`Clip ${i + 1}: ${c.name}`}>
          <div className="row"><strong>{i + 1}. {c.name}</strong><span className="muted small">{c.duration.toFixed(1)} s</span><span className="spacer" /><PlayButton path={c.path} label={c.name} />
            <button className="small danger icon" aria-label={`Remove ${c.name}`} onClick={() => { setClips((cs) => cs.filter((x) => x.key !== c.key)); setResult(null); }}><I.Trash size={14} /></button></div>
          <div style={{ margin: "8px 0" }}><PeaksView peaks={c.peaks} height={44} tone="gold" label={`Waveform of ${c.name}`} /></div>
          <div className="row" style={{ alignItems: "end" }}>
            <div><label htmlFor={`s-${c.key}`}>Start (s)</label><input id={`s-${c.key}`} type="number" min={0} max={c.duration} step={0.1} style={{ width: 84 }} value={c.start} onChange={(e) => set(c.key, { start: num(e.target.value, 0, c.end - 0.05, c.start) })} /></div>
            <div><label htmlFor={`e-${c.key}`}>End (s)</label><input id={`e-${c.key}`} type="number" min={0} max={c.duration} step={0.1} style={{ width: 84 }} value={Number(c.end.toFixed(2))} onChange={(e) => set(c.key, { end: num(e.target.value, c.start + 0.05, c.duration, c.end) })} /></div>
            <div><label htmlFor={`o-${c.key}`}>Place at (s)</label><input id={`o-${c.key}`} type="number" min={0} max={1800} step={0.1} style={{ width: 84 }} value={c.offset} onChange={(e) => set(c.key, { offset: num(e.target.value, 0, 1800, c.offset) })} /></div>
            <div><label htmlFor={`g-${c.key}`}>Level {c.gain} dB</label><input id={`g-${c.key}`} type="range" min={-30} max={12} step={1} value={c.gain} onChange={(e) => set(c.key, { gain: Number(e.target.value) })} /></div>
            <div><label htmlFor={`fi-${c.key}`}>Fade in (ms)</label><input id={`fi-${c.key}`} type="number" min={0} max={10000} step={50} style={{ width: 84 }} value={c.fadeIn} onChange={(e) => set(c.key, { fadeIn: num(e.target.value, 0, 10000, c.fadeIn) })} /></div>
            <div><label htmlFor={`fo-${c.key}`}>Fade out (ms)</label><input id={`fo-${c.key}`} type="number" min={0} max={10000} step={50} style={{ width: 84 }} value={c.fadeOut} onChange={(e) => set(c.key, { fadeOut: num(e.target.value, 0, 10000, c.fadeOut) })} /></div>
            {i > 0 && <button className="small" onClick={() => { const prev = clips[i - 1]!; set(c.key, { offset: Math.round((prev.offset + (prev.end - prev.start)) * 100) / 100 }); }}>Place after clip {i}</button>}
          </div>
        </div>
      ))}

      {clips.length > 0 && (
        <div className="card">
          <div className="row" style={{ alignItems: "center" }}>
            <label className="inline"><input type="checkbox" checked={normalize} onChange={(e) => { setNormalize(e.target.checked); setResult(null); }} /> Make it as loud as is safe</label>
            <span className="spacer" />
            <button className="primary" disabled={busy} onClick={() => void render()}>{busy ? <><span className="spin" aria-hidden="true" /> Making the file</> : "Make the file"}</button>
          </div>
          {result && (
            <div className="row" style={{ marginTop: 10 }} aria-live="polite"><strong>Done</strong><span className="muted small">{(result.duration_ms / 1000).toFixed(1)} s</span><span className="spacer" /><PlayButton path={result.path} label="the edited audio" /><button className="small" onClick={() => void saveAs()}>Save as…</button></div>
          )}
          <p className="small faint" style={{ marginBottom: 0 }}>Layers that would be too loud are turned down automatically so nothing distorts. The result starts as licence unknown, so use a project for anything you will sell.</p>
        </div>
      )}
    </div>
  );
}
