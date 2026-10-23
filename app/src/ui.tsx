import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { audioUrl } from "./api";
import { licenceClass, licenceText } from "./logic";
import type { Licence } from "./types";

/** One shared player so only one thing plays at a time. */
interface PlayerApi {
  playing: string | null;
  play: (path: string, bust?: string) => void;
  stop: () => void;
  error: string | null;
}
const PlayerCtx = createContext<PlayerApi | null>(null);

export function PlayerProvider({ children }: { children: ReactNode }) {
  const el = useRef<HTMLAudioElement | null>(null);
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
    a.src = audioUrl(path, bust);
    setPlaying(path);
    a.play().catch(() => { setPlaying(null); setError("This audio could not be played."); });
  }, [playing, stop]);
  return <PlayerCtx.Provider value={{ playing, play, stop, error }}>{children}</PlayerCtx.Provider>;
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
    <button className="small" disabled={!path} onClick={() => path && p.play(path, bust)} aria-label={`${on ? "Stop" : "Play"} ${label}`}>
      {on ? "■ Stop" : "▶ Play"}
    </button>
  );
}

export function LicenceChip({ licence, note }: { licence: Licence; note?: string }) {
  const icon = licence === "commercial" ? "✓" : licence === "non-commercial" ? "✕" : "?";
  return (
    <span className={`chip ${licenceClass(licence)}`} title={note}>
      <span aria-hidden="true">{icon}</span> {licenceText(licence)}
    </span>
  );
}

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
      {toast.text} <button className="ghost small" onClick={onClose} aria-label="Dismiss">✕</button>
    </div>
  );
}

/** Confirmation that names the thing being deleted (UX_GOLDEN_PATH section 3). */
export function Confirm(props: { title: string; body: ReactNode; confirmLabel: string; onConfirm: () => void; onCancel: () => void }) {
  const ref = useRef<HTMLButtonElement>(null);
  useEffect(() => { ref.current?.focus(); }, []);
  useEffect(() => {
    const h = (e: KeyboardEvent) => { if (e.key === "Escape") props.onCancel(); };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [props]);
  return (
    <div className="modal-back" role="dialog" aria-modal="true" aria-label={props.title}>
      <div className="modal">
        <h2>{props.title}</h2>
        <div style={{ marginBottom: 16 }}>{props.body}</div>
        <div className="row" style={{ justifyContent: "flex-end" }}>
          <button ref={ref} onClick={props.onCancel}>Cancel</button>
          <button className="danger" onClick={props.onConfirm}>{props.confirmLabel}</button>
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

export function Logo() {
  return (
    <svg width="22" height="22" viewBox="0 0 22 22" aria-hidden="true">
      {[4, 9, 14, 9, 5].map((h, i) => (
        <rect key={i} x={1 + i * 4.2} y={11 - h / 2 - 0.5} width="2.6" height={h + 1} rx="1.3" fill="var(--signal)" />
      ))}
    </svg>
  );
}
