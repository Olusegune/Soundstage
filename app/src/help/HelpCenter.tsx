import { useMemo, useState } from "react";
import * as I from "../icons";
import { ARTICLES, GLOSSARY, type Block } from "./content";
import { articleById, searchHelp } from "./search";
import { Kbd } from "../ui";

function Blocks({ blocks }: { blocks: Block[] }) {
  return (
    <>
      {blocks.map((b, i) => {
        switch (b.t) {
          case "p": return <p key={i}>{b.text}</p>;
          case "h": return <h3 key={i}>{b.text}</h3>;
          case "ul": return <ul key={i}>{b.items.map((x) => <li key={x}>{x}</li>)}</ul>;
          case "steps": return <ol className="steps" key={i}>{b.items.map((x) => <li key={x}><span>{x}</span></li>)}</ol>;
          case "tip": return <div className="callout" key={i}><strong>Tip</strong>{b.text}</div>;
          case "paid": return <div className="callout gold" key={i}><strong>Uses your provider account</strong>{b.text}</div>;
          case "keys": return (
            <table key={i}><tbody>{b.rows.map(([k, d]) => (
              <tr key={k}><td style={{ whiteSpace: "nowrap" }}>{k.split(" / ").map((x, n) => <span key={x}>{n > 0 && " / "}<Kbd combo={x.replace(/ to /, "+")} /></span>)}</td><td>{d}</td></tr>
            ))}</tbody></table>
          );
        }
      })}
    </>
  );
}

export function HelpCenter(props: { articleId: string; onSelect: (id: string) => void; onBack: () => void; onShortcuts: () => void; onTour: () => void }) {
  const [q, setQ] = useState("");
  const hits = useMemo(() => (q.trim() ? searchHelp(q) : []), [q]);
  const isGlossary = props.articleId === "glossary";
  const article = articleById(props.articleId);
  const groups = ["Start here", "The steps", "Understanding", "Fix a problem"] as const;

  return (
    <div className="page wide">
      <div className="stage-title">
        <h1>Help and guide</h1>
        <span className="spacer" />
        <button className="small" onClick={props.onTour}><I.Rocket size={14} /> Take the tour</button>
        <button className="small" onClick={props.onShortcuts}><I.Keyboard size={14} /> Shortcuts</button>
        <button className="small" onClick={props.onBack}>Close</button>
      </div>
      <div className="help-layout">
        <nav className="help-nav" aria-label="Help topics">
          <label htmlFor="help-q" className="sr-only">Search the guide</label>
          <input id="help-q" type="search" placeholder="Search the guide and glossary" value={q} onChange={(e) => setQ(e.target.value)} style={{ width: "100%" }} />
          {groups.map((g) => (
            <div key={g}>
              <h3>{g}</h3>
              {ARTICLES.filter((a) => a.group === g).map((a) => (
                <button key={a.id} className="help-link" aria-current={!isGlossary && props.articleId === a.id ? "page" : undefined} onClick={() => { setQ(""); props.onSelect(a.id); }}>{a.title}</button>
              ))}
            </div>
          ))}
          <h3>Reference</h3>
          <button className="help-link" aria-current={isGlossary ? "page" : undefined} onClick={() => { setQ(""); props.onSelect("glossary"); }}>Glossary of terms</button>
        </nav>

        <div className="article" aria-live="polite">
          {q.trim() ? (
            <>
              <h1>{hits.length} result{hits.length === 1 ? "" : "s"} for “{q}”</h1>
              {hits.length === 0 && <p className="muted">Nothing found. Try a simpler word, or open the glossary.</p>}
              {hits.map((h) => (
                <div key={`${h.kind}-${h.id}`} className="card flat">
                  <div className="row tight"><strong>{h.title}</strong><span className="chip">{h.kind === "glossary" ? "Glossary" : "Guide"}</span></div>
                  <p className="muted small" style={{ margin: "4px 0 8px" }}>{h.snippet}</p>
                  <button className="small" onClick={() => { setQ(""); props.onSelect(h.kind === "glossary" ? "glossary" : h.id); }}>Open</button>
                </div>
              ))}
            </>
          ) : isGlossary ? (
            <>
              <h1>Glossary</h1>
              <p className="summary">Plain-language meanings for the words you will meet in Soundstage.</p>
              <dl className="gloss">
                {GLOSSARY.slice().sort((a, b) => a.term.localeCompare(b.term)).map((g) => <div key={g.term}><dt>{g.term}</dt><dd>{g.meaning}</dd></div>)}
              </dl>
            </>
          ) : article ? (
            <>
              <h1>{article.title}</h1>
              <p className="summary">{article.summary}</p>
              <Blocks blocks={article.body} />
            </>
          ) : <p>That topic was not found.</p>}
        </div>
      </div>
    </div>
  );
}
