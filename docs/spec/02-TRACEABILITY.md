# 02. Requirements traceability matrix

Status: **Proposed, for owner review.** Written 2026-10-25 against code commit `f5ff35f` (v0.2.0).

Sources merged: PRD-002 (`docs/PRD_AI_MODELS_UPDATE.md`), the master `docs/PRD.md` (FR-1 to FR-16), `docs/SAFETY_LICENSING.md`, and the acceptance criteria in section 19 of PRD-002.

**Status meanings**
- **Existing**: built, with a test or a real-window check that shows it.
- **Partial**: some of it is built; the gap column says what is missing.
- **New**: not built.

"Evidence" points to a module or test in the repository. A dash means there is none. `MS` is the milestone in `06-MILESTONES-AND-TESTS.md` that delivers the missing part.

**Totals** (rows with a status, 171): **Existing 46**, **Partial 68**, **New 57**. Existing 26%, Partial 39%, New 33%. Counted by script from this file; update them when a row changes.

---

## 1. Scope items (PRD-002 section 3)

| ID | Item | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| SC-P0-01 | Windows application foundation | Existing | Tauri 2 shell, installer, 181 real-window checks | none | - |
| SC-P0-02 | Local project management | Existing | `project.rs`, atomic save with `.bak`, `projectpack.rs` backup | stable IDs (C-06) | MS1 |
| SC-P0-03 | Script import and scene parsing | Partial | `script.rs`, import `.txt`/`.fountain` | no action lines, no PDF, no per-element correction | MS2 |
| SC-P0-04 | Director and Emotional Engines | New | - | whole feature | MS2 |
| SC-P0-05 | Global voice library | Partial | `VoiceLibrary.tsx`, filters, flags | gender, style and provider filters; verification state | MS3 |
| SC-P0-06 | Physical Acting Engine foundation | New | non-verbal tags only | event model and hybrid pipeline | MS3 |
| SC-P0-07 | Multiple performance takes | Partial | takes per line, selection | side-by-side audition across providers | MS3 |
| SC-P0-08 | Selective regeneration | Partial | per-line render, cache keys | breaks when lines are inserted (C-06) | MS1 |
| SC-P0-09 | Cloud/local provider adapters | Existing | 8 adapters plus generators, contract suite | registry (see 4.4) | MS1 |
| SC-P0-10 | Basic audio timeline | Existing | `layout.rs`, Mix screen | editable (MS7) | - |
| SC-P0-11 | Individual audio export | Existing | OTIO plus per-clip WAV, stems, `exporter.rs` | none | - |
| SC-P0-12 | Model compatibility and hardware detection | Partial | `hardware.rs`, System check line, verdict logic | per-model requirements and measured thresholds | MS5 |
| SC-P1-01 | Soundscape and Foley generation | Partial | "Sound for your scenes", generators | no cue extraction from action lines; no Foley categories | MS6 |
| SC-P1-02 | Acoustic simulation | New | - | room, distance, occlusion DSP | MS6 |
| SC-P1-03 | Music generation | Existing | ElevenLabs, Kie/Suno, fal, ComfyUI | cue planning (MUS-001, 002) | MS6 |
| SC-P1-04 | Advanced multitrack editing | Partial | Audio editor (separate), timeline is derived | editable project timeline | MS7 |
| SC-P1-05 | Imported video synchronization | New | MP4 is output only | whole feature | MS8 |
| SC-P1-06 | Performance Reference Mode | Partial | voice changer with performer consent (`clone.rs`, `change_voice`) | not tied to a script line or plan | MS3 |
| SC-P1-07 | Sound and character continuity | New | - | ambience and character continuity tracking | MS6 |
| SC-P1-08 | Professional stem exports | Existing | master, 3 stems, manifest, OTIO | per-character and per-scene stems | MS7 |
| SC-P2-01 | Local video generation | New | - | deferred, needs separate approval (C-01) | MS9 |
| SC-P2-02 | Automated shot planning | New | - | deferred | MS9 |
| SC-P2-03 | Advanced lip-sync | New | - | deferred, safety review first (C-17) | MS9 |
| SC-P2-04 | Advanced spatial audio | New | - | deferred | after MS9 |
| SC-P2-05 | Automated sound continuity analysis | New | - | deferred | after MS9 |
| SC-P2-06 | Adaptive scoring | New | - | deferred | after MS9 |

