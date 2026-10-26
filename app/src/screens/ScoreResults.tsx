import { open, save } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { api, messageOf } from "../api";
import type { KeyInfo, ScoreReport } from "../types";

const pct = (x: number | null) => (x === null ? "n/a" : `${Math.round(x * 100)}%`);
const num = (x: number | null) => (x === null ? "n/a" : x.toFixed(1));
const VERDICT = { met: { label: "Met", cls: "ok" }, not_met: { label: "Not met", cls: "bad" }, not_enough: { label: "Not enough listeners", cls: "warn" } } as const;

/** Turn the listeners' score sheets into results, judged against the pass levels. Nothing here changes any voice's status. */
export function ScoreResults(props: { toast: (t: string, bad?: boolean) => void }) {
  const [keyPath, setKeyPath] = useState<string | null>(null);
  const [key, setKey] = useState<KeyInfo | null>(null);
  const [sheets, setSheets] = useState<string[]>([]);
  const [reference, setReference] = useState("");
  const [soundLabels, setSoundLabels] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<ScoreReport | null>(null);

  const chooseKey = async () => {
    const p = await open({ title: "Choose PRIVATE-KEY-do-not-share.csv", filters: [{ name: "CSV", extensions: ["csv"] }] });
    if (typeof p !== "string") return;
    try { setKey(await api.scoresReadKey(p)); setKeyPath(p); setReference(""); setSoundLabels([]); setReport(null); } catch (e) { props.toast(messageOf(e), true); }
  };
  const chooseSheets = async () => {
    const p = await open({ multiple: true, title: "Choose the listeners' score sheets", filters: [{ name: "CSV", extensions: ["csv"] }] });
    const list = Array.isArray(p) ? p : p ? [p] : [];
    if (list.length) { setSheets(list); setReport(null); }
  };
  const run = async () => {
    if (!keyPath) return;
    setBusy(true);
    try { setReport(await api.scoresReport(keyPath, sheets, { reference_label: reference || null, sound_labels: soundLabels, min_listeners: 5 })); }
    catch (e) { props.toast(messageOf(e), true); } finally { setBusy(false); }
  };
  const exportAs = async (ext: "md" | "csv") => {
    if (!report) return;
    const dest = await save({ title: "Save the report", defaultPath: `listening-results.${ext}`, filters: [{ name: ext === "md" ? "Markdown" : "CSV", extensions: [ext] }] });
    if (!dest) return;
    try { await api.scoresExport(report, dest, "Listening results"); props.toast("Saved."); } catch (e) { props.toast(messageOf(e), true); }
  };

  return (
    <div className="card" role="region" aria-label="Listening results" style={{ marginTop: 12 }}>
      <h2 style={{ marginTop: 0 }}>Results</h2>
      <p className="muted small" style={{ marginTop: 0 }}>When the score sheets are back, choose your private key and the sheets. Soundstage checks every row, tells you about any it cannot use, and judges each version against the pass levels. It never changes anything about a voice: it only reports.</p>
      <div className="row" style={{ alignItems: "center" }}>
        <button className="small" onClick={() => void chooseKey()}>Choose the private key…</button>
        <span className="small muted">{key ? `${key.clips} clips, ${key.listeners} listener${key.listeners === 1 ? "" : "s"}, versions: ${key.labels.join(", ")}` : "No key chosen"}</span>
      </div>
      <div className="row" style={{ alignItems: "center", marginTop: 6 }}>
        <button className="small" disabled={!key} onClick={() => void chooseSheets()}>Choose score sheets…</button>
        <span className="small muted">{sheets.length ? `${sheets.length} sheet${sheets.length === 1 ? "" : "s"} chosen` : "None chosen"}</span>
      </div>
      {key && (
        <div className="row" style={{ alignItems: "end", marginTop: 8 }}>
          <div><label htmlFor="sr-ref">Compare everything with</label>
            <select id="sr-ref" value={reference} onChange={(e) => setReference(e.target.value)}><option value="">No comparison</option>{key.labels.map((l) => <option key={l} value={l}>{l}</option>)}</select></div>
          <fieldset style={{ border: "none", padding: 0, margin: 0 }}>
            <legend className="small">These versions have sounds in them</legend>
            <div className="row tight">
              {key.labels.map((l) => (
                <label key={l} className="inline small" style={{ minHeight: 28 }}>
                  <input type="checkbox" checked={soundLabels.includes(l)} onChange={(e) => setSoundLabels((s) => (e.target.checked ? [...s, l] : s.filter((x) => x !== l)))} /> {l}
                </label>
              ))}
            </div>
          </fieldset>
        </div>
      )}
      <div className="row" style={{ marginTop: 10 }}>
        <button className="primary" disabled={!key || sheets.length === 0 || busy} onClick={() => void run()}>{busy ? <><span className="spin" aria-hidden="true" /> Working it out</> : "Work out the results"}</button>
      </div>

      {report && (
        <div style={{ marginTop: 12 }} role="status">
          <strong>{report.listeners} listener{report.listeners === 1 ? "" : "s"}, {report.rows_used} scored clips</strong>
          {report.notes.map((n, i) => <p key={i} className="small muted" style={{ margin: "4px 0" }}>{n}</p>)}
          {report.labels.map((l) => (
            <div key={l.label} className="linecard" style={{ margin: "8px 0" }}>
              <h3 style={{ margin: 0 }}>{l.label}</h3>
              <p className="small muted" style={{ margin: "2px 0 6px" }}>
                Natural median {num(l.natural_median)}, emotion median {num(l.emotion_median)}, below 3: {pct(l.low_share)}, same voice: {pct(l.same_voice_rate)}, understood: {pct(l.understood_rate)}
                {l.spoken_tag_words.length > 0 ? `, spoken tag words heard: ${l.spoken_tag_words.join(", ")}` : ""}
              </p>
              <ul style={{ listStyle: "none", padding: 0, margin: 0 }}>
                {l.gates.map((g) => (
                  <li key={g.name} className="small" style={{ marginBottom: 4 }}>
                    <span className={`chip ${VERDICT[g.verdict].cls}`}>{VERDICT[g.verdict].label}</span> {g.name} <span className="muted">({g.detail})</span>
                  </li>
                ))}
              </ul>
            </div>
          ))}
          {report.issues.length > 0 && (
            <details open>
              <summary className="small"><strong>{report.issues.length} problem{report.issues.length === 1 ? "" : "s"} in the score sheets</strong> (those rows were left out)</summary>
              <ul className="small" style={{ margin: "4px 0 0" }}>{report.issues.map((i, k) => <li key={k}>{i.file || "files"}{i.line ? `, line ${i.line}` : ""}: {i.message}</li>)}</ul>
            </details>
          )}
          <div className="row" style={{ marginTop: 8 }}>
            <button className="small" onClick={() => void exportAs("md")}>Save as a document (.md)</button>
            <button className="small" onClick={() => void exportAs("csv")}>Save as a spreadsheet (.csv)</button>
          </div>
        </div>
      )}
    </div>
  );
}
