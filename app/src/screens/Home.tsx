import { useEffect, useState } from "react";
import { api, messageOf } from "../api";
import { Confirm, Empty } from "../ui";
import type { Summary } from "../types";

const MAX_IMPORT = 2_000_000;

export function Home(props: { onOpen: (id: string) => void; toast: (t: string, bad?: boolean) => void }) {
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

  return (
    <div className="page">
      <h1>Projects</h1>
      <div className="card">
        <h2>New project</h2>
        <form className="row" style={{ alignItems: "end" }} onSubmit={(e) => { e.preventDefault(); void create(); }}>
          <div><label htmlFor="pname">Name</label><input id="pname" type="text" value={name} maxLength={80} onChange={(e) => setName(e.target.value)} placeholder="Untitled project" style={{ width: 280 }} autoFocus /></div>
          <label className="inline"><input type="checkbox" checked={sample} onChange={(e) => setSample(e.target.checked)} /> Start with a sample script</label>
          <button className="primary" type="submit">Create project</button>
          <label className="inline"><input type="file" accept=".txt,.fountain,text/plain" style={{ display: "none" }} onChange={(e) => void importScript(e.target.files?.[0])} /><span className="chipbtn">Import a script…</span></label>
        </form>
      </div>
      {list === null && <p className="muted">Loading…</p>}
      {list && list.length === 0 && <Empty title="No projects yet"><p className="muted">Create one above. The sample script lets you hear the whole flow in a few minutes.</p></Empty>}
      <div className="grid-cards" style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))", gap: 12 }}>
        {list?.map((s) => (
          <div key={s.id} className="card" style={{ margin: 0 }}>
            <h2 style={{ marginBottom: 2 }}>{s.name}</h2>
            <p className="small muted">{s.lines} line{s.lines === 1 ? "" : "s"} · edited {s.modified.slice(0, 10)}</p>
            <p className="small"><span className={`chip ${s.licence_target === "commercial" ? ((attention[s.id] ?? 0) > 0 ? "warn" : "ok") : ""}`}>
              {s.licence_target === "personal" ? "Personal" : attention[s.id] === undefined ? "Commercial target" : attention[s.id] === 0 ? "Commercial target, cleared" : `Commercial target, ${attention[s.id]} item${attention[s.id] === 1 ? "" : "s"} need attention`}
            </span></p>
            <div className="row"><button className="primary small" onClick={() => props.onOpen(s.id)}>Open</button><span className="spacer" /><button className="small danger" onClick={() => setDeleting(s)}>Delete…</button></div>
          </div>
        ))}
      </div>
      {deleting && <Confirm title={`Delete “${deleting.name}”?`} confirmLabel="Delete project" onCancel={() => setDeleting(null)}
        body={<p>This permanently deletes the project folder, including its script, rendered takes and imported audio. Files you exported are not touched.</p>}
        onConfirm={() => { const d = deleting; setDeleting(null); void api.deleteProject(d.id).then(refresh).catch((e) => props.toast(messageOf(e), true)); }} />}
    </div>
  );
}