## 2. Provider architecture (PRD-002 section 4)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| ARCH-4.1 | No single vendor; all AI behind standard interfaces | Existing | `Adapter`, `Generator` traits, `Registry` | LLM and video interfaces | MS1, MS2 |
| ARCH-4.1b | Provider selection weighs capability, language, accent, expressiveness, hardware, licence, cost, latency, preference | Partial | licence and `sends_data` shown; manual choice | automatic ranking from the registry | MS3 |
| ARCH-4.2 | Cloud candidates verified before use | Partial | ElevenLabs v3, VoiceKiller verified from docs; v4 unverified | verify v4, Gemini, GPT, video APIs | MS0 task T-0.4 |
| ARCH-4.3 | Local candidates validated per checkpoint | Partial | Whisper, Piper, ComfyUI, OpenAI-compatible servers | Chatterbox, Qwen3-TTS, ACE-Step, Stable Audio Open, WhisperX, Demucs | MS5 |
| ARCH-4.4 | Model Capability Registry (14 fields) | Partial | `capability.rs`, `data/models.json`, rule that nothing is verified without evidence | UI, accents and languages, adapters read from it | MS3 |
| ARCH-4.5 | Provider fallback with approval, no silent change of cost or privacy | Partial | confirm prompts before any online send; session totals | automatic alternatives list, cost estimate, "preserve plan" | MS3 |

## 3. Director Engine (PRD-002 section 5)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| DIR-001 | Extract scenes, characters, dialogue, actions, stage directions | Partial | scenes, speakers, lines | actions and stage directions | MS2 |
| DIR-002 | Dramatic objectives and relationships | New | - | LLM analysis | MS2 |
| DIR-003 | Infer emotional subtext | New | - | LLM analysis | MS2 |
| DIR-004 | Detect physical actions affecting voice | New | - | LLM analysis plus rules | MS2, MS3 |
| DIR-005 | Recommend pacing, pauses, emphasis, reactions | New | manual direction and prompt helper only | automatic suggestions | MS2 |
| DIR-006 | Alternative interpretations | New | - | n alternatives per line | MS2 |
| DIR-007 | Preserve user changes when regenerating others | Partial | script is the source of truth | needs stable IDs and a plan object | MS1, MS2 |
| DIR-008 | Confidence and uncertainty notes | New | - | schema field and UI | MS2 |
| DIR-PLAN | PerformancePlan, versioned and validated (13 fields) | Partial | `plan.rs` has the schema, validation and store; `direction.rs` resolves a subset | nothing produces or uses a plan yet | MS2 |

## 4. Emotional Intelligence Engine (section 6)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| EMO-001 | Several simultaneous emotional dimensions | New | single emotion per line | multi-dimension model | MS2 |
| EMO-002 | Gradual transitions | New | - | transition curve per line or span | MS2 |
| EMO-003 | Restrained, concealed, contradictory emotion | New | - | "surface vs inner" fields | MS2 |
| EMO-004 | Continuity between adjacent lines | Partial | style persists across pauses within a line | cross-line tracking and advisory check | MS2 |
| EMO-005 | Manual editing of intensity and transitions | Partial | emotion and pace editable per line | intensity and transition controls | MS2 |
| EMO-006 | Translate plans into model-specific controls | Partial | canonical tags to provider tags (`providers.rs`), VoiceKiller instructions | intensity mapping per model | MS3 |
| EMO-007 | Preserve intent when switching provider | Partial | direction is provider-independent | check and report what was lost | MS3 |
| EMO-NOTE | Emotion labels not treated as universal | Existing | adapters refuse unsupported emotions up front | none | - |

## 5. Physical Acting Engine (section 7)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| PHY-001 | Never read instructions aloud | Existing | tags are tokens, never spoken; adapter tests | add a regression test over all adapters | MS3 |
| PHY-002 | Preserve voice identity | Partial | same voice id for the whole line | listening gate for hybrid events | MS3 |
| PHY-003 | Separate control of speech and non-speech | Partial | inline tags | separate event tracks | MS3 |
| PHY-004 | Manual event timing | Partial | `[pause:ms]`, tag position | event timeline with drag | MS3, MS7 |
| PHY-005 | Report unsupported physical effects | Existing | "not supported by this voice" before render, with a fix button | none | - |
| PHY-006 | Preserve source and generated assets | Partial | takes kept | keep event assets separately | MS3 |
| PHY-CAT | 12 categories | Partial | laugh, cough, sneeze, gasp, cry, breath, whisper exist as tags where a provider allows | exertion, running, struggle, chewing, obstructed mouth, pain, recovery breathing | MS3 |
| PHY-PIPE | 8-step hybrid pipeline | New | - | analyse, synthesise, generate events, process, synchronise, assemble, present takes | MS3 |

