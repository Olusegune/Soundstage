import { useRef } from "react";
import * as I from "../icons";
import type { GoldenStep, NextAction } from "../golden";
import { CostBadge, Kbd } from "../ui";

/** The Golden Path made functional: where am I, why it matters, what happens if I skip it, what next. */
export function GuidePanel(props: {
  steps: GoldenStep[]; next: NextAction | null; current: number; viewing: number; busy: boolean; nextKey: string;
  onGoto: (i: number) => void; onNext: () => void; onHelp: (id: string) => void;
  width: number; onResize: (w: number) => void; hideNext?: boolean; onAuto?: () => void;
}) {
  const s = props.steps[props.viewing];
  const done = props.steps.filter((x) => x.status === "done").length;
  const recommended = props.steps[props.current];
  const drag = useRef<{ x: number; w: number } | null>(null);
  if (!s) return null;

  const onDown = (e: React.PointerEvent) => { drag.current = { x: e.clientX, w: props.width }; (e.target as HTMLElement).setPointerCapture(e.pointerId); };
  const onMove = (e: React.PointerEvent) => { if (drag.current) props.onResize(drag.current.w + (drag.current.x - e.clientX)); };
  const onUp = () => { drag.current = null; };
  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowLeft") { e.preventDefault(); props.onResize(props.width + 24); }
    if (e.key === "ArrowRight") { e.preventDefault(); props.onResize(props.width - 24); }
  };

  return (
    <aside className="guide" aria-label="Guide" data-tour="guide">
      <button className="guide-handle" aria-label="Resize the guide panel. Use the left and right arrow keys." role="separator" aria-orientation="vertical" aria-valuenow={props.width}
        onPointerDown={onDown} onPointerMove={onMove} onPointerUp={onUp} onKeyDown={onKey} />
      <div className="guide-body">
        <div className="guide-eyebrow">Step {s.index + 1} of {props.steps.length}{s.optional ? " · Optional" : ""}</div>
        <h2>{s.label}</h2>
        <div className="row tight">
          <CostBadge cost={s.cost} />
          {s.status === "done" && <span className="chip ok"><I.Check size={12} /> Done</span>}
          {s.status === "current" && <span className="chip gold">Recommended now</span>}
          <button className="link small" onClick={() => props.onHelp(s.helpId)} aria-label={`Read more about ${s.label}`}>Read more</button>
        </div>
        <div className="progress-line" aria-label={`${done} of ${props.steps.length} steps done`}>
          <span className="num">{done}/{props.steps.length}</span>
          <div className="bar" role="progressbar" aria-label="Steps done" aria-valuemin={0} aria-valuemax={props.steps.length} aria-valuenow={done}><div style={{ width: `${(100 * done) / props.steps.length}%` }} /></div>
        </div>
        <p style={{ margin: 0 }}><strong>{s.summary}</strong></p>

        {props.viewing !== props.current && recommended && recommended.status !== "done" && (
          <div className="callout gold" style={{ marginTop: 12 }}>
            <strong>Recommended: {recommended.label}</strong>
            <button className="small" style={{ marginTop: 6 }} onClick={() => props.onGoto(props.current)}>Go to {recommended.label}</button>
          </div>
        )}

        <div className="sect"><h3>Why this matters</h3><p>{s.why}</p></div>
        <div className="sect">
          <h3>Checklist</h3>
          <ul className="tasklist">
            {s.tasks.map((t) => (
              <li key={t.label} className={t.done ? "done" : ""}>
                <span className="tick" aria-hidden="true">{t.done && <I.Check size={11} />}</span>
                <span><span className="sr-only">{t.done ? "Done: " : "To do: "}</span>{t.label}</span>
              </li>
            ))}
          </ul>
        </div>
        <div className="sect"><h3>If you skip it</h3><p>{s.ifSkipped}</p></div>
        <div className="sect"><h3>Cost</h3><p>{s.costNote}</p></div>
      </div>

      {props.next && !props.hideNext && (
        <div className="guide-next">
          <div className="guide-eyebrow">Next</div>
          <div className="reason">{props.next.reason}</div>
          <button className="primary" onClick={props.onNext} disabled={props.busy} data-tour="next" aria-keyshortcuts="Control+Enter">
            {props.busy ? <><span className="spin" aria-hidden="true" /> Working…</> : <>{props.next.paid ? <I.Cloud size={16} /> : <I.Zap size={16} />}{props.next.label}</>}
            <span style={{ marginLeft: "auto" }}><Kbd combo={props.nextKey} /></span>
          </button>
          {props.onAuto && (
            <button className="small ghost" style={{ marginTop: 8, width: "100%" }} onClick={props.onAuto} disabled={props.busy} data-tip="Casts free voices, renders every line and builds a preview mix, then stops so you can listen before exporting." data-tip-pos="top">
              <I.Sparkle size={14} /> Do all the steps for me
            </button>
          )}
        </div>
      )}
    </aside>
  );
}
