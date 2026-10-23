import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState } from "react";
import { api, messageOf } from "../api";
import { JOB_PRESETS, PRESETS, formatVoices, parseVoices, reachOf, uniqueId, type JobPreset, type Preset } from "../enginePresets";
import * as I from "../icons";
import { Confirm, Help } from "../ui";
import { NON_VERBAL, type EnginesView, type JobEngine, type SpeechEngine } from "../types";

const TAGS = [...NON_VERBAL, "whisper", "shout", "excited", "sad", "tired", "sarcastic", "curious", "serious"];

interface JobDraft { j: JobEngine; isNew: boolean; voices: string }
const CLOUD_KEYS = [
  { name: "fal", label: "fal", help: "Used by every fal engine below." },
  { name: "kie", label: "Kie", help: "Used by Suno and every other Kie engine below." },
];

interface Draft { e: SpeechEngine; isNew: boolean; voices: string; key: string }

function ReachChip({ url }: { url: string }) {
  const r = reachOf(url);
  return r === "the internet"
    ? <span className="chip gold"><I.Cloud size={12} /> Sends text over the internet</span>
    : r === "your network"
      ? <span className="chip warn"><I.Device size={12} /> On your network</span>
      : <span className="chip info"><I.Device size={12} /> On this computer</span>;
}

