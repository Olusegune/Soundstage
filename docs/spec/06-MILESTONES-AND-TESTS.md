# 06. Milestones, tasks, acceptance tests and exit gates

Status: **Proposed.** Written 2026-10-25. No milestone after MS0 starts until the owner approves it in `DECISIONS.md`. Each milestone is a vertical slice that can be run, tested and demonstrated alone (Golden Path).

## 1. Vocabulary mapping (C-02)

| This plan | PRD-002 phase | Master PRD |
|---|---|---|
| MS0 Spec | - | - |
| MS1 Foundations | Phase 0 and 1 | v1 hardening |
| MS2 Director v1 | Phase 2 | v2 |
| MS3 Performance v1 | Phase 3 and 4 | v2 |
| MS4 Model Lab v1 | Phase 5 | v2 |
| MS5 Local model packs | Phase 6 | v2 |
| MS6 Sound design | Phase 7 | v2 |
| MS7 Timeline v2 | Phase 8 | v3 |
| MS8 Video import and sync | Phase 9 | v3 |
| MS9 Video generation | Phase 10 | v3, gated |

## 2. Rules for every milestone

- **Entry:** the previous exit gate passed, the owner approved the milestone, new dependencies and hosts approved (D-11).
- **Definition of done for a task:** code, unit test, contract test where it touches an adapter, docs updated, no new warnings in type check or clippy errors, no secrets, and (when it touches the security surface) a `rafter-code-review` pass and `rafter secrets .`.
- **Exit gate (all milestones):** `cargo test --workspace`, `npx vitest run`, `npx tsc --noEmit` clean; the real-window run `node tools/e2e/run.mjs` all green with new checks for the milestone; axe-core clean on new screens; projects from the previous version open and migrate; `PROGRESS.md` and traceability updated; the owner accepts a demo.
- Sizes: S under a day, M one to three days, L three to seven days, XL over a week. They are estimates for one developer with this assistant, not promises.

## 3. MS0. Spec (this step)

| Task | Output | Done |
|---|---|---|
| T-0.1 Conflicts list | `01-CONFLICTS.md` | yes |
| T-0.2 Traceability | `02-TRACEABILITY.md` | yes |
| T-0.3 Architecture, providers, capability, hardware, listening protocol | `03` to `05`, `07` | yes |
| T-0.4 Verify third-party facts marked VERIFY (v4, Stable Audio licence, Gemini/GPT terms) against official pages | evidence table in `05` | after approval |
| T-0.5 Milestones and gates | this file | yes |
| T-0.6 `DECISIONS.md`, `PROGRESS.md`, `AGENTS.md` | root files | yes |

**Exit gate:** the owner approves or amends the decisions marked Proposed, in particular C-06, C-16 and C-01. Nothing else.

## 4. MS1. Foundations (about 3 weeks)

Goal: remove the weaknesses every later feature leans on.

| Task | Depends on | Size | Test |
|---|---|---|---|
| T-1.1 Stable element ids, sidecar, matcher, migration schema 1 to 2 | approval of D-03 | L | unit: insert, delete, reorder, edit lines keep takes and directions; fixture project migrates; older app opens read-only |
| T-1.2 Move cache keys and stale marks to ids | T-1.1 | M | insert a line above a rendered line: nothing else goes stale (AC-006) |
| T-1.3 PerformancePlan schema, validation, migration hook, empty plan | T-1.1 | M | schema round trip, unknown fields kept, bad document rejected, old plan kept |
| T-1.4 Job record, queue, six states, cancel, interrupted recovery | none | L | kill the process mid-render, reopen: job is `interrupted`, project intact, resume works (AC-011) |
| T-1.5 Registry file, loader, status states, structured licence conditions, migrate current tag knowledge | T-1.3 | L | contract suite reads capabilities from the registry; export gate honours a `revenue_cap` condition |
| T-1.6 Hardware detection and profile verdicts (needs dependency approval) | D-11 | M | fake probe fixtures for each profile; a real run on the owner's PC recorded |
| T-1.7 Global "local only" switch | T-1.5 | S | with it on, every cloud action is disabled with a reason |
| T-1.8 Update `ARCHITECTURE.md` banner and project docs | none | S | links resolve |

**Status 2026-10-25: built, awaiting the full real-window run and the owner's demo.** Deviations from the plan above:
- T-1.1: ids kept in `project.json` (`line_ids`), not a sidecar file; lines only, scenes follow later; no user-confirmation screen for uncertain matches yet.
- T-1.3: the plan schema, validation and `plan.json` store exist (`plan.rs`); nothing writes a plan until MS2.
- T-1.4: job records cover render batches, with "Stop after this line" and recovery of finished takes after an interruption. A general queue with per-resource limits is not built; it is not needed until MS5.
- T-1.5: model registry file, loader and the no-verified-without-evidence rule exist (`capability.rs`, `data/models.json`); structured licence conditions are enforced at export and editable. Moving each adapter's tag knowledge into the registry is deferred to MS3.
- T-1.6: hardware detection with no new crate (Windows memory and disk calls through the already-used `windows-sys` crate with two more feature flags, and `nvidia-smi` from its fixed install paths). Shown in the System check. Measuring the thresholds on real machines is still to do.
- T-1.7: local-only mode refuses all non-loopback network use, including model downloads.

