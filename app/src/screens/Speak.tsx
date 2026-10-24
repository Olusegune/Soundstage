import { save } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";
import { api, messageOf } from "../api";
import { Flag } from "../flags";
import { Confirm, LicenceChip, PlayButton } from "../ui";
import type { HistoryView, SpeakOut, Voice } from "../types";

const STYLES = ["neutral", "whisper", "shout", "excited", "sad", "tired", "sarcastic", "curious", "serious"];
const SAVE_FILTERS = [
  { name: "WAV (lossless)", extensions: ["wav"] }, { name: "MP3", extensions: ["mp3"] }, { name: "FLAC (lossless)", extensions: ["flac"] },
  { name: "AAC (M4A)", extensions: ["m4a"] }, { name: "Ogg Vorbis", extensions: ["ogg"] }, { name: "Opus", extensions: ["opus"] }, { name: "MP4 video with waveform", extensions: ["mp4"] },
];

interface Item { id: string; path: string; voice_name: string; duration_ms: number; text: string; kind: "speak" | "clean"; voice_id: string; style: string; pace: number; when: string; provider_label: string }

/** Type, pick a voice, hear it, save it. The quick way to use a voice without making a project. */
export function Speak(props: { keepHistory: boolean; voices: Voice[]; toast: (t: string, bad?: boolean) => void; onBack: () => void; ffmpegPath: string; onSettings: () => void }) {
  const [q, setQ] = useState("");
  const [voiceId, setVoiceId] = useState("");
  const [text, setText] = useState("");
  const [style, setStyle] = useState("neutral");
  const [pace, setPace] = useState(1);
  const [busy, setBusy] = useState(false);
  const [ask, setAsk] = useState(false);
  const [history, setHistory] = useState<Item[]>([]);
  const [clearing, setClearing] = useState(false);
  const [last, setLast] = useState<SpeakOut | null>(null);
  const toItem = (h: HistoryView): Item => ({ id: h.item.id, path: h.path, voice_name: h.item.voice_name, duration_ms: h.item.duration_ms, text: h.item.text, kind: h.item.kind, voice_id: h.item.voice_id, style: h.item.style, pace: h.item.pace, when: h.item.created, provider_label: h.item.provider_label });
  const reload = () => { void api.historyList().then((l) => setHistory(l.map(toItem))).catch(() => setHistory([])); };
  useEffect(() => { reload(); }, []);

  const matches = useMemo(() => {
    const t = q.trim().toLowerCase();
    return props.voices.filter((v) => !t || `${v.name} ${v.accent} ${v.provider_label} ${v.language}`.toLowerCase().includes(t));
  }, [props.voices, q]);
  const shown = matches.slice(0, 200);
  const voice = props.voices.find((v) => v.id === voiceId) ?? shown[0];
  const cloud = !!voice && voice.sends_data;

  const generate = async () => {
    if (!voice) return;
    setAsk(false); setBusy(true);
    try {
      const r = await api.speakText(voice.id, text, style === "neutral" ? null : style, pace === 1 ? null : pace, props.keepHistory);
      setLast(r);
      reload();
    } catch (e) { props.toast(messageOf(e), true); } finally { setBusy(false); }
  };

  const saveAs = async (it: { path: string; voice_name: string }) => {
    const dest = await save({ title: "Save audio", defaultPath: `${it.voice_name}.wav`, filters: SAVE_FILTERS });
    if (typeof dest !== "string") return;
    try { const p = await api.saveAudioCopy(it.path, dest, props.ffmpegPath || null); props.toast(`Saved ${p.split(/[\\/]/).pop()}.`); }
    catch (e) { props.toast(messageOf(e), true); }
  };

  return (
    <div className="page">
      <div className="stage-title"><h1>Speak</h1><span className="spacer" /><button onClick={props.onBack}>Back</button></div>
      <p className="muted">Type something, choose a voice and hear it at once. For a whole scene with several speakers, music and a final mix, make a project instead.</p>
      <div className="card">
        <div className="grid2" style={{ gridTemplateColumns: "1fr 1fr" }}>
          <div>
            <label htmlFor="sp-q">Find a voice</label>
            <input id="sp-q" type="text" style={{ width: "100%" }} placeholder={`Search ${props.voices.length} voices by name, language or provider`} value={q} onChange={(e) => setQ(e.target.value)} />
            <label htmlFor="sp-v" style={{ marginTop: 8 }}>Voice {matches.length > shown.length && <span className="small muted">(showing the first {shown.length} of {matches.length}; search to narrow)</span>}</label>
            <select id="sp-v" style={{ width: "100%" }} value={voice?.id ?? ""} onChange={(e) => setVoiceId(e.target.value)}>
              {shown.map((v) => <option key={v.id} value={v.id}>{v.name} · {v.provider_label}</option>)}
            </select>
            {voice && <div className="row small" style={{ marginTop: 8 }}><Flag locale={voice.language} name /><LicenceChip licence={voice.licence.licence} note={voice.licence.notes} />{cloud ? <span className="chip gold">Sends text over the internet</span> : <span className="chip info">On this computer</span>}</div>}
          </div>
          <div>
            <label htmlFor="sp-style">Delivery</label>
            <select id="sp-style" value={style} onChange={(e) => setStyle(e.target.value)}>{STYLES.map((s) => <option key={s} value={s}>{s === "neutral" ? "Natural" : s[0]!.toUpperCase() + s.slice(1)}</option>)}</select>
            <label htmlFor="sp-pace" style={{ marginTop: 8 }}>Speed {pace.toFixed(2)}×</label>
            <input id="sp-pace" type="range" min={0.7} max={1.4} step={0.05} value={pace} onChange={(e) => setPace(Number(e.target.value))} />
            <p className="small faint" style={{ margin: "6px 0 0" }}>Not every voice can do every delivery. If one cannot, Soundstage says so instead of ignoring it.</p>
          </div>
        </div>
        <label htmlFor="sp-text" style={{ marginTop: 12 }}>What should it say? <span className="faint small">({text.length}/5000)</span></label>
        <textarea id="sp-text" rows={6} maxLength={5000} style={{ width: "100%" }} value={text} onChange={(e) => setText(e.target.value)} placeholder="Good morning, and welcome." />
        <p className="small faint" style={{ margin: "4px 0 0" }}>You can add [pause:short], [pause:long] or, for voices that support them, [laugh] and [sigh].</p>
        <div className="row" style={{ marginTop: 12 }}>
          <button className="primary" disabled={busy || !voice || text.trim().length === 0} onClick={() => (cloud ? setAsk(true) : void generate())}>{busy ? <><span className="spin" aria-hidden="true" /> Making audio</> : "Speak it"}</button>
          {props.voices.length === 0 && <span className="small muted">No voices yet. <button className="small ghost" onClick={props.onSettings}>Open Settings</button></span>}
        </div>
      </div>

      {last && !props.keepHistory && (
        <div className="card" aria-live="polite">
          <div className="row"><strong>{last.voice_name}</strong><span className="muted small">{(last.duration_ms / 1000).toFixed(1)} s</span><span className="spacer" /><PlayButton path={last.path} label={`${last.voice_name}`} /><button className="small" onClick={() => void saveAs(last)}>Save as…</button></div>
          <p className="small faint" style={{ margin: "6px 0 0" }}>History is off, so this is not kept after you close Soundstage. Save it now to keep it.</p>
        </div>
      )}

      <div className="card">
        <div className="row"><h2 style={{ margin: 0 }}>History</h2><span className="muted small">{history.length} item{history.length === 1 ? "" : "s"}</span><span className="spacer" />
          {history.length > 0 && <button className="small danger" onClick={() => setClearing(true)}>Clear history…</button>}</div>
        {!props.keepHistory && <p className="small muted" style={{ margin: "6px 0 0" }}>New audio is not being kept. Turn history on in Settings.</p>}
        {history.length === 0 && <p className="small muted" style={{ margin: "6px 0 0" }}>Everything you make here is kept on this computer, so a good take is never lost. It appears here.</p>}
        {history.map((it) => (
          <div key={it.id} className="linecard">
            <div className="row"><strong>{it.voice_name}</strong>{it.kind === "clean" && <span className="chip">Cleaned</span>}<span className="muted small">{(it.duration_ms / 1000).toFixed(1)} s · {it.when.replace("T", " ").slice(0, 16)}</span><span className="spacer" />
              <PlayButton path={it.path} label={`${it.voice_name}: ${it.text.slice(0, 40)}`} />
              {it.kind === "speak" && <button className="small" onClick={() => { setText(it.text); setStyle(it.style); setPace(it.pace); if (props.voices.some((v) => v.id === it.voice_id)) setVoiceId(it.voice_id); window.scrollTo({ top: 0 }); }}>Use again</button>}
              <button className="small" onClick={() => void saveAs(it)}>Save as…</button>
              <button className="small danger icon" aria-label={`Delete ${it.voice_name}: ${it.text.slice(0, 40)}`} onClick={() => { void api.historyDelete(it.id).then(reload).catch((e) => props.toast(messageOf(e), true)); }}>×</button></div>
            <div className="small muted" style={{ marginTop: 4 }}>{it.text.length > 160 ? `${it.text.slice(0, 160)}…` : it.text}</div>
          </div>
        ))}
        <p className="small faint" style={{ marginBottom: 0 }}>Save as WAV, MP3, FLAC, M4A, Ogg, Opus or MP4. Audio made here is not tied to a project and starts as licence unknown, so use a project for anything you will sell. The history holds your text and audio on this computer only; keep up to 200 items.</p>
      </div>

      {ask && voice && (
        <Confirm title={`Send this text to ${voice.provider_label}?`} confirmLabel="Send and speak" onCancel={() => setAsk(false)}
          body={<p>The text ({text.trim().length} characters) is sent over the internet to {voice.provider_label} and is billed to your account there.</p>}
          onConfirm={() => void generate()} />
      )}
      {clearing && (
        <Confirm title="Clear the whole history?" confirmLabel="Clear history" onCancel={() => setClearing(false)}
          body={<p>This deletes every saved audio file and the text beside it from this computer. Files you saved with Save as are not touched.</p>}
          onConfirm={() => { setClearing(false); void api.historyClear().then(reload).catch((e) => props.toast(messageOf(e), true)); }} />
      )}
    </div>
  );
}
