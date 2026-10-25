# Progress

Last updated 2026-10-25.

## Where the project stands

| Area | State |
|---|---|
| App v0.1 and v0.2 (everything in `CHANGELOG.md`) | Built, tested, pushed. Last code commit `f5ff35f` |
| Tests at that commit | 233 Rust, 82 front-end, 181 of 181 real-window checks, types clean, 0 audit findings |
| PRD-002 (AI models update) | **MS0 spec written, awaiting owner approval.** No code written for it |
| Next implementation milestone | MS1 Foundations, blocked on D-03 |

## Milestones

| MS | Name | State |
|---|---|---|
| MS0 | Spec | Done, pending owner review |
| MS1 | Foundations | Not started, needs D-03, D-04, D-13 |
| MS2 | Director v1 | Not started, needs D-06 and a host choice |
| MS3 | Performance v1 | Not started |
| MS4 | Model Lab v1 | Not started |
| MS5 | Local model packs | Not started |
| MS6 | Sound design | Not started |
| MS7 | Timeline v2 | Not started |
| MS8 | Video import and sync | Not started, needs D-07 |
| MS9 | Video generation | Not scheduled |

Traceability at MS0: 46 Existing, 68 Partial, 57 New of 171 rows (`docs/spec/02-TRACEABILITY.md`).

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
