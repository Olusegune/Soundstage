import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useMemo, useState } from "react";
import { api, messageOf } from "../api";
import type { Ctx } from "../ctx";
import { withConditions } from "../logic";
import * as I from "../icons";
import { Empty, Help } from "../ui";
import type { Aspect, Codec, ExportCaps, ExportError, ExportOptions, ExportResult, GateItem, GateReport } from "../types";
import { LicenceEditor } from "./LicenceEditor";

const KIND: Record<string, string> = { voice: "Voice", model: "Voice engine", music: "Music", effect: "Sound effect", info: "Note" };

function checkKey(i: GateItem): string {
  return i.kind === "music" || i.kind === "effect" ? `asset:${i.id}` : `${i.kind}:${i.id}`;
}

const FORMATS: { kind: Codec["kind"]; name: string; blurb: string; tag: "Lossless" | "Lossy" | "Video" }[] = [
  { kind: "wav", name: "WAV", blurb: "Uncompressed. The standard for editors.", tag: "Lossless" },
  { kind: "flac", name: "FLAC", blurb: "Lossless at about half the size.", tag: "Lossless" },
  { kind: "mp3", name: "MP3", blurb: "Plays on everything.", tag: "Lossy" },
  { kind: "aac", name: "AAC (M4A)", blurb: "Small, good for Apple and the web.", tag: "Lossy" },
  { kind: "opus", name: "Opus", blurb: "Very small and clear for speech.", tag: "Lossy" },
  { kind: "vorbis", name: "Ogg Vorbis", blurb: "Open format for games and the web.", tag: "Lossy" },
  { kind: "mp4", name: "MP4 video", blurb: "Your audio over a moving waveform, for social sites.", tag: "Video" },
];

function codecLabel(c: Codec): string {
  switch (c.kind) {
    case "wav": return `WAV ${c.bits === 32 ? "32-bit float" : `${c.bits}-bit`}`;
    case "flac": return "FLAC";
    case "mp3": return `MP3 ${c.kbps} kbps`;
    case "aac": return `AAC ${c.kbps} kbps`;
    case "vorbis": return `Ogg Vorbis, quality ${c.quality}`;
    case "opus": return `Opus ${c.kbps} kbps`;
    case "mp4": return `MP4 video, ${c.kbps} kbps`;
  }
}

export function defaultCodec(kind: Codec["kind"]): Codec {
  switch (kind) {
    case "wav": return { kind, bits: 24 };
    case "flac": return { kind };
    case "mp3": return { kind, kbps: 192 };
    case "aac": return { kind, kbps: 192 };
    case "opus": return { kind, kbps: 128 };
    case "vorbis": return { kind, quality: 5 };
    case "mp4": return { kind, aspect: "wide", kbps: 192 };
  }
}

const PRESETS: { id: string; name: string; blurb: string; needs: Codec["kind"]; master: Codec; stems: Codec | null; rate: number }[] = [
  { id: "editor", name: "Hand off to an editor", blurb: "WAV 24-bit master and stems", needs: "wav", master: { kind: "wav", bits: 24 }, stems: { kind: "wav", bits: 24 }, rate: 48000 },
  { id: "web", name: "Podcast or web", blurb: "MP3 192 kbps, no stems", needs: "mp3", master: { kind: "mp3", kbps: 192 }, stems: null, rate: 44100 },
  { id: "social", name: "Social video", blurb: "Vertical MP4 with a waveform", needs: "mp4", master: { kind: "mp4", aspect: "tall", kbps: 192 }, stems: null, rate: 48000 },
  { id: "archive", name: "Archive", blurb: "FLAC master and stems", needs: "flac", master: { kind: "flac" }, stems: { kind: "flac" }, rate: 48000 },
];