**Exit gate:** AC-006 and AC-011 pass automatically; migration test on fixture projects; hardware verdict shown in System check; the real-window run covers insert-line, kill-and-reopen, and local-only.

## 5. MS2. Director v1 (about 3 weeks)

Needs C-16 decided and the Director host approved.

| Task | Depends on | Size | Test |
|---|---|---|---|
| T-2.1 Parser: action lines and stage directions; Fountain elements | T-1.1 | M | fixtures for scenes with actions; original text unchanged |
| T-2.2 Text-based PDF import with image-only detection (needs dependency approval) | D-11 | M | text PDF imports; scanned PDF gives a plain message |
| T-2.3 `Planner` trait, JSON schema, validator, prompt-injection fence, rule-based fallback | T-1.3 | L | adversarial script fixtures ("ignore previous instructions", fake JSON, huge output, control characters) never change behaviour or break validation |
| T-2.4 Consent flow and cost preview for the Director | T-2.3 | M | nothing sent before confirmation; local-only blocks cloud |
| T-2.5 Plan UI: per-line view, multi-dimensional emotion, confidence, alternatives, lock | T-2.3 | L | edit, lock, re-run keeps locked fields (DIR-007); axe clean |
| T-2.6 Diff and accept on re-run | T-2.5 | M | re-run proposes a diff; nothing applied without accept |

**Exit gate:** AC-001 passes on three fixture scripts; the injection suite passes; privacy notice updated; no script text in any log; the owner reviews real plans from a real model on their own scripts and accepts the quality as a draft aid (not a claim).

**MS2 status 2026-10-25: first slice built (208 of 208 real-window checks).** Done: T-2.1 action lines (no Fountain elements yet), T-2.3 Planner with fence, schema checks and rules fallback (tested against fake servers and hostile replies), T-2.4 consent and cost screen, T-2.5/2.6 plan panel with confidence, hold-back emotion, locks, accept per line. Not done: text PDF import (T-2.2, needs your approval of a PDF library), alternatives UI, multi-dimensional emotion editing beyond the main feeling, a real-model quality review by the owner (the exit gate). Host decision D-06 was applied as the existing Settings chat server only.

## 6. MS3. Performance v1 (about 4 weeks)

| Task | Depends on | Size | Test |
|---|---|---|---|
| T-3.1 Plan to provider controls, `plan_support` reports, drop reports | T-1.5, T-2.5 | L | each adapter reports what it honours; unsupported shown before render (PHY-005) |
| T-3.2 Vocal events model and placement UI for provider-native tags | T-3.1 | L | event tags never spoken across all adapters (PHY-001, AC-003) |
| T-3.3 Hybrid events: generated or library event audio mixed under a speech take | T-3.2, T-1.4 | XL | identity preserved; event timing within tolerance; assets stored separately |
| T-3.4 Voice profile object (accent, style, pronunciation) and filters (gender, style, provider) | T-1.5 | M | filter tests; verification state visible |
| T-3.5 Take audition: several providers on one line, blind toggle optional | T-3.1 | L | side by side, keeps script (AC-002) |
| T-3.6 Fallback alternatives flow with cost and data notice | T-3.1 | M | no silent charge; plan preserved (AC-009) |
| T-3.7 Performance reference tied to a line | T-3.1 | M | consent gate still enforced |

**Exit gate:** AC-002, AC-003, AC-004, AC-009 pass; first listening run on events (protocol 07) reported honestly; owner demo.

## 7. MS4. Model Lab v1 (about 3 weeks)

| Task | Size | Test |
|---|---|---|
| T-4.1 `tools/bakeoff` runner scaffold (C-27) | M | runs a fixture set against the fake provider |
| T-4.2 Write the 13 scenario scripts with native-speaker review | L | files exist, reviewed, checksummed |
| T-4.3 Lab run record: settings, outputs, latency, cost | M | record round trip |
| T-4.4 Blind package export and blind player; score entry | L | clip names hide models; order randomised per reviewer |
| T-4.5 Report builder (CSV, Markdown) and status promotion with owner approval | M | numbers match hand calculation |

**Exit gate:** a full dry run with the owner and at least one other listener on two providers; a status moves from `unverified` to `experimental` correctly; AC-005 shows evaluation state.

## 8. MS5. Local model packs (about 4 weeks)

Requires per-pack owner approval (D-11), a hardware verdict, and the pickle ban.

| Task | Size | Test |
|---|---|---|
| T-5.1 Worker protocol, process manager, cancel, crash handling | L | protocol fuzz tests; killed worker leaves project intact (HW-007) |
| T-5.2 Model manager UI: list, sizes, licence, disk check, verified download, remove | L | pinned size and hash; remove frees space; offline works |
| T-5.3 Pack 1: one TTS (Chatterbox or Qwen3-TTS, chosen after T-0.4 and owner choice) | XL | contract suite passes; local run sends nothing (AC-008); Lab run |
| T-5.4 Pack 2: ACE-Step music | L | duration limits; contract suite |
| T-5.5 Worker sandbox rules and output validation | M | wrong format or length rejected |
| T-5.6 Hardware gating wired to downloads and runs | M | "will not run" blocks start (HW-005) |