## 6. Voice and accent (section 8)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| VOICE-001 | Search and filter by language, accent, gender, style, provider | Partial | search, language, licence filters | gender, style, provider filters | MS3 |
| VOICE-002 | Reusable character voice profiles | Partial | voice per character, remembered by speaker name | named profile object with accent, style, pronunciation | MS3 |
| VOICE-003 | Pronunciation dictionaries | Existing | `pronounce.rs`, tests | none | - |
| VOICE-004 | Multilingual and code-switching where verified | New | one language per voice | per-span language in the plan | MS3 |
| VOICE-005 | Provider comparison for one line | New | - | side-by-side across providers | MS3 |
| VOICE-006 | Preserve accent selections across sessions | Existing | voice ids saved in the project | none | - |
| VOICE-007 | Label unverified accent claims | Partial | accent shown as "not stated" | per-voice verification state | MS1, MS4 |
| VOICE-008 | Authorisation for cloning and reference voices | Existing | `consent.rs`, `clone.rs`, gates in render and export | none | - |
| VOICE-LANG | Priority languages and accents (13) | Partial | depends on provider; none verified | evaluation in the Model Lab | MS4 |

## 7. Script import (section 9)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| SCR-001 | Preserve original script text | Existing | script text is stored unchanged | none | - |
| SCR-002 | Identify scenes and speakers | Existing | `script.rs` tests | none | - |
| SCR-003 | Extract action descriptions | New | - | parser support | MS2 |
| SCR-004 | Correct mis-parsed elements | Partial | edit the text | per-element correction UI | MS2 |
| SCR-005 | Preserve source references | Partial | row numbers | stable source spans | MS1 |
| SCR-006 | Editable scene plan | Partial | scene list, scene directions | plan with cues and notes | MS2 |
| SCR-007 | Never overwrite user corrections | Existing | the script is the single source of truth | keep true for plans | MS2 |
| SCR-FMT-1 | TXT | Existing | import button | none | - |
| SCR-FMT-2 | Fountain | Partial | scene headings recognised | full Fountain elements | MS2 |
| SCR-FMT-3 | Text-based PDF | New | - | library licence check, extraction, image-only detection | MS2 |
| SCR-FMT-4 | DOCX, FDX | New | - | later, only if requested | later |

## 8. Soundscape and Foley (section 10)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| SFX-001 | Suggest cues from scripts | Partial | per-scene drafts from the scene heading | cues from action lines and plan | MS6 |
| SFX-002 | Generate effects through providers | Existing | ElevenLabs, fal, Kie, ComfyUI | none | - |
| SFX-003 | Import licensed or user-owned audio | Existing | import dialog, licence editor | none | - |
| SFX-004 | Position effects on a timeline | Existing | cue scene and offset | none | - |
| SFX-005 | Looping and crossfades | Partial | loop to scene, fades | crossfades between loops | MS6 |
| SFX-006 | Room-specific acoustic processing | New | - | DSP and room profiles | MS6 |
| SFX-007 | Ambience continuity between shots | New | - | continuity model | MS6 |
| SFX-008 | Export Foley, ambience and effects separately | Partial | effects stem and per-clip files | per-category stems | MS7 |
| SFX-DSP | Gain, panning, fades, EQ, reverb, distance, occlusion, stereo placement | Partial | gain and fades exist | pan, EQ, reverb, distance, occlusion, placement | MS6 |

## 9. Music (section 11)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| MUS-001 | Analyse dramatic beats | New | - | beat extraction from plan | MS6 |
| MUS-002 | Music prompts and cue plans | Partial | per-scene music prompt draft | beat-aware plans | MS6 |
| MUS-003 | Cloud and local providers | Existing | ElevenLabs, Kie, fal, ComfyUI | ACE-Step pack | MS5 |
| MUS-004 | Duration and timing constraints | Existing | length clamped to each provider's limits | none | - |
| MUS-005 | Regenerate independently of dialogue | Existing | separate assets | none | - |
| MUS-006 | Fade, trim, gain, placement | Partial | fade, gain, placement, loop | trim on the project timeline | MS7 |
| MUS-007 | Metadata and licence kept | Existing | `GenInfo`, licence record, manifest | none | - |
| MUS-008 | Export independently | Existing | music stem, OTIO clips | none | - |

