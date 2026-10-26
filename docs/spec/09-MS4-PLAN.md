# 09. MS4 Model Lab v1: detailed plan

Status: **Proposed, planning only. No code until the owner approves it.** Written 2026-10-25. Expands section 7 of `06-MILESTONES-AND-TESTS.md`. Follows the listening method in `07-LISTENING-TEST-PROTOCOL.md`.

**Status 2026-10-25: D-21 to D-25 approved. T-4.1 built** (test-set format, loader, checksum, 13 English scenes in `fixtures/lab/english-v1/`, 314 Rust tests). Owner answers: English only at first; no in-app blind player in version 1 (folder-and-spreadsheet method, score import in T-4.5); first dry run compares ElevenLabs with the Windows voices as a free baseline (assumed from "ElevenLabs"; change if you want a second paid provider). The code-switching scene is marked as needing native review and is left out of general results.

## 1. Goal

Replace opinion with evidence. The Model Lab runs the **same short scripts** through **several voices and providers**, lets **blind listeners** score them, and turns the scores into **a report and a status** for each voice, language, accent and sound. Soundstage then shows the status honestly: `unverified` until a run exists, `experimental` when there is a run that did not pass, `verified` only when the gates in the protocol are met and the owner approves.

## 2. What already exists

- **Listening pack** (shipped): loudness-matched, randomly coded, per-listener order, score sheets, private key. This is the blind-package half of the Lab, made by hand from clips the owner chooses.
- **Registry** (`capability.rs`, `data/models.json`): every model has a `status`, and languages and accents can carry `evaluation` plus `evidence`. The loader already refuses `verified` without evidence.
- **Takes remember what made them** (voice, provider version, direction), so a clip can be traced back to its settings.
- **Adapters, consent and licence gates, local-only mode, job records**, all reusable.
- **Not built:** the test sets, a headless runner, score import, a report, and the promotion step. `tools/bakeoff` does not exist.

## 3. Proposed decisions (D-21 to D-25)

