import { useState } from "react";
import { today } from "../logic";
import type { Licence, LicenceInfo } from "../types";

/** Soundstage cannot read a provider's terms for you. This records what *you* found, with its source and date. */
export function LicenceEditor(props: { initial: LicenceInfo; onSave: (info: LicenceInfo) => void; saveLabel?: string }) {
  const [licence, setLicence] = useState<Licence>(props.initial.licence);
  const [source, setSource] = useState(props.initial.source ?? "");
  const [checked, setChecked] = useState(props.initial.checked ?? today());
  const [notes, setNotes] = useState(props.initial.notes);
  const [err, setErr] = useState<string | null>(null);

  const save = () => {
    const src = source.trim();
    if (licence === "commercial" && !/^https?:\/\/\S+$/i.test(src)) {
      setErr("To tag something commercial, paste the web address of the terms you read (starting with https://).");
      return;
    }
    if (src && !/^https?:\/\/\S+$/i.test(src)) { setErr("The source must be a web address starting with http:// or https://."); return; }
    if (!/^\d{4}-\d{2}-\d{2}$/.test(checked)) { setErr("Enter the date you checked, as YYYY-MM-DD."); return; }
    setErr(null);
    props.onSave({ licence, source: src || null, checked, notes: notes.trim() });
  };

  return (
    <div className="col" style={{ background: "var(--obsidian)", padding: 10, borderRadius: 6, border: "1px solid var(--line)" }}>
      <div className="row" style={{ alignItems: "end" }}>
        <div>
          <label>What the terms say</label>
          <select value={licence} onChange={(e) => setLicence(e.target.value as Licence)} aria-label="Licence">
            <option value="unknown">I have not checked</option>
            <option value="commercial">Commercial use is allowed</option>
            <option value="non-commercial">Commercial use is not allowed</option>
          </select>
        </div>
        <div style={{ flex: 1, minWidth: 220 }}>
          <label>Where I read them (web address)</label>
          <input type="url" style={{ width: "100%" }} value={source} onChange={(e) => setSource(e.target.value)} placeholder="https://…" aria-label="Licence source" maxLength={500} />
        </div>
        <div>
          <label>Date checked</label>
          <input type="date" value={checked} onChange={(e) => setChecked(e.target.value)} aria-label="Date checked" />
        </div>
      </div>
      <div>
        <label>Notes (limits, attribution, prohibited uses)</label>
        <input type="text" style={{ width: "100%" }} value={notes} maxLength={300} onChange={(e) => setNotes(e.target.value)} aria-label="Licence notes" />
      </div>
      {err && <div className="callout bad small" role="alert">{err}</div>}
      <div className="row">
        <button className="small primary" onClick={save}>{props.saveLabel ?? "Record my check"}</button>
        <span className="small muted">Checks older than 90 days need to be redone before a commercial export.</span>
      </div>
    </div>
  );
}
