import { useMemo, useState } from "react";
import { api, messageOf } from "../api";
import { licenceNote } from "../logic";
import { LicenceChip, PlayButton } from "../ui";
import { NON_VERBAL, type Voice } from "../types";

const ALL_TAGS = [...NON_VERBAL, "whisper", "shout", "excited", "sad", "tired", "sarcastic", "curious", "serious"];

export function VoiceLibrary(props: { voices: Voice[]; toast: (t: string, bad?: boolean) => void; onBack: () => void; inProject: boolean }) {
  const { voices } = props;
  const [q, setQ] = useState("");
  const [lang, setLang] = useState("");
  const [lic, setLic] = useState("");
  const [paths, setPaths] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState<string | null>(null);

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
      <div className="row" style={{ marginBottom: 12 }}>
        <h1 style={{ margin: 0 }}>Voice library</h1><span className="spacer" />
        <button onClick={props.onBack}>{props.inProject ? "Back to project" : "Back"}</button>
      </div>
      <div className="card">
        <div className="row" style={{ alignItems: "end" }}>
          <div><label htmlFor="vq">Search</label><input id="vq" type="text" value={q} onChange={(e) => setQ(e.target.value)} placeholder="Name or accent" /></div>
          <div><label htmlFor="vl">Language</label><select id="vl" value={lang} onChange={(e) => setLang(e.target.value)}><option value="">All</option>{languages.map((l) => <option key={l} value={l}>{l === "zxx" ? "zxx (tones only)" : l}</option>)}</select></div>
          <div><label htmlFor="vli">Licence</label><select id="vli" value={lic} onChange={(e) => setLic(e.target.value)}><option value="">All</option><option value="commercial">Commercial</option><option value="unknown">Unknown</option><option value="non-commercial">Non-commercial</option></select></div>
        </div>
      </div>
      {!voices.some((v) => v.sends_data) && <div className="callout small"><strong>Want cloud voices?</strong>Save an ElevenLabs key in Settings and their premade and designed voices appear here. Cloned and professional voices from your account are never listed, because they need a consent record.</div>}
      {voices.length === 0 && <div className="callout warn"><strong>Looking for voices…</strong>If this stays empty, Windows has no speech voices installed. Add one in Windows Settings under Time &amp; language, then Speech.</div>}
      <div className="card">
        <table>
          <thead><tr><th>Voice</th><th>Language · accent</th><th>Gender · age</th><th>Licence</th><th>Can perform</th><th /></tr></thead>
          <tbody>
            {shown.map((v) => (
              <tr key={v.id}>
                <td><strong>{v.name}</strong><div className="small muted">{v.provider_label}{v.sends_data ? " · sends text online" : " · on this computer"}</div></td>
                <td>{v.language} · {v.accent}</td>
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
        {voices.length > 0 && shown.length === 0 && <p className="muted">No voices match those filters.</p>}
      </div>
      <div className="card">
        <h2>Voice design and cloning</h2>
        <p><strong>Voice design</strong> (describe a voice and generate it) needs a provider that supports it. None of the installed voices do, so it is not offered yet.</p>
        <p style={{ marginBottom: 0 }}><strong>Voice cloning</strong> arrives in a later version. It will only ever work with a verified, revocable consent record from the voice's owner, and it will be impossible without one. Cloning public figures, children and deceased people is refused.</p>
      </div>
      <p className="small muted">Accent and non-verbal quality are not rated here. Soundstage lists a language or sound as supported only after native-speaker listening tests show it works.</p>
    </div>
  );
}