| ID | Question | Proposal |
|---|---|---|
| D-21 | Where does Lab data live? | In its own folder, `Documents\Soundstage\.lab\<run id>\`: run record, test set copy, generated clips, key, imported scores, report. It is never inside a project, never sent anywhere, and can be deleted without affecting any project. |
| D-22 | Who may promote a status? | Only the owner, in the Lab, after reading the report. The app computes whether the gates are met and shows the numbers; it never promotes by itself, and an automatic score (such as a speech-recognition check) can never promote (LAB-008, D-09). |
| D-23 | Spending in a Lab run | A run lists every paid call it will make (provider, number of clips, characters) and needs one approval for that list, plus a per-run cap the owner sets. Local voices cost nothing. |
| D-24 | Test content | Scripts are short (10 to 25 seconds), original (no copyrighted text), reviewed by a native speaker for any non-English language, and stored with the repo under `fixtures/lab/` with a checksum. No real person's words, voice or likeness. |
| D-25 | Reviewers | Minimums from the protocol: 5 listeners for naturalness and events, 2 native speakers per accent or language for any accent or language status. Reviewers are identified only by a code. |

## 4. The 13 test scenarios (PRD section 13)

Written once, reused for every run. Each has a **reference direction** (what the line is meant to sound like) and a **checklist of what a listener should notice**. They cover: neutral narration, a calm conversation, an argument, grief, joy, whispering, shouting, a laugh in a line, a cough or interruption, a gasp, a long sentence with a name to pronounce, a code-switched line, and a multi-speaker scene. Native-language versions are added only where the owner can supply native reviewers.

## 5. Tasks in dependency order

**T-4.1 Test set format and scripts (M)**
- A test set is a folder: `set.json` (id, title, language, accent claim if any, scenes) and one script per scene, plus the reference direction and checklist. A checksum covers the folder.
- Write the 13 English scenes and check them for length, originality and safety rules. Native-language sets are added later per language.
- Tests: every set loads and validates; length limits hold; the safety check passes on every script; a changed file breaks the checksum.

**T-4.2 Run record and runner (L)**
- A **run** is: a test set, a list of candidates (voice plus provider plus settings), the plan option (with or without the Director plan), and an approval record. The runner renders each scene with each candidate through the normal render path and stores the clips, the settings and the time taken.
- Runs as a job (existing job records), resumable after a crash, stoppable.
- Consent and licence gates apply: a cloned voice needs its consent record; a voice with an unknown licence is allowed in the Lab (it is not a commercial export) but its licence state is recorded.
- Tests: the runner makes exactly the clips listed and no others; a failed candidate is recorded and does not stop the rest; resume after an interruption does not re-render finished clips; local-only runs make no network call.

**T-4.3 Cost and latency capture (S)**
- Record per clip: characters sent, time to first audio and total time, and the provider's price when it publishes one. Show a run estimate before approval and the actual after.
- Tests: estimate shown before any send; cap enforced; totals match the clips.

**T-4.4 Blind package (S, mostly done)**
- Reuse the listening pack: the Lab feeds it the run's clips and labels, and keeps the key inside the run folder.
- Tests: as the existing pack tests, plus the key links back to the run's candidate, scene and settings.

**T-4.5 Score entry and import (M)**
- Two ways in: paste or import the listeners' `score-sheet.csv` files, or enter scores in the app (a blind player that plays a clip and shows the scales). Both end up in the same table.
- Validation: scores in range, codes known, one row per clip per listener, no duplicate listeners; bad rows are reported with the line, not silently dropped. Formula-like text is neutralised.
- Tests: good and bad sheets, duplicates, out-of-range, wrong code, spreadsheet-formula cells, listeners who skipped clips.

**T-4.6 Report and gates (M)**
- For each candidate: median naturalness and emotion, share of low scores, spoken-tag words, voice consistency, intelligibility, preference against the reference, number and kind of reviewers, latency and cost.
- Compare with the protocol gates and show **met / not met / not enough reviewers** for each, with the numbers. Export as CSV and Markdown.
- Tests: the maths against hand-worked examples; each gate at its boundary; fewer reviewers than the minimum says "not enough", never "met".

**T-4.7 Status promotion (S)**
- The owner presses "Record as verified" for a candidate whose gates are met. This writes the evidence (run id, report hash, date, reviewer counts) into a local overrides file that the registry reads, and updates the status shown in the Voice library and in `plan_support`.
- Verified can be withdrawn. Anything changed in the provider version (a new model version) returns it to `experimental`, because a result belongs to one version.
- Tests: promotion refused unless the gates are met; evidence recorded; a new provider version demotes; the shipped registry file is never edited by the app.

**T-4.8 Model Lab screen (L)**
- Steps in the Golden Path style: choose a test set, choose candidates, review cost and approve, run, make the blind pack, import scores, read the report, decide.
- Empty, running, error, interrupted and finished states; keyboard and screen reader complete; clear plain words throughout.
- Tests: real-window run of the whole flow with a fake provider; accessibility audit on every state.

## 6. Order and effort

| Step | Tasks | Effort |
|---|---|---|
| 1 | T-4.1 | 1 week (most of it is writing and reviewing scripts) |
| 2 | T-4.2, T-4.3 | 1 week |
| 3 | T-4.5, T-4.6 | 1 week |
| 4 | T-4.7, T-4.8 | 1 week |
| 5 | A full dry run with the owner and at least one other listener on two providers | 2 to 3 days |

About 4 weeks, and the owner's own time for the listening sessions is on top of that.

## 7. Exit gate

- A full dry run: two providers, three scenes, six listeners, score import, report, and an owner decision.
- A status moves from `unverified` to `experimental` correctly, and to `verified` only in a test that meets the gates; the app then shows the evidence.
- AC-005 holds: language and accent choice show their evaluation state.
- Real-window run covers the flow; accessibility clean; no Lab data inside any project; a deleted run leaves projects untouched.

## 8. Risks

| Risk | Mitigation |
|---|---|
| Too few listeners, so results are noise | The report says how many and refuses to call it "met" below the minimum |
| Native speakers are hard to find | Statuses for accents and languages stay `unverified`; the Lab still works for English |
| Paid runs cost more than expected | Estimate, cap and per-run approval (D-23) |
| Providers change a model silently | A result belongs to one provider version; a new version demotes |
| Listeners are influenced by names | Blind packages only; the key stays in the owner's run folder |
| Writing 13 good scenes takes longer than the code | Start with 5 and add the rest as runs need them |
| Scores misread as a quality claim in the app | The Voice library shows the state and the date, never "best" or "supports" |

## 9. Questions for the owner

1. Approve D-21 to D-25, or change them?
2. Start with the English set only and add other languages as native reviewers become available?
3. Do you want the in-app blind player? (Answered: owner deferred; decision is no, not in version 1.)
4. Which two providers should the first dry run compare? (For example one local voice and ElevenLabs.)