## 10. Video (section 12)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| VID-001 | Import video | New | - | decode and probe with FFmpeg | MS8 |
| VID-002 | Synchronised preview | New | - | player pinned to the timeline | MS8 |
| VID-003 | Associate cues with video time | New | - | timecode on clips | MS8 |
| VID-004 | Frame-rate-aware timing | New | OTIO uses milliseconds | frame rates, drop-frame | MS8 |
| VID-005 | Preserve sync in export | New | - | export test with tolerance | MS8 |
| VID-006 | Manual correction of suggested timings | New | - | nudge controls | MS8 |
| VID-GEN | Local video generation (hardware detection, downloads, disk check, VRAM, progress, cancel, local storage, licence notes, separate worker) | New | - | deferred | MS9 |

## 11. Model Lab (section 13)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| LAB-001 | Identical scripts across models | New | fixture scripts exist in `fixtures/scripts/`; harness does not | runner | MS4 |
| LAB-002 | Keep settings and outputs | New | takes record provider version and direction | lab run record | MS4 |
| LAB-003 | Blind listening comparison | New | - | blind player | MS4 |
| LAB-004 | Record human scores | New | - | score store and reviewer codes | MS4 |
| LAB-005 | Latency and cost | New | - | timing and cost capture | MS4 |
| LAB-006 | Strengths by task and language | New | - | report | MS4 |
| LAB-007 | Export reports | New | - | CSV and Markdown | MS4 |
| LAB-008 | No accent verification from automated scoring alone | Existing (as a rule) | no accent is marked supported today | enforce in the registry | MS1 |
| LAB-SET | 13 acceptance scenarios | New | - | write scripts and references | MS4 |

## 12. Architecture and storage (section 14)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| ARCH-14.1 | Tauri 2, React, TypeScript, Rust, FFmpeg, local worker | Partial | all except Tailwind (rejected, C-05) and workers | workers | MS5 |
| ARCH-14.2 | 16 modules | Partial | 7 exist in some form | Director, Emotional, Physical Acting, Registry, Model Lab, Foley/DSP, Video | MS1 to MS8 |
| ARCH-14.3 | Project contents (manifest, assets, scenes, plans, takes, timeline, export config, provenance) | Partial | manifest, assets, scenes, takes, export manifest | plans, timeline state, export config in the project, per-take provenance | MS1, MS7 |
| ARCH-14.3b | Stable IDs and relative paths | Partial | relative paths validated | stable IDs | MS1 |
| ARCH-14.3c | Generated source media immutable | Partial | takes are written once | enforce and test | MS1 |
| ARCH-14.3d | No database for the first release | Existing | JSON files | none | - |

## 13. Hardware runtime (section 15)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| HW-001 | Detect CPU, RAM, GPU, VRAM | Existing | `hardware::detect` (NVIDIA only for GPU) | other GPUs show as unknown | - |
| HW-002 | Detect inference backends | New | - | CUDA, Vulkan, DirectML probes | MS1 |
| HW-003 | Show compatibility before download | Partial | model size shown | hardware verdict | MS1, MS5 |
| HW-004 | Estimate storage | Partial | sizes shown for Whisper models | for all packs, free-space check | MS5 |
| HW-005 | Avoid starting incompatible models | New | - | gate | MS5 |
| HW-006 | Cloud fallback with approval | Partial | confirm before any send | alternatives list | MS3 |
| HW-007 | Out-of-memory without corrupting projects | Partial | atomic project save; errors are plain | worker crash handling | MS5 |
| HW-008 | Remove downloaded models | Existing (Whisper) | Remove button | general model manager | MS5 |
| HW-PROF | 5 profiles | New | - | definitions in `05-...` | MS1 |

## 14. UX (section 16)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| UX-12 | 12 workspaces | Partial | 7 map to existing screens (C-04) | Director plan tab, Performance Lab compare, Video, Model Lab | MS2 to MS8 |
| UX-STATES | Empty, loading, error, offline, success, keyboard, accessibility, recovery per workspace | Partial | most screens have empty and error states; WCAG 2.2 AA checks pass | audit all states per screen and add a state test | each MS |
| UX-RESP | Responsive during generation | Partial | work runs on background threads | job queue with progress and cancel | MS1 |

