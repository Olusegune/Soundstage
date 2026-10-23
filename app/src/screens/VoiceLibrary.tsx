import { useEffect, useMemo, useState } from "react";
import { api, messageOf } from "../api";
import { licenceNote } from "../logic";
import { Flag, languageName } from "../flags";
import { LicenceChip, PlayButton } from "../ui";
import { PromptHelper } from "../shell/PromptHelper";
import { CloneVoice } from "../shell/CloneVoice";
import { NON_VERBAL, type DesignerInfo, type DesignPreview, type Voice } from "../types";

const ALL_TAGS = [...NON_VERBAL, "whisper", "shout", "excited", "sad", "tired", "sarcastic", "curious", "serious"];

export function VoiceLibrary(props: { voices: Voice[]; toast: (t: string, bad?: boolean) => void; onBack: () => void; inProject: boolean; onSettings: () => void; onEngines: () => void; onChanged?: () => void }) {
  const { voices } = props;
  const [q, setQ] = useState("");
  const [lang, setLang] = useState("");
  const [lic, setLic] = useState("");
  const [paths, setPaths] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const [limit, setLimit] = useState(100);

  const [designers, setDesigners] = useState<DesignerInfo[]>([]);
  const [des, setDes] = useState({ adapter: "", text: "", busy: false, confirm: false, previews: [] as DesignPreview[], pick: 0, name: "", saving: false });
  useEffect(() => { void api.listDesigners().then(setDesigners).catch(() => setDesigners([])); }, []);
  const designable = designers.filter((d) => d.design);
  const designer = designable.find((d) => d.id === des.adapter) ?? designable.find((d) => d.ready) ?? designable[0];
  const designLen = des.text.trim().length;

  const makePreviews = async () => {
    if (!designer) return;
    setDes((d) => ({ ...d, busy: true, confirm: false, previews: [] }));
    try {
      const previews = await api.designVoice(designer.id, des.text.trim(), null);
      setDes((d) => ({ ...d, previews, pick: 0 }));
    } catch (e) { props.toast(messageOf(e), true); } finally { setDes((d) => ({ ...d, busy: false })); }
  };
  const keep = async () => {
    const p = des.previews[des.pick];
    if (!designer || !p || !des.name.trim()) return;
    setDes((d) => ({ ...d, saving: true }));
    try {
      await api.saveDesignedVoice(designer.id, p.generated_voice_id, des.name.trim(), des.text.trim());
      props.toast(`${des.name.trim()} is in your library. Its licence is unknown until you record the provider's terms.`);
      setDes((d) => ({ ...d, previews: [], name: "", text: "" }));
      props.onChanged?.();
    } catch (e) { props.toast(messageOf(e), true); } finally { setDes((d) => ({ ...d, saving: false })); }
  };

  const languages = useMemo(() => [...new Set(voices.map((v) => v.language))].sort(), [voices]);
  const shown = voices.filter((v) =>
    (!lang || v.language === lang) && (!lic || v.licence.licence === lic) &&
    (!q || `${v.name} ${v.accent} ${v.provider_label}`.toLowerCase().includes(q.toLowerCase())));

  const sample = async (v: Voice) => {
    if (v.sends_data && !window.confirm(`This sends a short sample line to ${v.provider_label} and uses your account's credits. Continue?`)) return;
    setBusy(v.id);
    try { const r = await api.previewVoice(v.id, v.sample_text); setPaths((s) => ({ ...s, [v.id]: r.path })); }
    catch (e) { props.toast(messageOf(e), true); } finally { setBusy(null); }
  };

  return (
    <div className="page">
      <div className="stage-title">
        <h1>Voice library</h1><span className="spacer" />
        <button onClick={props.onBack}>{props.inProject ? "Back to project" : "Back"}</button>
      </div>
      <div className="card">
        <div className="row" style={{ alignItems: "end" }}>
          <div><label htmlFor="vq">Search</label><input id="vq" type="text" value={q} onChange={(e) => { setQ(e.target.value); setLimit(100); }} placeholder="Name or accent" /></div>
          <div><label htmlFor="vl">Language</label><select id="vl" value={lang} onChange={(e) => { setLang(e.target.value); setLimit(100); }}><option value="">All</option>{languages.map((l) => <option key={l} value={l}>{languageName(l)}</option>)}</select></div>
          <div><label htmlFor="vli">Licence</label><select id="vli" value={lic} onChange={(e) => setLic(e.target.value)}><option value="">All</option><option value="commercial">Commercial</option><option value="unknown">Unknown</option><option value="non-commercial">Non-commercial</option></select></div>
        </div>
      </div>
      {!voices.some((v) => v.sends_data) && <div className="callout small"><strong>Want cloud voices?</strong>Save an ElevenLabs key, or add a local engine, and more voices appear here. Cloned and professional voices from your account are never listed, because they need a consent record.<div className="row" style={{ marginTop: 8 }}><button className="small" onClick={props.onSettings}>Open Settings</button><button className="small" onClick={props.onEngines}>Add a local engine</button></div></div>}
      {voices.length === 0 && <div className="callout warn"><strong>Looking for voices…</strong>If this stays empty, Windows has no speech voices installed. Add one in Windows Settings under Time &amp; language, then Speech.</div>}
      <div className="card">
        <table>
          <thead><tr><th>Voice</th><th>Language · accent</th><th>Gender · age</th><th>Licence</th><th>Can perform</th><th /></tr></thead>
          <tbody>
            {shown.slice(0, limit).map((v) => (
              <tr key={v.id}>
                <td><strong>{v.name}</strong><div className="small muted">{v.provider_label}{v.sends_data ? " · sends text online" : " · on this computer"}</div></td>
                <td><Flag locale={v.language} name /><div className="small muted">{v.accent}</div></td>
                <td>{v.gender} · {v.age_range}</td>
                <td><LicenceChip licence={v.licence.licence} note={v.licence.notes} /><div className="small muted">{licenceNote(v.licence)}</div></td>
                <td className="small" style={{ maxWidth: 260 }}>
                  {ALL_TAGS.filter((t) => v.tags.includes(t)).join(", ") || "Pauses only"}
                  <div className="muted">Cannot: {ALL_TAGS.filter((t) => !v.tags.includes(t)).join(", ") || "nothing listed"}</div>
                </td>
                <td><div className="row"><button className="small" disabled={busy === v.id} onClick={() => void sample(v)}>{busy === v.id ? "…" : "Make sample"}</button><PlayButton path={paths[v.id] ?? null} label={v.name} bust={paths[v.id]} /></div></td>
              </tr>
            ))}
          </tbody>
        </table>
        {shown.length > limit && <div className="row" style={{ marginTop: 10 }}><span className="small muted">Showing {limit} of {shown.length} voices. Use the search and language filters to narrow them.</span><button className="small" onClick={() => setLimit(limit + 200)}>Show 200 more</button></div>}
        {voices.length > 0 && shown.length === 0 && <p className="muted">No voices match those filters.</p>}
      </div>
      <div className="card" role="group" aria-label="Design a voice">
        <h2>Design a voice</h2>
        {designable.length === 0 ? (
          <p className="muted">Describe a voice in words and have it made for you. This needs a provider that supports it. Save an ElevenLabs key in Settings and it appears here.</p>
        ) : (
          <>
            <p className="muted small">Describe the character, not a real person: age, tone, pace, accent. You hear three candidates and keep the one you like.</p>
            <label htmlFor="d-text">Description <span className="faint small">({designLen}/1000, at least 20)</span></label>
            <textarea id="d-text" rows={3} maxLength={1000} style={{ width: "100%" }} value={des.text} onChange={(e) => setDes({ ...des, text: e.target.value })} placeholder="A calm older woman with a warm Lagos accent, slow and kind" />
            <PromptHelper kind="voice" value={des.text} onChange={(v) => setDes((d) => ({ ...d, text: v }))} toast={props.toast} />
            <div className="row" style={{ marginTop: 8, alignItems: "end" }}>
              {designable.length > 1 && <div><label htmlFor="d-src">With</label><select id="d-src" value={designer?.id ?? ""} onChange={(e) => setDes({ ...des, adapter: e.target.value })}>{designable.map((d) => <option key={d.id} value={d.id}>{d.label}</option>)}</select></div>}
              <button className="primary" disabled={des.busy || !designer?.ready || designLen < 20} onClick={() => setDes({ ...des, confirm: true })}>{des.busy ? <><span className="spin" aria-hidden="true" /> Designing</> : "Design 3 voices"}</button>
              {designer && !designer.ready && <span className="small muted">Save a key for {designer.label} first.</span>}
            </div>
            {des.confirm && designer && (
              <div className="callout" role="alert" style={{ marginTop: 10 }}><strong>Send this description to {designer.label}?</strong>It is sent over the internet and may use credits on your account. Nothing from your projects is sent.
                <div className="row" style={{ marginTop: 8 }}><button className="primary small" onClick={() => void makePreviews()}>Send and design</button><button className="small" onClick={() => setDes({ ...des, confirm: false })}>Cancel</button></div></div>
            )}
            {des.previews.length > 0 && (
              <div style={{ marginTop: 14 }} role="radiogroup" aria-label="Candidate voices">
                {des.previews.map((p, i) => (
                  <div key={p.generated_voice_id} className="row" style={{ marginBottom: 6 }}>
                    <label className="inline"><input type="radio" name="d-pick" checked={des.pick === i} onChange={() => setDes({ ...des, pick: i })} /> Candidate {i + 1}</label>
                    <PlayButton path={p.path} label={`candidate ${i + 1}`} bust={p.path} />
                  </div>
                ))}
                <div className="row" style={{ alignItems: "end", marginTop: 8 }}>
                  <div><label htmlFor="d-name">Name this voice</label><input id="d-name" type="text" maxLength={60} value={des.name} onChange={(e) => setDes({ ...des, name: e.target.value })} /></div>
                  <button className="primary" disabled={des.saving || !des.name.trim()} onClick={() => void keep()}>{des.saving ? "Keeping…" : "Keep this voice"}</button>
                </div>
              </div>
            )}
          </>
        )}
      </div>
      <CloneVoice designers={designers} toast={props.toast} onChanged={() => props.onChanged?.()} />
      <p className="small muted">Accent and non-verbal quality are not rated here. Soundstage lists a language or sound as supported only after native-speaker listening tests show it works.</p>
    </div>
  );
}
