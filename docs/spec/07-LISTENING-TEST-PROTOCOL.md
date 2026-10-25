# 07. Listening test protocol

Status: **Proposed.** Written 2026-10-25. Builds on `docs/TEST_PLAN.md` section 4 (the existing method) and adds what PRD-002 needs. Soundstage does not claim a quality, accent or language result without one of these runs behind it.

## 1. Principles

- Humans decide. Automated scores (for example speech-recognition word error rate) may rank candidates for listening or catch breakage. They never set a `verified` status.
- Blind and randomised: reviewers do not see the provider or model, and clip order is shuffled per reviewer.
- Same text, same direction, same level (loudness matched to -23 LUFS before playback) for every candidate.
- Everything recorded: script, settings, model version, seed if any, reviewer code, score, comment, date. No personal data beyond a code.
- A result belongs to one model version. A new version needs a new run.

## 2. Test sets

Written in T-4.2 and stored in `fixtures/lab/`. 13 scenarios from PRD section 13, each as a short script (10 to 30 seconds of speech) with a reference direction. Native-language scripts are written or checked by a native speaker, not machine-translated.

## 3. Reviewers

| Check | Minimum reviewers | Who |
|---|---|---|
| Accent and language naturalness | 2 native speakers per language or accent | recruited by the owner |
| Emotion and performance (MOS) | 5 blind listeners | any, with 1 per run being a native speaker for non-English |
| Pairwise preference (A vs B) | 5 blind listeners, 20 pairs minimum | any |
| Non-verbal events (laugh, cough, gasp) | 5 listeners | any |
| Intelligibility of text | 3 listeners write what they hear | any |

If fewer reviewers are available, the status stays `experimental` and the report says how many took part.

## 4. Scales

- **Naturalness / performance (MOS):** 1 to 5, anchored: 1 unusable, 3 acceptable for a draft, 5 indistinguishable from a good human take.
- **Accent fit:** 1 to 5 plus a flag "wrong accent" (counts against).
- **Event believability:** 1 to 5 plus "spoken instead of performed" (automatic fail for PHY-001).
- **Identity consistency:** same voice before, during and after an event, yes/no.

## 5. Gates (what moves a status)

A candidate becomes `verified` for a task and language when all hold:
1. Median MOS at least 4 (TEST_PLAN 4.3), and no listener below 3 on more than 10% of clips.
2. For accents: at least 2 native reviewers each rate fit at 4 or more, with no "wrong accent" flag on more than 10% of clips.
3. For events: believability median at least 3.5, zero "spoken instead of performed", identity consistent on at least 90% of clips.
4. For intelligibility: transcribed word accuracy at least 95% by listeners.
5. For realism comparisons: at least 5 listeners and the candidate is preferred or tied in 50% or more of pairs against the reference.
6. Results reproduced on a second day with the same settings (guards against a lucky run).

Not meeting a gate leaves the status `experimental`, with the numbers shown in the registry. Failing PHY-001 or identity gates marks that event `unsupported` for that model.

## 6. Process

1. Pick the set and candidates; the Lab renders every clip and records settings (T-4.3).
2. Export the blind package: clips named by code, a score sheet, no model names.
3. Reviewers score in the blind player or the sheet (T-4.4).
4. The Lab joins scores to settings, computes the numbers, and writes a report (CSV and Markdown) (T-4.5).
5. The owner reads the report and approves the status change. The change and report id go into the registry `evidence` list.

## 7. Reporting honesty rules

- Say how many reviewers, in which languages, and on what date.
- Say what was not tested.
- Never describe an `experimental` result as support in the UI, docs or marketing (C-15).
