import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import { api, messageOf } from "../api";
import * as I from "../icons";
import type { Prefs, Theme } from "../prefs";
import type { SystemCheck } from "../types";
import { Confirm } from "../ui";

export function Settings(props: {
  info: { version: string; projects_root: string } | null;
  toast: (t: string, bad?: boolean) => void;
  onKeysChanged: () => void;
  onBack: () => void;
  inProject: boolean;
  prefs: Prefs;
  onPrefs: (p: Partial<Prefs>) => void;
  onTheme: (t: Theme) => void;
  onShortcuts: () => void;
  onTour: () => void;
  onEngines: () => void;
}) {
  const [has, setHas] = useState<boolean | null>(null);
  const [key, setKey] = useState("");
  const [removing, setRemoving] = useState(false);
  const [helperKey, setHelperKey] = useState("");
  const [checks, setChecks] = useState<SystemCheck[]>([]);
  const [checking, setChecking] = useState(false);
  const runCheck = async () => { setChecking(true); try { setChecks(await api.systemCheck(props.prefs.ffmpegPath || null)); } catch (e) { props.toast(messageOf(e), true); } finally { setChecking(false); } };
  useEffect(() => { void runCheck(); }, []); // eslint-disable-line react-hooks/exhaustive-deps
  const [testing, setTesting] = useState(false);
  const [test, setTest] = useState<{ ok: boolean; text: string; at: string } | null>(null);

  useEffect(() => { void api.hasApiKey("elevenlabs").then(setHas).catch(() => setHas(null)); }, []);

  const save = async () => {
    try {
      await api.setApiKey("elevenlabs", key);
      setKey(""); setHas(true); setTest(null);
      props.onKeysChanged();
      props.toast("Key saved to the Windows Credential Manager. Use Test connection to check it.");
    } catch (e) { props.toast(messageOf(e), true); }
  };

  const runTest = async () => {
    setTesting(true);
    const at = new Date().toLocaleTimeString();
    try { setTest({ ok: true, text: await api.testConnection("elevenlabs"), at }); props.onKeysChanged(); }
    catch (e) { setTest({ ok: false, text: messageOf(e), at }); }
    finally { setTesting(false); }
  };

  return (
    <div className="page">
      <div className="stage-title"><h1>Settings</h1><span className="spacer" /><button onClick={props.onBack}>{props.inProject ? "Back to project" : "Back"}</button></div>

      <div className="card">
        <h2>Appearance and workspace</h2>
        <div className="row" style={{ alignItems: "end" }}>
          <div>
            <label>Theme</label>
            <div className="seg" role="radiogroup" aria-label="Theme">
              {([["dark", "Dark", <I.Moon key="m" size={14} />], ["light", "Light", <I.Sun key="s" size={14} />], ["system", "Match Windows", <I.Device key="d" size={14} />]] as const).map(([v, l, ic]) => (
                <button key={v} role="radio" aria-checked={props.prefs.theme === v} onClick={() => props.onTheme(v)}>{ic} {l}</button>
              ))}
            </div>
          </div>
          <div>
            <label>Interface</label>
            <div className="seg" role="radiogroup" aria-label="Interface mode">
              {(["simple", "advanced"] as const).map((m) => <button key={m} role="radio" aria-checked={props.prefs.mode === m} onClick={() => props.onPrefs({ mode: m })}>{m === "simple" ? "Simple" : "Advanced"}</button>)}
            </div>
          </div>
        </div>
        <p className="small muted" style={{ marginTop: 10 }}>Simple shows the essentials. Advanced adds finer controls. It is the same project in both, and hidden controls keep their values.</p>
        <div className="row" style={{ marginTop: 6 }}>
          <button onClick={props.onShortcuts}><I.Keyboard size={16} /> Keyboard shortcuts…</button>
          <button onClick={props.onTour}><I.Rocket size={16} /> Take the guided tour</button>
          <button className="ghost" onClick={() => { props.onPrefs({ welcomed: false, toured: false }); props.toast("The welcome screen will show next time Soundstage starts."); }}>Show the welcome screen again</button>
        </div>
      </div>

      <div className="card" role="group" aria-label="System check">
        <div className="row"><h2 style={{ margin: 0 }}>System check</h2><span className="spacer" />
          <button className="small" onClick={() => void runCheck()}>{checking ? "Checking…" : "Check again"}</button>
          <button className="small" onClick={() => { void api.writeDiagnostics(props.prefs.ffmpegPath || null).then((p) => { props.toast("Report saved. Attach it when you ask for help. It holds no keys or script text."); void revealItemInDir(p); }).catch((e) => props.toast(messageOf(e), true)); }}>Report a problem…</button></div>
        <p className="small muted">What this computer has. Nothing here is required: every missing piece has a free alternative.</p>
        {checks.map((c) => (
          <div key={c.id} className="linecard">
            <div className="row"><span className={`chip ${c.status === "ok" ? "ok" : c.status === "warn" ? "warn" : "info"}`}>{c.status === "ok" ? "Ready" : c.status === "warn" ? "Needs attention" : "Optional"}</span><strong>{c.label}</strong></div>
            <div className="small muted" style={{ marginTop: 4 }}>{c.detail}</div>
            {c.fix && <div className="small" style={{ marginTop: 4 }}>{c.fix}</div>}
          </div>
        ))}
      </div>

      <div className="card">
        <h2>AI prompt helper <span className="chip info" style={{ verticalAlign: "middle" }}>Optional</span></h2>
        <p className="small muted">The “Help me write this” panels always work offline. To also get an “Improve with AI” button, point Soundstage at a chat server. Ollama running on this computer is free and sends nothing out. Ollama writes descriptions only; it cannot make sound.</p>
        <div className="grid2" style={{ gridTemplateColumns: "1fr 1fr" }}>
          <div><label htmlFor="a-url">Server address</label><input id="a-url" type="text" style={{ width: "100%" }} spellCheck={false} value={props.prefs.assistantUrl} onChange={(e) => props.onPrefs({ assistantUrl: e.target.value.trim() })} /></div>
          <div><label htmlFor="a-model">Model name</label><input id="a-model" type="text" style={{ width: "100%" }} spellCheck={false} placeholder="for example llama3.2" value={props.prefs.assistantModel} onChange={(e) => props.onPrefs({ assistantModel: e.target.value.trim() })} /></div>
        </div>
        <div className="row" style={{ marginTop: 8, alignItems: "end" }}>
          <div style={{ flex: 1, minWidth: 240 }}><label htmlFor="a-key">Key (only if the server asks for one)</label><input id="a-key" type="password" autoComplete="off" style={{ width: "100%" }} maxLength={512} value={helperKey} onChange={(e) => setHelperKey(e.target.value)} placeholder="Paste a key" /></div>
          <button disabled={!helperKey.trim()} onClick={() => { void api.setApiKey("assistant", helperKey).then(() => { setHelperKey(""); props.toast("Helper key saved in the Windows Credential Manager."); }).catch((e) => props.toast(messageOf(e), true)); }}>Save key</button>
        </div>
      </div>

      <div className="card hero">
        <div className="row"><div style={{ flex: 1, minWidth: 260 }}><h2 style={{ margin: 0 }}>Local and custom voice engines</h2>
          <p className="small muted" style={{ margin: "4px 0 0" }}>No account or key needed. Connect an engine running on your own computer, or add Piper, and use its voices next to the built-in ones.</p></div>
          <button className="primary" onClick={props.onEngines}><I.Layers size={16} /> Manage voice engines</button></div>
      </div>

      <div className="card">
        <h2>Voice providers</h2>
        <table><tbody>
          <tr><td><strong>Windows voices</strong><div className="small muted">Offline. Built into Windows.</div></td><td><span className="chip ok"><I.Check size={12} /> Ready, no key needed</span></td><td className="small muted">Runs on this computer. Licence unknown until you check the terms yourself.</td></tr>
          <tr><td><strong>Test tones</strong><div className="small muted">Beeps for checking timing.</div></td><td><span className="chip ok"><I.Check size={12} /> Ready</span></td><td className="small muted">Generated by Soundstage.</td></tr>
          <tr><td><strong>ElevenLabs</strong><div className="small muted">Cloud voices (Eleven v3) with expressive tags.</div></td>
            <td>{has === null ? <span className="chip">Checking…</span> : !has ? <span className="chip warn">No key saved</span> : test?.ok ? <span className="chip ok"><I.Check size={12} /> Connected</span> : test && !test.ok ? <span className="chip bad"><I.Close size={12} /> Not working</span> : <span className="chip info">Key saved, not tested</span>}</td>
            <td className="small muted">Script text is sent to ElevenLabs and billed to your account there. You are asked before each render.</td></tr>
        </tbody></table>

        <h3 style={{ marginTop: 18 }}>ElevenLabs key</h3>
        <p className="small muted">Stored in the Windows Credential Manager. It is never written to project files or logs, and the app never shows a saved key again. Get a key from your ElevenLabs account page.</p>
        <div className="row">
          <label htmlFor="el-key" className="sr-only">ElevenLabs key</label>
          <input id="el-key" type="password" autoComplete="off" value={key} maxLength={512} onChange={(e) => setKey(e.target.value)} placeholder={has ? "A key is saved. Paste a new one to replace it" : "Paste a key"} style={{ minWidth: 340 }} />
          <button disabled={!key.trim()} onClick={() => void save()}>Save key</button>
          <button disabled={!has || testing} onClick={() => void runTest()}>{testing ? "Testing…" : "Test connection"}</button>
          <button className="danger" disabled={!has} onClick={() => setRemoving(true)}>Remove saved key</button>
        </div>
        {test && <div className={`callout ${test.ok ? "ok" : "bad"} small`} style={{ marginTop: 10 }} role="status"><strong>{test.ok ? "Connected" : "Not working"}. Last checked {test.at}</strong>{test.text}</div>}
        <p className="small muted" style={{ marginBottom: 0, marginTop: 10 }}>
          Voice rights: whether you may use ElevenLabs output commercially depends on your plan. Voices start as “licence unknown”. Read their current terms, then record your check on the Export screen so a Commercial project can use them.
        </p>
      </div>

      <div className="card">
        <h2>Safety and licensing</h2>
        <ul>
          <li>Voice cloning is off in this version. It will require a verified consent record, and is refused without one. Cloned, professional and “famous” voices in your ElevenLabs account are never listed.</li>
          <li>Commercial export is blocked for any voice, engine, music or effect whose licence is unknown, non-commercial, or not checked in the last 90 days.</li>
          <li>Scripts that look like fraud, impersonate public officials, or sexualise minors are not rendered.</li>
          <li>Every export carries a manifest that records the audio is AI-generated.</li>
        </ul>
      </div>

      <div className="card">
        <h2>About</h2>
        <p className="small">Soundstage {props.info?.version ?? ""}. Projects are stored in <span className="mono">{props.info?.projects_root ?? "…"}</span>. Deleting a project removes its folder. The only network host Soundstage contacts is <span className="mono">api.elevenlabs.io</span>, and only when you render or sample an ElevenLabs voice or test the connection.</p>
      </div>
      {removing && <Confirm title="Remove the saved ElevenLabs key?" confirmLabel="Remove key" onCancel={() => setRemoving(false)} body={<p>The key will be deleted from the Windows Credential Manager and ElevenLabs voices will disappear from the library. Projects that use them keep their rendered takes but cannot render new ones until a key is saved again.</p>}
        onConfirm={() => { void api.deleteApiKey("elevenlabs").then(() => { setHas(false); setTest(null); props.onKeysChanged(); props.toast("Key removed."); }).catch((e) => props.toast(messageOf(e), true)); setRemoving(false); }} />}
    </div>
  );
}
