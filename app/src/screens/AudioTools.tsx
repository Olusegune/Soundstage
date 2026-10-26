import { open, save } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";
import { api, messageOf } from "../api";
import { loadPrefs } from "../prefs";
import { OfflineSpeech } from "../shell/OfflineSpeech";
import { ListeningPack } from "./ListeningPack";
import { ConsentFields, EMPTY_ATTESTATION, consentComplete } from "../shell/ConsentFields";
import { Confirm, PlayButton } from "../ui";
import type { Attestation, CleanOut, CloneInfo, Cue2, ToolInfo, TranscriptOut, Voice } from "../types";

type Tab = "transcribe" | "clean" | "change" | "dub";

const LANGS: [string, string][] = [["", "Detect automatically"], ["en", "English"], ["yo", "Yoruba"], ["ig", "Igbo"], ["ha", "Hausa"], ["hi", "Hindi"], ["ta", "Tamil"], ["tl", "Tagalog"], ["ko", "Korean"], ["pt", "Portuguese"], ["es", "Spanish"], ["fr", "French"], ["de", "German"], ["ar", "Arabic"], ["ja", "Japanese"], ["zh", "Chinese"]];
const TARGETS = ["English", "French", "Spanish", "Portuguese", "German", "Italian", "Dutch", "Polish", "Turkish", "Russian", "Arabic", "Hindi", "Tamil", "Japanese", "Korean", "Chinese (Simplified)", "Vietnamese", "Indonesian", "Tagalog", "Swahili", "Yoruba", "Igbo", "Hausa"];
const SAVE_FILTERS = [{ name: "WAV (lossless)", extensions: ["wav"] }, { name: "MP3", extensions: ["mp3"] }, { name: "FLAC (lossless)", extensions: ["flac"] }, { name: "AAC (M4A)", extensions: ["m4a"] }];
const BLURB: Record<Tab, string> = {
  transcribe: "Turn a recording into text and subtitles (SRT or WebVTT), with the time of every word. Recordings up to 25 minutes.",
  clean: "Remove background noise and music and keep the voice. Your original file is not changed. Recordings up to 10 minutes.",
  change: "Re-speak a recording in a different voice, keeping the words, timing and feeling. Needs the speaker's consent. Recordings up to 10 minutes.",
  dub: "Transcribe a recording, translate it into another language, then save translated subtitles or start a project with the translated script and cast it in any voice. The timing of the new voice is not matched to the original.",
};

