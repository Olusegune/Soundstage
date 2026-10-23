import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { TOUR_STEPS } from "../help/content";
import { useDialog } from "../ui";

interface Rect { top: number; left: number; width: number; height: number }

/** A spotlight tour over the real interface. Steps whose target is not on screen are skipped. */
export function Tour({ onClose }: { onClose: (completed: boolean) => void }) {
  const [i, setI] = useState(0);
  const [rect, setRect] = useState<Rect | null>(null);
  const close = useRef(onClose);
  close.current = onClose;
  const ref = useDialog(() => close.current(false));

  const present = (idx: number) => !!document.querySelector(`[data-tour="${TOUR_STEPS[idx]?.target}"]`);
  // Skip any step whose target is not currently visible.
  useEffect(() => {
    if (i < TOUR_STEPS.length && !present(i)) {
      let n = i + 1;
      while (n < TOUR_STEPS.length && !present(n)) n++;
      if (n >= TOUR_STEPS.length) close.current(true); else setI(n);
    }
  }, [i]);

  useLayoutEffect(() => {
    const place = () => {
      const el = document.querySelector<HTMLElement>(`[data-tour="${TOUR_STEPS[i]?.target}"]`);
      if (!el) { setRect(null); return; }
      const r = el.getBoundingClientRect();
      setRect({ top: r.top - 6, left: r.left - 6, width: r.width + 12, height: r.height + 12 });
    };
    place();
    window.addEventListener("resize", place);
    return () => window.removeEventListener("resize", place);
  }, [i]);

  const step = TOUR_STEPS[i];
  if (!step || !rect) return null;
  const last = TOUR_STEPS.map((_, k) => k).filter(present).pop() === i;
  const W = 340;
  const left = Math.max(12, Math.min(window.innerWidth - W - 12, rect.left + rect.width + 16 > window.innerWidth - W ? rect.left - W - 16 : rect.left + rect.width + 16));
  const top = Math.max(12, Math.min(window.innerHeight - 230, rect.top));
  const next = () => { let n = i + 1; while (n < TOUR_STEPS.length && !present(n)) n++; if (n >= TOUR_STEPS.length) onClose(true); else setI(n); };

  return (
    <div className="tour-scrim">
      <div className="tour-hole" style={{ top: rect.top, left: rect.left, width: rect.width, height: rect.height }} />
      <div className="tour-pop" role="dialog" aria-modal="true" aria-label={`Tour: ${step.title}`} style={{ top, left }} ref={ref}>
        <div className="guide-eyebrow">Tour · {i + 1} of {TOUR_STEPS.length}</div>
        <h2>{step.title}</h2>
        <p className="muted">{step.body}</p>
        <div className="row" style={{ justifyContent: "space-between", marginTop: 12 }}>
          <button className="ghost small" onClick={() => onClose(false)}>Skip tour</button>
          <button className="primary small" onClick={next}>{last ? "Finish" : "Next"}</button>
        </div>
      </div>
    </div>
  );
}
