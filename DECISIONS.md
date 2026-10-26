# Decisions

One line per decision, newest group last. **Approved** means the owner said yes in chat (date given). **Proposed** means written down for the owner and not yet approved; nothing may be built on it. Spec files: `docs/spec/`.

## A. Already approved (history)

| ID | Decision | State |
|---|---|---|
| H-01 | Windows desktop app, Tauri 2 + React + Rust; local-first; no telemetry | Approved (project start) |
| H-02 | ElevenLabs allowed as a paid cloud provider and pinned host | Approved |
| H-03 | kie.ai and fal.ai hosts allowed for music and effects | Approved |
| H-04 | VoiceKiller allowed as a provider and host | Approved |
| H-05 | Bundle FFmpeg (LGPL build) with the installer | Approved |
| H-06 | Bundle whisper.cpp; speech models download on request, pinned by size and SHA-256 | Approved ("yes i do") |
| H-07 | Keys in Windows Credential Manager; consent records encrypted with DPAPI | Approved |
| H-08 | Public repository `Olusegune/Soundstage`; commits and pushes allowed | Approved |
| H-09 | OTIO export with per-take files | Approved |

## B. Proposed by the PRD-002 spec step (2026-10-25), awaiting the owner

| ID | Decision | Resolves | Needs before | State |
|---|---|---|---|---|
| D-01 | Adopt `MS0` to `MS9` as the milestone vocabulary, mapped to PRD-002 phases and master v1/v2/v3 | C-02 | MS1 | Approved 2026-10-25 (owner) |
| D-02 | The `dev_spec` is `docs/ARCHITECTURE.md` plus `docs/spec/03-ARCHITECTURE-UPDATE.md`; `ARCHITECTURE.md` gets a stale banner | C-10, C-11 | MS1 | Approved 2026-10-25 (owner) |
| D-03 | **Stable element ids** with a sidecar file and project schema 2 migration, before any new feature | C-06 | **MS1 (blocking)** | Approved 2026-10-25 (owner) |
| D-04 | Job records and six states for all long operations; interrupted jobs recoverable | C-07 | MS1 | Approved 2026-10-25 (owner) |
| D-05 | Keep plain CSS design tokens; do not adopt Tailwind | C-05 | MS2 | Proposed |
| D-06 | Director safety design (consent, fenced untrusted text, validated JSON, no agency, rule-based fallback). The Director host(s) to approve are the owner's choice | C-16 | **MS2 (blocking)** | Proposed |
| D-07 | Video: import, preview and sync (MS8) are in scope; generation and lip-sync (MS9) are out until the owner decides and a likeness safety document exists | C-01, C-17 | **MS8 and MS9 (blocking)** | Proposed |
| D-08 | The 12 PRD workspaces map onto the existing six-step Golden Path; new ones appear only when their milestone ships | C-04 | MS2 | Proposed |
| D-09 | A language, accent or capability is `verified` only after a Model Lab run meets `docs/spec/07-LISTENING-TEST-PROTOCOL.md`; automated scores never promote | C-13, C-15, C-19 | MS4 | Proposed |
| D-10 | Local model packs accept safetensors and GGUF only (no pickle); every file pinned by size and SHA-256 | C-26 | MS5 | Proposed |
| D-11 | Every new dependency, runtime, model pack and network host needs the owner's explicit yes at its milestone | C-08, C-22 | each MS | Approved 2026-10-25 (owner) |
| D-12 | NVIDIA with CUDA first for local models; other GPUs fall back with an honest verdict | C-09 | MS5 | Proposed |
| D-13 | Licence conditions (revenue cap, territory, attribution) are structured and enforced at export | C-18 | MS1 | Approved 2026-10-25 (owner) |
| D-14 | Keep `eleven_v3` as default; add v4 only after official-page verification, as an explicit choice | C-21 | MS3 | Proposed |
| D-15 | Product name stays Soundstage; PRD working titles are not user-facing | C-14 | - | Proposed |
| D-16 | Use `tools/bakeoff` and the Lab for evaluation; scoring method from `TEST_PLAN` section 4.3 | C-13, C-27 | MS4 | Proposed |

## C. How decisions change

Change a row by editing this file, with the date and who decided. A rejected proposal stays in the table marked Rejected with the reason, so it is not proposed again by accident.
