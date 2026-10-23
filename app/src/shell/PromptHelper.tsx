import { useState } from "react";
import { api, messageOf } from "../api";
import { loadPrefs } from "../prefs";
import { reachOf } from "../enginePresets";
import { EXAMPLES, FIELDS, TIPS, buildPrompt, strength, type HelperKind, type Picked } from "../promptBuilder";

const TITLES: Record<HelperKind, string> = { music: "music", sfx: "sound effect", voice: "voice" };

/** Guides a first-time user to a good description. Offline choices first; an optional AI rewrite on top. */
export function PromptHelper(props: { kind: HelperKind; value: string; onChange: (v: string) => void; toast: (t: string, bad?: boolean) => void }) {
  const { kind, value } = props;
  const [open, setOpen] = useState(false);
  const [picked, setPicked] = useState<Picked>({});
  const [busy, setBusy] = useState(false);
  const [ask, setAsk] = useState(false);
  const prefs = loadPrefs();
  const s = strength(kind, value);
  const configured = prefs.assistantModel.trim().length > 0 && prefs.assistantUrl.trim().length > 0;
  const remote = reachOf(prefs.assistantUrl) === "the internet";

  const pick = (key: string, id: string) => {
    const next = { ...picked, [key]: picked[key] === id ? "" : id };
    setPicked(next);
    const labelled: Picked = {};
    for (const f of FIELDS[kind]) labelled[f.key] = f.options.find((o) => o.id === next[f.key])?.label ?? "";
    props.onChange(buildPrompt(kind, labelled, ""));
  };

  const improve = async () => {
    setAsk(false);
    setBusy(true);
    try { props.onChange(await api.improvePrompt(prefs.assistantUrl, prefs.assistantModel, kind, value)); props.toast("Improved. Read it and change anything you like."); }
    catch (e) { props.toast(messageOf(e), true); } finally { setBusy(false); }
  };

  return (
    <div className="prompt-helper">
      <div className="row tight" style={{ alignItems: "center" }}>
        <button type="button" className="small" aria-expanded={open} onClick={() => setOpen(!open)}>{open ? "Hide the prompt helper" : "Help me write this"}</button>
        <span className={`small ${s.level === "good" ? "ok-text" : "muted"}`} role="status">{s.message}</span>
      </div>
      {open && (
        <div className="linecard" style={{ marginTop: 8 }} role="group" aria-label={`Prompt helper for a ${TITLES[kind]}`}>
          <p className="small muted" style={{ marginTop: 0 }}>Tap the choices that fit. Your description fills in above, and you can still edit it.</p>
          {FIELDS[kind].map((f) => (
            <div key={f.key} style={{ marginBottom: 8 }}>
              <div className="small"><strong>{f.label}</strong> <span className="muted">{f.help}</span></div>
              <div className="row tight" role="group" aria-label={f.label}>
                {f.options.map((o) => <button type="button" key={o.id} className={`small chip-btn ${picked[f.key] === o.id ? "on" : ""}`} aria-pressed={picked[f.key] === o.id} onClick={() => pick(f.key, o.id)}>{o.label}</button>)}
              </div>
            </div>
          ))}
          <div className="small"><strong>Or start from an example</strong></div>
          <div className="row tight" style={{ marginBottom: 8 }}>
            {EXAMPLES[kind].map((e) => <button type="button" key={e} className="small ghost" onClick={() => props.onChange(e)}>{e.length > 52 ? `${e.slice(0, 50)}…` : e}</button>)}
          </div>
          <ul className="small muted" style={{ margin: "0 0 8px", paddingLeft: 18 }}>{TIPS[kind].map((t) => <li key={t}>{t}</li>)}</ul>
          <div className="row">
            <button type="button" className="small" disabled={busy || !configured || value.trim().length < 3} onClick={() => (remote ? setAsk(true) : void improve())}>{busy ? "Improving…" : "Improve with AI"}</button>
            {!configured && <span className="small muted">Optional. Set an AI helper (such as Ollama on this computer) in Settings.</span>}
          </div>
          {ask && (
            <div className="callout small" role="alert" style={{ marginTop: 8 }}><strong>Send this text to {prefs.assistantUrl}?</strong>It goes over the internet to the helper you chose, and may use credits there. Only this description is sent.
              <div className="row" style={{ marginTop: 6 }}><button type="button" className="primary small" onClick={() => void improve()}>Send and improve</button><button type="button" className="small" onClick={() => setAsk(false)}>Cancel</button></div></div>
          )}
        </div>
      )}
    </div>
  );
}
