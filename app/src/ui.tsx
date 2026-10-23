import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { api, audioUrl } from "./api";
import * as I from "./icons";
import { licenceClass, licenceText } from "./logic";
import type { Licence } from "./types";

/* ------------------------------------------------------------------ one shared player */
interface PlayerApi {
  playing: string | null;
  play: (path: string, bust?: string) => void;
  stop: () => void;
  /** Toggle whatever played last, or stop if something is playing. Used by the Space shortcut. */
  toggleLast: () => void;
  error: string | null;
}
const PlayerCtx = createContext<PlayerApi | null>(null);

export function PlayerProvider({ children }: { children: ReactNode }) {
  const el = useRef<HTMLAudioElement | null>(null);
  const last = useRef<{ path: string; bust?: string } | null>(null);
  const [playing, setPlaying] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  if (!el.current && typeof Audio !== "undefined") el.current = new Audio();
  useEffect(() => {
    const a = el.current;
    if (!a) return;
    const done = () => setPlaying(null);
    const fail = () => { setPlaying(null); setError("This audio could not be played."); };
    a.addEventListener("ended", done);
    a.addEventListener("error", fail);
    return () => { a.removeEventListener("ended", done); a.removeEventListener("error", fail); a.pause(); };
  }, []);
  const stop = useCallback(() => { el.current?.pause(); setPlaying(null); }, []);
  const play = useCallback((path: string, bust?: string) => {
    const a = el.current;
    if (!a) return;
    if (playing === path) { stop(); return; }
    setError(null);
    last.current = { path, bust };
    a.src = audioUrl(path, bust);
    setPlaying(path);
    a.play().catch(() => { setPlaying(null); setError("This audio could not be played."); });
  }, [playing, stop]);
  const toggleLast = useCallback(() => {
    if (playing) { stop(); return; }
    if (last.current) play(last.current.path, last.current.bust);
  }, [playing, play, stop]);
  const value = useMemo(() => ({ playing, play, stop, toggleLast, error }), [playing, play, stop, toggleLast, error]);
  return <PlayerCtx.Provider value={value}>{children}</PlayerCtx.Provider>;
}

export function usePlayer(): PlayerApi {
  const c = useContext(PlayerCtx);
  if (!c) throw new Error("PlayerProvider is missing");
  return c;
}

export function PlayButton({ path, label, bust }: { path: string | null; label: string; bust?: string }) {
  const p = usePlayer();
  const on = path !== null && p.playing === path;
  return (
    <button className="small icon" disabled={!path} onClick={() => path && p.play(path, bust)} aria-label={`${on ? "Stop" : "Play"} ${label}`} aria-pressed={on} data-tip={path ? undefined : "Nothing to play yet"}>
      {on ? <I.Stop size={14} /> : <I.Play size={14} />}
    </button>
  );
}

/* ------------------------------------------------------------------ waveform */
const waveCache = new Map<string, number[]>();

/** A take's or asset's waveform: the shape of the sound over time. Fetched once per file. */
export function Waveform({ projectId, file, bins = 96, tone = "teal", dim = false, height = 36, label, fill = false }: {
  projectId: string; file: string | null; bins?: number; tone?: "teal" | "gold"; dim?: boolean; height?: number; label?: string; fill?: boolean;
}) {
  const key = file ? `${projectId}/${file}/${bins}` : "";
  const [peaks, setPeaks] = useState<number[] | null>(key ? waveCache.get(key) ?? null : null);
  useEffect(() => {
    if (!file) { setPeaks(null); return; }
    const hit = waveCache.get(key);
    if (hit) { setPeaks(hit); return; }
    let live = true;
    api.waveform(projectId, file, bins).then((p) => { waveCache.set(key, p); if (live) setPeaks(p); }).catch(() => { if (live) setPeaks([]); });
    return () => { live = false; };
  }, [projectId, file, bins, key]);
  return <PeaksView peaks={peaks} bins={bins} tone={tone} dim={dim} height={height} label={label} fill={fill} />;
}

export function PeaksView({ peaks, bins = 96, tone = "teal", dim = false, height = 36, label, fill = false }: { peaks: number[] | null; bins?: number; tone?: "teal" | "gold"; dim?: boolean; height?: number; label?: string; fill?: boolean }) {
  if (!peaks) return <div className="skeleton" style={{ height }} aria-hidden="true" />;
  if (peaks.length === 0) return <div className="faint small" style={{ height, display: "grid", alignItems: "center" }}>No waveform available</div>;
  const n = peaks.length;
  const w = 100 / n;
  return (
    <svg className={`wave ${tone === "gold" ? "gold" : ""} ${dim ? "dim" : ""}`} viewBox={`0 0 100 ${height}`} preserveAspectRatio="none" style={fill ? { height: "100%" } : { height }} role="img" aria-label={label ?? "Waveform"}>
      {peaks.map((p, i) => {
        const h = Math.max(1, p * (height - 2));
        return <rect key={i} x={i * w + w * 0.14} y={(height - h) / 2} width={w * 0.72} height={h} rx={0.3} />;
      })}
      <title>{label ?? `Waveform of ${bins} slices`}</title>
    </svg>
  );
}

