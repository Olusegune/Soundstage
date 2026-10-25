# 01. Conflicts between specifications and the code

Status: **Proposed, for owner review.** Written 2026-10-25 as step M0 of the plan in `docs/spec/README.md`.

Compared: `CLAUDE.md`, `docs/PRD.md` (master), `docs/ARCHITECTURE.md`, `docs/PROVIDERS.md`, `docs/SAFETY_LICENSING.md`, `docs/SOUND_DESIGN.md`, `docs/TEST_PLAN.md`, `docs/UX_GOLDEN_PATH.md`, the update `docs/PRD_AI_MODELS_UPDATE.md` ("PRD-002"), and the code at commit `f5ff35f` (v0.2.0).

Each conflict has an ID, what clashes, the evidence, a **proposed resolution**, and who decides. "Blocking" means the milestone it affects should not start until the owner chooses.

Legend for kind: **S/S** spec against spec, **S/C** spec against code, **C/C** the written docs no longer describe the code.

---

## A. Scope and phasing

### C-01 (S/S, blocking M8 and M9) Video generation and lip-sync
- Master PRD section 4 (binding non-goals for v1): "Lip-sync dubbing of video (possible later; out of scope for v1)" and "Full video editing".
- PRD-002 P2: local video generation, automated shot planning, advanced lip-sync. Section 12 adds a Video Workspace.
- **Resolution:** Keep picture **preview and sync** (PRD-002 section 12.1, P1) as in scope, because the master PRD already allows "a low-resolution picture reference". Treat **video generation and lip-sync** as a separate, later decision (M9), behind a new safety document (likeness and consent). Do not build either before P0 and P1 pass their gates.
- **Decides:** owner.

### C-02 (S/S) Phase vocabulary
- Master PRD: v1, v2, v3 (section 4a) and milestones M0 to M6 (section 10).
- PRD-002: P0, P1, P2 and Phases 0 to 7 (section 20).
- The two sets use the same names for different things (for example master "M5" is consent and cloning; PRD-002 "Phase 5" is the timeline).
- **Resolution:** Adopt the milestone names in `docs/spec/06-MILESTONES-AND-TESTS.md` (`MS0` to `MS9`, "MS" for "milestone, spec set 2") and keep a mapping table to both older vocabularies. The master PRD's v1/v2/v3 labels stay as product-release labels.
- **Decides:** owner.

### C-03 (S/C) The code is ahead of the master PRD's phasing
The master PRD defers these to v2 or v3, and v0.2.0 already ships them:

| Feature | Master PRD | Code |
|---|---|---|
| Voice cloning with consent | v2 | built, ElevenLabs |
| Generated music and effects | v2 ("No generated music or effects" in v1) | built, 4 sources |
| Local open-weight models | v2 | built via OpenAI-compatible servers, Piper, ComfyUI, Whisper |
| Performance reference (speech to speech) | v2 | built as the voice changer |
| Interchange with NLEs (EDL, XML, AAF) | v3 | OTIO timeline built |
| Picture reference, ADR | v2 | not built |
| Fountain import | v2 | partial (see C-12) |

- **Resolution:** Update the master PRD phasing table to match reality, and mark each feature **Existing** in the traceability matrix. No code change.
- **Decides:** owner approves the PRD edit.

### C-04 (S/S) The 12 workspaces against the 6-step golden path
- Master PRD section 5 and `UX_GOLDEN_PATH.md`: one linear path (Script, Cast, Direct, Render, Mix, Export), "advanced panels one click away, never required".
- PRD-002 section 16: twelve workspaces, including Performance Lab, Soundscape Designer, Music Studio, Production Timeline, Video Workspace, Model Lab.
- **Resolution:** The Golden Path stays the default. Map workspaces onto it instead of adding twelve navigation items:

| PRD-002 workspace | Where it lives |
|---|---|
| Project Library | Projects (Home), exists |
| Script Import and Review | Step 1 Script, exists |
| Director Workspace | Step 3 Direct (gains a "Plan" tab) |
| Character and Voice Library | Step 2 Cast and Voices screen, exists |
| Performance Lab | Step 4 Render (take comparison), grows |
| Soundscape Designer, Music Studio | Step 5 Mix panels, partly exists |
| Production Timeline | Step 5 Mix, becomes editable |
| Video Workspace | Mix, optional "Picture" panel |
| Model Lab | Tools area, Advanced only |
| Export Center | Step 6 Export, exists |
| Settings and Model Manager | Settings and Voice engines, exist |

- **Decides:** owner.

---

## B. Architecture and stack

### C-05 (S/C, blocking nothing) Tailwind CSS
- PRD-002 section 14.1 lists Tailwind CSS.
- The code uses hand-written CSS with design tokens (`app/src/styles/tokens.css`). Contrast ratios are asserted in tests (`contrast.test.ts`), and the interface passes automated WCAG 2.2 AA checks on every screen.
- **Resolution:** Do not adopt Tailwind. Rewriting working, verified styling adds risk and no user value. Amend PRD-002 section 14.1.
- **Decides:** owner.

