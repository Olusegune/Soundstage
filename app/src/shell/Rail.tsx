import * as I from "../icons";
import type { GoldenStep, StepId } from "../golden";

export type View = "home" | "project" | "library" | "providers" | "settings" | "help" | "speak" | "tools" | "pronunciation" | "editor";

const STEP_ICON: Record<StepId, (p: { size?: number }) => React.JSX.Element> = {
  script: I.Script, cast: I.Cast, direct: I.Direct, render: I.Render, mix: I.Mix, export: I.Export,
};

function StepDot({ step }: { step: GoldenStep }) {
  if (step.status === "done") return <span className="step-dot done" aria-hidden="true"><I.Check size={12} /></span>;
  if (step.status === "blocked") return <span className="step-dot blocked" aria-hidden="true"><I.Lock size={11} /></span>;
  return <span className={`step-dot ${step.status === "current" ? "current" : ""}`} aria-hidden="true">{step.index + 1}</span>;
}

function statusWord(s: GoldenStep): string {
  return s.status === "done" ? "done" : s.status === "current" ? "recommended next" : s.status === "blocked" ? "locked" : "available";
}

export function Rail(props: {
  view: View; projectName: string | null; steps: GoldenStep[] | null; step: number;
  collapsed: boolean; onHome: () => void; onStep: (i: number) => void; onVoices: () => void; onEngines: () => void; onSpeak: () => void; onTools: () => void; onPron: () => void; onEditor: () => void;
  onHelp: () => void; onShortcuts: () => void; onSettings: () => void; onToggle: () => void; shortcutHint: (s: string) => string;
}) {
  const { view, steps } = props;
  return (
    <nav className="rail" aria-label="Main" data-tour="rail">
      <button className="brand" onClick={props.onHome} aria-label="Soundstage, home" data-tip="Home" data-tip-pos="right">
        <img src="/brand/logo-64.png" width={34} height={34} alt="" aria-hidden="true" />
        <span className="brand-name">Sound<b>stage</b></span>
      </button>

      <button className="rail-item" aria-current={view === "home" ? "page" : undefined} onClick={props.onHome} data-tip={props.collapsed ? "Projects" : undefined} data-tip-pos="right" aria-label={props.collapsed ? "Projects" : undefined}>
        <I.Home size={18} /><span className="grow">Projects</span>
      </button>

      {steps && (
        <>
          <div className="rail-label">Golden Path</div>
          <div className="rail-project" title={props.projectName ?? ""}>{props.projectName}</div>
          {steps.map((s) => {
            const Icon = STEP_ICON[s.id];
            const blocked = s.status === "blocked";
            const meta = s.progress && s.id !== "script" ? `${s.progress.done}/${s.progress.of}` : s.optional && s.status !== "done" ? "optional" : "";
            const tip = blocked ? (s.blockedReason ?? "Finish the earlier steps first") : `${s.label}: ${s.summary}`;
            return (
              <button key={s.id} className="rail-item" aria-current={view === "project" && props.step === s.index ? "step" : undefined}
                aria-disabled={blocked || undefined} aria-label={props.collapsed ? `${s.label}, ${statusWord(s)}. ${blocked ? s.blockedReason : s.summary}` : undefined}
                data-tip={tip} data-tip-pos="right"
                onClick={() => { if (!blocked) props.onStep(s.index); }}>
                <StepDot step={s} />
                <Icon size={16} />
                <span className="grow">{s.label}<span className="sr-only">, {statusWord(s)}{blocked && s.blockedReason ? `. ${s.blockedReason}` : ""}</span></span>
                {meta && <span className="step-meta">{meta}</span>}
              </button>
            );
          })}
        </>
      )}

      <div className="rail-label">Library</div>
      <button className="rail-item" aria-current={view === "library" ? "page" : undefined} onClick={props.onVoices} aria-label={props.collapsed ? "Voices" : undefined} data-tip={props.collapsed ? "Voices" : undefined} data-tip-pos="right">
        <I.Voices size={18} /><span className="grow">Voices</span>
      </button>

      <button className="rail-item" aria-current={view === "speak" ? "page" : undefined} onClick={props.onSpeak} aria-label={props.collapsed ? "Speak" : undefined} data-tip={props.collapsed ? "Speak" : undefined} data-tip-pos="right">
        <I.Speaker size={18} /><span className="grow">Speak</span>
      </button>

      <button className="rail-item" aria-current={view === "editor" ? "page" : undefined} onClick={props.onEditor} aria-label={props.collapsed ? "Audio editor" : undefined} data-tip={props.collapsed ? "Audio editor" : undefined} data-tip-pos="right">
        <I.Mix size={18} /><span className="grow">Audio editor</span>
      </button>

      <button className="rail-item" aria-current={view === "tools" ? "page" : undefined} onClick={props.onTools} aria-label={props.collapsed ? "Audio tools" : undefined} data-tip={props.collapsed ? "Audio tools" : undefined} data-tip-pos="right">
        <I.Wave size={18} /><span className="grow">Audio tools</span>
      </button>

      <button className="rail-item" aria-current={view === "pronunciation" ? "page" : undefined} onClick={props.onPron} aria-label={props.collapsed ? "Pronunciation" : undefined} data-tip={props.collapsed ? "Pronunciation" : undefined} data-tip-pos="right">
        <I.Book size={18} /><span className="grow">Pronunciation</span>
      </button>

      <button className="rail-item" aria-current={view === "providers" ? "page" : undefined} onClick={props.onEngines} aria-label={props.collapsed ? "Voice engines" : undefined} data-tip={props.collapsed ? "Voice engines" : undefined} data-tip-pos="right">
        <I.Layers size={18} /><span className="grow">Voice engines</span>
      </button>

      <div className="rail-foot">
        <button className="rail-item" aria-current={view === "help" ? "page" : undefined} onClick={props.onHelp} aria-label={props.collapsed ? "Help and guide" : undefined} data-tip={props.collapsed ? `Help (${props.shortcutHint("help")})` : undefined} data-tip-pos="right" data-tour="help">
          <I.Book size={18} /><span className="grow">Help and guide</span><span className="kbd-hint step-meta" aria-hidden="true">{props.shortcutHint("help")}</span>
        </button>
        <button className="rail-item" onClick={props.onShortcuts} aria-label={props.collapsed ? "Keyboard shortcuts" : undefined} data-tip={props.collapsed ? "Keyboard shortcuts" : undefined} data-tip-pos="right">
          <I.Keyboard size={18} /><span className="grow">Shortcuts</span><span className="kbd-hint step-meta" aria-hidden="true">{props.shortcutHint("shortcuts")}</span>
        </button>
        <button className="rail-item" aria-current={view === "settings" ? "page" : undefined} onClick={props.onSettings} aria-label={props.collapsed ? "Settings" : undefined} data-tip={props.collapsed ? "Settings" : undefined} data-tip-pos="right">
          <I.Gear size={18} /><span className="grow">Settings</span>
        </button>
        <button className="rail-item" onClick={props.onToggle} aria-label={props.collapsed ? "Expand sidebar" : undefined} data-tip={props.collapsed ? "Expand sidebar" : undefined} data-tip-pos="right">
          <I.PanelLeft size={18} /><span className="grow">Collapse</span><span className="kbd-hint step-meta" aria-hidden="true">{props.shortcutHint("toggle-rail")}</span>
        </button>
      </div>
    </nav>
  );
}
