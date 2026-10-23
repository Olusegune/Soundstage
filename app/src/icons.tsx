// One icon family: 24 px grid, 1.6 px stroke, round caps and joins, monochrome (colour comes from the context).
// No emoji anywhere in the interface.
import type { ReactNode } from "react";

type P = { size?: number; className?: string };

function Svg({ size = 18, className, children }: P & { children: ReactNode }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.6} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false" className={className}>
      {children}
    </svg>
  );
}
const mk = (d: ReactNode) => (p: P) => <Svg {...p}>{d}</Svg>;

export const Home = mk(<><path d="M4 11l8-7 8 7" /><path d="M6 10v9h4v-5h4v5h4v-9" /></>);
export const Script = mk(<><path d="M7 3h8l4 4v14H7z" /><path d="M15 3v4h4M10 12h6M10 16h6" /></>);
export const Cast = mk(<><circle cx="9" cy="8" r="3.2" /><path d="M3.5 19c.5-3.2 2.8-5 5.5-5s5 1.8 5.5 5" /><path d="M16 5.5a3 3 0 0 1 0 5.4M18.5 14.4c1.4.7 2.3 2.1 2.6 4.1" /></>);
export const Direct = mk(<><path d="M4 7h10M18 7h2M4 17h2M10 17h10" /><circle cx="16" cy="7" r="2" /><circle cx="8" cy="17" r="2" /></>);
export const Render = mk(<><path d="M3 12h2M7 8v8M11 5v14M15 9v6M19 11v2" /></>);
export const Mix = mk(<><path d="M6 4v16M12 4v16M18 4v16" /><rect x="4" y="13" width="4" height="3" rx="1" /><rect x="10" y="7" width="4" height="3" rx="1" /><rect x="16" y="11" width="4" height="3" rx="1" /></>);
export const Export = mk(<><path d="M12 15V4M8 8l4-4 4 4" /><path d="M5 14v4a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-4" /></>);
export const Voices = mk(<><rect x="9" y="3" width="6" height="11" rx="3" /><path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M9 21h6" /></>);
export const Book = mk(<><path d="M5 4h10a3 3 0 0 1 3 3v13H8a3 3 0 0 1-3-3z" /><path d="M5 17a3 3 0 0 1 3-3h10" /></>);
export const Gear = mk(<><circle cx="12" cy="12" r="3" /><path d="M12 2.8v2.4M12 18.8v2.4M4.6 7.4l2 1.2M17.4 15.4l2 1.2M4.6 16.6l2-1.2M17.4 8.6l2-1.2" /></>);
export const Search = mk(<><circle cx="11" cy="11" r="6.5" /><path d="m20 20-4.2-4.2" /></>);
export const Command = mk(<path d="M9 9h6v6H9zM9 9V6.5A2.5 2.5 0 1 0 6.5 9H9zM15 9V6.5A2.5 2.5 0 1 1 17.5 9H15zM9 15v2.5A2.5 2.5 0 1 1 6.5 15H9zM15 15v2.5a2.5 2.5 0 1 0 2.5-2.5H15z" />);
export const Undo = mk(<><path d="M9 14 4 9l5-5" /><path d="M4 9h9a6 6 0 0 1 0 12h-3" /></>);
export const Redo = mk(<><path d="m15 14 5-5-5-5" /><path d="M20 9h-9a6 6 0 0 0 0 12h3" /></>);
export const Check = mk(<path d="m5 12.5 4.5 4.5L19 7.5" />);
export const Alert = mk(<><path d="M12 3.5 2.8 19.5h18.4z" /><path d="M12 10v4.5M12 17.4v.1" /></>);
export const Info = mk(<><circle cx="12" cy="12" r="9" /><path d="M12 11v5.5M12 7.6v.1" /></>);
export const Lock = mk(<><rect x="5" y="11" width="14" height="9" rx="2" /><path d="M8 11V8a4 4 0 0 1 8 0v3" /></>);
export const Cloud = mk(<path d="M7 18a4.5 4.5 0 0 1-.6-8.96A6 6 0 0 1 18 9.5 4 4 0 0 1 17.5 18z" />);
export const Device = mk(<><rect x="3" y="4" width="18" height="12" rx="2" /><path d="M8 20h8M12 16v4" /></>);
export const Play = (p: P) => <svg width={p.size ?? 18} height={p.size ?? 18} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" focusable="false" className={p.className}><path d="M8 5.2v13.6a.9.9 0 0 0 1.4.7l10.3-6.8a.9.9 0 0 0 0-1.5L9.4 4.5A.9.9 0 0 0 8 5.2z" /></svg>;
export const Stop = (p: P) => <svg width={p.size ?? 18} height={p.size ?? 18} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" focusable="false" className={p.className}><rect x="6" y="6" width="12" height="12" rx="1.5" /></svg>;
export const Plus = mk(<path d="M12 5v14M5 12h14" />);
export const Trash = mk(<><path d="M4 7h16M10 11v6M14 11v6" /><path d="M6 7l1 13h10l1-13M9 7V4h6v3" /></>);
export const Close = mk(<path d="m6 6 12 12M18 6 6 18" />);
export const ChevronRight = mk(<path d="m9 6 6 6-6 6" />);
export const ChevronLeft = mk(<path d="m15 6-6 6 6 6" />);
export const ChevronDown = mk(<path d="m6 9 6 6 6-6" />);
export const PanelLeft = mk(<><rect x="3" y="4" width="18" height="16" rx="2" /><path d="M9 4v16" /></>);
export const PanelRight = mk(<><rect x="3" y="4" width="18" height="16" rx="2" /><path d="M15 4v16" /></>);
export const Music = mk(<><path d="M9 18V6l11-2v12" /><circle cx="6.5" cy="18" r="2.5" /><circle cx="17.5" cy="16" r="2.5" /></>);
export const Wave = mk(<path d="M2 12h2M6 8v8M10 4v16M14 8v8M18 6v12M22 12h-1" />);
export const Sparkle = mk(<><path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z" /><path d="M19 16v4M17 18h4" /></>);
export const Folder = mk(<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />);
export const Keyboard = mk(<><rect x="2.5" y="6" width="19" height="12" rx="2" /><path d="M6 10h.1M10 10h.1M14 10h.1M18 10h.1M7 14h10" /></>);
export const Shield = mk(<><path d="M12 3l7 3v5c0 4.6-3 8.2-7 10-4-1.8-7-5.4-7-10V6z" /><path d="m9 12 2.2 2.2L15.5 10" /></>);
export const Sun = mk(<><circle cx="12" cy="12" r="4" /><path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6 7 7M17 17l1.4 1.4M5.6 18.4 7 17M17 7l1.4-1.4" /></>);
export const Moon = mk(<path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z" />);
export const Clock = mk(<><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></>);
export const Zap = mk(<path d="M13 3 5 13.5h6L10 21l8-10.5h-6z" />);
export const Layers = mk(<><path d="m12 3 9 5-9 5-9-5z" /><path d="m3 13 9 5 9-5" /></>);
export const Upload = mk(<><path d="M12 16V5M8 9l4-4 4 4" /><path d="M5 15v3a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-3" /></>);
export const Speaker = mk(<><path d="M4 10v4h3l5 4V6L7 10z" /><path d="M16 9a4 4 0 0 1 0 6M18.5 6.5a8 8 0 0 1 0 11" /></>);
export const Rocket = mk(<><path d="M5 19c-.6-2.6.2-4.4 2-6l5-5c2-2 5-3 8-3 0 3-1 6-3 8l-5 5c-1.6 1.8-3.4 2.6-6 2z" /><circle cx="15.5" cy="8.5" r="1.4" /><path d="M5 19l-2 2" /></>);
export const Eye = mk(<><path d="M2 12s3.6-6.5 10-6.5S22 12 22 12s-3.6 6.5-10 6.5S2 12 2 12z" /><circle cx="12" cy="12" r="3" /></>);

/** The brand mark, drawn small for the rail: five bars in the logo's gold and teal. */
export function Mark({ size = 24 }: { size?: number }) {
  const bars: [number, number, string][] = [[3, 5, "var(--teal)"], [7, 10, "var(--gold)"], [11, 16, "var(--gold-hi)"], [15, 10, "var(--gold)"], [19, 5, "var(--teal)"]];
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      {bars.map(([x, h, c]) => <rect key={x} x={x - 1.1} y={12 - h / 2} width="2.2" height={h} rx="1.1" fill={c} />)}
    </svg>
  );
}
