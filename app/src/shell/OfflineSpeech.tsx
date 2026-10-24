import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { api, messageOf } from "../api";
import * as I from "../icons";
import { Confirm } from "../ui";
import type { ModelInfo, WhisperStatus } from "../types";

/** Offline speech to text: pick a model, download it once, and transcribe with nothing sent anywhere. */
export function OfflineSpeech(props: { toast: (t: string, bad?: boolean) => void; onChanged: () => void }) {
  const [st, setSt] = useState<WhisperStatus | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [pct, setPct] = useState(0);
  const [ask, setAsk] = useState<ModelInfo | null>(null);
  const [open, setOpen] = useState(false);

  const load = useCallback(() => { void api.whisperStatus().then(setSt).catch(() => setSt(null)); }, []);
  useEffect(() => { load(); }, [load]);
  useEffect(() => {
    let off: (() => void) | undefined;
    void listen<{ id: string; done: number; total: number }>("whisper-progress", (e) => setPct(e.payload.total ? Math.round((100 * e.payload.done) / e.payload.total) : 0)).then((u) => { off = u; });
    return () => off?.();
  }, []);

  const download = async (m: ModelInfo) => {
    setAsk(null); setBusy(m.id); setPct(0);
    try { await api.whisperDownload(m.id); props.toast(`${m.label} is ready. Transcription can now run on this computer.`); load(); props.onChanged(); }
    catch (e) { props.toast(messageOf(e), true); } finally { setBusy(null); }
  };
  const remove = async (m: ModelInfo) => { try { await api.whisperRemove(m.id); load(); props.onChanged(); } catch (e) { props.toast(messageOf(e), true); } };

  if (!st) return null;
  const any = st.models.some((m) => m.installed);
  return (
    <div className="card" role="group" aria-label="Offline speech to text">
      <div className="row">
        <h2 style={{ margin: 0 }}>Offline speech to text</h2>
        {any ? <span className="chip ok"><I.Check size={12} /> Ready</span> : <span className="chip">Not set up</span>}
        <span className="chip info"><I.Device size={12} /> Nothing leaves this computer</span>
        <span className="spacer" />
        <button className="small" aria-expanded={open || !any} onClick={() => setOpen(!open)}>{open || !any ? "Hide models" : "Manage models"}</button>
      </div>
      {!st.program && <div className="callout warn small" style={{ marginTop: 8 }}><strong>The speech program is missing.</strong>Reinstall Soundstage to get it back.</div>}
      {(open || !any) && (
        <>
          <p className="small muted" style={{ margin: "8px 0" }}>Whisper turns speech into text on your own computer, with no account and no internet once a model is downloaded. Download one model once; it stays until you remove it.</p>
          {st.models.map((m) => (
            <div key={m.id} className="linecard">
              <div className="row"><strong>{m.label}</strong><span className="muted small">{m.mb} MB</span>{m.installed && <span className="chip ok">Installed</span>}<span className="spacer" />
                {m.installed
                  ? <button className="small danger" onClick={() => void remove(m)}>Remove</button>
                  : <button className="small" disabled={busy !== null || !st.program} onClick={() => setAsk(m)}>{busy === m.id ? `Downloading… ${pct}%` : "Download"}</button>}
              </div>
              <div className="small muted" style={{ marginTop: 4 }}>{m.blurb}</div>
              {busy === m.id && <div className="bar" role="progressbar" aria-label={`Downloading ${m.label}`} aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct} style={{ marginTop: 6 }}><div style={{ width: `${pct}%` }} /></div>}
            </div>
          ))}
          <p className="small faint" style={{ marginBottom: 0 }}>Speed depends on your computer: a few minutes of speech takes about as long again with the faster models. Accuracy is good for clear speech and weaker for names, accents and smaller languages, so read the result.</p>
        </>
      )}
      {ask && (
        <Confirm title={`Download “${ask.label}”?`} confirmLabel={`Download ${ask.mb} MB`} danger={false} onCancel={() => setAsk(null)}
          body={<p>This downloads one file ({ask.mb} MB) from huggingface.co, where the Whisper project publishes its models. Soundstage checks its size and fingerprint before using it. Nothing you have made is sent.</p>}
          onConfirm={() => void download(ask)} />
      )}
    </div>
  );
}
