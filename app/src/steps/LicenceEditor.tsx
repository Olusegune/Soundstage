import { useState } from "react";
import { today } from "../logic";
import type { ConditionKind, Licence, LicenceCondition, LicenceInfo } from "../types";

const KINDS: [ConditionKind, string][] = [["revenue_cap", "Revenue or size limit"], ["territory", "Country or region limit"], ["attribution", "Credit required"], ["field_of_use", "Limited uses"], ["other", "Other"]];

/** Soundstage cannot read a provider's terms for you. This records what *you* found, with its source and date. */
export function LicenceEditor(props: { initial: LicenceInfo; onSave: (info: LicenceInfo) => void; saveLabel?: string }) {
  const [licence, setLicence] = useState<Licence>(props.initial.licence);
  const [source, setSource] = useState(props.initial.source ?? "");
  const [checked, setChecked] = useState(props.initial.checked ?? today());
  const [notes, setNotes] = useState(props.initial.notes);
  const [conditions, setConditions] = useState<LicenceCondition[]>(props.initial.conditions ?? []);
  const [err, setErr] = useState<string | null>(null);
  const setCond = (i: number, patch: Partial<LicenceCondition>) => setConditions((cs) => cs.map((c, k) => (k === i ? { ...c, ...patch } : c)));

  const save = () => {
    const src = source.trim();
    if (licence === "commercial" && !/^https?:\/\/\S+$/i.test(src)) {
      setErr("To tag something commercial, paste the web address of the terms you read (starting with https://).");
      return;
    }
    if (src && !/^https?:\/\/\S+$/i.test(src)) { setErr("The source must be a web address starting with http:// or https://."); return; }
    if (!/^\d{4}-\d{2}-\d{2}$/.test(checked)) { setErr("Enter the date you checked, as YYYY-MM-DD."); return; }
    setErr(null);
    props.onSave({ licence, source: src || null, checked, notes: notes.trim(), conditions: conditions.filter((c) => c.text.trim()).map((c) => ({ ...c, text: c.text.trim() })) });
  };

  return (
    <div className="col" style={{ background: "var(--bg)", padding: 10, borderRadius: 6, border: "1px solid var(--line-strong)" }}>
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
      <fieldset style={{ border: "none", padding: 0, margin: 0 }}>
        <legend className="small" style={{ fontWeight: 600 }}>Conditions in the terms</legend>
        {conditions.length === 0 && <p className="small muted" style={{ margin: "4px 0" }}>None recorded. Add one if the terms limit revenue, countries, uses, or ask for a credit.</p>}
        {conditions.map((c, i) => (
          <div className="row" key={i} style={{ alignItems: "center", marginBottom: 4 }}>
            <select value={c.kind} aria-label={`Condition ${i + 1} type`} onChange={(e) => setCond(i, { kind: e.target.value as ConditionKind })}>
              {KINDS.map(([k, l]) => <option key={k} value={k}>{l}</option>)}
            </select>
            <input type="text" style={{ flex: 1, minWidth: 180 }} value={c.text} maxLength={200} aria-label={`Condition ${i + 1} wording`} onChange={(e) => setCond(i, { text: e.target.value })} />
            {c.kind !== "attribution" && (
              <label className="small" style={{ display: "flex", gap: 6, alignItems: "center" }}>
                <input type="checkbox" checked={!!c.confirmed} onChange={(e) => setCond(i, { confirmed: e.target.checked ? today() : null })} />
                {c.confirmed ? `My project meets this (confirmed ${c.confirmed})` : "My project meets this"}
              </label>
            )}
            <button className="small" onClick={() => setConditions((cs) => cs.filter((_, k) => k !== i))} aria-label={`Remove condition ${i + 1}`}>Remove</button>
          </div>
        ))}
        {conditions.length < 10 && <button className="small" onClick={() => setConditions((cs) => [...cs, { kind: "other", text: "", value: null, confirmed: null }])}>Add a condition</button>}
      </fieldset>
      {err && <div className="callout bad small" role="alert">{err}</div>}
      <div className="row">
        <button className="small primary" onClick={save}>{props.saveLabel ?? "Record my check"}</button>
        <span className="small muted">Checks older than 90 days need to be redone before a commercial export.</span>
      </div>
    </div>
  );
}