export function Providers(props: { toast: (t: string, bad?: boolean) => void; onBack: () => void; inProject: boolean; onChanged: () => void }) {
  const [view, setView] = useState<EnginesView | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [results, setResults] = useState<Record<string, { ok: boolean; text: string }>>({});
  const [removing, setRemoving] = useState<SpeechEngine | null>(null);
  const [piper, setPiper] = useState({ exe: "", voices_dir: "" });
  const [jd, setJd] = useState<JobDraft | null>(null);
  const [jobRemoving, setJobRemoving] = useState<JobEngine | null>(null);
  const [cloudKey, setCloudKey] = useState<Record<string, string>>({});

  const refresh = useCallback(async () => {
    try {
      const v = await api.enginesGet();
      setView(v);
      setPiper(v.config.piper ?? { exe: "", voices_dir: "" });
    } catch (e) { props.toast(messageOf(e), true); }
  }, [props]);
  useEffect(() => { void refresh(); }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const save = async (next: EnginesView["config"], msg: string) => {
    await api.enginesSave(next);
    await refresh();
    props.onChanged();
    props.toast(msg);
  };

  const startPreset = (p: Preset) => {
    const taken = view?.config.speech.map((e) => e.id) ?? [];
    const id = uniqueId(p.engine.label, taken);
    setDraft({ e: { ...p.engine, id }, isNew: true, voices: formatVoices(p.engine.voices), key: "" });
    setResults({});
  };
  const edit = (e: SpeechEngine) => { setDraft({ e: { ...e }, isNew: false, voices: formatVoices(e.voices), key: "" }); setResults({}); };

  const commit = async () => {
    if (!draft || !view) return;
    setBusy("save");
    try {
      const e: SpeechEngine = { ...draft.e, voices: parseVoices(draft.voices), tag_map: Object.fromEntries(Object.entries(draft.e.tag_map).filter(([k, v]) => draft.e.tags.includes(k) && v.trim() && v !== `[${k}]`)) };
      const list = draft.isNew ? [...view.config.speech, e] : view.config.speech.map((x) => (x.id === e.id ? e : x));
      await save({ ...view.config, speech: list }, `${e.label} saved.`);
      if (draft.key.trim()) { await api.setApiKey(`engine:${e.id}`, draft.key); await refresh(); props.onChanged(); }
      setDraft(null);
    } catch (e) { props.toast(messageOf(e), true); } finally { setBusy(null); }
  };

  const test = async (id: string) => {
    setBusy(`test-${id}`);
    try { const text = await api.engineTest(id); setResults((r) => ({ ...r, [id]: { ok: true, text } })); }
    catch (e) { const text = messageOf(e); setResults((r) => ({ ...r, [id]: { ok: false, text } })); }
    finally { setBusy(null); }
  };

  const discover = async () => {
    if (!draft || draft.isNew) { props.toast("Save the engine first, then press Find voices.", true); return; }
    setBusy("discover");
    try {
      const found = await api.engineDiscoverVoices(draft.e.id);
      setDraft({ ...draft, voices: formatVoices(found) });
      props.toast(`Found ${found.length} voice${found.length === 1 ? "" : "s"}. Check the language of each, then save.`);
    } catch (e) { props.toast(messageOf(e), true); } finally { setBusy(null); }
  };

  const remove = async (e: SpeechEngine) => {
    if (!view) return;
    try {
      await save({ ...view.config, speech: view.config.speech.filter((x) => x.id !== e.id) }, `${e.label} removed.`);
      await api.engineForgetKey(e.id).catch(() => undefined);
    } catch (err) { props.toast(messageOf(err), true); }
  };

  const choose = async (kind: "exe" | "dir") => {
    const p = kind === "exe"
      ? await open({ multiple: false, title: "Choose piper.exe", filters: [{ name: "Piper", extensions: ["exe"] }] })
      : await open({ directory: true, multiple: false, title: "Choose the folder with your Piper voices" });
    if (typeof p === "string") setPiper((x) => (kind === "exe" ? { ...x, exe: p } : { ...x, voices_dir: p }));
  };
  const savePiper = async () => {
    if (!view) return;
    try {
      await save({ ...view.config, piper: piper.exe.trim() || piper.voices_dir.trim() ? piper : null }, "Piper settings saved.");
      if (piper.exe.trim()) await test("piper");
    } catch (e) { props.toast(messageOf(e), true); }
  };

  const saveCloudKey = async (name: string) => {
    const k = (cloudKey[name] ?? "").trim();
    if (!k) return;
    try { await api.setApiKey(name, k); setCloudKey((x) => ({ ...x, [name]: "" })); await refresh(); props.onChanged(); props.toast("Key saved in the Windows Credential Manager."); }
    catch (e) { props.toast(messageOf(e), true); }
  };
  const forgetCloudKey = async (name: string) => {
    try { await api.deleteApiKey(name); await refresh(); props.onChanged(); props.toast("Key removed."); } catch (e) { props.toast(messageOf(e), true); }
  };

  const startJob = (p: JobPreset) => {
    const taken = [...(view?.config.speech.map((e) => e.id) ?? []), ...(view?.config.jobs.map((e) => e.id) ?? [])];
    setJd({ j: { ...p.job, id: uniqueId(p.job.label, taken) }, isNew: true, voices: formatVoices(p.job.voices) });
  };
  const commitJob = async () => {
    if (!jd || !view) return;
    setBusy("job");
    try {
      const j: JobEngine = { ...jd.j, voices: parseVoices(jd.voices) };
      const list = jd.isNew ? [...view.config.jobs, j] : view.config.jobs.map((x) => (x.id === j.id ? j : x));
      await save({ ...view.config, jobs: list }, `${j.label} saved.`);
      setJd(null);
    } catch (e) { props.toast(messageOf(e), true); } finally { setBusy(null); }
  };
  const removeJob = async (j: JobEngine) => {
    if (!view) return;
    try { await save({ ...view.config, jobs: view.config.jobs.filter((x) => x.id !== j.id) }, `${j.label} removed.`); } catch (e) { props.toast(messageOf(e), true); }
  };
  const testJob = async (j: JobEngine) => {
    setBusy(`jt-${j.id}`);
    try {
      const text = j.kind === "speech" ? await api.engineTest(j.id) : await api.generatorTest(`job-${j.id}`);
      setResults((x) => ({ ...x, [j.id]: { ok: true, text } }));
    } catch (e) { setResults((x) => ({ ...x, [j.id]: { ok: false, text: messageOf(e) } })); } finally { setBusy(null); }
  };
  const setJob = (patch: Partial<JobEngine>) => jd && setJd({ ...jd, j: { ...jd.j, ...patch } });

  const d = draft;
  const set = (patch: Partial<SpeechEngine>) => d && setDraft({ ...d, e: { ...d.e, ...patch } });

  return (
    <div className="page">
      <div className="stage-title"><h1>Voice engines</h1><span className="spacer" /><button onClick={props.onBack}>{props.inProject ? "Back to project" : "Back"}</button></div>
      <div className="callout">
        <strong>Bring your own engine, with no account needed.</strong>
        Connect a speech engine running on your own computer, or add Piper. Soundstage then offers its voices next to the built-in ones. Nothing leaves your computer unless you choose an address on the internet, and you are asked before any text is sent.
      </div>
      {view?.load_error && <div className="callout bad" role="alert"><strong>Your saved engines could not be read.</strong>{view.load_error}</div>}

      <div className="card">
        <h2>Your engines</h2>
        {view && view.config.speech.length === 0 && <p className="muted">None yet. Pick a starting point below.</p>}
        {view?.config.speech.map((e) => (
          <div key={e.id} className="linecard">
            <div className="row">
              <strong>{e.label}</strong><ReachChip url={e.base_url} />
              <span className="chip">{e.voices.length} voice{e.voices.length === 1 ? "" : "s"}</span>
              {e.needs_key && (view.keys[e.id] ? <span className="chip ok"><I.Check size={12} /> Key saved</span> : <span className="chip warn"><I.Alert size={12} /> Key needed</span>)}
              <span className="spacer" />
              <button className="small" disabled={busy === `test-${e.id}`} onClick={() => void test(e.id)}>{busy === `test-${e.id}` ? "Testing…" : "Test connection"}</button>
              <button className="small" onClick={() => edit(e)}>Edit</button>
              <button className="small danger icon" onClick={() => setRemoving(e)} aria-label={`Remove ${e.label}`}><I.Trash size={14} /></button>
            </div>
            <div className="small muted mono" style={{ marginTop: 4 }}>{e.base_url} · model {e.model || "not set"}</div>
            {results[e.id] && <div className={`callout ${results[e.id]!.ok ? "ok" : "bad"} small`} style={{ margin: "8px 0 0" }} role="status"><strong>{results[e.id]!.ok ? "Connected" : "Not working"}</strong>{results[e.id]!.text}</div>}
          </div>
        ))}
      </div>

      {d && (
        <div className="card hero" role="group" aria-label={`${d.isNew ? "Add" : "Edit"} engine`}>
          <h2>{d.isNew ? "Add" : "Edit"} {d.e.label || "engine"}</h2>
          <div className="grid2" style={{ gridTemplateColumns: "1fr 1fr" }}>
            <div><label htmlFor="e-label">Name</label><input id="e-label" type="text" style={{ width: "100%" }} maxLength={60} value={d.e.label} onChange={(e) => set({ label: e.target.value })} /></div>
            <div><label htmlFor="e-url">Address <Help text="Where the server listens. Local servers use http://127.0.0.1 and a port. Servers on the internet must use https://." /></label>
              <input id="e-url" type="text" style={{ width: "100%" }} value={d.e.base_url} onChange={(e) => set({ base_url: e.target.value })} spellCheck={false} /></div>
            <div><label htmlFor="e-model">Model name <Help text="The model the server should use. It is whatever name that server expects." /></label><input id="e-model" type="text" style={{ width: "100%" }} value={d.e.model} onChange={(e) => set({ model: e.target.value })} spellCheck={false} /></div>
            <div><label htmlFor="e-stt">Transcription model (optional) <Help text="If this server can also turn speech into text (a Whisper model, for example), enter its model name to use it in Audio tools." /></label><input id="e-stt" type="text" style={{ width: "100%" }} value={d.e.transcribe_model} onChange={(e) => set({ transcribe_model: e.target.value })} spellCheck={false} placeholder="for example whisper-1" /></div>
            <div><label htmlFor="e-fmt">Audio format</label>
              <select id="e-fmt" value={d.e.response_format} onChange={(e) => set({ response_format: e.target.value as SpeechEngine["response_format"] })}>
                <option value="wav">WAV (best for local servers)</option><option value="mp3">MP3</option><option value="flac">FLAC</option><option value="opus">Opus</option><option value="aac">AAC</option>
              </select></div>
          </div>
          <div style={{ marginTop: 10 }}><ReachChip url={d.e.base_url} /></div>

          <div style={{ marginTop: 14 }}>
            <label htmlFor="e-voices">Voices <Help text="One per line: id | name | language | accent | gender. Only the id is required. The language lets Soundstage show the right flag." /></label>
            <textarea id="e-voices" rows={5} className="mono" value={d.voices} onChange={(e) => setDraft({ ...d, voices: e.target.value })} spellCheck={false} placeholder={"af_bella | Bella | en-US | American | female"} />
            <div className="row" style={{ marginTop: 6 }}><button className="small" disabled={busy === "discover"} onClick={() => void discover()}>{busy === "discover" ? "Looking…" : "Find voices on the server"}</button><span className="small muted">Works with servers that list their voices.</span></div>
          </div>

          <div style={{ marginTop: 14 }}>
            <label>Directions this engine understands <Help text="Tick only what you know this engine can speak. Soundstage cannot test it. A direction you do not tick is shown as not supported, so nothing is silently dropped." /></label>
            <div className="row tight">
              {TAGS.map((t) => (
                <label key={t} className="inline"><input type="checkbox" checked={d.e.tags.includes(t)} onChange={(ev) => set({ tags: ev.target.checked ? [...d.e.tags, t] : d.e.tags.filter((x) => x !== t) })} /> <span className="mono small">[{t}]</span></label>
              ))}
            </div>
            {d.e.tags.length > 0 && (
              <div className="row" style={{ marginTop: 8 }}>
                {d.e.tags.map((t) => (
                  <div key={t}><label htmlFor={`m-${t}`}>[{t}] is written as</label>
                    <input id={`m-${t}`} type="text" style={{ width: 130 }} maxLength={40} placeholder={`[${t}]`} value={d.e.tag_map[t] ?? ""} onChange={(ev) => set({ tag_map: { ...d.e.tag_map, [t]: ev.target.value } })} /></div>
                ))}
              </div>
            )}
          </div>

          <div className="row" style={{ marginTop: 14, alignItems: "end" }}>
            <div style={{ flex: 1, minWidth: 260 }}>
              <label htmlFor="e-key">{d.e.needs_key ? "Key (required)" : "Key (only if the server asks for one)"}</label>
              <input id="e-key" type="password" autoComplete="off" style={{ width: "100%" }} maxLength={512} placeholder={view?.keys[d.e.id] ? "A key is saved. Paste a new one to replace it" : "Paste a key"} value={d.key} onChange={(e) => setDraft({ ...d, key: e.target.value })} />
            </div>
            <label className="inline"><input type="checkbox" checked={d.e.needs_key} onChange={(e) => set({ needs_key: e.target.checked })} /> This server needs a key</label>
          </div>

          <div style={{ marginTop: 14 }}>
            <label htmlFor="e-lic">Licence note (what you know about the model's terms)</label>
            <input id="e-lic" type="text" style={{ width: "100%" }} maxLength={300} value={d.e.licence_note} onChange={(e) => set({ licence_note: e.target.value })} />
            <p className="small muted" style={{ margin: "6px 0 0" }}>Voices from this engine start as “licence unknown”. Read the terms of the model you run, then record your check on the Export screen before a commercial export.</p>
          </div>

          <div className="row" style={{ marginTop: 16 }}>
            <button className="primary" disabled={busy === "save"} onClick={() => void commit()}>{busy === "save" ? "Saving…" : "Save engine"}</button>
            <button onClick={() => setDraft(null)}>Cancel</button>
          </div>
        </div>
      )}

      <div className="card">
        <h2>Add an engine</h2>
        <div className="voice-grid">
          {PRESETS.map((p) => (
            <div key={p.key} className="voice-card">
              <div className="row" style={{ marginBottom: 4 }}><strong>{p.label}</strong><span className="spacer" />
                <span className={`chip ${p.where === "the internet" ? "gold" : "info"}`}>{p.where === "the internet" ? <I.Cloud size={12} /> : <I.Device size={12} />} {p.where === "your choice" ? "Your choice" : p.where === "the internet" ? "Internet" : "This computer"}</span></div>
              <p className="small muted">{p.blurb}</p>
              <p className="small faint">{p.help}</p>
              <button className="small" onClick={() => startPreset(p)}><I.Plus size={14} /> Add</button>
            </div>
          ))}
        </div>
      </div>

      <div className="card">
        <h2>Piper <span className="chip info" style={{ verticalAlign: "middle" }}>Offline, no graphics card needed</span></h2>
        <p className="muted small">Piper is a free, fast voice engine that runs on any computer. Soundstage does not include it. Download Piper and some voices from the Piper project, put the voices in one folder, then choose the program and that folder here. Every Piper voice has its own licence, so check each one.</p>
        <div className="grid2" style={{ gridTemplateColumns: "1fr 1fr" }}>
          <div><label htmlFor="p-exe">piper.exe</label>
            <div className="row tight"><input id="p-exe" type="text" style={{ flex: 1 }} value={piper.exe} onChange={(e) => setPiper({ ...piper, exe: e.target.value })} placeholder="Not chosen" spellCheck={false} /><button className="small" onClick={() => void choose("exe")}>Choose…</button></div></div>
          <div><label htmlFor="p-dir">Voices folder</label>
            <div className="row tight"><input id="p-dir" type="text" style={{ flex: 1 }} value={piper.voices_dir} onChange={(e) => setPiper({ ...piper, voices_dir: e.target.value })} placeholder="Not chosen" spellCheck={false} /><button className="small" onClick={() => void choose("dir")}>Choose…</button></div></div>
        </div>
        <div className="row" style={{ marginTop: 10 }}>
          <button className="primary small" onClick={() => void savePiper()}>Save and test</button>
          {results["piper"] && <span className={`chip ${results["piper"].ok ? "ok" : "bad"}`}>{results["piper"].ok ? "Piper started" : results["piper"].text}</span>}
        </div>
        <p className="small faint" style={{ marginBottom: 0 }}>Piper cannot laugh, sigh or whisper. It can speak at different speeds and Soundstage adds exact pauses.</p>
      </div>

      <div className="card">
        <h2>Music, sound effects and more <span className="chip gold" style={{ verticalAlign: "middle" }}>Optional</span></h2>
        <p className="muted small">ElevenLabs music and effects appear in the Mix step once you save an ElevenLabs key in Settings. Add Suno through Kie, models on fal, or your own ComfyUI workflow here. Each service is paid on its own account, and your description is sent over the internet. ComfyUI runs on your own computer and sends nothing.</p>
        <div className="grid2" style={{ gridTemplateColumns: "1fr 1fr" }}>
          {CLOUD_KEYS.map((c) => (
            <div key={c.name}>
              <label htmlFor={`ck-${c.name}`}>{c.label} key <Help text={c.help} /></label>
              <div className="row tight">
                <input id={`ck-${c.name}`} type="password" autoComplete="off" style={{ flex: 1 }} maxLength={512} placeholder={view?.keys[`provider:${c.name}`] ? "A key is saved. Paste a new one to replace it" : "Paste a key"} value={cloudKey[c.name] ?? ""} onChange={(e) => setCloudKey({ ...cloudKey, [c.name]: e.target.value })} />
                <button className="small" disabled={!(cloudKey[c.name] ?? "").trim()} onClick={() => void saveCloudKey(c.name)}>Save</button>
                {view?.keys[`provider:${c.name}`] && <button className="small danger" onClick={() => void forgetCloudKey(c.name)}>Remove</button>}
              </div>
            </div>
          ))}
        </div>
        <h3 style={{ marginTop: 16 }}>Your music, effects and ComfyUI engines</h3>
        {view && view.config.jobs.length === 0 && <p className="muted small">None yet. Pick a starting point below.</p>}
        {view?.config.jobs.map((j) => (
          <div key={j.id} className="linecard">
            <div className="row">
              <strong>{j.label}</strong>
              <span className="chip">{j.kind === "music" ? "Music" : j.kind === "sfx" ? "Effects" : "Speech"}</span>
              {j.provider === "comfy" ? <span className="chip info"><I.Device size={12} /> On this computer</span> : <span className="chip gold"><I.Cloud size={12} /> Sends text over the internet</span>}
              {j.provider !== "comfy" && (view.keys[`provider:${j.provider}`] ? <span className="chip ok"><I.Check size={12} /> Key saved</span> : <span className="chip warn"><I.Alert size={12} /> Key needed</span>)}
              <span className="spacer" />
              <button className="small" disabled={busy === `jt-${j.id}`} onClick={() => void testJob(j)}>{busy === `jt-${j.id}` ? "Testing…" : "Test"}</button>
              <button className="small" onClick={() => setJd({ j: { ...j }, isNew: false, voices: formatVoices(j.voices) })}>Edit</button>
              <button className="small danger icon" onClick={() => setJobRemoving(j)} aria-label={`Remove ${j.label}`}><I.Trash size={14} /></button>
            </div>
            {results[j.id] && <div className={`callout ${results[j.id]!.ok ? "ok" : "bad"} small`} style={{ margin: "8px 0 0" }} role="status"><strong>{results[j.id]!.ok ? "Ready" : "Not working"}</strong>{results[j.id]!.text}</div>}
          </div>
        ))}

        {jd && (
          <div className="linecard" role="group" aria-label={`${jd.isNew ? "Add" : "Edit"} generator`} style={{ marginTop: 10 }}>
            <div className="grid2" style={{ gridTemplateColumns: "1fr 1fr" }}>
              <div><label htmlFor="j-label">Name</label><input id="j-label" type="text" style={{ width: "100%" }} maxLength={60} value={jd.j.label} onChange={(e) => setJob({ label: e.target.value })} /></div>
              {jd.j.provider === "comfy"
                ? <div><label htmlFor="j-url">ComfyUI address</label><input id="j-url" type="text" style={{ width: "100%" }} value={jd.j.base_url} spellCheck={false} onChange={(e) => setJob({ base_url: e.target.value })} /></div>
                : <div><label htmlFor="j-model">Model <Help text="The fal model id (for example owner/model) or the Kie model path." /></label><input id="j-model" type="text" style={{ width: "100%" }} value={jd.j.model} spellCheck={false} onChange={(e) => setJob({ model: e.target.value })} /></div>}
            </div>
            <div style={{ marginTop: 10 }}>
              <label htmlFor="j-tpl">{jd.j.provider === "comfy" ? "Workflow (API format)" : "Request template"} <Help text="JSON. Write {{prompt}} where the description goes. Optional: {{seconds}}, {{instrumental}}, {{seed}}; speech uses {{text}} and {{voice}}." /></label>
              <textarea id="j-tpl" rows={jd.j.provider === "comfy" ? 8 : 4} className="mono" style={{ width: "100%" }} spellCheck={false} value={jd.j.template} onChange={(e) => setJob({ template: e.target.value })} />
            </div>
            {jd.j.kind === "speech" && (
              <div style={{ marginTop: 10 }}><label htmlFor="j-voices">Voices <Help text="One per line: id | name | language | accent | gender." /></label><textarea id="j-voices" rows={3} className="mono" style={{ width: "100%" }} spellCheck={false} value={jd.voices} onChange={(e) => setJd({ ...jd, voices: e.target.value })} /></div>
            )}
            {jd.j.kind !== "speech" && (
              <div className="row" style={{ marginTop: 10 }}>
                <div><label htmlFor="j-min">Shortest (s)</label><input id="j-min" type="number" min={1} max={600} style={{ width: 90 }} value={jd.j.min_s} onChange={(e) => setJob({ min_s: Number(e.target.value) || 1 })} /></div>
                <div><label htmlFor="j-max">Longest (s)</label><input id="j-max" type="number" min={1} max={600} style={{ width: 90 }} value={jd.j.max_s} onChange={(e) => setJob({ max_s: Number(e.target.value) || 1 })} /></div>
                <div><label htmlFor="j-def">Usual (s)</label><input id="j-def" type="number" min={1} max={600} style={{ width: 90 }} value={jd.j.default_s} onChange={(e) => setJob({ default_s: Number(e.target.value) || 1 })} /></div>
              </div>
            )}
            <div style={{ marginTop: 10 }}>
              <label htmlFor="j-lic">Licence note (what you know about the terms)</label>
              <input id="j-lic" type="text" style={{ width: "100%" }} maxLength={300} value={jd.j.licence_note} onChange={(e) => setJob({ licence_note: e.target.value })} />
              <p className="small muted" style={{ margin: "6px 0 0" }}>Everything this engine makes starts as “licence unknown”. Read its terms, then record your check before a commercial export.</p>
            </div>
            <div className="row" style={{ marginTop: 12 }}>
              <button className="primary" disabled={busy === "job"} onClick={() => void commitJob()}>{busy === "job" ? "Saving…" : "Save"}</button>
              <button onClick={() => setJd(null)}>Cancel</button>
            </div>
          </div>
        )}

        <div className="voice-grid" style={{ marginTop: 12 }}>
          {JOB_PRESETS.map((p) => (
            <div key={p.key} className="voice-card">
              <div className="row" style={{ marginBottom: 4 }}><strong>{p.label}</strong><span className="spacer" />
                <span className={`chip ${p.job.provider === "comfy" ? "info" : "gold"}`}>{p.job.provider === "comfy" ? <I.Device size={12} /> : <I.Cloud size={12} />} {p.job.provider === "comfy" ? "This computer" : "Internet"}</span></div>
              <p className="small muted">{p.blurb}</p>
              <p className="small faint">{p.help}</p>
              <button className="small" onClick={() => startJob(p)}><I.Plus size={14} /> Add</button>
            </div>
          ))}
        </div>
      </div>

      <div className="card flat">
        <h2>About other tools</h2>
        <ul className="small muted" style={{ margin: 0 }}>
          <li><strong>Ollama</strong> runs language models, not speech, so it cannot be a voice engine. It may help with optional script suggestions in a later version.</li>
          <li><strong>ComfyUI</strong> is supported through your own saved workflow. Soundstage does not install it or any model.</li>
          <li>Which model you run decides accent, quality and what sounds it can make. Soundstage does not claim any of that for engines it did not test.</li>
        </ul>
      </div>

      {jobRemoving && (
        <Confirm title={`Remove “${jobRemoving.label}”?`} confirmLabel="Remove engine" onCancel={() => setJobRemoving(null)}
          body={<p>It disappears from the Mix and voice lists. Audio already made with it stays in your projects. Your saved fal or Kie key is kept, because other engines may use it.</p>}
          onConfirm={() => { const j = jobRemoving; setJobRemoving(null); void removeJob(j); }} />
      )}
      {removing && (
        <Confirm title={`Remove “${removing.label}”?`} confirmLabel="Remove engine" onCancel={() => setRemoving(null)}
          body={<p>Its voices disappear from the library, and its saved key is deleted from the Windows Credential Manager. Takes already rendered with it are kept, but lines cast to its voices cannot render again until you cast another voice.</p>}
          onConfirm={() => { const e = removing; setRemoving(null); void remove(e); }} />
      )}
    </div>
  );
}
