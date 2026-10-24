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
  const [vk, setVk] = useState({ key: "", user: "", has: false, result: "" as string, bad: false, busy: false });
  const vkRefresh = () => { void Promise.all([api.hasApiKey("voicekiller"), api.hasApiKey("voicekiller:userid")]).then(([a, b]) => setVk((v) => ({ ...v, has: a && b }))).catch(() => undefined); };
  useEffect(() => { vkRefresh(); }, []); // eslint-disable-line react-hooks/exhaustive-deps
  const vkSave = async () => {
    setVk((v) => ({ ...v, busy: true, result: "" }));
    try {
      if (vk.key.trim()) await api.setApiKey("voicekiller", vk.key);
      if (vk.user.trim()) await api.setApiKey("voicekiller:userid", vk.user);
      setVk((v) => ({ ...v, key: "", user: "" })); vkRefresh(); props.onKeysChanged();
      props.toast("VoiceKiller details saved in the Windows Credential Manager.");
    } catch (e) { props.toast(messageOf(e), true); } finally { setVk((v) => ({ ...v, busy: false })); }
  };
  const vkTest = async () => {
    setVk((v) => ({ ...v, busy: true, result: "" }));
    try { await api.testConnection("voicekiller"); setVk((v) => ({ ...v, result: "Connected. The key and user ID work.", bad: false })); props.onKeysChanged(); }
    catch (e) { setVk((v) => ({ ...v, result: messageOf(e), bad: true })); } finally { setVk((v) => ({ ...v, busy: false })); }
  };
  const vkRemove = async () => { try { await api.deleteApiKey("voicekiller"); await api.deleteApiKey("voicekiller:userid"); setVk((v) => ({ ...v, has: false, result: "" })); props.onKeysChanged(); props.toast("VoiceKiller details removed."); } catch (e) { props.toast(messageOf(e), true); } };
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
          <div><label htmlFor="ui-scale">Interface size {Math.round(props.prefs.uiScale * 100)}%</label><input id="ui-scale" type="range" min={0.8} max={1.8} step={0.05} value={props.prefs.uiScale} onChange={(e) => props.onPrefs({ uiScale: Number(e.target.value) })} /></div>
          <label className="inline"><input type="checkbox" checked={props.prefs.keepHistory} onChange={(e) => props.onPrefs({ keepHistory: e.target.checked })} /> Keep a history of what Speak and Audio tools make</label>
          <button className="ghost" onClick={() => { props.onPrefs({ welcomed: false, toured: false }); props.toast("The welcome screen will show next time Soundstage starts."); }}>Show the welcome screen again</button>
        </div>
      </div>

      <div className="card" role="group" aria-label="VoiceKiller">
        <div className="row"><h2 style={{ margin: 0 }}>VoiceKiller</h2>{vk.has ? <span className="chip ok">Saved</span> : <span className="chip">Not set up</span>}<span className="chip gold">Sends text over the internet</span></div>
        <p className="small muted">About two thousand voices in many languages, and your own voices cloned on voicekiller.com. Find your API key and user ID in the VoiceKiller dashboard under API. Both are kept in the Windows Credential Manager.</p>
        <div className="grid2" style={{ gridTemplateColumns: "1fr 1fr" }}>
          <div><label htmlFor="vk-key">API key</label><input id="vk-key" type="password" autoComplete="off" style={{ width: "100%" }} maxLength={512} placeholder={vk.has ? "Saved. Paste a new one to replace it" : "Paste your API key"} value={vk.key} onChange={(e) => setVk({ ...vk, key: e.target.value })} /></div>
          <div><label htmlFor="vk-user">User ID</label><input id="vk-user" type="password" autoComplete="off" style={{ width: "100%" }} maxLength={512} placeholder={vk.has ? "Saved. Paste a new one to replace it" : "Paste your user ID"} value={vk.user} onChange={(e) => setVk({ ...vk, user: e.target.value })} /></div>
        </div>
        <div className="row" style={{ marginTop: 10 }}>
          <button className="primary small" disabled={vk.busy || (!vk.key.trim() && !vk.user.trim())} onClick={() => void vkSave()}>Save</button>
          <button className="small" disabled={vk.busy || !vk.has} onClick={() => void vkTest()}>{vk.busy ? "Testing…" : "Test connection"}</button>
          {vk.has && <button className="small danger" onClick={() => void vkRemove()}>Remove</button>}
          {vk.result && <span className={`chip ${vk.bad ? "bad" : "ok"}`} role="status">{vk.result}</span>}
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
        <p className="small">Soundstage {props.info?.version ?? ""}. Projects are stored in <span className="mono">{props.info?.projects_root ?? "…"}</span>. Deleting a project removes its folder. Soundstage contacts the internet only when you use an online service you set up: <span className="mono">api.elevenlabs.io</span>, <span className="mono">voicekiller.com</span>, <span className="mono">api.kie.ai</span>, <span className="mono">queue.fal.run</span>, or an address you add yourself. Each asks before it sends anything.</p>
      </div>
      {removing && <Confirm title="Remove the saved ElevenLabs key?" confirmLabel="Remove key" onCancel={() => setRemoving(false)} body={<p>The key will be deleted from the Windows Credential Manager and ElevenLabs voices will disappear from the library. Projects that use them keep their rendered takes but cannot render new ones until a key is saved again.</p>}
        onConfirm={() => { void api.deleteApiKey("elevenlabs").then(() => { setHas(false); setTest(null); props.onKeysChanged(); props.toast("Key removed."); }).catch((e) => props.toast(messageOf(e), true)); setRemoving(false); }} />}
    </div>
  );
}