**Exit gate:** AC-008; installer size unchanged by packs; clean-PC install and pack download tested on a second machine.

## 9. MS6. Sound design (about 3 weeks)

| Task | Size | Test |
|---|---|---|
| T-6.1 Cue extraction from action lines and plan | M | fixtures; user can edit every cue |
| T-6.2 DSP chain: pan, EQ, reverb, distance, occlusion | L | per-effect unit tests against reference signals |
| T-6.3 Room profiles and per-category layers | M | layers independent (AC-007) |
| T-6.4 Ambience continuity across scenes and crossfades | M | no click at joins; level continuity test |
| T-6.5 Music cue plan from beats | M | plan to prompts, editable |

**Exit gate:** AC-007; stems still sum to the master within the existing tolerance; loudness tests unchanged.

## 10. MS7. Timeline v2 (about 4 weeks)

| Task | Size | Test |
|---|---|---|
| T-7.1 `timeline.json`, clip model, derive-from-script | L | round trip; unedited projects export identically |
| T-7.2 Editing: move, trim, split, fade, gain, mute; undo and redo | XL | property tests on edits; undo returns to byte-identical state |
| T-7.3 Re-layout preview that never overwrites edits | M | diff shown |
| T-7.4 Export from timeline; per-character and per-category stems | L | sum test; naming scheme (AC-010) |
| T-7.5 Event timeline for vocal events (drag) | M | nudge test |

**Exit gate:** AC-010 reviewed on a long project; performance with 500 clips acceptable on Local-standard.

## 11. MS8. Video import and sync (about 3 weeks)

| Task | Size | Test |
|---|---|---|
| T-8.1 Import and probe with FFmpeg, proxy preview | M | formats, bad files |
| T-8.2 Player pinned to timeline, scrubbing | L | seek accuracy |
| T-8.3 Frame-rate-aware timecode, cue pinning, nudge | M | 23.976, 24, 25, 29.97, 30 fps maths |
| T-8.4 Export audio aligned to video; re-mux option | M | drift under tolerance (AC-012; tolerance set by owner, proposal one frame) |

**Exit gate:** AC-012; clean handling of missing codecs.

## 12. MS9. Video generation (not scheduled)

Blocked on C-01 and C-17. When unblocked it is planned as its own spec: safety document first, then one pack, then a Lab run, then UI. No tasks are estimated here.

## 13. Acceptance test map

| AC | Milestone | Automated check | Human check |
|---|---|---|---|
| AC-001 | MS2 | plan produced from fixtures, validated | owner reads plans |
| AC-002 | MS3 | takes from two adapters kept, script unchanged | owner audition |
| AC-003 | MS3 | tags never in spoken text, all adapters | listening on events |
| AC-004 | MS3 | pipeline output files and timing | listening |
| AC-005 | MS4 | state shown from registry | Lab reviewers |
| AC-006 | MS1 | insert-line test | - |
| AC-007 | MS6 | layers independent, stems sum | listening |
| AC-008 | MS5 | no network during local run (blocked by firewall rule in test) | - |
| AC-009 | MS3 | no call without approval | - |
| AC-010 | MS7 | names, durations, sum | open in an editor |
| AC-011 | MS1 | kill-and-reopen | - |
| AC-012 | MS8 | drift measurement | owner check on real video |

## 14. Non-goals for this release

Cloud accounts and sync; mobile; multi-user collaboration; a plugin marketplace; training or fine-tuning models in the app; real-person voice or likeness without the consent gate; any quality claim without a Lab run; Mac and Linux.

## 15. Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Stable-id matching mis-assigns takes after big edits | medium | high | uncertain matches ask the user; orphans kept, never deleted; migration backup |
| Director output is poor or unsafe | medium | medium | draft aid only; validation; user locks; rule-based fallback |
| Local models do not fit users' hardware | high | medium | verdicts before download; cloud-first stays complete |
| Model licences change or are misread | medium | high | structured conditions, checked-on dates, owner reviews each pack |
| Installer or pack size grows | medium | medium | optional packs, pinned downloads, remove button |
| Python or native runtimes add attack surface | medium | high | no pickle, sandboxed folder, no keys to workers, per-pack approval |
| Listening tests are too few to be meaningful | high | medium | report counts, keep `experimental` |
| Scope too large for the schedule | high | medium | milestones are independent; MS8 and MS9 can be dropped |
| CI blocked by GitHub billing | known | low | local gates are authoritative |
| Third-party facts are wrong or outdated | medium | medium | VERIFY marks, dates, evidence table |

## 16. Rough total

MS1 to MS8 are about 27 weeks of focused work by estimate, with MS5 and MS8 being the most uncertain. MS9 is not estimated. The first demonstrable improvement for the owner arrives at the end of MS1 (safe inserts, resumable jobs) and the first visible AI feature at the end of MS2.