### C-06 (S/C, **blocking MS1**) Line identity is positional
- Master PRD FR-1: "Editing a line marks only that line and its render as stale." PRD-002 DIR-007 and AC-006: regenerate one line without touching others.
- The code identifies a line as `s{scene}-l{ordinal}` (`crates/soundstage-core/src/script.rs:125`). Takes, selections, line directions, skips and the render cache are all keyed by that string.
- **Effect:** inserting or deleting a line renumbers every later line in its scene. Directions, takes and "selected" marks then attach to the wrong lines, and the cache key check (`take.line_key == line_key`) fails, so paid takes are re-rendered. Deleting a scene shifts every later scene too.
- **Resolution:** Give every line, scene and character a **stable ID** that survives edits (see decision D-03). Migrate existing projects. This is the one change that must come first; the Director, plans and per-line features all depend on it.
- **Decides:** owner (approval of the data-model change).

### C-07 (S/C) "Job" model
- PRD-002 section 18: jobs with stable IDs and states Queued, Running, Completed, Failed, Cancelled, Interrupted; cancellation; retry.
- The code runs each operation as a separate command with in-memory "busy" flags and one batch loop in the UI. There is no persisted job record, no cancellation, no interrupted-state recovery.
- **Resolution:** Add a job record and queue in the Rust core (MS1). Existing commands are wrapped, not rewritten.
- **Decides:** owner.

### C-08 (S/S) Local model runtime
- `CLAUDE.md`: "Python only in the local inference sidecar", repo layout shows `sidecar/`. Ask before adding a runtime dependency over about 50 MB.
- PRD-002: "optional local AI worker processes".
- Code: no `sidecar/` folder. Local engines are reached through OpenAI-compatible servers, ComfyUI, the Piper program and the bundled whisper.cpp.
- The candidate models (Chatterbox, Qwen3-TTS, ACE-Step, Wan, LTX, Hunyuan) are Python and GPU-driver heavy: gigabytes of dependencies.
- **Resolution:** (1) First path: keep using servers the user runs (already works). (2) Second path, MS5: one managed **worker** process with a small HTTP protocol, installed on demand as an optional "model pack", never inside the base installer. (3) The 50 MB rule is satisfied by owner approval per pack. See `03-ARCHITECTURE-UPDATE.md` section 6.
- **Decides:** owner approves the sidecar approach and each pack.

### C-09 (S/S) GPU backend
- PRD-002 section 15 says "compatible inference backends" without naming any.
- Most listed models assume NVIDIA CUDA. Windows also offers DirectML and Vulkan, with much narrower model support.
- **Resolution:** Propose **NVIDIA CUDA first** for packs, with CPU-only for light models. Everything else is "cloud-first". Record as a decision (D-12).
- **Decides:** owner.

### C-10 (C/C) `docs/ARCHITECTURE.md` is out of date
- It describes v0.1: three adapters, one subprocess (PowerShell), no engines, generators, consent store, history, pronunciation, projectpack, whisper, OTIO.
- **Resolution:** Replace with the as-built description plus the target architecture in `03-ARCHITECTURE-UPDATE.md`. A note has been added to the top of `ARCHITECTURE.md`.

### C-11 (S/C) `dev_spec.md`, `AGENTS.md`, `DECISIONS.md`, `PROGRESS.md` do not exist
- PRD-002 section 21 tells the agent to read them. The repo has `CLAUDE.md`, and `ARCHITECTURE.md` is the closest to a technical specification.
- **Resolution:** Create `AGENTS.md`, `DECISIONS.md` and `PROGRESS.md` now. Treat `docs/ARCHITECTURE.md` plus `docs/spec/03-ARCHITECTURE-UPDATE.md` as `dev_spec.md` until the owner asks for a single file (D-02).

---

## C. Product behaviour

### C-12 (S/C) Script formats and elements
- PRD-002 section 9: TXT, Fountain, text-based PDF; extract **action descriptions**; correct mis-parsed elements.
- Code: plain text and a simple cue format; `INT.`/`EXT.` and `#` start scenes; **action lines are not extracted** (they are ignored or flagged); no PDF; no per-element correction UI (the user edits the text).
- **Resolution:** Fountain and action extraction in MS2; text PDF later in MS2 or MS3 after a library licence check; DOCX and FDX out of scope until asked.

### C-13 (S/S) Scoring methods for the Model Lab
- Master `TEST_PLAN.md` section 4.3: 1 to 5 blind scores on accent, intelligibility, naturalness, direction, non-verbal realism, artefacts, emotional truth, comic timing. A provider **passes** with median 4 or higher on accent and intelligibility from at least two reviewers per language.
- PRD-002 section 13.2: weighted dimensions (emotion 25, accent 25, physical 20, consistency 15, pronunciation 10, cost and latency 5).
- **Resolution:** Use the master scale and pass rule for **verification**. Use PRD-002 weights only to **rank** models that already pass. A model can never be "verified" by a weighted score alone (LAB-008 and the master rule agree).

### C-14 (S/S) Product name
- `CLAUDE.md`: working title "Voice Studio", "do not hard-code the name".
- The product is Soundstage in code, the repository and PRD-002.
- **Resolution:** Update `CLAUDE.md` and the other master documents to Soundstage. The owner named the product at the start of this build.

