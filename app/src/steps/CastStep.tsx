import { useState } from "react";
import { api, messageOf } from "../api";
import type { Ctx } from "../ctx";
import { licenceNote } from "../logic";
import { Confirm, Empty, LicenceChip, PlayButton } from "../ui";
import type { Voice } from "../types";

const TAG_RE = /\[\s*([a-z]+)(?::[^\]]*)?\s*\]/gi;
const NON_PAUSE = (t: string) => t.toLowerCase() !== "pause";

/** Canonical tags a speaker's lines need, ignoring pauses (every voice can pause). */
function neededTags(texts: string[]): string[] {
  const out = new Set<string>();
  for (const t of texts) for (const m of t.matchAll(TAG_RE)) { const n = (m[1] ?? "").toLowerCase(); if (NON_PAUSE(n)) out.add(n); }
  return [...out];
}

function stripTags(t: string): string {
  return t.replace(/\[[^\]]*\]/g, " ").replace(/\s+/g, " ").trim();
}

export function CastStep({ ctx }: { ctx: Ctx }) {
  const { project, analysis, voices, edit } = ctx;
  const [previews, setPreviews] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState<string | null>(null);
  const [pending, setPending] = useState<{ speaker: string; voiceId: string; takes: number } | null>(null);

  if (!analysis || analysis.parsed.speakers.length === 0) {
    return <div className="page"><Empty title="No speakers yet"><p className="muted">Add dialogue in the Script step first.</p><button onClick={() => ctx.goto(0)}>Go to Script</button></Empty></div>;
  }

  const setVoice = (speaker: string, voiceId: string) =>
    edit((p) => ({ ...p, characters: { ...p.characters, [speaker]: { voice_id: voiceId || null, bible: p.characters[speaker]?.bible ?? emptyBible(), direction: p.characters[speaker]?.direction ?? {} } } }));

  const choose = (speaker: string, voiceId: string) => {
    const cur = project.characters[speaker]?.voice_id;
    const takes = project.takes.filter((t) => t.speaker === speaker).length;
    if (cur && cur !== voiceId && takes > 0) setPending({ speaker, voiceId, takes });
    else setVoice(speaker, voiceId);
  };

  const sample = async (speaker: string, v: Voice) => {
    const first = analysis.lines.find((l) => l.line.speaker === speaker);
    const text = (first && stripTags(first.line.text)) || v.sample_text;
    setLoading(speaker);
    try {
      const r = await api.previewVoice(v.id, text);
      setPreviews((s) => ({ ...s, [speaker]: r.path }));
      ctx.toast(`Sample ready for ${speaker}.`);
    } catch (e) {
      ctx.toast(messageOf(e), true);
    } finally {
      setLoading(null);
    }
  };

  const byProvider = voices.reduce<Record<string, Voice[]>>((acc, v) => { (acc[v.provider_label] ??= []).push(v); return acc; }, {});

  return (
    <div className="page">
      {voices.length === 0 && <div className="callout warn"><strong>Looking for voices on this computer…</strong>This takes a moment the first time.</div>}
      {analysis.parsed.speakers.map((speaker) => {
        const ch = project.characters[speaker];
        const v = voices.find((x) => x.id === ch?.voice_id);
        const lines = analysis.lines.filter((l) => l.line.speaker === speaker);
        const needs = neededTags(lines.map((l) => l.line.text));
        const missing = v ? needs.filter((t) => !v.tags.includes(t)) : [];
        const bible = ch?.bible ?? emptyBible();
        return (
          <div className="card" key={speaker}>
            <div className="row" style={{ marginBottom: 8 }}>
              <h2 style={{ margin: 0 }} className="speaker">{speaker}</h2>
              <span className="muted small">{lines.length} line{lines.length === 1 ? "" : "s"}</span>
              <span className="spacer" />
              {v && <LicenceChip licence={v.licence.licence} note={licenceNote(v.licence)} />}
            </div>
            <div className="row">
              <div style={{ minWidth: 280 }}>
                <label htmlFor={`voice-${speaker}`}>Voice</label>
                <select id={`voice-${speaker}`} value={ch?.voice_id ?? ""} onChange={(e) => choose(speaker, e.target.value)} style={{ width: "100%" }}>
                  <option value="">Choose a voice…</option>
                  {Object.entries(byProvider).map(([label, list]) => (
                    <optgroup key={label} label={label}>
                      {list.map((x) => <option key={x.id} value={x.id}>{x.name} · {x.accent} · {x.gender}</option>)}
                    </optgroup>
                  ))}
                </select>
              </div>
              <div style={{ alignSelf: "end" }} className="row">
                <button disabled={!v || loading === speaker} onClick={() => v && void sample(speaker, v)}>{loading === speaker ? "Preparing…" : "Make a sample"}</button>
                <PlayButton path={previews[speaker] ?? null} label={`sample for ${speaker}`} bust={previews[speaker]} />
                <button className="ghost" onClick={ctx.openLibrary}>Find a voice…</button>
              </div>
            </div>
            {v && (
              <p className="small muted" style={{ marginTop: 8 }}>
                {v.provider_label} · {v.language} · {v.notes}
                {" "}Runs on this computer; nothing is sent anywhere.
              </p>
            )}
            {missing.length > 0 && (
              <div className="callout warn small" role="alert">
                <strong>⚠ Not supported by this voice</strong>
                {v?.name} cannot perform {missing.map((t) => `[${t}]`).join(", ")}, which {speaker} uses. Pick another voice or remove the tag in Direct. Nothing is dropped silently.
              </div>
            )}
            <details style={{ marginTop: 8 }}>
              <summary className="small muted" style={{ cursor: "pointer" }}>Character notes (age, background, habits)</summary>
              <div className="grid2" style={{ marginTop: 8, gridTemplateColumns: "1fr 1fr" }}>
                {(["age", "background", "speech_habits", "emotional_range", "notes"] as const).map((f) => (
                  <div key={f}>
                    <label htmlFor={`${speaker}-${f}`}>{f.replace("_", " ")}</label>
                    <input id={`${speaker}-${f}`} type="text" style={{ width: "100%" }} value={bible[f]} maxLength={300}
                      onChange={(e) => edit((p) => { const c = p.characters[speaker]; return { ...p, characters: { ...p.characters, [speaker]: { voice_id: c?.voice_id ?? null, direction: c?.direction ?? {}, bible: { ...(c?.bible ?? emptyBible()), [f]: e.target.value } } } }; }, `bible-${speaker}-${f}`)} />
                  </div>
                ))}
              </div>
              <p className="small muted">These notes stay with the project. Today they are context for you; a voice uses them only where its provider accepts context.</p>
            </details>
          </div>
        );
      })}
      {pending && (
        <Confirm
          title={`Change ${pending.speaker}'s voice?`}
          body={<p>{pending.speaker} already has {pending.takes} rendered take{pending.takes === 1 ? "" : "s"}. A new voice makes those lines stale so they can be rendered again. Earlier takes are kept.</p>}
          confirmLabel="Change voice"
          onCancel={() => setPending(null)}
          onConfirm={() => { setVoice(pending.speaker, pending.voiceId); setPending(null); }}
        />
      )}
    </div>
  );
}

function emptyBible() {
  return { age: "", background: "", speech_habits: "", emotional_range: "", notes: "" };
}
