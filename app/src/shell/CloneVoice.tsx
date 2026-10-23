import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { api, messageOf } from "../api";
import { Confirm } from "../ui";
import type { Attestation, CloneEntry, CloneInfo, DesignerInfo, RefReport } from "../types";

const EMPTY: Attestation = { subject: "", subject_is_self: true, adult: false, alive: false, not_public_figure: false, agrees: false, purpose: "", commercial: false, expires: null };

/** Cloning a voice, only with the owner's recorded consent. Nothing is sent until the last confirmation. */
export function CloneVoice(props: { designers: DesignerInfo[]; toast: (t: string, bad?: boolean) => void; onChanged: () => void }) {
  const cloners = props.designers.filter((d) => d.clone);
  const [info, setInfo] = useState<CloneInfo | null>(null);
  const [entries, setEntries] = useState<CloneEntry[]>([]);
  const [att, setAtt] = useState<Attestation>(EMPTY);
  const [name, setName] = useState("");
  const [file, setFile] = useState<string | null>(null);
  const [report, setReport] = useState<RefReport | null>(null);
  const [adapter, setAdapter] = useState("");
  const [busy, setBusy] = useState(false);
  const [ask, setAsk] = useState(false);
  const [revoking, setRevoking] = useState<CloneEntry | null>(null);
  const [alsoDelete, setAlsoDelete] = useState(true);
  const provider = cloners.find((c) => c.id === adapter) ?? cloners.find((c) => c.ready) ?? cloners[0];

  const reload = () => { void api.consentList().then(setEntries).catch(() => setEntries([])); };
  useEffect(() => { void api.cloneInfo().then(setInfo).catch(() => setInfo(null)); reload(); }, []);

  const choose = async () => {
    const p = await open({ multiple: false, title: "Choose the recording of the voice owner", filters: [{ name: "Audio", extensions: ["wav", "mp3", "flac", "ogg", "m4a"] }] });
    if (typeof p !== "string") return;
    setFile(p); setReport(null);
    try { setReport(await api.cloneCheckReference(p)); } catch (e) { props.toast(messageOf(e), true); }
  };

  const ready = att.adult && att.alive && att.not_public_figure && att.agrees && att.subject.trim().length >= 2 && att.purpose.trim().length > 0 && name.trim().length > 0 && !!file && !!report && report.problems.length === 0;

  const create = async () => {
    if (!provider || !file) return;
    setAsk(false); setBusy(true);
    try {
      const v = await api.cloneVoice(provider.id, file, name.trim(), { ...att, expires: att.expires?.trim() ? att.expires.trim() : null });
      props.toast(`${v.name} was created and is in your library. Its consent record is saved on this computer.`);
      setAtt(EMPTY); setName(""); setFile(null); setReport(null);
      reload(); props.onChanged();
    } catch (e) { props.toast(messageOf(e), true); } finally { setBusy(false); }
  };

  const revoke = async () => {
    const e = revoking; if (!e) return;
    setRevoking(null);
    try { props.toast(await api.consentRevoke(e.consent.id, alsoDelete)); reload(); props.onChanged(); } catch (err) { props.toast(messageOf(err), true); }
  };

  const tick = (k: keyof Attestation, label: string) => (
    <label className="inline" style={{ display: "flex", gap: 8, alignItems: "flex-start", marginBottom: 6 }}>
      <input type="checkbox" checked={att[k] as boolean} onChange={(e) => setAtt({ ...att, [k]: e.target.checked })} /> <span className="small">{label}</span>
    </label>
  );

  return (
    <div className="card" role="group" aria-label="Clone a voice">
      <h2>Clone a voice</h2>
      {cloners.length === 0 ? (
        <p className="muted">Make a synthetic copy of a voice, with its owner's consent. This needs a provider that supports it. Save an ElevenLabs key in Settings and it appears here.</p>
      ) : (
        <>
          <div className="callout small"><strong>Only with the owner's clear consent.</strong>Clone your own voice, or someone's who has agreed. Children, people who have died, and public figures are refused. Soundstage cannot check who is speaking, so you are making a promise, and it is recorded.</div>
          <div className="grid2" style={{ gridTemplateColumns: "1fr 1fr" }}>
            <div><label htmlFor="c-subject">Whose voice is it?</label><input id="c-subject" type="text" style={{ width: "100%" }} maxLength={80} value={att.subject} onChange={(e) => setAtt({ ...att, subject: e.target.value })} /></div>
            <div><label htmlFor="c-name">Name for the voice</label><input id="c-name" type="text" style={{ width: "100%" }} maxLength={60} value={name} onChange={(e) => setName(e.target.value)} /></div>
            <div style={{ gridColumn: "1 / -1" }}><label htmlFor="c-purpose">What will it be used for?</label><input id="c-purpose" type="text" style={{ width: "100%" }} maxLength={200} placeholder="For example: narration for my short film" value={att.purpose} onChange={(e) => setAtt({ ...att, purpose: e.target.value })} /></div>
          </div>
          <div style={{ marginTop: 10 }}>
            {tick("subject_is_self", "This is my own voice.")}
            {tick("adult", "The owner is 18 or over.")}
            {tick("alive", "The owner is alive.")}
            {tick("not_public_figure", "The owner is not a public figure, celebrity, politician or religious leader.")}
            {tick("commercial", "The owner allows commercial use (for example a film sold or monetised).")}
          </div>
          <div className="row" style={{ alignItems: "end", marginTop: 4 }}>
            <div><label htmlFor="c-exp">Consent ends on (optional)</label><input id="c-exp" type="text" placeholder="YYYY-MM-DD" style={{ width: 150 }} value={att.expires ?? ""} onChange={(e) => setAtt({ ...att, expires: e.target.value })} /></div>
          </div>
          <blockquote className="small muted" style={{ margin: "12px 0", borderLeft: "3px solid var(--gold-line)", paddingLeft: 12 }}>{info?.statement}</blockquote>
          {tick("agrees", "The owner has read and agrees to this statement.")}

          <div className="row" style={{ marginTop: 10, alignItems: "center" }}>
            <button onClick={() => void choose()}>{file ? "Choose a different recording…" : "Choose the recording…"}</button>
            {file && <span className="small mono muted">{file.split(/[\\/]/).pop()}</span>}
          </div>
          {info && <p className="small muted" style={{ margin: "6px 0 0" }}>One person speaking clearly, with no music or other voices. At least {info.min_seconds} seconds; {info.good_seconds} seconds or more sounds better. Only the first {info.max_seconds} seconds are used.</p>}
          {report && (
            <div style={{ marginTop: 8 }} aria-live="polite">
              {report.problems.map((p) => <div key={p} className="callout bad small" role="alert" style={{ marginBottom: 6 }}><strong>Cannot use this recording</strong>{p}</div>)}
              {report.problems.length === 0 && <div className="callout ok small" style={{ marginBottom: 6 }}><strong>The recording passed</strong>{report.seconds.toFixed(0)} seconds, no clipping, enough sound.</div>}
              {report.warnings.map((w) => <p key={w} className="small muted" style={{ margin: "2px 0" }}>{w}</p>)}
            </div>
          )}
          <div className="row" style={{ marginTop: 12, alignItems: "end" }}>
            {cloners.length > 1 && <div><label htmlFor="c-src">With</label><select id="c-src" value={provider?.id ?? ""} onChange={(e) => setAdapter(e.target.value)}>{cloners.map((c) => <option key={c.id} value={c.id}>{c.label}</option>)}</select></div>}
            <button className="primary" disabled={!ready || busy || !provider?.ready} onClick={() => setAsk(true)}>{busy ? <><span className="spin" aria-hidden="true" /> Creating</> : "Create the voice"}</button>
            {provider && !provider.ready && <span className="small muted">Save a key for {provider.label} first.</span>}
          </div>
        </>
      )}

      {entries.length > 0 && (
        <>
          <h3 style={{ marginTop: 18 }}>Cloned voices and their consent</h3>
          {entries.map((e) => (
            <div key={e.consent.id} className="linecard">
              <div className="row">
                <strong>{e.voice.name}</strong>
                <span className={`chip ${e.consent.status === "active" ? "ok" : "bad"}`}>{e.consent.status === "active" ? "Consent active" : e.consent.status === "revoked" ? "Consent revoked" : "Consent expired"}</span>
                <span className="chip">{e.consent.commercial_use_permitted ? "Commercial use allowed" : "Personal use only"}</span>
                <span className="spacer" />
                {e.consent.status === "active" && <button className="small danger" onClick={() => setRevoking(e)}>Revoke consent…</button>}
              </div>
              <div className="small muted" style={{ marginTop: 4 }}>Owner: {e.consent.voice_subject} · {e.consent.purpose}{e.consent.expires ? ` · until ${e.consent.expires}` : ""}</div>
            </div>
          ))}
        </>
      )}

      {ask && provider && (
        <Confirm title={`Send this recording to ${provider.label}?`} confirmLabel="Send and create" onCancel={() => setAsk(false)}
          body={<p>The recording ({report ? `${Math.min(report.seconds, info?.max_seconds ?? 180).toFixed(0)} seconds` : "audio"}) and the voice name go over the internet to {provider.label}, and may use a voice slot or credits on your account there. The consent record stays on this computer and is not sent.</p>}
          onConfirm={() => void create()} />
      )}
      {revoking && (
        <Confirm title={`Revoke consent for “${revoking.voice.name}”?`} confirmLabel="Revoke consent" onCancel={() => setRevoking(null)}
          body={<><p>Soundstage will stop using this voice at once: no new takes, and exports that include it are blocked. Takes already made stay in your projects until you remove them. This cannot be undone.</p>
            <label className="inline"><input type="checkbox" checked={alsoDelete} onChange={(e) => setAlsoDelete(e.target.checked)} /> Also delete the voice from the provider</label></>}
          onConfirm={() => void revoke()} />
      )}
    </div>
  );
}