## 15. Security (section 17)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| SEC-001 | Credentials in OS-backed storage | Existing | Windows Credential Manager | none | - |
| SEC-002 | No credentials in logs or exports | Existing | tests check keys stay out of requests, reports and manifests | none | - |
| SEC-003 | Consent for cloning | Existing | `consent.rs`, render and export gates | none | - |
| SEC-004 | Generation provenance | Partial | take records, `GenInfo`, manifest | plan and LLM provenance; per-event | MS2 |
| SEC-005 | Validate commercial rights of checkpoints and assets | Partial | licence tags and gate | structured conditions (C-18) | MS1 |
| SEC-006 | Disclose when data goes to the cloud | Existing | confirm dialogs and session totals | extend to Director and video | MS2 |
| SEC-007 | Local-only generation switch | Existing | `net::local_guard` on every HTTP agent, Settings toggle | none | - |
| SEC-008 | Restrict local workers to approved folders | New | subprocesses use temp folders only | worker sandbox rules | MS5 |
| SEC-009 | Do not execute imported scripts or downloaded model code | Partial | no script execution; pinned, hashed downloads | pack format rule: no pickle (C-26) | MS5 |
| SEC-010 | Approval for paid generation | Existing | every online path asks first | extend to new paths | each MS |

## 16. Reliability (section 18)

| ID | Requirement | Status | Evidence | Gap | MS |
|---|---|---|---|---|---|
| REL-JOBS | Job IDs and six states | Partial | `jobrec.rs` for render batches | general queue | MS5 |
| REL-STATUS | Progress for long operations | Partial | batch progress, download progress | uniform | MS1 |
| REL-INTERRUPT | Interrupted work does not damage projects | Partial | atomic save, `.bak` recovery | interrupted state on restart | MS1 |
| REL-RETRY | Failed generations keep the error and can retry | Partial | per-line error and retry | stored on the job | MS1 |
| REL-SAVE | Atomic or recoverable saves | Existing | write to temp then rename, `.bak` | none | - |

## 17. Acceptance criteria (section 19)

| ID | Criterion | Status | What exists | What is missing | MS |
|---|---|---|---|---|---|
| AC-001 | Script to editable performance plan | Partial | import, parse, edit | the plan | MS2 |
| AC-002 | Audition several takes, keep the script | Partial | takes per line | audition screen | MS3 |
| AC-003 | Vocal events placed, not read aloud | Partial | inline tags, provider-dependent | event placement UI and test across adapters | MS3 |
| AC-004 | Hybrid physical performance | New | - | pipeline | MS3 |
| AC-005 | Language/accent selection with evaluation state | Partial | selection | state | MS4 |
| AC-006 | Regenerate one line without touching others | Existing | `inserting_a_line_leaves_every_other_line_rendered_and_costs_no_calls` and a real-window check | none | - |
| AC-007 | Layered, independently editable soundscape | Partial | cues are independent | layers per category, DSP | MS6 |
| AC-008 | Local model, no cloud transmission | Partial | Piper, Whisper, local servers | a validated TTS pack | MS5 |
| AC-009 | Fallback alternatives without silent charges | New | - | alternatives flow | MS3 |
| AC-010 | Export clips and stems with correct names and durations | Existing | OTIO clips, stems, tests | naming scheme review | MS7 |
| AC-011 | Reopen after interrupted generation | Existing | job records, adoption of finished takes, real-window check | a true process-kill test | - |
| AC-012 | Video sync within a tolerance | New | - | whole feature | MS8 |

## 18. Master PRD requirements carried forward (FR-1 to FR-16)

These are already tracked in the master PRD. This table only records where the code stands, so the two documents agree.

| ID | Status | Note |
|---|---|---|
| FR-1 Script editor | Partial | stale marking breaks on insertion (C-06) |
| FR-2 Voice library | Existing | |
| FR-3 Direction (tags, per-line controls) | Existing | |
| FR-4 Voice design | Existing | ElevenLabs only |
| FR-5 Voice cloning (gated) | Existing | ElevenLabs and VoiceKiller registration |
| FR-6 Rendering (queue, cache, retry) | Partial | no persistent queue (C-07) |
| FR-7 Mix and export | Existing | |
| FR-8 Projects and storage | Existing | migration present |
| FR-9 Settings and keys | Existing | |
| FR-10 Scenes, takes, direction levels | Existing | |
| FR-11 Picture sync and ADR | New | MS8 |
| FR-12 Music and effects | Existing | |
| FR-13 Stems and delivery | Existing | sum test in `TEST_PLAN.md` 3a |
| FR-14 Directing tools | Partial | take review notes and approvals not built |
| FR-15 Acting tools | Partial | character bible and read-through exist; performance reference via voice changer |
| FR-16 Global scope and language | Partial | no language has a bake-off result yet |
