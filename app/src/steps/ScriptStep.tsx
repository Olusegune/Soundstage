import { useRef } from "react";
import type { Ctx } from "../ctx";
import { insertAt, splitTags } from "../logic";
import { Empty } from "../ui";
import { NON_VERBAL } from "../types";

const QUICK = ["pause:short", "pause:long", ...NON_VERBAL, "whisper", "shout"];
const MAX_IMPORT_BYTES = 2_000_000;

export function ScriptStep({ ctx }: { ctx: Ctx }) {
  const { project, analysis, edit } = ctx;
  const ta = useRef<HTMLTextAreaElement>(null);

  const setScript = (script: string) => edit((p) => ({ ...p, script }), "script");

  const insertTag = (tag: string) => {
    const el = ta.current;
    const pos = el ? el.selectionStart : project.script.length;
    const r = insertAt(project.script, pos, `[${tag}] `);
    setScript(r.text);
    requestAnimationFrame(() => { el?.focus(); el?.setSelectionRange(r.caret, r.caret); });
  };

  const importFile = async (f: File | undefined) => {
    if (!f) return;
    if (f.size > MAX_IMPORT_BYTES) { ctx.toast("That file is over 2 MB, which is too large for a script.", true); return; }
    const text = await f.text();
    if (project.script.trim() && !window.confirm("Replace the current script with the contents of this file?")) return;
    setScript(text.replace(/\r\n/g, "\n"));
  };

  /** Ctrl+Enter previews the line under the caret (from step 2 onward, when its speaker has a voice). */
  const onKey = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (!(e.ctrlKey && e.key === "Enter")) return;
    e.preventDefault();
    const row = project.script.slice(0, e.currentTarget.selectionStart).split("\n").length;
    const info = analysis?.lines.find((l) => l.line.row === row);
    if (!info) { ctx.toast("Put the cursor on a line that starts with NAME: to preview it."); return; }
    if (info.voice_id === null) { ctx.toast(`${info.line.speaker} has no voice yet. Pick one in the Cast step.`); return; }
    void ctx.render([info.line.key]);
  };

  if (!project.script.trim()) {
    return (
      <div className="page">
        <Empty title="Start with a script">
          <p className="muted">Write or paste dialogue. Start each line with a speaker name and a colon, like <span className="mono">AMA: Good morning.</span></p>
          <div className="row">
            <button className="primary" onClick={() => setScript(SAMPLE)}>Use a sample script</button>
            <label className="inline"><input type="file" accept=".txt,.fountain,text/plain" onChange={(e) => void importFile(e.target.files?.[0])} style={{ display: "none" }} /><span className="chipbtn">Import a text file…</span></label>
          </div>
          <div style={{ marginTop: 16 }}>
            <textarea className="script-area" ref={ta} aria-label="Script" value={project.script} onChange={(e) => setScript(e.target.value)} placeholder={"# Kitchen, morning\nAMA: Good morning! [pause:short] You're up early.\nKOLA: [sigh] I couldn't sleep."} style={{ minHeight: 160 }} />
          </div>
        </Empty>
      </div>
    );
  }

  const a = analysis;
  const lineCount = (s: string) => a?.lines.filter((l) => l.line.speaker === s).length ?? 0;
  const tagIssues = a?.lines.flatMap((l) => l.line.tag_issues.map((t) => ({ row: l.line.row, ...t }))) ?? [];

  return (
    <div className="page grid2">
      <div>
        <div className="card">
          <div className="row" style={{ marginBottom: 8 }}>
            <h3 style={{ margin: 0 }}>Your script</h3>
            <span className="spacer" />
            <label className="inline"><input type="file" accept=".txt,.fountain,text/plain" onChange={(e) => void importFile(e.target.files?.[0])} style={{ display: "none" }} /><span className="small muted" style={{ cursor: "pointer", textDecoration: "underline" }}>Import a text file…</span></label>
          </div>
          <div className="row" style={{ marginBottom: 8 }} role="group" aria-label="Insert a tag at the cursor">
            {QUICK.map((t) => <button key={t} className="small mono" onClick={() => insertTag(t)} title={`Insert [${t}]`}>[{t}]</button>)}
          </div>
          <textarea ref={ta} className="script-area" aria-label="Script" spellCheck={false} value={project.script} onChange={(e) => setScript(e.target.value)} onKeyDown={onKey} />
          <p className="small muted" style={{ marginTop: 6 }}>
            <span className="mono">#</span> or <span className="mono">INT./EXT.</span> starts a scene. <span className="mono">NAME:</span> starts a line. Ctrl+Enter previews the line under the cursor.
          </p>
        </div>
      </div>
      <div>
        <div className="card">
          <h2>Speakers</h2>
          {a && a.parsed.speakers.length === 0 && <p className="muted">No speakers yet. Start a line with a name and a colon.</p>}
          {a?.parsed.speakers.map((s) => (
            <div key={s} className="row" style={{ justifyContent: "space-between", marginBottom: 4 }}>
              <span className="speaker">{s}</span>
              <span className="muted small">{lineCount(s)} line{lineCount(s) === 1 ? "" : "s"}</span>
            </div>
          ))}
          {a && a.parsed.scenes.length > 0 && (
            <>
              <h3 style={{ marginTop: 12 }}>Scenes</h3>
              {a.parsed.scenes.map((s) => (
                <div key={s.index} className="small" style={{ marginBottom: 2 }}>
                  {s.index + 1}. {s.title}{s.location ? ` · ${s.location}` : ""}{s.time_of_day ? ` · ${s.time_of_day}` : ""}
                </div>
              ))}
            </>
          )}
        </div>
        <div className="card" aria-live="polite">
          <h2>Checks</h2>
          {a && a.parsed.issues.length === 0 && tagIssues.length === 0 && <p className="chip ok">✓ Nothing to fix</p>}
          {a?.parsed.issues.map((i) => <div key={i.row} className="callout warn small"><strong>Line {i.row}</strong>{i.message}</div>)}
          {tagIssues.map((t, i) => <div key={i} className="callout bad small"><strong>Line {t.row}: <span className="tag-bad mono">{t.tag}</span></strong>{t.message} Use a tag from the buttons above, or remove it.</div>)}
        </div>
        {a && a.lines.length > 0 && (
          <div className="card">
            <h3>Preview of tags</h3>
            {a.lines.slice(0, 6).map((l) => (
              <div key={l.line.key} className="small" style={{ marginBottom: 4 }}>
                <span className="speaker">{l.line.speaker}:</span>{" "}
                {splitTags(l.line.text).map((p, i) => <span key={i} className={p.tag ? (p.bad ? "tag-bad" : "tag") : ""}>{p.text}</span>)}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export const SAMPLE = "# Kitchen, morning\nAMA: Good morning! [pause:short] You're up early.\nKOLA: [sigh] I couldn't sleep. [pause:long] The rain was loud.\nAMA: [whisper] Then let's not wake the others.\n\n# Market, later\nKOLA: [excited] Look at these mangoes!\nAMA: Careful. [pause:350] Last time you bought the whole stall.\n";
