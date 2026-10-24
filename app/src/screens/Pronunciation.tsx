import { useEffect, useState } from "react";
import { api, messageOf } from "../api";
import * as I from "../icons";
import { Help, PlayButton } from "../ui";
import type { Project, PronRule, Voice } from "../types";

const blank = (): PronRule => ({ from: "", to: "", whole_word: true, ignore_case: true });

function Editor(props: { title: string; blurb: string; rules: PronRule[]; onChange: (r: PronRule[]) => void; onSave?: () => void; dirty?: boolean; saving?: boolean; idp: string }) {
  const { rules } = props;
  const set = (i: number, patch: Partial<PronRule>) => props.onChange(rules.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  return (
    <div className="card" role="group" aria-label={props.title}>
      <div className="row"><h2 style={{ margin: 0 }}>{props.title}</h2><span className="spacer" />
        <button className="small" onClick={() => props.onChange([...rules, blank()])}><I.Plus size={14} /> Add a word</button>
        {props.onSave && <button className="primary small" disabled={!props.dirty || props.saving} onClick={props.onSave}>{props.saving ? "Saving…" : "Save"}</button>}
      </div>
      <p className="small muted" style={{ margin: "6px 0 10px" }}>{props.blurb}</p>
      {rules.length === 0 && <p className="muted small">No words yet.</p>}
      {rules.map((r, i) => (
        <div key={i} className="row" style={{ alignItems: "end", marginBottom: 8 }}>
          <div><label htmlFor={`${props.idp}-f-${i}`}>When the script says</label><input id={`${props.idp}-f-${i}`} type="text" style={{ width: 190 }} maxLength={60} value={r.from} onChange={(e) => set(i, { from: e.target.value })} spellCheck={false} /></div>
          <div><label htmlFor={`${props.idp}-t-${i}`}>Say it like</label><input id={`${props.idp}-t-${i}`} type="text" style={{ width: 230 }} maxLength={120} value={r.to} onChange={(e) => set(i, { to: e.target.value })} spellCheck={false} placeholder="for example Oh-kah-for" /></div>
          <label className="inline"><input type="checkbox" checked={r.whole_word} onChange={(e) => set(i, { whole_word: e.target.checked })} /> Whole word</label>
          <label className="inline"><input type="checkbox" checked={!r.ignore_case} onChange={(e) => set(i, { ignore_case: !e.target.checked })} /> Match capitals</label>
          <button className="small danger icon" onClick={() => props.onChange(rules.filter((_, j) => j !== i))} aria-label={`Remove the rule for ${r.from || "this word"}`}><I.Trash size={14} /></button>
        </div>
      ))}
    </div>
  );
}

/** Teach the voices how to say names and terms. Only what the voice is given changes; the script keeps its spelling. */
export function Pronunciation(props: { voices: Voice[]; project: Project | null; onEditProject: (fn: (p: Project) => Project) => void; toast: (t: string, bad?: boolean) => void; onBack: () => void }) {
  const [rules, setRules] = useState<PronRule[]>([]);
  const [saved, setSaved] = useState<string>("[]");
  const [saving, setSaving] = useState(false);
  const [sample, setSample] = useState("Ama Okafor said hello.");
  const [result, setResult] = useState("");
  const [heard, setHeard] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [voiceId, setVoiceId] = useState("");

  useEffect(() => { void api.pronunciationGet().then((r) => { setRules(r); setSaved(JSON.stringify(r)); }).catch((e) => props.toast(messageOf(e), true)); }, []); // eslint-disable-line react-hooks/exhaustive-deps
  const dirty = JSON.stringify(rules) !== saved;
  const projectRules = props.project?.pronunciations ?? [];

  const save = async () => {
    setSaving(true);
    try { await api.pronunciationSave(rules.filter((r) => r.from.trim() || r.to.trim())); const r = await api.pronunciationGet(); setRules(r); setSaved(JSON.stringify(r)); props.toast("Pronunciation saved. Lines that change will need rendering again."); }
    catch (e) { props.toast(messageOf(e), true); } finally { setSaving(false); }
  };

  const tryIt = async () => {
    try { setResult(await api.pronunciationPreview(sample, projectRules.filter((r) => r.from.trim() && r.to.trim()), rules.filter((r) => r.from.trim() && r.to.trim()))); }
    catch (e) { props.toast(messageOf(e), true); }
  };

  const local = props.voices.filter((v) => !v.sends_data && v.language !== "zxx");
  const voice = props.voices.find((v) => v.id === voiceId) ?? local[0] ?? props.voices[0];
  const hear = async () => {
    if (!voice) return;
    if (voice.sends_data && !window.confirm(`This sends the text to ${voice.provider_label} and uses your account's credits. Continue?`)) return;
    setBusy(true);
    try { const spoken = await api.pronunciationPreview(sample, projectRules, rules); setResult(spoken); const r = await api.speakText(voice.id, sample, null, null, false); setHeard(r.path); }
    catch (e) { props.toast(messageOf(e), true); } finally { setBusy(false); }
  };

  return (
    <div className="page">
      <div className="stage-title"><h1>Pronunciation</h1><span className="spacer" /><button onClick={props.onBack}>Back</button></div>
      <div className="callout small"><strong>Names and terms a voice gets wrong.</strong>Add a word and how it should sound. Only what the voice is given changes: your script, subtitles and exports keep the original spelling. Spell it the way you would write it for a person to read aloud, such as <span className="mono">Oh-kah-for</span>. Some voices follow spelling hints better than others, so listen to check.</div>

      <Editor title="All projects" blurb="Used in every project, on the Speak screen and in samples." rules={rules} onChange={setRules} onSave={() => void save()} dirty={dirty} saving={saving} idp="g" />
      {props.project ? (
        <Editor title={`This project: ${props.project.name}`} blurb="Only for this project. These win over the all-projects list when both match." rules={projectRules} idp="p"
          onChange={(r) => props.onEditProject((p) => ({ ...p, pronunciations: r }))} />
      ) : <p className="small muted">Open a project to add words that only apply to it.</p>}

      <div className="card">
        <h2>Try it <Help text="See what the voice is given, and hear it. The first voice on this computer is used unless you choose another." /></h2>
        <label htmlFor="pr-s">Some text</label>
        <textarea id="pr-s" rows={2} style={{ width: "100%" }} maxLength={500} value={sample} onChange={(e) => setSample(e.target.value)} />
        <div className="row" style={{ marginTop: 8, alignItems: "end" }}>
          <div><label htmlFor="pr-v">Voice</label><select id="pr-v" value={voice?.id ?? ""} onChange={(e) => setVoiceId(e.target.value)}>{props.voices.slice(0, 200).map((v) => <option key={v.id} value={v.id}>{v.name} · {v.provider_label}</option>)}</select></div>
          <button className="small" onClick={() => void tryIt()}>Show what the voice is given</button>
          <button className="primary small" disabled={busy || !voice || !sample.trim()} onClick={() => void hear()}>{busy ? "Making audio…" : "Hear it"}</button>
          <PlayButton path={heard} label="the pronunciation sample" bust={heard ?? undefined} />
        </div>
        {result && <p className="small" style={{ margin: "10px 0 0" }}>The voice is given: <span className="mono">{result}</span></p>}
      </div>
    </div>
  );
}