function CodecSettings({ codec, onChange, id }: { codec: Codec; onChange: (c: Codec) => void; id: string }) {
  const kbps = (opts: number[]) => (
    <div><label htmlFor={`${id}-k`}>Quality</label>
      <select id={`${id}-k`} value={(codec as { kbps: number }).kbps} onChange={(e) => onChange({ ...codec, kbps: Number(e.target.value) } as Codec)}>
        {opts.map((k) => <option key={k} value={k}>{k} kbps</option>)}
      </select></div>
  );
  switch (codec.kind) {
    case "wav": return (
      <div><label htmlFor={`${id}-b`}>Bit depth <Help text="More bits hold more detail. 24-bit is the standard for editing. 16-bit is for CDs and the web. 32-bit float is for further processing." /></label>
        <select id={`${id}-b`} value={codec.bits} onChange={(e) => onChange({ kind: "wav", bits: Number(e.target.value) as 16 | 24 | 32 })}>
          <option value={16}>16-bit</option><option value={24}>24-bit</option><option value={32}>32-bit float</option>
        </select></div>
    );
    case "mp3": return kbps([128, 192, 256, 320]);
    case "aac": return kbps([128, 192, 256, 320]);
    case "opus": return kbps([64, 96, 128, 192]);
    case "vorbis": return (
      <div><label htmlFor={`${id}-q`}>Quality</label>
        <select id={`${id}-q`} value={codec.quality} onChange={(e) => onChange({ kind: "vorbis", quality: Number(e.target.value) })}>
          {[3, 4, 5, 6, 7, 8].map((q) => <option key={q} value={q}>{q} {q === 5 ? "(good)" : q >= 7 ? "(high)" : ""}</option>)}
        </select></div>
    );
    case "mp4": return (
      <>
        <div><label>Shape</label>
          <div className="seg" role="radiogroup" aria-label="Video shape">
            {([["wide", "16:9 wide"], ["tall", "9:16 vertical"], ["square", "1:1 square"]] as [Aspect, string][]).map(([a, l]) => (
              <button key={a} role="radio" aria-checked={codec.aspect === a} onClick={() => onChange({ ...codec, aspect: a })}>{l}</button>
            ))}
          </div></div>
        {kbps([128, 192, 256])}
      </>
    );
    case "flac": return <div className="small muted" style={{ alignSelf: "end" }}>Lossless. No settings needed.</div>;
  }
}

