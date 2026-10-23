import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import * as I from "../icons";
import { score } from "../help/search";
import { Kbd, useDialog } from "../ui";

export interface PaletteItem {
  id: string;
  group: "Do" | "Go to" | "Projects" | "Script lines" | "Voices" | "Help";
  title: string;
  sub?: string;
  icon?: ReactNode;
  keys?: string;
  run: () => void;
  /** Shown when the palette is empty. */
  suggested?: boolean;
}

const GROUP_ORDER: PaletteItem["group"][] = ["Do", "Go to", "Projects", "Script lines", "Voices", "Help"];

export function CommandPalette({ items, onClose }: { items: PaletteItem[]; onClose: () => void }) {
  const [q, setQ] = useState("");
  const [sel, setSel] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const ref = useDialog(onClose, input);

  const results = useMemo(() => {
    const base = q.trim()
      ? items.map((it) => ({ it, s: Math.max(score(q, it.title) * 2, score(q, it.sub ?? "")) })).filter((x) => x.s > 0).sort((a, b) => b.s - a.s).slice(0, 40).map((x) => x.it)
      : items.filter((i) => i.suggested);
    // keep groups together in a fixed order, preserving score order inside a group
    return GROUP_ORDER.flatMap((g) => base.filter((i) => i.group === g));
  }, [q, items]);

  useEffect(() => { setSel(0); }, [q]);
  useEffect(() => { list.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" }); }, [sel, results]);

  const run = (it: PaletteItem | undefined) => { if (!it) return; onClose(); setTimeout(it.run, 0); };
  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") { e.preventDefault(); setSel((s) => Math.min(results.length - 1, s + 1)); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setSel((s) => Math.max(0, s - 1)); }
    else if (e.key === "Enter") { e.preventDefault(); run(results[sel]); }
  };

  let lastGroup = "";
  return (
    <div className="palette-scrim" onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="palette" role="dialog" aria-modal="true" aria-label="Command palette" ref={ref} onKeyDown={onKey}>
        <div className="palette-input">
          <I.Search size={18} />
          <input ref={input} type="text" value={q} onChange={(e) => setQ(e.target.value)} placeholder="Type a command, step, project, script line, voice or help topic…"
            role="combobox" aria-expanded="true" aria-controls="palette-list" aria-activedescendant={results[sel] ? `pal-${results[sel]!.id}` : undefined} aria-label="Search" autoComplete="off" spellCheck={false} />
          <kbd>Esc</kbd>
        </div>
        <div className="palette-list" id="palette-list" role="listbox" ref={list} aria-label="Results">
          {results.length === 0 && <div className="empty" style={{ padding: 28 }}><p style={{ margin: 0 }}>Nothing matches “{q}”.</p><p className="small faint">Try a step name, a speaker, or a word from the help guide.</p></div>}
          {results.map((it, i) => {
            const head = it.group !== lastGroup ? (lastGroup = it.group, <div className="palette-group" role="presentation" key={`g-${it.group}`}>{it.group}</div>) : null;
            return (
              <div key={it.id} role="presentation">
                {head}
                <button className="palette-item" id={`pal-${it.id}`} role="option" aria-selected={i === sel} onMouseMove={() => setSel(i)} onClick={() => run(it)} tabIndex={-1}>
                  <span style={{ color: "var(--teal-text)", display: "grid" }}>{it.icon ?? <I.Zap size={16} />}</span>
                  <span className="grow">{it.title}{it.sub && <span className="sub">{it.sub}</span>}</span>
                  {it.keys && <span><Kbd combo={it.keys} /></span>}
                </button>
              </div>
            );
          })}
        </div>
        <div className="palette-foot"><span><kbd>↑</kbd> <kbd>↓</kbd> move</span><span><kbd>Enter</kbd> run</span><span><kbd>Esc</kbd> close</span></div>
      </div>
    </div>
  );
}
