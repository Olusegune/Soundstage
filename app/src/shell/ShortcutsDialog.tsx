import { useEffect, useState } from "react";
import { Dialog, Kbd } from "../ui";
import { DEFAULT_SHORTCUTS, comboOf, isReserved, ownerOf, resolveBindings, type ShortcutDef, type ShortcutId } from "../shortcuts";
import { score } from "../help/search";

export function ShortcutsDialog(props: {
  overrides: Partial<Record<ShortcutId, string[]>>; onChange: (o: Partial<Record<ShortcutId, string[]>>) => void; onClose: () => void;
}) {
  const [q, setQ] = useState("");
  const [capturing, setCapturing] = useState<ShortcutId | null>(null);
  const [msg, setMsg] = useState<{ text: string; bad: boolean } | null>(null);
  const [pending, setPending] = useState<{ id: ShortcutId; combo: string; owner: ShortcutId } | null>(null);
  const b = resolveBindings(props.overrides);
  const label = (id: ShortcutId) => DEFAULT_SHORTCUTS.find((d) => d.id === id)?.label ?? id;

  useEffect(() => {
    if (!capturing) return;
    const h = (e: KeyboardEvent) => {
      e.preventDefault(); e.stopPropagation();
      if (e.key === "Escape") { setCapturing(null); setMsg(null); return; }
      const combo = comboOf(e);
      if (!combo) return;
      if (isReserved(combo)) { setMsg({ text: `${combo} keeps its normal meaning (copy, paste, tab and so on) and cannot be used.`, bad: true }); return; }
      if (!/^(Ctrl|Alt)\+|^F\d+$/.test(combo) && combo !== "Space") { setMsg({ text: "Add Ctrl or Alt so it never gets in the way of typing.", bad: true }); return; }
      const owner = ownerOf(b, combo, capturing);
      if (owner) { setPending({ id: capturing, combo, owner }); setCapturing(null); setMsg(null); return; }
      props.onChange({ ...props.overrides, [capturing]: [combo] });
      setCapturing(null); setMsg({ text: `${label(capturing)} is now ${combo}.`, bad: false });
    };
    window.addEventListener("keydown", h, true);
    return () => window.removeEventListener("keydown", h, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [capturing, props.overrides]);

  const groups = (["General", "Workflow", "Panels", "Help"] as ShortcutDef["group"][]).map((g) => ({
    g, rows: DEFAULT_SHORTCUTS.filter((d) => d.group === g && (!q || score(q, d.label) > 0 || b[d.id].some((k) => k.toLowerCase().includes(q.toLowerCase())))),
  })).filter((x) => x.rows.length);
  const customised = Object.keys(props.overrides).length > 0;

  return (
    <Dialog title="Keyboard shortcuts" onClose={props.onClose} wide>
      <div className="row" style={{ marginBottom: 12 }}>
        <h2 style={{ margin: 0 }}>Keyboard shortcuts</h2><span className="spacer" />
        <button className="small" disabled={!customised} onClick={() => { props.onChange({}); setMsg({ text: "All shortcuts are back to their defaults.", bad: false }); }}>Reset all</button>
        <button className="small" onClick={props.onClose}>Done</button>
      </div>
      <label htmlFor="sc-q" className="sr-only">Search shortcuts</label>
      <input id="sc-q" type="search" placeholder="Search shortcuts" value={q} onChange={(e) => setQ(e.target.value)} style={{ width: "100%", marginBottom: 12 }} />
      <p className="small muted">Shortcuts never take over typing: inside a text box, undo and redo belong to the box. Press Change, then the keys you want. Esc cancels.</p>
      <div aria-live="polite">{msg && <div className={`callout ${msg.bad ? "bad" : "ok"} small`}>{msg.text}</div>}</div>
      {pending && (
        <div className="callout warn small" role="alert">
          <strong>{pending.combo} is already used for “{label(pending.owner)}”.</strong>
          <div className="row" style={{ marginTop: 6 }}>
            <button className="small primary" onClick={() => {
              const o = { ...props.overrides };
              o[pending.id] = [pending.combo];
              // The previous owner falls back to its own default if that is free, otherwise it is left unbound by choosing a safe empty override.
              const d = DEFAULT_SHORTCUTS.find((x) => x.id === pending.owner)!;
              o[pending.owner] = d.keys.filter((k) => k !== pending.combo).length ? d.keys.filter((k) => k !== pending.combo) : o[pending.owner] ?? [];
              props.onChange(o); setPending(null);
            }}>Use it here and free it there</button>
            <button className="small" onClick={() => setPending(null)}>Keep the old one</button>
          </div>
        </div>
      )}
      <table className="keytable">
        <caption className="sr-only">Keyboard shortcuts</caption>
        <thead><tr><th scope="col">Action</th><th scope="col">Keys</th><th scope="col"><span className="sr-only">Change</span></th></tr></thead>
        <tbody>
          {groups.map(({ g, rows }) => (
            <>
              <tr key={g}><th colSpan={3} scope="colgroup" style={{ paddingTop: 16 }}>{g}</th></tr>
              {rows.map((d) => (
                <tr key={d.id}>
                  <td>{d.label}{d.project && <span className="small faint"> (in a project)</span>}</td>
                  <td>{b[d.id].map((k) => <span key={k} className="combo"><Kbd combo={k} /></span>)}</td>
                  <td style={{ textAlign: "right", whiteSpace: "nowrap" }}>
                    {capturing === d.id ? <span className="chip gold key-capture">Press the new keys…</span> : (
                      <>
                        <button className="small" onClick={() => { setCapturing(d.id); setMsg(null); setPending(null); }} aria-label={`Change shortcut for ${d.label}`}>Change</button>
                        {props.overrides[d.id] && <button className="small ghost" onClick={() => { const o = { ...props.overrides }; delete o[d.id]; props.onChange(o); }} aria-label={`Reset shortcut for ${d.label}`}>Reset</button>}
                      </>
                    )}
                  </td>
                </tr>
              ))}
            </>
          ))}
        </tbody>
      </table>
      {groups.length === 0 && <p className="muted">No shortcut matches “{q}”.</p>}
    </Dialog>
  );
}
