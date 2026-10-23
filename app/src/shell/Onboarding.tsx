import { useEffect, useState } from "react";
import { useDialog } from "../ui";

/** The supplied splash artwork, shown while voices load. Fades out; respects reduced motion. */
export function Splash({ ready, onDone }: { ready: boolean; onDone: () => void }) {
  const [minTime, setMinTime] = useState(false);
  const [timedOut, setTimedOut] = useState(false);
  const [leaving, setLeaving] = useState(false);
  useEffect(() => {
    const a = setTimeout(() => setMinTime(true), 1600);
    // Never trap the user behind the splash: after 7 seconds it goes whether or not the voices answered.
    const b = setTimeout(() => setTimedOut(true), 7000);
    return () => { clearTimeout(a); clearTimeout(b); };
  }, []);
  useEffect(() => { if ((ready && minTime) || timedOut) setLeaving(true); }, [ready, minTime, timedOut]);
  useEffect(() => {
    if (!leaving) return;
    const t = setTimeout(onDone, 520);
    return () => clearTimeout(t);
  }, [leaving, onDone]);
  return (
    <div className={`splash ${leaving ? "leaving" : ""}`} style={{ backgroundImage: "url(/brand/splash.webp)" }} role="img" aria-label="Soundstage. Voice. Sound. Music. Mix.">
      <div className="loading" aria-live="polite"><span className="spin" aria-hidden="true" /> Finding your voices</div>
    </div>
  );
}

const PANELS = [
  { t: "Everything for the sound", b: "Dialogue, music, effects and the final mix, in one place." },
  { t: "A path that guides you", b: "Script, Cast, Direct, Render, Mix, Export. Always one clear next step." },
  { t: "Free until you choose", b: "Windows voices cost nothing. Cloud voices always ask first." },
];

export function Welcome({ onTour, onExplore }: { onTour: () => void; onExplore: () => void }) {
  const ref = useDialog(onExplore);
  return (
    <div className="welcome" style={{ backgroundImage: "url(/brand/splash.webp)" }}>
      <div className="welcome-card" role="dialog" aria-modal="true" aria-labelledby="welcome-h" ref={ref}>
        <img className="welcome-art" src="/brand/logo-256.png" alt="" aria-hidden="true" />
        <h1 id="welcome-h">Welcome to Soundstage</h1>
        <p>The whole sound side of your film, with a guide beside you from the first line.</p>
        <div className="panels">
          {PANELS.map((p) => <div key={p.t}><b>{p.t}</b>{p.b}</div>)}
        </div>
        <div className="row" style={{ justifyContent: "center" }}>
          <button className="primary" onClick={onTour} style={{ minHeight: 44, padding: "0 28px" }}>Make a first scene with me</button>
          <button className="ghost" onClick={onExplore}>I will explore on my own</button>
        </div>
        <p className="small" style={{ marginTop: 16, color: "#b9b2a4" }}>You can start the tour any time with Ctrl+Shift+H, or press F1 for help.</p>
      </div>
    </div>
  );
}