/* ------------------------------------------------------------------ chips and small parts */
export function LicenceChip({ licence, note }: { licence: Licence; note?: string }) {
  const icon = licence === "commercial" ? <I.Check size={12} /> : licence === "non-commercial" ? <I.Close size={12} /> : <I.Info size={12} />;
  return <span className={`chip ${licenceClass(licence)}`} title={note}>{icon} {licenceText(licence)}</span>;
}

export function CostBadge({ cost }: { cost: "free" | "paid" | "mixed" }) {
  return cost === "free"
    ? <span className="chip info" title="Runs on this computer"><I.Device size={12} /> Free</span>
    : cost === "paid"
      ? <span className="chip gold" title="Uses your provider account"><I.Cloud size={12} /> Paid</span>
      : <span className="chip gold" title="Free with Windows voices; cloud voices use your provider account"><I.Cloud size={12} /> Free or paid</span>;
}

export function Kbd({ combo }: { combo: string }) {
  return <>{combo.split("+").map((k, i) => <kbd key={i} style={{ marginLeft: i ? 3 : 0 }}>{k}</kbd>)}</>;
}

/** A small "?" that explains a term in plain language. Keyboard and screen-reader friendly. */
export function Help({ text }: { text: string }) {
  return <button type="button" className="help-dot" data-tip={text} aria-label={`What is this? ${text}`}>?</button>;
}

/* ------------------------------------------------------------------ toast */
export interface ToastMsg { text: string; bad?: boolean }

export function Toast({ toast, onClose }: { toast: ToastMsg | null; onClose: () => void }) {
  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(onClose, toast.bad ? 9000 : 4500);
    return () => clearTimeout(t);
  }, [toast, onClose]);
  if (!toast) return null;
  return (
    <div className={`toast ${toast.bad ? "bad" : ""}`} role={toast.bad ? "alert" : "status"}>
      <span style={{ color: toast.bad ? "var(--err)" : "var(--teal-text)", paddingTop: 2 }}>{toast.bad ? <I.Alert size={16} /> : <I.Check size={16} />}</span>
      <span style={{ flex: 1 }}>{toast.text}</span>
      <button className="ghost small icon" onClick={onClose} aria-label="Dismiss"><I.Close size={14} /></button>
    </div>
  );
}

/* ------------------------------------------------------------------ dialog with focus trap */
const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function useDialog(onClose: () => void, initialFocus?: React.RefObject<HTMLElement | null>) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const prev = document.activeElement as HTMLElement | null;
    const node = ref.current;
    (initialFocus?.current ?? node?.querySelector<HTMLElement>(FOCUSABLE) ?? node)?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") { e.stopPropagation(); onClose(); return; }
      if (e.key !== "Tab" || !node) return;
      const items = [...node.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((x) => x.offsetParent !== null);
      if (items.length === 0) { e.preventDefault(); return; }
      const first = items[0]!, lastEl = items[items.length - 1]!;
      if (e.shiftKey && document.activeElement === first) { e.preventDefault(); lastEl.focus(); }
      else if (!e.shiftKey && document.activeElement === lastEl) { e.preventDefault(); first.focus(); }
    };
    document.addEventListener("keydown", onKey, true);
    return () => { document.removeEventListener("keydown", onKey, true); prev?.focus?.(); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return ref;
}

export function Dialog(props: { title: string; children: ReactNode; onClose: () => void; wide?: boolean; label?: string }) {
  const ref = useDialog(props.onClose);
  return (
    <div className="scrim" onMouseDown={(e) => { if (e.target === e.currentTarget) props.onClose(); }}>
      <div className={`dialog ${props.wide ? "wide" : ""}`} role="dialog" aria-modal="true" aria-label={props.label ?? props.title} ref={ref}>
        {props.children}
      </div>
    </div>
  );
}

/** Confirmation that names the thing being changed (UX_GOLDEN_PATH section 3). */
export function Confirm(props: { title: string; body: ReactNode; confirmLabel: string; onConfirm: () => void; onCancel: () => void; danger?: boolean }) {
  const cancel = useRef<HTMLButtonElement>(null);
  const ref = useDialog(props.onCancel, cancel);
  return (
    <div className="scrim" onMouseDown={(e) => { if (e.target === e.currentTarget) props.onCancel(); }}>
      <div className="dialog" role="alertdialog" aria-modal="true" aria-label={props.title} ref={ref}>
        <h2>{props.title}</h2>
        <div style={{ marginBottom: 20 }} className="muted">{props.body}</div>
        <div className="row" style={{ justifyContent: "flex-end" }}>
          <button ref={cancel} onClick={props.onCancel}>Cancel</button>
          <button className={props.danger === false ? "primary" : "danger"} onClick={props.onConfirm}>{props.confirmLabel}</button>
        </div>
      </div>
    </div>
  );
}

export function Empty({ title, children }: { title: string; children?: ReactNode }) {
  return (
    <div className="empty">
      <h2>{title}</h2>
      {children}
    </div>
  );
}

export function Logo({ size = 28 }: { size?: number }) {
  return <img src="/brand/logo-64.png" width={size} height={size} alt="" aria-hidden="true" style={{ display: "block" }} />;
}