/** Speech to text, cleaning, changing a voice, and a dubbing helper. */
export function AudioTools(props: { keepHistory: boolean; voices: Voice[]; toast: (t: string, bad?: boolean) => void; onBack: () => void; ffmpegPath: string; onSettings: () => void; onEngines: () => void; onOpenProject: (id: string) => void }) {
  const [tools, setTools] = useState<ToolInfo[]>([]);
  const [info, setInfo] = useState<CloneInfo | null>(null);
  const [tab, setTab] = useState<Tab>("transcribe");
  const [listen, setListen] = useState(false);
  const [file, setFile] = useState<string | null>(null);
  const [lang, setLang] = useState("");
  const [engine, setEngine] = useState("");
  const [busy, setBusy] = useState(false);
  const [ask, setAsk] = useState(false);
  const [tr, setTr] = useState<TranscriptOut | null>(null);
  const [clean, setClean] = useState<CleanOut | null>(null);
  const [att, setAtt] = useState<Attestation>(EMPTY_ATTESTATION);
  const [vq, setVq] = useState("");
  const [voiceId, setVoiceId] = useState("");
  const [target, setTarget] = useState("French");
  const [cues, setCues] = useState<Cue2[]>([]);
  const [translated, setTranslated] = useState<string[] | null>(null);

  useEffect(() => { void api.listAudioTools().then(setTools).catch(() => setTools([])); void api.cloneInfo().then(setInfo).catch(() => setInfo(null)); }, []);
  const capable = tools.filter((t) => (tab === "clean" ? t.isolate : tab === "change" ? t.convert : t.transcribe));
  const chosen = capable.find((t) => t.id === engine) ?? capable.find((t) => t.ready) ?? capable[0];

  const changeVoices = useMemo(() => {
    const t = vq.trim().toLowerCase();
    return props.voices.filter((v) => v.provider === chosen?.id && (!t || `${v.name} ${v.accent} ${v.language}`.toLowerCase().includes(t))).slice(0, 200);
  }, [props.voices, chosen?.id, vq]);
  const target_voice = changeVoices.find((v) => v.id === voiceId) ?? changeVoices[0];

  const switchTab = (t: Tab) => { setListen(false); setTab(t); setEngine(""); setFile(null); setTr(null); setClean(null); setCues([]); setTranslated(null); };

  const pick = async () => {
    const p = await open({ multiple: false, title: "Choose a recording", filters: [{ name: "Audio", extensions: ["wav", "mp3", "flac", "ogg", "m4a", "aac", "mp4"] }] });
    if (typeof p === "string") { setFile(p); setTr(null); setClean(null); setCues([]); setTranslated(null); }
  };

  const run = async () => {
    if (!chosen || !file) return;
    setAsk(false); setBusy(true);
    try {
      if (tab === "transcribe") setTr(await api.transcribeAudio(chosen.id, file, lang || null));
      else if (tab === "clean") setClean(await api.cleanAudio(chosen.id, file, props.keepHistory));
      else if (tab === "change") { if (!target_voice) return; setClean(await api.changeVoice(target_voice.id, file, att, props.keepHistory)); }
      else { const t = await api.transcribeAudio(chosen.id, file, lang || null); setTr(t); setCues(t.cues); setTranslated(null); }
    } catch (e) { props.toast(messageOf(e), true); } finally { setBusy(false); }
  };

  const translate = async () => {
    const p = loadPrefs();
    if (!p.assistantModel.trim()) { props.toast("Set up the AI helper in Settings first (a chat server such as Ollama and its model name).", true); return; }
    setBusy(true);
    try { setTranslated(await api.translateCues(p.assistantUrl, p.assistantModel, target, cues.map((c) => c.text))); }
    catch (e) { props.toast(messageOf(e), true); } finally { setBusy(false); }
  };

  const outCues: Cue2[] = cues.map((c, i) => ({ ...c, text: translated?.[i] ?? c.text }));
  const saveText = async (kind: "txt" | "srt" | "vtt", from: "original" | "translated" = "original") => {
    if (!tr) return;
    const dest = await save({ title: "Save", defaultPath: `${from === "translated" ? "translation" : "transcript"}.${kind}`, filters: [{ name: kind.toUpperCase(), extensions: [kind] }] });
    if (typeof dest !== "string") return;
    try {
      const content = kind === "txt" ? (from === "translated" ? outCues.map((c) => c.text).join("\n") : tr.text) : from === "translated" ? await api.cuesSubtitles(outCues, kind) : kind === "srt" ? tr.srt : tr.vtt;
      await api.saveTextCopy(dest, content); props.toast("Saved.");
    } catch (e) { props.toast(messageOf(e), true); }
  };
  const saveAudio = async () => {
    if (!clean) return;
    const dest = await save({ title: "Save audio", defaultPath: tab === "change" ? "changed-voice.wav" : "cleaned.wav", filters: SAVE_FILTERS });
    if (typeof dest !== "string") return;
    try { const p = await api.saveAudioCopy(clean.path, dest, props.ffmpegPath || null); props.toast(`Saved ${p.split(/[\\/]/).pop()}.`); } catch (e) { props.toast(messageOf(e), true); }
  };
  const makeProject = async () => {
    setBusy(true);
    try {
      const base = (file?.split(/[\\/]/).pop() ?? "recording").replace(/\.[^.]+$/, "").slice(0, 40);
      const p = await api.createProject(`Dub: ${base} (${target})`, false);
      const script = `# ${base}\n${outCues.filter((c) => c.text.trim()).map((c) => `NARRATOR: ${c.text.trim()}`).join("\n")}\n`;
      await api.saveProject({ ...p, script });
      props.toast("Project created. Cast a voice for NARRATOR, or split the lines between speakers.");
      props.onOpenProject(p.id);
    } catch (e) { props.toast(messageOf(e), true); } finally { setBusy(false); }
  };

  const needsConsent = tab === "change";
  const canRun = !!file && !!chosen?.ready && (!needsConsent || (consentComplete(att) && !!target_voice));
  const verb = tab === "transcribe" ? "Transcribe" : tab === "clean" ? "Clean it" : tab === "change" ? "Change the voice" : "Transcribe";

  return (
    <div className="page">
      <div className="stage-title"><h1>Audio tools</h1><span className="spacer" /><button onClick={props.onBack}>Back</button></div>
      <div className="seg" role="radiogroup" aria-label="Tool" style={{ marginBottom: 12 }}>
        <button role="radio" aria-checked={!listen && tab === "transcribe"} onClick={() => switchTab("transcribe")}>Speech to text</button>
        <button role="radio" aria-checked={!listen && tab === "clean"} onClick={() => switchTab("clean")}>Clean audio</button>
        <button role="radio" aria-checked={!listen && tab === "change"} onClick={() => switchTab("change")}>Change voice</button>
        <button role="radio" aria-checked={!listen && tab === "dub"} onClick={() => switchTab("dub")}>Dub</button>
        <button role="radio" aria-checked={listen} onClick={() => setListen(true)}>Listening pack</button>
      </div>

      {listen ? <ListeningPack toast={props.toast} /> : (<>
      {(tab === "transcribe" || tab === "dub") && <OfflineSpeech toast={props.toast} onChanged={() => { void api.listAudioTools().then(setTools).catch(() => setTools([])); }} />}

      <div className="card">
        <p className="muted small" style={{ marginTop: 0 }}>{BLURB[tab]}</p>
        {capable.length === 0 ? (
          <div className="callout small"><strong>{tab === "clean" ? "No engine can clean audio yet." : tab === "change" ? "No engine can change voices yet." : "No engine can transcribe yet."}</strong>
            {tab === "transcribe" || tab === "dub" ? "Download an offline speech model above (nothing is sent anywhere), save an ElevenLabs key in Settings, or add a local Whisper server under Voice engines." : "Save an ElevenLabs key in Settings."}
            <div className="row" style={{ marginTop: 8 }}><button className="small" onClick={props.onSettings}>Open Settings</button><button className="small" onClick={props.onEngines}>Voice engines</button></div></div>
        ) : (
          <>
            {needsConsent && info && (
              <div style={{ marginBottom: 12 }}>
                <div className="callout small"><strong>Only with the speaker's consent.</strong>The person speaking in the recording must agree that their performance can drive another voice. Children, people who have died and public figures are refused. The record is saved on this computer and is not sent.</div>
                <ConsentFields id="cv" att={att} onChange={setAtt} statement={info.performance_statement} whose="Who is speaking in the recording?" use="What is it for?" />
              </div>
            )}
            <div className="row" style={{ alignItems: "end" }}>
              <button onClick={() => void pick()}>{file ? "Choose a different file…" : "Choose a recording…"}</button>
              {file && <span className="small mono muted">{file.split(/[\\/]/).pop()}</span>}
            </div>
            <div className="row" style={{ alignItems: "end", marginTop: 10 }}>
              {capable.length > 1 && <div><label htmlFor="at-e">With</label><select id="at-e" value={chosen?.id ?? ""} onChange={(e) => setEngine(e.target.value)}>{capable.map((t) => <option key={t.id} value={t.id}>{t.label}{t.ready ? "" : " (set up first)"}</option>)}</select></div>}
              {(tab === "transcribe" || tab === "dub") && <div><label htmlFor="at-l">Spoken language</label><select id="at-l" value={lang} onChange={(e) => setLang(e.target.value)}>{LANGS.map(([c, n]) => <option key={c} value={c}>{n}</option>)}</select></div>}
              {tab === "change" && (
                <>
                  <div><label htmlFor="at-vq">Find a voice</label><input id="at-vq" type="text" value={vq} onChange={(e) => setVq(e.target.value)} placeholder="Name or accent" /></div>
                  <div><label htmlFor="at-v">New voice</label><select id="at-v" value={target_voice?.id ?? ""} onChange={(e) => setVoiceId(e.target.value)}>{changeVoices.map((v) => <option key={v.id} value={v.id}>{v.name}</option>)}</select></div>
                </>
              )}
              <button className="primary" disabled={busy || !canRun} onClick={() => (chosen?.sends_data ? setAsk(true) : void run())}>{busy ? <><span className="spin" aria-hidden="true" /> Working</> : verb}</button>
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

      {(tab === "clean" || tab === "change") && clean && (
        <div className="card" aria-live="polite">
          <div className="row"><h2 style={{ margin: 0 }}>{tab === "change" ? "Changed voice" : "Cleaned audio"}</h2><span className="muted small">{(clean.duration_ms / 1000).toFixed(1)} s</span><span className="spacer" /><PlayButton path={clean.path} label={tab === "change" ? "the changed recording" : "the cleaned audio"} /><button className="small" onClick={() => void saveAudio()}>Save as…</button></div>
          {tab === "change" && <p className="small faint" style={{ margin: "6px 0 0" }}>Made from a performance with the speaker's consent on record. It starts as licence unknown; record the provider's terms before selling anything made with it.</p>}
        </div>
      )}

      {tab === "dub" && tr && cues.length > 0 && (
        <div className="card" aria-live="polite">
          <div className="row" style={{ alignItems: "end" }}>
            <h2 style={{ margin: 0 }}>Translate</h2><span className="spacer" />
            <div><label htmlFor="d-t">Into</label><select id="d-t" value={target} onChange={(e) => { setTarget(e.target.value); setTranslated(null); }}>{TARGETS.map((t) => <option key={t}>{t}</option>)}</select></div>
            <button className="primary small" disabled={busy} onClick={() => void translate()}>{busy ? "Translating…" : translated ? "Translate again" : "Translate"}</button>
          </div>
          <p className="small faint" style={{ margin: "6px 0 8px" }}>Uses the AI helper from Settings. Small models are weaker in some languages (Yoruba, Igbo and Hausa especially), so read and fix every line.</p>
          <div style={{ maxHeight: 360, overflow: "auto" }}>
            {cues.map((c, i) => (
              <div key={i} className="grid2" style={{ gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 6 }}>
                <div className="small muted" style={{ padding: "6px 0" }}>{(c.start_ms / 1000).toFixed(1)}s · {c.text}</div>
                {translated ? <textarea rows={2} aria-label={`Translation of line ${i + 1}`} value={translated[i] ?? ""} maxLength={600} onChange={(e) => setTranslated(translated.map((t, j) => (j === i ? e.target.value : t)))} /> : <div className="small faint" style={{ padding: "6px 0" }}>Not translated yet</div>}
              </div>
            ))}
          </div>
          <div className="row" style={{ marginTop: 10 }}>
            <button className="small" disabled={!translated} onClick={() => void saveText("srt", "translated")}>Save translated SRT…</button>
            <button className="small" disabled={!translated} onClick={() => void saveText("vtt", "translated")}>Save translated VTT…</button>
            <button className="primary small" disabled={!translated || busy} onClick={() => void makeProject()}>Create a dubbing project</button>
          </div>
          <p className="small faint" style={{ marginBottom: 0 }}>The project has the translated lines as one speaker, NARRATOR. Cast any voice you may use (a cloned voice needs its consent record), split lines between speakers, then render and mix as usual. Soundstage does not clone the original speakers.</p>
        </div>
      )}

      {ask && chosen && (
        <Confirm title={`Send this recording to ${chosen.label}?`} confirmLabel="Send" onCancel={() => setAsk(false)}
          body={<p>The recording is converted on this computer and sent over the internet to {chosen.label}. It may use credits on your account there.{tab === "change" ? " The speaker's consent record stays on this computer and is not sent." : ""}</p>}
          onConfirm={() => void run()} />
      )}
      </>)}
    </div>
  );
}
