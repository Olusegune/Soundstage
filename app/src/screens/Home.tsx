import { useEffect, useState } from "react";
import { api, messageOf } from "../api";
import * as I from "../icons";
import { Confirm, Empty } from "../ui";
import type { Summary } from "../types";

const MAX_IMPORT = 2_000_000;

function ago(iso: string): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return iso.slice(0, 10);
  const days = Math.floor((Date.now() - t) / 86_400_000);
  return days <= 0 ? "Today" : days === 1 ? "Yesterday" : days < 14 ? `${days} days ago` : iso.slice(0, 10);
}

export function Home(props: { onOpen: (id: string) => void; toast: (t: string, bad?: boolean) => void; onQuickStart: () => void; onHelp: () => void }) {
  const [list, setList] = useState<Summary[] | null>(null);
  const [attention, setAttention] = useState<Record<string, number>>({});
  const [name, setName] = useState("");
  const [sample, setSample] = useState(true);
  const [deleting, setDeleting] = useState<Summary | null>(null);

  const refresh = async () => {
    try {
      const l = await api.listProjects();
      setList(l);
      // Show licence status for commercial projects up front (UX_GOLDEN_PATH 2.1).
      const next: Record<string, number> = {};
      for (const s of l.filter((x) => x.licence_target === "commercial")) {
        try { next[s.id] = (await api.checkExport(await api.loadProject(s.id))).blocking.length; } catch { /* leave blank */ }
      }
      setAttention(next);
    } catch (e) { props.toast(messageOf(e), true); setList([]); }
  };
  useEffect(() => { void refresh(); }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const create = async (script?: string, projectName?: string) => {
    try {
      const p = await api.createProject(projectName ?? (name.trim() || "Untitled project"), script === undefined && sample);
      if (script !== undefined) await api.saveProject({ ...p, script });
      props.onOpen(p.id);
    } catch (e) { props.toast(messageOf(e), true); }
  };

  const importScript = async (f: File | undefined) => {
    if (!f) return;
    if (f.size > MAX_IMPORT) { props.toast("That file is over 2 MB, which is too large for a script.", true); return; }
    await create((await f.text()).replace(/\r\n/g, "\n"), f.name.replace(/\.[^.]+$/, ""));
  };

  const first = list !== null && list.length === 0;

  return (
    <div className="page">
      <section className="home-hero" style={{ backgroundImage: "url(/brand/splash.webp)" }} aria-label="Welcome">
        <div className="home-hero-inner">
          <div>
            <h1 className="display" style={{ letterSpacing: "0.14em" }}>{first ? "Welcome" : "Your projects"}</h1>
            <p>{first ? "Voice, sound, music and mix for your film, with a guide beside you from the first line." : "Pick up where you left off, or start something new."}</p>
          </div>
          <div className="row">
            <button className="primary" onClick={props.onQuickStart} style={{ minHeight: 44, padding: "0 22px" }}><I.Sparkle size={18} /> Quick start: make a first scene</button>
            <button onClick={props.onHelp}><I.Book size={16} /> How it works</button>
          </div>
        </div>
      </section>

      {first && (
        <div className="card">
          <h2>What happens in a quick start</h2>
          <div className="qs-steps">
            <div><b>1. A sample script</b>Two people in a kitchen, with sighs, whispers and pauses.</div>
            <div><b>2. Free Windows voices</b>Cast, direct and render on your own computer. Nothing is sent anywhere.</div>
            <div><b>3. Real files at the end</b>A mixed master, three stems and a record of what was used.</div>
          </div>
          <p className="small muted" style={{ margin: 0 }}>It takes about ten minutes, and the guide panel tells you what to do at every step.</p>
        </div>
      )}

      <div className="card">
        <h2>New project</h2>
        <form className="row" style={{ alignItems: "end" }} onSubmit={(e) => { e.preventDefault(); void create(); }}>
          <div><label htmlFor="pname">Name</label><input id="pname" type="text" value={name} maxLength={80} onChange={(e) => setName(e.target.value)} placeholder="Untitled project" style={{ width: 280 }} /></div>
          <label className="inline"><input type="checkbox" checked={sample} onChange={(e) => setSample(e.target.checked)} /> Start with a sample script</label>
          <button className="primary" type="submit"><I.Plus size={16} /> Create project</button>
          <label className="inline"><input type="file" accept=".txt,.fountain,text/plain" style={{ display: "none" }} onChange={(e) => void importScript(e.target.files?.[0])} /><span className="chipbtn"><I.Upload size={16} /> Import a script</span></label>
        </form>
      </div>

      {list === null && <div className="proj-grid">{[0, 1, 2].map((i) => <div key={i} className="skeleton" style={{ height: 110 }} />)}</div>}
      {list && list.length === 0 && <Empty title="No projects yet"><p className="muted">Create one above, or press Quick start.</p></Empty>}
      <div className="proj-grid">
        {list?.map((s) => (
          <div key={s.id} className="proj-card">
            <img src="/brand/file-256.png" alt="" aria-hidden="true" />
            <div style={{ flex: 1, minWidth: 0 }}>
              <h2 style={{ marginBottom: 2, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{s.name}</h2>
              <p className="small muted">{s.lines} line{s.lines === 1 ? "" : "s"} · {ago(s.modified)}</p>
              <p className="small">
                <span className={`chip ${s.licence_target === "commercial" ? ((attention[s.id] ?? 0) > 0 ? "warn" : "ok") : ""}`}>
                  {s.licence_target === "personal" ? "Personal" : attention[s.id] === undefined ? "Commercial target" : attention[s.id] === 0 ? "Commercial, cleared" : `Commercial, ${attention[s.id]} item${attention[s.id] === 1 ? "" : "s"} need attention`}
                </span>
              </p>
              <div className="row tight"><button className="primary small" onClick={() => props.onOpen(s.id)}>Open</button><span className="spacer" /><button className="small danger" onClick={() => setDeleting(s)} aria-label={`Delete ${s.name}`}><I.Trash size={14} /></button></div>
            </div>
          </div>
        ))}
      </div>
      {deleting && <Confirm title={`Delete “${deleting.name}”?`} confirmLabel="Delete project" onCancel={() => setDeleting(null)}
        body={<p>This permanently deletes the project folder, including its script, rendered takes and imported audio. Files you exported are not touched.</p>}
        onConfirm={() => { const d = deleting; setDeleting(null); void api.deleteProject(d.id).then(refresh).catch((e) => props.toast(messageOf(e), true)); }} />}
    </div>
  );
}
