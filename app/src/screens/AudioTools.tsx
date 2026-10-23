import { open, save } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { api, messageOf } from "../api";
import { Confirm, PlayButton } from "../ui";
import type { CleanOut, ToolInfo, TranscriptOut } from "../types";

type Tab = "transcribe" | "clean";

const LANGS: [string, string][] = [["", "Detect automatically"], ["en", "English"], ["yo", "Yoruba"], ["ig", "Igbo"], ["ha", "Hausa"], ["hi", "Hindi"], ["ta", "Tamil"], ["tl", "Tagalog"], ["ko", "Korean"], ["pt", "Portuguese"], ["es", "Spanish"], ["fr", "French"], ["de", "German"], ["ar", "Arabic"], ["ja", "Japanese"], ["zh", "Chinese"]];
const SAVE_FILTERS = [{ name: "WAV (lossless)", extensions: ["wav"] }, { name: "MP3", extensions: ["mp3"] }, { name: "FLAC (lossless)", extensions: ["flac"] }, { name: "AAC (M4A)", extensions: ["m4a"] }];

/** Speech to text, and removing noise and music from a recording. */
export function AudioTools(props: { toast: (t: string, bad?: boolean) => void; onBack: () => void; ffmpegPath: string; onSettings: () => void; onEngines: () => void }) {
  const [tools, setTools] = useState<ToolInfo[]>([]);
  const [tab, setTab] = useState<Tab>("transcribe");
  const [file, setFile] = useState<string | null>(null);
  const [lang, setLang] = useState("");
  const [engine, setEngine] = useState("");
  const [busy, setBusy] = useState(false);
  const [ask, setAsk] = useState(false);
  const [tr, setTr] = useState<TranscriptOut | null>(null);
  const [clean, setClean] = useState<CleanOut | null>(null);

  useEffect(() => { void api.listAudioTools().then(setTools).catch(() => setTools([])); }, []);
  const capable = tools.filter((t) => (tab === "transcribe" ? t.transcribe : t.isolate));
  const chosen = capable.find((t) => t.id === engine) ?? capable.find((t) => t.ready) ?? capable[0];

  const pick = async () => {
    const p = await open({ multiple: false, title: tab === "transcribe" ? "Choose a recording to transcribe" : "Choose a recording to clean", filters: [{ name: "Audio", extensions: ["wav", "mp3", "flac", "ogg", "m4a", "aac", "mp4"] }] });
    if (typeof p === "string") { setFile(p); setTr(null); setClean(null); }
  };

  const run = async () => {
    if (!chosen || !file) return;
    setAsk(false); setBusy(true);
    try {
      if (tab === "transcribe") setTr(await api.transcribeAudio(chosen.id, file, lang || null));
      else setClean(await api.cleanAudio(chosen.id, file));
    } catch (e) { props.toast(messageOf(e), true); } finally { setBusy(false); }
  };

  const saveText = async (kind: "txt" | "srt" | "vtt") => {
    if (!tr) return;
    const dest = await save({ title: "Save", defaultPath: `transcript.${kind}`, filters: [{ name: kind.toUpperCase(), extensions: [kind] }] });
    if (typeof dest !== "string") return;
    try { await api.saveTextCopy(dest, kind === "txt" ? tr.text : kind === "srt" ? tr.srt : tr.vtt); props.toast("Saved."); } catch (e) { props.toast(messageOf(e), true); }
  };
  const saveAudio = async () => {
    if (!clean) return;
    const dest = await save({ title: "Save cleaned audio", defaultPath: "cleaned.wav", filters: SAVE_FILTERS });
    if (typeof dest !== "string") return;
    try { const p = await api.saveAudioCopy(clean.path, dest, props.ffmpegPath || null); props.toast(`Saved ${p.split(/[\\/]/).pop()}.`); } catch (e) { props.toast(messageOf(e), true); }
  };

  return (
    <div className="page">
      <div className="stage-title"><h1>Audio tools</h1><span className="spacer" /><button onClick={props.onBack}>Back</button></div>
      <div className="seg" role="radiogroup" aria-label="Tool" style={{ marginBottom: 12 }}>
        <button role="radio" aria-checked={tab === "transcribe"} onClick={() => { setTab("transcribe"); setEngine(""); }}>Speech to text</button>
        <button role="radio" aria-checked={tab === "clean"} onClick={() => { setTab("clean"); setEngine(""); }}>Clean audio</button>
      </div>

      <div className="card">
        <p className="muted small" style={{ marginTop: 0 }}>{tab === "transcribe"
          ? "Turn a recording into text and subtitles (SRT or WebVTT), with the time of every word. Recordings up to 25 minutes."
          : "Remove background noise and music and keep the voice. Your original file is not changed. Recordings up to 10 minutes."}</p>
        {capable.length === 0 ? (
          <div className="callout small"><strong>{tab === "transcribe" ? "No engine can transcribe yet." : "No engine can clean audio yet."}</strong>
            {tab === "transcribe" ? "Save an ElevenLabs key in Settings, or add a local server with a Whisper model under Voice engines (set its transcription model)." : "Save an ElevenLabs key in Settings."}
            <div className="row" style={{ marginTop: 8 }}><button className="small" onClick={props.onSettings}>Open Settings</button><button className="small" onClick={props.onEngines}>Voice engines</button></div></div>
        ) : (
          <>
            <div className="row" style={{ alignItems: "end" }}>
              <button onClick={() => void pick()}>{file ? "Choose a different file…" : "Choose a recording…"}</button>
              {file && <span className="small mono muted">{file.split(/[\\/]/).pop()}</span>}
            </div>
            <div className="row" style={{ alignItems: "end", marginTop: 10 }}>
              {capable.length > 1 && <div><label htmlFor="at-e">With</label><select id="at-e" value={chosen?.id ?? ""} onChange={(e) => setEngine(e.target.value)}>{capable.map((t) => <option key={t.id} value={t.id}>{t.label}{t.ready ? "" : " (set up first)"}</option>)}</select></div>}
              {tab === "transcribe" && <div><label htmlFor="at-l">Language</label><select id="at-l" value={lang} onChange={(e) => setLang(e.target.value)}>{LANGS.map(([c, n]) => <option key={c} value={c}>{n}</option>)}</select></div>}
              <button className="primary" disabled={busy || !file || !chosen?.ready} onClick={() => (chosen?.sends_data ? setAsk(true) : void run())}>{busy ? <><span className="spin" aria-hidden="true" /> Working</> : tab === "transcribe" ? "Transcribe" : "Clean it"}</button>
            </div>
            {chosen && chosen.sends_data && <p className="small faint" style={{ margin: "8px 0 0" }}>The recording is sent over the internet to {chosen.label}.</p>}
          </>
        )}
      </div>

      {tab === "transcribe" && tr && (
        <div className="card" aria-live="polite">
          <div className="row"><h2 style={{ margin: 0 }}>Transcript</h2>{tr.language && <span className="chip">{tr.language}</span>}<span className="muted small">{Math.round(tr.seconds)} s of audio</span><span className="spacer" />
            <button className="small" onClick={() => { void navigator.clipboard?.writeText(tr.text).then(() => props.toast("Copied."), () => props.toast("Could not copy.", true)); }}>Copy</button>
            <button className="small" onClick={() => void saveText("txt")}>Save text…</button>
            <button className="small" disabled={!tr.has_times} onClick={() => void saveText("srt")}>Save SRT…</button>
            <button className="small" disabled={!tr.has_times} onClick={() => void saveText("vtt")}>Save VTT…</button></div>
          <textarea readOnly rows={12} style={{ width: "100%", marginTop: 8 }} value={tr.text} aria-label="Transcript text" />
          {!tr.has_times && <p className="small faint" style={{ marginBottom: 0 }}>This engine did not return word times, so subtitles are not available.</p>}
        </div>
      )}

      {tab === "clean" && clean && (
        <div className="card" aria-live="polite">
          <div className="row"><h2 style={{ margin: 0 }}>Cleaned audio</h2><span className="muted small">{(clean.duration_ms / 1000).toFixed(1)} s</span><span className="spacer" /><PlayButton path={clean.path} label="the cleaned audio" /><button className="small" onClick={() => void saveAudio()}>Save as…</button></div>
        </div>
      )}

      {ask && chosen && (
        <Confirm title={`Send this recording to ${chosen.label}?`} confirmLabel="Send" onCancel={() => setAsk(false)}
          body={<p>The recording is converted on this computer and sent over the internet to {chosen.label}. It may use credits on your account there.</p>}
          onConfirm={() => void run()} />
      )}
    </div>
  );
}
