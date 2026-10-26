# Progress

Last updated 2026-10-25.

## Where the project stands

| Area | State |
|---|---|
| App v0.1 and v0.2 (everything in `CHANGELOG.md`) | Built, tested, pushed. Last code commit `f5ff35f` |
| Tests now | 304 Rust, 84 front-end, 225 of 225 real-window checks, types clean |
| PRD-002 (AI models update) | MS0 spec approved in part (see `DECISIONS.md`). **MS1 Foundations built 2026-10-25**, awaiting the owner's demo and sign-off |
| Next implementation milestone | MS1 Foundations: D-01, D-02, D-03, D-04, D-11, D-13 approved; awaiting the detailed MS1 task plan and the owner go-ahead |

## Milestones

| MS | Name | State |
|---|---|---|
| MS0 | Spec | Done |
| MS1 | Foundations | Built and tested; awaiting owner demo and sign-off |
| MS2 | Director v1 | First slice built and tested with fake servers; needs the owner to try it on a real model (Ollama or another chat server) and review plan quality; PDF import waits for approval |
| MS3 | Performance v1 | MS3a built (plan to render, sounds, filters, audition of 4, fallback); awaiting listening runs and owner demo. MS3b (hybrid events, reference) after listening |
| MS4 | Model Lab v1 | Not started |
| MS5 | Local model packs | Not started |
| MS6 | Sound design | Not started |
| MS7 | Timeline v2 | Not started |
| MS8 | Video import and sync | Not started, needs D-07 |
| MS9 | Video generation | Not scheduled |

Traceability after MS1: 50 Existing, 69 Partial, 52 New of 171 rows (MS0: 46, 68, 57) (`docs/spec/02-TRACEABILITY.md`).

## Waiting on the owner

1. Approve or amend the Proposed rows in `DECISIONS.md`, first D-03, D-06, D-07.
2. Live tests with real accounts: ElevenLabs (render, music, design), Kie, fal, OpenRouter, ComfyUI, Piper, Ollama. Record in `docs/PROVIDERS.md`.
3. Download test of a Whisper model (75 MB or more).
4. Open the OTIO export in DaVinci Resolve.
5. Install on a clean PC.
6. A code-signing certificate (installer is unsigned).
7. Legal review of `PRIVACY.md` and `docs/SAFETY_LICENSING.md`; support address in the privacy notice.
8. Listening tests (nothing about accent or tag quality is claimed yet).
9. Decide whether to rewrite the public git author email.

## Known limits

- GitHub Actions may refuse to start because of an account billing setting, not the code. Local gates in `docs/QA.md` are authoritative.
- Positional line ids (C-06) mean inserting a line can mark later lines stale.
- Third-party model facts in `docs/spec/04` and `05` are unverified (VERIFY).

## Log

- 2026-10-25: MS0 spec written: PRD-002 stored, conflicts (27), traceability (171 rows), architecture, providers, capability and hardware matrices, milestones and gates, listening protocol, decisions. No code or dependency changes.
- 2026-10-24: v0.2 work finished and pushed (project backup, e2e fixes, CI).
- 2026-10-23: Full QA run recorded in `docs/QA.md`.
