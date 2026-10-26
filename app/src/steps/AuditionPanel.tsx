import { useEffect, useMemo, useState } from "react";
import { api, joinPath, messageOf } from "../api";
import type { Ctx } from "../ctx";
import { Confirm, PlayButton, Waveform } from "../ui";
import type { FallbackOption, LineInfo, Take, Voice } from "../types";

/** The most voices tried on one line at a time. Matches the cap in the core (decision D-20). */
export const AUDITION_CAP = 4;

const LETTERS = "ABCD";

/** Try one line with other voices and listen side by side. The cast and the chosen take only change when you choose. */
export function AuditionPanel(props: { ctx: Ctx; info: LineInfo; onClose: () => void }) {
  const { ctx, info } = props;
  const key = info.line.key;
  const current = ctx.voices.find((v) => v.id === info.voice_id);
  const [q, setQ] = useState("");
  const [sameLang, setSameLang] = useState(true);
  const [picked, setPicked] = useState<string[]>([]);
  const [blind, setBlind] = useState(false);
  const [revealed, setRevealed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [ask, setAsk] = useState(false);
  const [made, setMade] = useState<Take[]>([]);

  const choices = useMemo(() => {
    const needle = q.trim().toLowerCase();
    return ctx.voices
      .filter((v) => v.id !== info.voice_id && v.language !== "zxx")
      .filter((v) => !sameLang || !current || v.language.split("-")[0] === current.language.split("-")[0])
      .filter((v) => !needle || `${v.name} ${v.accent} ${v.provider_label}`.toLowerCase().includes(needle))
      .slice(0, 60);
  }, [ctx.voices, info.voice_id, current, sameLang, q]);

  const chosen = picked.map((id) => ctx.voices.find((v) => v.id === id)).filter((v): v is Voice => !!v);
  const cloud = chosen.filter((v) => v.sends_data);
  const chars = info.line.text.replace(/\[[^\]]*\]/g, "").length;

  const toggle = (id: string) => setPicked((p) => (p.includes(id) ? p.filter((x) => x !== id) : p.length < AUDITION_CAP ? [...p, id] : p));

  const go = async (confirmed: boolean) => {
    setAsk(false);
    setBusy(true);
    try {
      const out = await ctx.audition(key, picked, confirmed);
      if (out) {
        setMade(out.new_takes);
        const bad = out.results.filter((r) => !r.ok);
        if (bad.length > 0) ctx.toast(bad[0]?.message ?? "A voice could not be used.", true);
        else ctx.toast(`Made ${out.new_takes.length} take${out.new_takes.length === 1 ? "" : "s"}. Listen, then choose one.`);
      }
    } catch (e) { ctx.toast(messageOf(e), true); } finally { setBusy(false); }
  };

  const start = () => { if (cloud.length > 0) setAsk(true); else void go(false); };
  const voiceName = (t: Take) => ctx.voices.find((v) => v.id === t.voice_id)?.name ?? t.voice_id;
  const hidden = blind && !revealed;

  return (
    <div className="take-detail col" role="region" aria-label={`Try ${info.line.speaker}'s line with other voices`}>
      <div className="row" style={{ alignItems: "flex-start" }}>
        <div style={{ flex: 1, minWidth: 240 }}>
          <strong>Try this line with other voices</strong>
          <p className="small muted" style={{ margin: "2px 0 0" }}>Pick up to {AUDITION_CAP}. Each makes its own take. Your cast and chosen take stay as they are until you choose.</p>
        </div>
        <button className="small ghost" onClick={props.onClose}>Close</button>
      </div>

      <div className="row tight" style={{ alignItems: "end" }}>
        <div><label htmlFor={`aq-${key}`}>Search voices</label><input id={`aq-${key}`} type="text" value={q} onChange={(e) => setQ(e.target.value)} placeholder="Name or accent" /></div>
        <label className="inline"><input type="checkbox" checked={sameLang} onChange={(e) => setSameLang(e.target.checked)} /> Same language as {current?.name ?? "the cast voice"}</label>
        <label className="inline"><input type="checkbox" checked={blind} onChange={(e) => { setBlind(e.target.checked); setRevealed(false); }} /> Blind: hide voice names until I choose</label>
      </div>

      <div className="col" style={{ gap: 4, maxHeight: 220, overflow: "auto" }} role="group" aria-label="Voices to try">
        {choices.length === 0 && <p className="small muted">No other voices match.</p>}
        {choices.map((v) => (
          <label key={v.id} className="inline small" style={{ minHeight: 28 }}>
            <input type="checkbox" checked={picked.includes(v.id)} disabled={!picked.includes(v.id) && picked.length >= AUDITION_CAP} onChange={() => toggle(v.id)} />
            <span>{v.name} <span className="muted">{v.accent} · {v.provider_label}</span></span>
            {v.sends_data && <span className="chip gold">sends text</span>}
          </label>
        ))}
      </div>

      <div className="row">
        <button className="primary small" disabled={picked.length === 0 || busy} onClick={start}>
          {busy ? <><span className="spin" aria-hidden="true" /> Making takes</> : `Make ${picked.length} take${picked.length === 1 ? "" : "s"}`}
        </button>
        <span className="small muted">{picked.length} of {AUDITION_CAP} chosen{cloud.length > 0 ? `. ${cloud.length} will be sent over the internet.` : ". Nothing is sent."}</span>
      </div>

      {made.length > 0 && (
        <div className="col" role="list" aria-label="Audition takes">
          {made.map((t, i) => (
            <div key={t.id} className="linecard row" role="listitem" style={{ margin: 0, alignItems: "center" }}>
              <strong style={{ minWidth: 70 }}>{hidden ? `Take ${LETTERS[i] ?? i + 1}` : voiceName(t)}</strong>
              <PlayButton path={joinPath(ctx.dir, t.file)} label={hidden ? `Take ${LETTERS[i] ?? i + 1}` : voiceName(t)} bust={t.id} />
              <Waveform projectId={ctx.project.id} file={t.file} bins={48} height={28} dim={false} label={`Waveform of ${hidden ? `take ${LETTERS[i] ?? i + 1}` : voiceName(t)}`} />
              <button className="small" onClick={() => { ctx.edit((p) => ({ ...p, selected: { ...p.selected, [key]: t.id } })); setRevealed(true); ctx.toast(`Using ${voiceName(t)} for this line. Your cast is unchanged.`); }}>Use this take</button>
            </div>
          ))}
          {hidden && <button className="small" onClick={() => setRevealed(true)}>Reveal the voices</button>}
          <p className="small muted" style={{ margin: 0 }}>To use a voice for every line this character speaks, change it in the Cast step.</p>
        </div>
      )}

      {ask && (
        <Confirm title="Send this line to try other voices?" confirmLabel="Send and make takes" danger={false}
          body={<p>{chars} characters of dialogue will be sent over the internet to {[...new Set(cloud.map((v) => v.provider_label))].join(" and ")}, once for each of {cloud.length} voice{cloud.length === 1 ? "" : "s"}. It is billed to your account there, and Soundstage cannot show the price in advance.</p>}
          onCancel={() => setAsk(false)} onConfirm={() => void go(true)} />
      )}
    </div>
  );
}

