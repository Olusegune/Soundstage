import { useEffect, useRef, useState } from "react";
import * as I from "../icons";
import { Kbd } from "../ui";
import type { Mode } from "../prefs";
import type { Project } from "../types";

export function Toolbar(props: {
  hereLabel: string; project: Project | null; saveState: "saved" | "saving" | "error";
  canUndo: boolean; canRedo: boolean; onUndo: () => void; onRedo: () => void;
  mode: Mode; onMode: (m: Mode) => void; onModeHelp: () => void; onPalette: () => void; paletteKey: string;
  guideOpen: boolean; showGuide: boolean; onToggleGuide: () => void; onToggleRail: () => void; railCollapsed: boolean;
  onEdit: (fn: (p: Project) => Project, coalesce?: string) => void; guideKey: string;
}) {
  const [details, setDetails] = useState(false);
  const pop = useRef<HTMLDivElement>(null);
  const p = props.project;

  useEffect(() => {
    if (!details) return;
    const click = (e: MouseEvent) => { if (pop.current && !pop.current.contains(e.target as Node) && !(e.target as HTMLElement).closest(".proj")) setDetails(false); };
    const key = (e: KeyboardEvent) => { if (e.key === "Escape") setDetails(false); };
    document.addEventListener("mousedown", click);
    document.addEventListener("keydown", key);
    return () => { document.removeEventListener("mousedown", click); document.removeEventListener("keydown", key); };
  }, [details]);

  return (
    <header className="toolbar" role="banner">
      <button className="ghost icon" onClick={props.onToggleRail} aria-label={props.railCollapsed ? "Show sidebar" : "Hide sidebar"} aria-expanded={!props.railCollapsed}><I.PanelLeft size={18} /></button>
      <div className="crumbs" role="group" aria-label="You are here">
        {p ? (
          <>
            <button className="proj ghost" onClick={() => setDetails((d) => !d)} aria-expanded={details} aria-haspopup="dialog" data-tip="Project details: name, licence target, rating">{p.name}</button>
            <span className="sep" aria-hidden="true">/</span>
            <span className="here">{props.hereLabel}</span>
          </>
        ) : <span className="here">{props.hereLabel}</span>}
      </div>
      {details && p && (
        <div className="popover" ref={pop} role="dialog" aria-label="Project details" style={{ left: 330 }}>
          <h3>Project details</h3>
          <div className="col">
            <div><label htmlFor="pd-name">Name</label><input id="pd-name" type="text" style={{ width: "100%" }} value={p.name} maxLength={80} onChange={(e) => props.onEdit((x) => ({ ...x, name: e.target.value }), "name")} /></div>
            <div>
              <label>Where will this be used?</label>
              <div className="seg" role="radiogroup" aria-label="Licence target">
                {(["personal", "commercial"] as const).map((v) => (
                  <button key={v} role="radio" aria-checked={p.licence_target === v} onClick={() => props.onEdit((x) => ({ ...x, licence_target: v }))}>{v === "personal" ? "Personal" : "Commercial"}</button>
                ))}
              </div>
              <p className="small muted" style={{ marginTop: 6 }}>Commercial projects may only use voices, music and effects cleared for commercial use. Export checks this.</p>
            </div>
            <div>
              <label htmlFor="pd-rating">Content rating (a label you set, not a certification)</label>
              <select id="pd-rating" value={p.rating} onChange={(e) => props.onEdit((x) => ({ ...x, rating: e.target.value as Project["rating"] }))}>
                <option value="unrated">Unrated</option><option value="all_ages">All ages</option><option value="teen">Teen</option><option value="adult">Adult</option>
              </select>
            </div>
          </div>
        </div>
      )}
      <span className="spacer" />
      {p && (
        <span className={`save-state ${props.saveState === "error" ? "err" : ""}`} aria-live="polite">
          {props.saveState === "saving" ? <><span className="spin" aria-hidden="true" /> Saving</> : props.saveState === "error" ? <><I.Alert size={14} /> Not saved</> : <><I.Check size={14} /> Saved</>}
        </span>
      )}
      {p && (
        <>
          <button className="ghost icon" onClick={props.onUndo} disabled={!props.canUndo} aria-label="Undo" data-tip="Undo (Ctrl+Z)" data-tip-pos="bottom"><I.Undo size={17} /></button>
          <button className="ghost icon" onClick={props.onRedo} disabled={!props.canRedo} aria-label="Redo" data-tip="Redo (Ctrl+Shift+Z)" data-tip-pos="bottom"><I.Redo size={17} /></button>
        </>
      )}
      <div className="seg" role="radiogroup" aria-label="Interface mode" data-tour="mode">
        {(["simple", "advanced"] as const).map((m) => (
          <button key={m} role="radio" aria-checked={props.mode === m} onClick={() => props.onMode(m)}
            data-tip={m === "simple" ? "The essentials. Everything you hide here keeps its value." : "Every control: per-line pace and pauses, ducking, fades, loops."} data-tip-pos="bottom">
            {m === "simple" ? "Simple" : "Advanced"}
          </button>
        ))}
      </div>
      <button className="ghost icon" onClick={props.onModeHelp} aria-label="What does Advanced mode add?" data-tip="What does Advanced mode add?" data-tip-pos="bottom"><I.Info size={16} /></button>
      <button className="searchbtn" onClick={props.onPalette} aria-keyshortcuts="Control+K" data-tour="palette">
        <span><I.Search size={16} /><span className="label">Search or run a command</span></span>
        <Kbd combo={props.paletteKey} />
      </button>
      {props.showGuide && (
        <button className="ghost icon" onClick={props.onToggleGuide} aria-label={props.guideOpen ? "Hide guide panel" : "Show guide panel"} aria-expanded={props.guideOpen} data-tip={`${props.guideOpen ? "Hide" : "Show"} guide (${props.guideKey})`} data-tip-pos="bottom"><I.PanelRight size={18} /></button>
      )}
    </header>
  );
}