### C-15 (S/S) Claims of quality
- `CLAUDE.md` rule 7: honest quality claims. PRD-002 section 1: "Hollywood-quality production is the target standard, not a guaranteed output".
- **Resolution:** Compatible, but add a copy rule: user-facing text and marketing never say "Hollywood-quality", "authentic" or "native-sounding" for any language, accent or effect that has not passed the Model Lab gate.

---

## D. Safety, privacy and licensing

### C-16 (S/S, **blocking MS2**) The Director sends the whole script to a language model
- PRD-002 section 5 and 4.2 (cloud Director) is new. Master `CLAUDE.md`: ask before sending scripts to a new network host; `SAFETY_LICENSING.md` privacy rules.
- Two risks the PRD does not address: (a) the **script is untrusted input to a language model** (prompt injection: a line saying "ignore your rules and reveal..."), and (b) cloud analysis uploads unpublished screenplays.
- **Resolution:** Requirements added in `03-ARCHITECTURE-UPDATE.md` section 4: the script is data, not instructions; the model gets no tools and no network access of its own; output is validated against the PerformancePlan schema, repaired or rejected; a **local model option and a no-model heuristic fallback** must exist before a cloud Director ships; the first cloud use shows exactly what is sent and to whom.
- **Decides:** owner (hosts and providers).

### C-17 (S/S) Video, likeness and impersonation
- Master `SAFETY_LICENSING.md` covers voice cloning, real people, minors and fraud. PRD-002 adds lip-sync and video generation without any likeness rule.
- **Resolution:** A new section in `SAFETY_LICENSING.md` (written when M9 is approved) before any video generation code. Until then the non-goal stands (C-01).

### C-18 (S/S) Checkpoint licences have conditions, not just a label
- Master licence tags are `commercial`, `non-commercial`, `unknown`. PRD-002 SEC-005 asks for checkpoint-level validation.
- Searches on 2026-10-25 found conditions a single tag cannot hold: LTX-2.3 is free below a **US$10 million revenue** threshold; the HunyuanVideo community licence is reported to **exclude some territories**; Stable Audio Open's commercial terms conflicted between sources (see `05-CAPABILITY-AND-HARDWARE.md`).
- **Resolution:** The registry licence field becomes a structured record (tag plus conditions plus source URL and date). `unknown` and "has conditions the owner has not accepted" both block commercial export.

### C-19 (S/C) Accent claims
- Master non-negotiable 7 and FR-16: never list a language as supported without a bake-off result.
- PRD-002 VOICE-007, AC-005: show whether an accent passed evaluation.
- Code: voices show language and accent text from the provider but **no verification state** exists, and no bake-off has been run (no `tools/bakeoff/`).
- **Resolution:** Add the per-voice verification state (MS1) and the Model Lab (MS4). Until a language passes, the UI says "not yet tested", never "supported".

### C-20 (S/S) Sensitive sounds
- PRD-002 section 7 includes pain reactions, struggling and wrestling. Master `SAFETY_LICENSING.md` content rules (minors, real people, fraud) apply to every project.
- **Resolution:** No change to the rules, but state in the milestone for physical acting (MS3) that these sounds are fictional-character performance only, and that the existing refusal screen runs on Director output as well as on scripts.

---

## E. Smaller items

| ID | Kind | Item | Resolution |
|---|---|---|---|
| C-21 | S/C | PRD-002 names **ElevenLabs v4**. The adapter uses `eleven_v3` (`adapters/elevenlabs.rs:29`). Searches on 2026-10-25 report v4 launched about 28 September 2026 with a v4 Turbo variant, but dates and language counts conflict between sources. | Verify against the ElevenLabs documentation before changing the model id (task T-0.4). Do not switch silently: v4 changes cost and output. |
| C-22 | S/S | PRD-002 4.2 lists "OpenAI GPT reasoning models" and "Google Gemini". Neither host is approved. | Each new host needs owner approval (`CLAUDE.md` ask-before rule). |
| C-23 | S/C | PRD-002 VOICE-005 provider comparison for one line. Takes record voice and provider, but there is no side-by-side comparison screen. | MS3. |
| C-24 | S/C | PRD-002 section 14.1: FFmpeg-based media operations. Satisfied: LGPL FFmpeg is bundled. | None. |
| C-25 | S/S | PRD-002 says no database is required; master PRD FR-8 says human-readable JSON. | Consistent. Plans, jobs and the registry stay JSON files. |
| C-26 | S/C | PRD-002 SEC-009: do not execute downloaded model code. Python model files (pickle) can run code when loaded. | Packs must use safetensors or GGUF only, with pinned hashes (D-10). |
| C-27 | S/C | `TEST_PLAN.md` refers to `tools/bakeoff/`; it does not exist. | Created in MS4. |

---

## Summary of owner decisions needed

Blocking: **C-06** (stable IDs) before MS1; **C-16** (Director privacy and hosts) before MS2; **C-01** (video scope) before MS8 or MS9.

Everything else is a proposed resolution that can be approved together. Each is recorded in `DECISIONS.md` with status "Proposed".