/** When a line cannot be rendered as cast, suggest voices that can do what it needs. Choosing one is a separate,
 *  approved step, and the plan is left exactly as it was. */
export function FallbackList(props: { ctx: Ctx; info: LineInfo; onClose: () => void }) {
  const { ctx, info } = props;
  const key = info.line.key;
  const [options, setOptions] = useState<FallbackOption[] | null>(null);
  const [ask, setAsk] = useState<FallbackOption | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    api.fallbackOptions(ctx.project, key).then((o) => { if (live) setOptions(o); }).catch((e) => { if (live) { setOptions([]); ctx.toast(messageOf(e), true); } });
    return () => { live = false; };
  }, [key]); // eslint-disable-line react-hooks/exhaustive-deps

  const use = async (o: FallbackOption, confirmed: boolean) => {
    setAsk(null);
    setBusy(o.voice_id);
    try {
      const out = await ctx.audition(key, [o.voice_id], confirmed);
      const take = out?.new_takes[0] ?? (out?.results[0]?.result ? ctx.project.takes.find((t) => t.id === out.results[0]?.result?.take_id) : undefined);
      if (out && out.results.every((r) => r.ok) && take) {
        ctx.edit((p) => ({ ...p, selected: { ...p.selected, [key]: take.id } }));
        ctx.toast(`Used ${o.name} for this line. Your cast and plan are unchanged.`);
        props.onClose();
      } else if (out) ctx.toast(out.results.find((r) => !r.ok)?.message ?? "That voice could not be used.", true);
    } catch (e) { ctx.toast(messageOf(e), true); } finally { setBusy(null); }
  };

  const chars = info.line.text.replace(/\[[^\]]*\]/g, "").length;
  return (
    <div className="take-detail col" role="region" aria-label="Other voices that could do this line">
      <div className="row"><strong style={{ flex: 1 }}>Other voices that could do this line</strong><button className="small ghost" onClick={props.onClose}>Close</button></div>
      {options === null && <p className="small muted">Looking…</p>}
      {options !== null && options.length === 0 && <p className="small muted">No other voice here can do everything this line needs{ctx.project ? "" : ""}. You can remove the tag that is the problem, or add a voice engine.</p>}
      {(options ?? []).map((o) => (
        <div key={o.voice_id} className="linecard row" style={{ margin: 0, alignItems: "center" }}>
          <div style={{ flex: 1, minWidth: 200 }}>
            <strong>{o.name}</strong> <span className="muted small">{o.provider_label}</span>{" "}
            {o.sends_data ? <span className="chip gold">sends text, may be billed</span> : <span className="chip ok">free, on this computer</span>}
            {o.plan && o.plan.ignored.length > 0 && <div className="small muted">Would not use from the plan: {o.plan.ignored.join("; ")}.</div>}
          </div>
          <button className="small" disabled={busy !== null} onClick={() => (o.sends_data ? setAsk(o) : void use(o, false))}>{busy === o.voice_id ? <span className="spin" aria-hidden="true" /> : null}Use for this line</button>
        </div>
      ))}
      {ask && (
        <Confirm title={`Send this line to ${ask.provider_label}?`} confirmLabel="Send and use" danger={false}
          body={<p>{chars} characters of dialogue will be sent over the internet to {ask.provider_label} to be spoken by {ask.name}. It is billed to your account there, and Soundstage cannot show the price in advance. The character&apos;s cast voice and the plan do not change.</p>}
          onCancel={() => setAsk(null)} onConfirm={() => void use(ask, true)} />
      )}
    </div>
  );
}