export function ExportStep({ ctx }: { ctx: Ctx }) {
  const { project, analysis, edit } = ctx;
  const [report, setReport] = useState<GateReport | null>(null);
  const [folder, setFolder] = useState<string | null>(null);
  const [ack, setAck] = useState(false);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<ExportResult | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  const [caps, setCaps] = useState<ExportCaps | null>(null);
  const [master, setMaster] = useState<Codec>({ kind: "wav", bits: 24 });
  const [stems, setStems] = useState<Codec | null>({ kind: "wav", bits: 24 });
  const [rate, setRate] = useState(48000);
  const [subs, setSubs] = useState<{ srt: boolean; vtt: boolean }>({ srt: false, vtt: false });
  const [labels, setLabels] = useState(false);
  const [timeline, setTimeline] = useState(false);

  // Start from what was used last time, and keep it up to date.
  const [loaded, setLoaded] = useState(false);
  useEffect(() => {
    try {
      const m = ctx.exportMemory ? JSON.parse(ctx.exportMemory) as { master?: Codec; stems?: Codec | null; rate?: number; subs?: { srt: boolean; vtt: boolean }; labels?: boolean; timeline?: boolean } : null;
      if (m) {
        if (m.master && typeof m.master.kind === "string") setMaster(m.master);
        if (m.stems === null || (m.stems && typeof m.stems.kind === "string")) setStems(m.stems ?? null);
        if (typeof m.rate === "number" && [44100, 48000, 96000].includes(m.rate)) setRate(m.rate);
        if (m.subs) setSubs({ srt: !!m.subs.srt, vtt: !!m.subs.vtt });
        if (typeof m.labels === "boolean") setLabels(m.labels);
        if (typeof m.timeline === "boolean") setTimeline(m.timeline);
      }
    } catch { /* a damaged memory is ignored */ }
    setLoaded(true);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => { if (loaded) ctx.setExportMemory(JSON.stringify({ master, stems, rate, subs, labels, timeline })); }, [loaded, master, stems, rate, subs, labels, timeline]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    let live = true;
    void api.checkExport(project).then((r) => live && setReport(r)).catch((e) => ctx.toast(messageOf(e), true));
    return () => { live = false; };
  }, [project.licence_target, project.licence_checks, project.assets, project.cues, project.selected, project.takes, project.mix.destination, project.script]); // eslint-disable-line react-hooks/exhaustive-deps

  const refreshCaps = useCallback(() => { void api.exportCapabilities(ctx.ffmpegPath || null).then(setCaps).catch(() => setCaps(null)); }, [ctx.ffmpegPath]);
  useEffect(refreshCaps, [refreshCaps]);

  const available = useMemo(() => new Set((caps?.formats ?? []).filter((f) => f.available).map((f) => f.kind)), [caps]);
  const masterOk = caps === null ? master.kind === "wav" : available.has(master.kind);
  const stemsOk = stems === null || (caps === null ? stems.kind === "wav" : available.has(stems.kind));

  if (!analysis || !analysis.lines.some((l) => l.status === "rendered")) {
    return <Empty title="Nothing to export yet"><p className="muted">Render at least one line first.</p><button className="primary" onClick={() => ctx.goto(3)}>Go to Render</button></Empty>;
  }

  const choose = async () => {
    const d = await open({ directory: true, multiple: false, title: "Choose where to export" });
    if (typeof d === "string") setFolder(d);
  };
  const chooseFfmpeg = async () => {
    const f = await open({ multiple: false, title: "Choose ffmpeg.exe", filters: [{ name: "FFmpeg", extensions: ["exe"] }] });
    if (typeof f === "string") ctx.setFfmpegPath(f);
  };

  const pickFormat = (kind: Codec["kind"]) => {
    const c = defaultCodec(kind);
    setMaster(c);
    if (kind === "mp4") setStems((s) => (s && s.kind !== "mp4" ? s : null));
  };
  const usePreset = (p: (typeof PRESETS)[number]) => { setMaster(p.master); setStems(p.stems); setRate(p.rate); };

  const needsAck = project.licence_target === "personal" && (report?.notices.some((n) => n.kind !== "info") ?? false);
  const blocking = report?.blocking ?? [];
  const reason = !report ? "Checking licences…" : blocking.length > 0 ? `${blocking.length} item${blocking.length === 1 ? "" : "s"} must be cleared first.`
    : !masterOk || !stemsOk ? "This format needs FFmpeg. Install it below, or pick WAV."
    : !folder ? "Choose a folder to export into." : needsAck && !ack ? "Acknowledge the notice above to continue." : null;

  const options = (): ExportOptions => ({
    master, stems, sample_rate: rate, speaker_labels: labels, ffmpeg_path: ctx.ffmpegPath || null, timeline,
    subtitles: [...(subs.srt ? (["srt"] as const) : []), ...(subs.vtt ? (["vtt"] as const) : [])],
  });

  const run = async () => {
    if (!folder) return;
    setBusy(true);
    setResult(null);
    try {
      setResult(await api.exportProject(project, folder, ack, options()));
      ctx.markExported();
      ctx.toast("Export finished.");
    } catch (e) {
      const err = e as ExportError;
      if (err && typeof err === "object" && "kind" in err) {
        if (err.kind === "blocked") { setReport(err.report); ctx.toast("Export was blocked by the licence check. Nothing was written.", true); }
        else if (err.kind === "needs_acknowledgement") ctx.toast("Please acknowledge the notice first.", true);
        else ctx.toast(err.message, true);
      } else ctx.toast(messageOf(e), true);
    } finally { setBusy(false); }
  };

  return (
    <div className="page">
      <div className="card">
        <div className="row"><h2 style={{ margin: 0 }}>Check before export</h2><span className="spacer" />
          <span className={`chip ${project.licence_target === "commercial" ? "info" : ""}`}>Target: {project.licence_target === "commercial" ? "Commercial" : "Personal"}</span>
          {project.licence_target === "commercial" && <button className="small" onClick={() => edit((p) => ({ ...p, licence_target: "personal" }))}>Mark as personal</button>}
          {project.licence_target === "personal" && <button className="small" onClick={() => edit((p) => ({ ...p, licence_target: "commercial" }))}>Switch to commercial</button>}
        </div>
        {report && blocking.length === 0 && <div className="callout ok" style={{ marginTop: 10 }}><strong><I.Check size={14} /> Cleared for {project.licence_target === "commercial" ? "commercial" : "personal"} export</strong>Checked {report.checked_on}.</div>}
        {blocking.map((i) => {
          const k = `${i.kind}:${i.id}`;
          return (
            <div key={k} className="callout bad" style={{ marginTop: 10 }}>
              <strong><I.Close size={14} /> {KIND[i.kind] ?? i.kind}: {i.name}</strong>
              {i.reason}
              {i.source && <div className="small muted mono">Source: {i.source}</div>}
              <div className="row" style={{ marginTop: 6 }}>
                <button className="small" aria-expanded={editing === k} onClick={() => setEditing(editing === k ? null : k)}>Record my licence check…</button>
                {(i.kind === "voice" || i.kind === "model") && <button className="small" onClick={() => ctx.goto(1)}>Replace voice</button>}
                {(i.kind === "music" || i.kind === "effect") && <button className="small" onClick={() => ctx.goto(4)}>Open in Mix</button>}
                <button className="small" onClick={() => edit((p) => ({ ...p, licence_target: "personal" }))}>Mark project as personal</button>
              </div>
              {editing === k && (
                <div style={{ marginTop: 8 }}>
                  <LicenceEditor initial={withConditions(project.licence_checks[checkKey(i)] ?? { licence: "unknown", source: i.source, checked: null, notes: "" }, i.conditions)}
                    onSave={(info) => { edit((p) => ({ ...p, licence_checks: { ...p.licence_checks, [checkKey(i)]: info } })); setEditing(null); }} />
                </div>
              )}
            </div>
          );
        })}
        {report && report.notices.length > 0 && (
          <div className="callout warn" style={{ marginTop: 10 }}>
            <strong>Notice for a personal project</strong>
            {report.notices.map((n) => <div key={`${n.kind}-${n.id}`} className="small">• {n.kind === "info" ? n.name : `${KIND[n.kind] ?? n.kind}: ${n.name}`}. {n.reason}</div>)}
            {needsAck && <label className="inline" style={{ marginTop: 8 }}><input type="checkbox" checked={ack} onChange={(e) => setAck(e.target.checked)} /> I understand these items are not cleared for commercial use.</label>}
          </div>
        )}
      </div>

      <div className="card">
        <h2>Format</h2>
        <div className="row" style={{ marginBottom: 12 }}>
          <span className="small muted">Quick choices:</span>
          {PRESETS.map((p) => (
            <button key={p.id} className="small" disabled={caps !== null && !available.has(p.needs)} onClick={() => usePreset(p)} data-tip={p.blurb}>{p.name}</button>
          ))}
        </div>
        <div className="format-grid" role="radiogroup" aria-label="Master format">
          {FORMATS.map((f) => {
            const ok = f.kind === "wav" || (caps !== null && available.has(f.kind));
            return (
              <button key={f.kind} role="radio" aria-checked={master.kind === f.kind} className={`format-tile ${master.kind === f.kind ? "on" : ""}`} onClick={() => pickFormat(f.kind)} aria-disabled={!ok || undefined}
                data-tip={ok ? undefined : "Needs FFmpeg. See below."}>
                <span className="row tight" style={{ justifyContent: "space-between", width: "100%" }}><strong>{f.name}</strong><span className={`chip ${f.tag === "Lossless" ? "info" : f.tag === "Video" ? "gold" : ""}`}>{f.tag}</span></span>
                <span className="small muted" style={{ textAlign: "left" }}>{f.blurb}</span>
                {!ok && <span className="small" style={{ color: "var(--warn)" }}>Needs FFmpeg</span>}
              </button>
            );
          })}
        </div>

        {ctx.mode === "advanced" ? (
        <div className="row" style={{ alignItems: "end", marginTop: 16 }}>
          <CodecSettings codec={master} onChange={setMaster} id="master" />
          <div><label htmlFor="rate">Sample rate <Help text="How many times per second the sound is measured. 48 kHz is the film and video standard; 44.1 kHz is for music and podcasts." /></label>
            <select id="rate" value={rate} onChange={(e) => setRate(Number(e.target.value))}><option value={44100}>44.1 kHz</option><option value={48000}>48 kHz</option><option value={96000}>96 kHz</option></select></div>
        </div>
        ) : <p className="small muted" style={{ margin: "14px 0 0" }}>Using {codecLabel(master)} at {rate / 1000} kHz. Switch to Advanced mode to change bit depth, quality and sample rate.</p>}

        <hr className="divider" />
        <label className="inline"><input type="checkbox" checked={stems !== null} onChange={(e) => setStems(e.target.checked ? (master.kind === "mp4" ? { kind: "wav", bits: 24 } : master.kind === "wav" || master.kind === "flac" ? master : { kind: "wav", bits: 24 }) : null)} /> Also write separate dialogue, effects and music stems <Help text="Stems are the three parts of the mix as their own files, the same length as the master, so an editor can rebalance them later." /></label>
        {stems !== null && ctx.mode === "advanced" && (
          <div className="row" style={{ alignItems: "end", marginTop: 10 }}>
            <div><label htmlFor="stem-fmt">Stem format</label>
              <select id="stem-fmt" value={stems.kind === "wav" ? `wav${stems.bits}` : stems.kind} onChange={(e) => { const v = e.target.value; setStems(v.startsWith("wav") ? { kind: "wav", bits: Number(v.slice(3)) as 16 | 24 | 32 } : defaultCodec(v as Codec["kind"])); }}>
                <option value="wav24">WAV 24-bit</option><option value="wav16">WAV 16-bit</option><option value="wav32">WAV 32-bit float</option>
                {available.has("flac") && <option value="flac">FLAC (lossless)</option>}
                {available.has("mp3") && <option value="mp3">MP3 192 kbps</option>}
              </select></div>
            <p className="small muted" style={{ margin: 0, maxWidth: 420 }}>Editors prefer lossless stems. Lossy stems are for previews and sharing.</p>
          </div>
        )}

        <hr className="divider" />
        <div className="row">
          <strong className="small">Subtitles</strong>
          <label className="inline"><input type="checkbox" checked={subs.srt} onChange={(e) => setSubs({ ...subs, srt: e.target.checked })} /> SRT</label>
          <label className="inline"><input type="checkbox" checked={subs.vtt} onChange={(e) => setSubs({ ...subs, vtt: e.target.checked })} /> VTT</label>
          <label className="inline"><input type="checkbox" checked={labels} disabled={!subs.srt && !subs.vtt} onChange={(e) => setLabels(e.target.checked)} /> Show speaker names</label>
          <span className="spacer" />
          <label className="inline"><input type="checkbox" checked={timeline} onChange={(e) => setTimeline(e.target.checked)} /> Timeline for video editors (OTIO)</label>
          <Help text="Writes an OpenTimelineIO file and every line, sound effect and music clip as its own WAV file, in the right place. DaVinci Resolve and other editors can open it, so you can move or replace single clips." />
          <Help text="Subtitle files are timed from your real timeline, so they match the audio exactly. Directions in [brackets] are never shown." />
        </div>
      </div>

      {(ctx.mode === "advanced" || !caps?.ffmpeg) && <div className="card">
        <div className="row"><h2 style={{ margin: 0 }}>FFmpeg</h2><span className="spacer" />
          {caps?.ffmpeg ? <span className="chip ok"><I.Check size={12} /> Found</span> : <span className="chip warn"><I.Alert size={12} /> Not found</span>}
        </div>
        <p className="small muted" style={{ margin: "8px 0" }}>WAV is always available. FLAC, MP3, AAC, Opus, Ogg Vorbis and MP4 are written by FFmpeg, a free open-source tool that comes with Soundstage. It runs on this computer and never sends your audio anywhere.</p>
        {caps?.ffmpeg ? (
          <p className="small mono faint" style={{ margin: 0 }}>{caps.ffmpeg.version}<br />{caps.ffmpeg.path}</p>
        ) : (
          <div className="callout warn small">
            <strong>{caps?.configured_failed ? "The FFmpeg you chose could not be used." : "FFmpeg was not found on this computer."}</strong>
            To get it, open Windows Terminal and run this, then press Check again:
            <pre className="mono" style={{ margin: "6px 0 0", userSelect: "all" }}>winget install Gyan.FFmpeg</pre>
          </div>
        )}
        <div className="row" style={{ marginTop: 10 }}>
          <button className="small" onClick={refreshCaps}>Check again</button>
          <button className="small" onClick={() => void chooseFfmpeg()}>Choose ffmpeg.exe…</button>
          {ctx.ffmpegPath && <button className="small ghost" onClick={() => ctx.setFfmpegPath("")}>Look automatically</button>}
        </div>
      </div>}

      <div className="card">
        <h2>Export</h2>
        <div className="row" style={{ alignItems: "end" }}>
          <div style={{ flex: 1, minWidth: 260 }}>
            <label htmlFor="folder">Folder</label>
            <div className="row"><input id="folder" type="text" readOnly style={{ flex: 1 }} value={folder ?? ""} placeholder="No folder chosen" /><button onClick={() => void choose()}>Choose…</button></div>
          </div>
        </div>
        <p id="fmt-note" className="small muted">You get {master.kind === "mp4" ? "a video" : "a master"}{stems ? ", three stems of equal length," : ""} {subs.srt || subs.vtt ? "subtitles, " : ""}and a manifest listing every voice, licence decision and file fingerprint. Files are marked as AI-generated. Every export goes into a new folder.</p>
        <div className="row">
          <button className="primary" disabled={reason !== null || busy} onClick={() => void run()} style={{ minHeight: 44 }}>{busy ? <><span className="spin" aria-hidden="true" /> Exporting</> : <><I.Export size={16} /> Export</>}</button>
          {reason && <span className="small" style={{ color: "var(--warn)" }}>{reason}</span>}
        </div>
      </div>

      {result && (
        <div className="card" aria-live="polite">
          <h2><I.Check size={18} /> Exported</h2>
          <p className="mono small">{result.folder}</p>
          <ul className="small">{result.files.map((f) => <li key={f} className="mono">{f}</li>)}</ul>
          <p className="small">Integrated {result.loudness.integrated_lufs?.toFixed(1) ?? "n/a"} LUFS (target {result.target_lufs}), true peak {result.loudness.true_peak_dbtp.toFixed(1)} dBTP, gain {result.gain_db.toFixed(1)} dB. Measured before encoding.</p>
          {result.warnings.map((w, i) => <div key={i} className="callout warn small"><strong>Flagged</strong>{w}</div>)}
          <div className="row">
            <button className="primary" onClick={() => void revealItemInDir(result.folder)}><I.Folder size={16} /> Show in folder</button>
            {!subs.srt && !subs.vtt && <button onClick={() => { setSubs({ srt: true, vtt: false }); setResult(null); }}>Add subtitles and export again</button>}
            <button onClick={() => setResult(null)}>Export again in another format</button>
            <button onClick={ctx.newProject}><I.Plus size={14} /> Start another project</button>
          </div>
          <h3 style={{ marginTop: 14 }}>What next?</h3>
          <ul className="small" style={{ margin: 0 }}>
            <li>Play the master through headphones once, start to end, before you share it.</li>
            <li>Keep the stems (dialogue, effects, music) if an editor will mix it further.</li>
            <li>The manifest records which voices and licences were used. Keep it with the files.</li>
            <li>A name sounded wrong? Teach it in Pronunciation, then render the lines again.</li>
          </ul>
        </div>
      )}
    </div>
  );
}
