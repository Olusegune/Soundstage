import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useState } from "react";
import { api, messageOf } from "../api";
import type { PackReport } from "../types";

interface Clip { path: string; label: string }
interface Group { name: string; clips: Clip[] }

const stem = (p: string) => (p.split(/[\\/]/).pop() ?? p).replace(/\.[^.]+$/, "");
const AUDIO = [{ name: "Audio", extensions: ["wav", "mp3", "flac", "ogg", "m4a", "aac"] }];

/** Make a blind, loudness-matched listening test: random codes, a different order for each listener, and a private key. */
export function ListeningPack(props: { toast: (t: string, bad?: boolean) => void }) {
  const [groups, setGroups] = useState<Group[]>([{ name: "Scene 1", clips: [] }]);
  const [listeners, setListeners] = useState(6);
  const [target, setTarget] = useState(-23);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<PackReport | null>(null);

  const setGroup = (i: number, patch: Partial<Group>) => setGroups((g) => g.map((x, k) => (k === i ? { ...x, ...patch } : x)));
  const add = async (i: number) => {
    const sel = await open({ multiple: true, title: "Choose the clips for this scene", filters: AUDIO });
    const paths = Array.isArray(sel) ? sel : sel ? [sel] : [];
    if (paths.length) setGroup(i, { clips: [...(groups[i]?.clips ?? []), ...paths.map((p) => ({ path: p, label: stem(p) }))] });
  };
  const ready = groups.length > 0 && groups.every((g) => g.name.trim() && g.clips.length >= 2 && g.clips.every((c) => c.label.trim()));
  const total = groups.reduce((n, g) => n + g.clips.length, 0);

  const make = async () => {
    const dest = await open({ directory: true, title: "Where should the listening pack go?" });
    if (typeof dest !== "string") return;
    setBusy(true);
    setDone(null);
    try {
      const flat = groups.flatMap((g) => g.clips.map((c) => ({ group: g.name.trim(), label: c.label.trim(), path: c.path })));
      const r = await api.makeListeningPack(flat, dest, listeners, target);
      setDone(r);
      props.toast("The listening pack is ready.");
    } catch (e) { props.toast(messageOf(e), true); } finally { setBusy(false); }
  };

  return (
    <div className="card" role="region" aria-label="Listening pack">
      <p className="muted small" style={{ marginTop: 0 }}>Make a fair listening test. Choose a few short clips for each scene (for example plain, planned, and planned with sounds). Soundstage matches their loudness, gives each listener their own folder with the clips under random codes in a random order, and writes a score sheet for each listener and a private key for you. Nothing here is sent anywhere.</p>
      {groups.map((g, i) => (
        <fieldset key={i} style={{ border: "1px solid var(--line)", borderRadius: 8, padding: 10, margin: "0 0 10px" }}>
          <legend className="small" style={{ fontWeight: 600 }}>Scene {i + 1}</legend>
          <div className="row" style={{ alignItems: "end" }}>
            <div><label htmlFor={`lg-${i}`}>Scene name</label><input id={`lg-${i}`} type="text" value={g.name} maxLength={60} onChange={(e) => setGroup(i, { name: e.target.value })} /></div>
            <button className="small" onClick={() => void add(i)}>Add clips…</button>
            {groups.length > 1 && <button className="small ghost" onClick={() => setGroups((x) => x.filter((_, k) => k !== i))}>Remove scene</button>}
          </div>
          {g.clips.length === 0 && <p className="small muted" style={{ margin: "6px 0 0" }}>Add at least two clips to compare.</p>}
          {g.clips.map((c, k) => (
            <div className="row tight" key={k} style={{ alignItems: "center", marginTop: 6 }}>
              <label className="small" htmlFor={`ll-${i}-${k}`} style={{ minWidth: 90 }}>What it is</label>
              <input id={`ll-${i}-${k}`} type="text" value={c.label} maxLength={80} style={{ flex: 1, minWidth: 160 }} onChange={(e) => setGroup(i, { clips: g.clips.map((x, j) => (j === k ? { ...x, label: e.target.value } : x)) })} />
              <span className="small muted" title={c.path} style={{ maxWidth: 220, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{stem(c.path)}</span>
              <button className="small ghost" aria-label={`Remove ${c.label}`} onClick={() => setGroup(i, { clips: g.clips.filter((_, j) => j !== k) })}>Remove</button>
            </div>
          ))}
        </fieldset>
      ))}
      <div className="row" style={{ alignItems: "end" }}>
        <button className="small" disabled={groups.length >= 10} onClick={() => setGroups((g) => [...g, { name: `Scene ${g.length + 1}`, clips: [] }])}>Add a scene</button>
        <div><label htmlFor="lp-n">Listeners</label><input id="lp-n" type="number" min={1} max={12} value={listeners} onChange={(e) => setListeners(Math.max(1, Math.min(12, Number(e.target.value) || 1)))} style={{ width: 80 }} /></div>
        <div><label htmlFor="lp-t">Loudness</label>
          <select id="lp-t" value={target} onChange={(e) => setTarget(Number(e.target.value))}>
            <option value={-23}>-23 LUFS (comfortable on headphones)</option>
            <option value={-16}>-16 LUFS (podcast)</option>
            <option value={-14}>-14 LUFS (streaming)</option>
          </select></div>
        <button className="primary" disabled={!ready || busy} onClick={() => void make()}>{busy ? <><span className="spin" aria-hidden="true" /> Making the pack</> : `Make the pack (${total} clips)`}</button>
      </div>
      {!ready && <p className="small muted" style={{ margin: "8px 0 0" }}>Every scene needs a name and at least two clips, each with a label.</p>}
      {done && (
        <div className="callout" style={{ marginTop: 12 }} role="status">
          <strong>Ready: {done.listeners} listener folder{done.listeners === 1 ? "" : "s"}, {done.files_written} files</strong>
          Give each listener only their own folder (listener-L1, L2 and so on). Keep PRIVATE-KEY-do-not-share.csv to yourself until every score sheet is back.
          {done.notes.map((n, i) => <div key={i} className="small">{n}</div>)}
          <div className="row" style={{ marginTop: 8 }}><button className="small" onClick={() => void revealItemInDir(done.key_file)}>Show the folder</button></div>
        </div>
      )}
    </div>
  );
}
