import type { Attestation } from "../types";

export const EMPTY_ATTESTATION: Attestation = { subject: "", subject_is_self: true, adult: false, alive: false, not_public_figure: false, agrees: false, purpose: "", commercial: false, expires: null };

export function consentComplete(a: Attestation): boolean {
  return a.adult && a.alive && a.not_public_figure && a.agrees && a.subject.trim().length >= 2 && a.purpose.trim().length > 0;
}

/** The promises a person makes before their voice or performance is used. Shared by the screens that need them. */
export function ConsentFields(props: { id: string; att: Attestation; onChange: (a: Attestation) => void; statement: string; whose: string; use: string }) {
  const { att, id } = props;
  const tick = (k: keyof Attestation, label: string) => (
    <label className="inline" style={{ display: "flex", gap: 8, alignItems: "flex-start", marginBottom: 6 }}>
      <input type="checkbox" checked={att[k] as boolean} onChange={(e) => props.onChange({ ...att, [k]: e.target.checked })} /> <span className="small">{label}</span>
    </label>
  );
  return (
    <div role="group" aria-label="Consent">
      <div className="grid2" style={{ gridTemplateColumns: "1fr 1fr" }}>
        <div><label htmlFor={`${id}-subject`}>{props.whose}</label><input id={`${id}-subject`} type="text" style={{ width: "100%" }} maxLength={80} value={att.subject} onChange={(e) => props.onChange({ ...att, subject: e.target.value })} /></div>
        <div><label htmlFor={`${id}-purpose`}>{props.use}</label><input id={`${id}-purpose`} type="text" style={{ width: "100%" }} maxLength={200} value={att.purpose} onChange={(e) => props.onChange({ ...att, purpose: e.target.value })} /></div>
      </div>
      <div style={{ marginTop: 10 }}>
        {tick("subject_is_self", "This is my own voice.")}
        {tick("adult", "The speaker is 18 or over.")}
        {tick("alive", "The speaker is alive.")}
        {tick("not_public_figure", "The speaker is not a public figure, celebrity, politician or religious leader.")}
        {tick("commercial", "Commercial use is allowed.")}
      </div>
      <blockquote className="small muted" style={{ margin: "8px 0", borderLeft: "3px solid var(--gold-line)", paddingLeft: 12 }}>{props.statement}</blockquote>
      {tick("agrees", "The speaker has read and agrees to this statement.")}
    </div>
  );
}
