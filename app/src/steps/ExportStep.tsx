import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import { api, messageOf } from "../api";
import type { Ctx } from "../ctx";
import { Empty } from "../ui";
import type { ExportError, ExportResult, GateItem, GateReport } from "../types";
import { LicenceEditor } from "./LicenceEditor";

const KIND: Record<string, string> = { voice: "Voice", model: "Voice engine", music: "Music", effect: "Sound effect", info: "Note" };

function checkKey(i: GateItem): string {
  return i.kind === "music" || i.kind === "effect" ? `asset:${i.id}` : `${i.kind}:${i.id}`;
}

export function ExportStep({ ctx }: { ctx: Ctx }) {
  const { project, analysis, edit } = ctx;
  const [report, setReport] = useState<GateReport | null>(null);
  const [folder, setFolder] = useState<string | null>(null);
  const [ack, setAck] = useState(false);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<ExportResult | null>(null);
  const [editing, setEditing] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    void api.checkExport(project).then((r) => live && setReport(r)).catch((e) => ctx.toast(messageOf(e), true));
    return () => { live = false; };
  }, [project.licence_target, project.licence_checks, project.assets, project.cues, project.selected, project.takes, project.mix.destination, project.script]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!analysis || !analysis.lines.some((l) => l.status === "rendered")) {
    return <div className="page"><Empty title="Nothing to export yet"><p className="muted">Render at least one line first.</p><button onClick={() => ctx.goto(3)}>Go to Render</button></Empty></div>;
  }

  const choose = async () => {
    const d = await open({ directory: true, multiple: false, title: "Choose where to export" });
    if (typeof d === "string") setFolder(d);
  };

  const needsAck = project.licence_target === "personal" && (report?.notices.some((n) => n.kind !== "info") ?? false);
  const blocking = report?.blocking ?? [];
  const reason = !report ? "Checking licences…" : blocking.length > 0 ? `${blocking.length} item${blocking.length === 1 ? "" : "s"} must be cleared first.` : !folder ? "Choose a folder to export into." : needsAck && !ack ? "Acknowledge the notice above to continue." : null;

  const run = async () => {
    if (!folder) return;
    setBusy(true);
    setResult(null);
    try {
      setResult(await api.exportProject(project, folder, ack));
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
        {report && blocking.length === 0 && <div className="callout ok" style={{ marginTop: 10 }}><strong>✓ Cleared for {project.licence_target === "commercial" ? "commercial" : "personal"} export</strong>Checked {report.checked_on}.</div>}
        {blocking.map((i) => {
          const k = `${i.kind}:${i.id}`;
          return (
            <div key={k} className="callout bad" style={{ marginTop: 10 }}>
              <strong>✕ {KIND[i.kind] ?? i.kind}: {i.name}</strong>
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
                  <LicenceEditor initial={project.licence_checks[checkKey(i)] ?? { licence: "unknown", source: i.source, checked: null, notes: "" }}
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
        <h2>Export</h2>
        <div className="row" style={{ alignItems: "end" }}>
          <div><label htmlFor="fmt">Format</label>
            <select id="fmt" value="wav" disabled aria-describedby="fmt-note"><option value="wav">WAV 24-bit 48 kHz: master + 3 stems</option></select></div>
          <div style={{ flex: 1, minWidth: 260 }}>
            <label htmlFor="folder">Folder</label>
            <div className="row"><input id="folder" type="text" readOnly style={{ flex: 1 }} value={folder ?? ""} placeholder="No folder chosen" /><button onClick={() => void choose()}>Choose…</button></div>
          </div>
        </div>
        <p id="fmt-note" className="small muted">MP3 is not in this version. The export writes a master, separate dialogue, effects and music stems of equal length, and a manifest listing every voice, licence decision and file hash. The manifest records that the dialogue is AI-generated.</p>
        <div className="row">
          <button className="primary" disabled={reason !== null || busy} onClick={() => void run()}>{busy ? "Exporting…" : "Export"}</button>
          {reason && <span className="small" style={{ color: "var(--warning)" }}>{reason}</span>}
        </div>
      </div>

      {result && (
        <div className="card" aria-live="polite">
          <h2>✓ Exported</h2>
          <p className="mono small">{result.folder}</p>
          <ul className="small">{result.files.map((f) => <li key={f} className="mono">{f}</li>)}</ul>
          <p className="small">Integrated {result.loudness.integrated_lufs?.toFixed(1) ?? "n/a"} LUFS (target {result.target_lufs}), true peak {result.loudness.true_peak_dbtp.toFixed(1)} dBTP, gain {result.gain_db.toFixed(1)} dB.</p>
          {result.warnings.map((w, i) => <div key={i} className="callout warn small"><strong>⚠ Flagged</strong>{w}</div>)}
          <button onClick={() => void revealItemInDir(result.folder)}>Show in folder</button>
        </div>
      )}
    </div>
  );
}
