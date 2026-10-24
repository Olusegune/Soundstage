import { useMemo, useRef, useState } from "react";
import type { Ctx } from "../ctx";
import { insertAt, splitTags } from "../logic";
import { Empty } from "../ui";
import { NON_VERBAL } from "../types";

const QUICK = ["pause:short", "pause:long", ...NON_VERBAL, "whisper", "shout"];
const MAX_IMPORT_BYTES = 2_000_000;

const TAG_GUIDE: [string, string][] = [
  ["[pause:short]  [pause:long]", "A quarter second, or most of a second, of silence."],
  ["[pause:350]", "An exact pause in milliseconds (0 to 10000)."],
  ["[laugh] [chuckle] [sigh] [cough] [sneeze] [gasp] [cry] [breath] [sniff]", "Sounds. Only voices that can make them accept them; others say so instead of ignoring them."],
  ["[whisper] [shout] [excited] [sad] [tired] [sarcastic] [curious] [serious]", "A way of speaking, from that point on in the line."],
];

function escapeRe(s: string) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** Extra tools for people who want them: counts, rename a speaker, find and replace, and a tag guide. */
function AdvancedScriptTools({ ctx }: { ctx: Ctx }) {
  const { project, analysis: a, edit } = ctx;
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [find, setFind] = useState("");
  const [repl, setRepl] = useState("");
  const [matchCase, setMatchCase] = useState(false);

  const stats = useMemo(() => {
    const lines = a?.lines ?? [];
    const words = lines.reduce((n, l) => n + splitTags(l.line.text).filter((p) => !p.tag).map((p) => p.text).join(" ").split(/\s+/).filter(Boolean).length, 0);
    const bySpeaker = new Map<string, number>();
    for (const l of lines) bySpeaker.set(l.line.speaker, (bySpeaker.get(l.line.speaker) ?? 0) + splitTags(l.line.text).filter((p) => !p.tag).map((p) => p.text).join(" ").split(/\s+/).filter(Boolean).length);
    return { words, seconds: Math.round(words / 2.5), bySpeaker: [...bySpeaker.entries()] };
  }, [a]);

  const count = useMemo(() => {
    if (!find) return 0;
    return (project.script.match(new RegExp(escapeRe(find), matchCase ? "g" : "gi")) ?? []).length;
  }, [project.script, find, matchCase]);

  const speakers = a?.parsed.speakers ?? [];
  const newName = to.trim().toUpperCase();
  const validNew = /^[A-Z][A-Z0-9 _-]{0,23}$/.test(newName);
  const rename = () => {
    if (!from || !validNew || newName === from) return;
    const re = new RegExp(`^(\\s*)${escapeRe(from)}(\\s*:)`, "gm");
    edit((p) => {
      const characters = { ...p.characters };
      if (characters[from] && !characters[newName]) characters[newName] = characters[from];
      delete characters[from];
      return { ...p, script: p.script.replace(re, `$1${newName}$2`), characters };
    });
    ctx.toast(`${from} is now ${newName}.`);
    setFrom(""); setTo("");
  };
  const replaceAll = () => {
    if (!find || count === 0) return;
    const re = new RegExp(escapeRe(find), matchCase ? "g" : "gi");
    edit((p) => ({ ...p, script: p.script.replace(re, () => repl) }), "script");
    ctx.toast(`Replaced ${count} ${count === 1 ? "place" : "places"}.`);
  };

  return (
    <div className="card" role="group" aria-label="Script tools">
      <h2>Script tools <span className="chip info" style={{ verticalAlign: "middle" }}>Advanced</span></h2>
      <div className="small" style={{ marginBottom: 8 }}>{stats.words} spoken words, about {Math.floor(stats.seconds / 60)} min {stats.seconds % 60} s read aloud (an estimate; pauses add time).</div>
      {stats.bySpeaker.length > 0 && <div className="small muted" style={{ marginBottom: 10 }}>{stats.bySpeaker.map(([s, w]) => `${s}: ${w} words`).join(" · ")}</div>}

      <h3 style={{ margin: "10px 0 4px" }}>Rename a speaker</h3>
      <div className="row" style={{ alignItems: "end" }}>
        <div><label htmlFor="sr-from">Speaker</label><select id="sr-from" value={from} onChange={(e) => setFrom(e.target.value)}><option value="">Choose…</option>{speakers.map((s) => <option key={s}>{s}</option>)}</select></div>
        <div><label htmlFor="sr-to">New name</label><input id="sr-to" type="text" style={{ width: 160 }} maxLength={24} value={to} onChange={(e) => setTo(e.target.value)} placeholder="FOR EXAMPLE NARRATOR" /></div>
        <button className="small" disabled={!from || !validNew || newName === from} onClick={rename}>Rename</button>
      </div>
      <p className="small faint" style={{ margin: "4px 0 0" }}>Changes the name at the start of every line, and keeps the voice and directions you set. If the new name already exists, the two speakers become one.</p>

      <h3 style={{ margin: "14px 0 4px" }}>Find and replace</h3>
      <div className="row" style={{ alignItems: "end" }}>
        <div><label htmlFor="fr-f">Find</label><input id="fr-f" type="text" style={{ width: 150 }} maxLength={100} value={find} onChange={(e) => setFind(e.target.value)} /></div>
        <div><label htmlFor="fr-r">Replace with</label><input id="fr-r" type="text" style={{ width: 150 }} maxLength={200} value={repl} onChange={(e) => setRepl(e.target.value)} /></div>
        <label className="inline"><input type="checkbox" checked={matchCase} onChange={(e) => setMatchCase(e.target.checked)} /> Match capitals</label>
        <button className="small" disabled={!find || count === 0} onClick={replaceAll}>Replace all{find ? ` (${count})` : ""}</button>
      </div>

      <h3 style={{ margin: "14px 0 4px" }}>Say a name differently</h3>
      <p className="small muted" style={{ margin: "0 0 6px" }}>A voice mispronouncing a name? Teach it for this project without changing your script.</p>
      <button className="small" onClick={ctx.openPronunciation}>Open Pronunciation</button>

      <details style={{ marginTop: 14 }}>
        <summary className="small">Every direction you can write</summary>
        <table className="small" style={{ marginTop: 6 }}><tbody>{TAG_GUIDE.map(([t, d]) => <tr key={t}><td className="mono" style={{ paddingRight: 10, verticalAlign: "top" }}>{t}</td><td className="muted">{d}</td></tr>)}</tbody></table>
      </details>
    </div>
  );
}

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
        {ctx.mode === "advanced" && <AdvancedScriptTools ctx={ctx} />}
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
