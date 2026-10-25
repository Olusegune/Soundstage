# SOUNDSTAGE
## Product Requirements Document — AI Models & Cinematic Intelligence Update

**Document ID:** SOUNDSTAGE-PRD-002
**Version:** 1.1
**Status:** Proposed — Pending Approval
**Target implementation agent:** Claude Code
**Platform:** Windows Desktop
**Architecture:** Local-first, hybrid cloud/local AI
**Development methodology:** Golden Path
**Implementation authorization:** Not granted by this document

> Source: supplied by the product owner on 2026-10-25 and stored here unchanged so that requirement IDs can be traced.
> Review, conflicts, traceability and the milestone plan are in `docs/spec/`. This file is the owner's text; do not edit it
> without the owner's approval. Accepted changes are recorded in `DECISIONS.md`.

---

# 1. Executive Summary

Soundstage is a professional Windows desktop AI cinematic audio and video production application.

The application must transform screenplays, dialogue, scene descriptions, and imported video into emotionally expressive, professionally directed, synchronized audio productions.

Soundstage must go beyond conventional text-to-speech applications by understanding:

- Character motivations and dramatic subtext.
- Emotional transitions throughout scenes.
- Physical acting conditions and vocal effort.
- Authentic international languages and accents.
- Environmental sound, Foley, and acoustic properties.
- Cinematic music and dramatic pacing.
- Audio-to-picture synchronization.
- Character and performance continuity.

The application must support both commercial cloud AI providers and locally executed AI models.

**Primary product objective:** Enable creators to produce cinematic voice performances and soundscapes with granular creative direction and professional export workflows.

Hollywood-quality production is the target standard, not a guaranteed output of any particular model.

# 2. Product Goals

## 2.1 Primary Goals

1. Provide a unified AI cinematic production environment.
2. Deliver expressive, directable AI voice performances.
3. Support authentic international languages and regional accents.
4. Generate believable simulated physical acting sounds.
5. Analyze screenplays and generate editable performance plans.
6. Generate environmental audio, Foley, and music.
7. Synchronize dialogue and sound with imported video.
8. Support cloud and local AI inference.
9. Allow independent export of every generated audio asset.
10. Maintain a premium, responsive Windows desktop experience.

## 2.2 Target Users

- Film and animation producers.
- Independent filmmakers.
- Voiceover artists and audio producers.
- Game developers.
- YouTube and digital content creators.
- Screenwriters and storytellers.
- Multilingual media production teams.
- Creators producing dialogue-heavy animated content.

## 2.3 Success Criteria

The product succeeds when users can import a screenplay, inspect and correct its interpretation, assign character voices, direct performances, generate layered audio, synchronize it to video, and export reusable production assets.

The workflow must preserve editable intermediate results rather than flattening all generated material into one file.

# 3. Product Scope and Release Priorities

**P0 — Core release**

- Windows application foundation.
- Local project management.
- Script import and scene parsing.
- Director and Emotional Engines.
- Global voice library.
- Physical Acting Engine foundation.
- Multiple performance takes.
- Selective regeneration.
- Cloud/local provider adapters.
- Basic audio timeline.
- Individual audio export.
- Model compatibility and hardware detection.

**P1 — Cinematic production release**

- Soundscape and Foley generation.
- Acoustic simulation.
- Music generation.
- Advanced multitrack editing.
- Imported video synchronization.
- Performance Reference Mode.
- Sound and character continuity.
- Professional stem exports.

**P2 — Advanced cinematic release**

- Local AI video generation.
- Automated shot planning.
- Advanced lip-sync.
- More sophisticated spatial audio.
- Automated sound continuity analysis.
- Advanced adaptive scoring.

P2 features must not delay P0 completion.

# 4. AI Model Architecture

## 4.1 Fundamental Architecture Rule

Soundstage must not depend on a single AI vendor.

All AI capabilities must be accessed through standardized provider interfaces.

Provider selection must consider:

- Task capability.
- Supported languages.
- Accent fidelity.
- Emotional expressiveness.
- Non-speech vocal support.
- Available hardware.
- Licensing and commercial-use permissions.
- Estimated cost.
- Generation latency.
- User preferences.

Model names in this PRD are candidates, not proof of production suitability.

## 4.2 Cloud AI Candidates

| Capability | Candidate |
|---|---|
| Expressive speech | ElevenLabs v4 |
| Director reasoning | OpenAI GPT reasoning models |
| Alternative reasoning | Google Gemini |
| Video understanding | Gemini multimodal models |
| Sound effects | ElevenLabs Sound Effects |
| Music | Eleven Music |
| Speech recognition | Managed transcription provider |
| Video generation | Licensed commercial video APIs, later release |

Claude must verify the availability, exact model identifiers, API access, licensing, and relevant features before implementation.

Do not hardcode unverified model IDs.

## 4.3 Local AI Candidates

| Capability | Candidate |
|---|---|
| Expressive TTS | Chatterbox Turbo |
| Multilingual TTS | Chatterbox Multilingual |
| Voice design | Qwen3-TTS |
| Additional speech research | Fish Speech, Orpheus, Dia |
| Local Director | Qwen or DeepSeek instruct/reasoning models |
| Local visual understanding | Compatible Qwen vision-language model |
| Local SFX | Stable Audio Open |
| Alternative SFX | AudioLDM family |
| Local music | ACE-Step 1.5 |
| Speech recognition | Whisper |
| Word alignment | WhisperX |
| Source separation | Demucs |
| Local video generation | Wan 2.2 |
| Alternative video | LTX-2.3, HunyuanVideo 1.5 |

All local models must undergo checkpoint-specific licensing, dependency, hardware, and capability validation.

## 4.4 Model Capability Registry

Every provider/model entry must contain:

- Unique model identifier.
- Provider identifier.
- Version.
- Supported task types.
- Supported languages.
- Verified accents.
- Supported performance controls.
- Supported input/output formats.
- Local or cloud execution mode.
- VRAM/RAM requirements, where applicable.
- License and commercial-use status.
- Cost information, where available.
- Known limitations.
- Benchmark results.
- Verification date.

The UI must distinguish **verified**, **experimental**, and **unsupported** capabilities.

## 4.5 Provider Fallback

If a model cannot satisfy a requested capability, Soundstage must:

1. Explain the limitation.
2. Identify compatible alternatives.
3. Show estimated costs where available.
4. Request approval before switching to a paid provider.
5. Preserve the original performance plan.

No silent provider substitution is permitted when it changes cost, privacy, or output characteristics.

# 5. Director Engine

## 5.1 Purpose

The Director Engine interprets scripts and scenes to produce editable cinematic performance instructions.

## 5.2 Functional Requirements

**DIR-001:** Extract scenes, characters, dialogue, actions, and stage directions.

**DIR-002:** Identify dramatic objectives and character relationships.

**DIR-003:** Infer emotional subtext from surrounding context.

**DIR-004:** Detect physical actions affecting vocal performance.

**DIR-005:** Recommend dialogue pacing, pauses, emphasis, and vocal reactions.

**DIR-006:** Generate alternative performance interpretations.

**DIR-007:** Preserve user modifications when regenerating other parts of a scene.

**DIR-008:** Provide confidence indicators or uncertainty notes for ambiguous interpretations.

## 5.3 Required Output

A structured PerformancePlan containing:

- Scene ID.
- Character ID.
- Dialogue line ID.
- Original text.
- Dramatic intent.
- Emotional state.
- Physical state.
- Vocal direction.
- Timing requirements.
- Accent and language requirements.
- Vocal-event cues.
- Suggested model.
- Review status.

The output must conform to a versioned, validated schema.

# 6. Emotional Intelligence Engine

## 6.1 Purpose

Model emotional development across dialogue, shots, and scenes.

## 6.2 Requirements

**EMO-001:** Support multiple simultaneous emotional dimensions.

**EMO-002:** Allow gradual emotional transitions.

**EMO-003:** Support restrained, concealed, and contradictory emotions.

**EMO-004:** Track emotional continuity between adjacent lines.

**EMO-005:** Allow manual editing of intensity and transitions.

**EMO-006:** Translate emotional plans into model-specific controls.

**EMO-007:** Preserve emotional intent when switching providers where feasible.

Emotion labels must not be treated as universal controls supported identically by all speech models.

# 7. Physical Acting Engine

## 7.1 Purpose

Generate believable vocal performances affected by simulated physical conditions.

## 7.2 Required Performance Categories

- Heavy lifting and exertion.
- Running and breathlessness.
- Struggling or wrestling.
- Crying while speaking.
- Suppressed laughter.
- Coughing and sneezing.
- Gasps and startled reactions.
- Whispering under stress.
- Speaking while chewing.
- Speaking with a simulated obstructed mouth.
- Pain reactions.
- Exhaustion and recovery breathing.

## 7.3 Implementation Strategy

Use a hybrid pipeline:

1. Analyze physical action.
2. Generate physical performance instructions.
3. Synthesize speech.
4. Generate or retrieve non-speech vocal events.
5. Apply appropriate processing where useful.
6. Synchronize events with speech.
7. Assemble the resulting performance.
8. Present multiple takes for review.

**PHY-001:** Never read performance instructions as spoken dialogue.

**PHY-002:** Preserve character voice identity as much as technically possible.

**PHY-003:** Allow separate control of speech and non-speech effects.

**PHY-004:** Support manual event timing.

**PHY-005:** Clearly report unsupported physical effects.

**PHY-006:** Preserve source and generated assets for subsequent editing.

Physical effects must be simulated; no unsafe real-world performance is required.

# 8. Global Voice and Accent Engine

## 8.1 Language and Accent Separation

Soundstage must distinguish:

- Spoken language.
- Regional accent.
- Dialect.
- Voice identity.
- Speaking style.
- Emotional delivery.

## 8.2 Priority Languages and Accents

Initial evaluation must include:

- Nigerian English.
- Nigerian Pidgin.
- Korean.
- Korean-accented English.
- Filipino/Tagalog.
- Filipino-accented English.
- Taglish.
- Japanese.
- British English.
- American English.
- Indian English.
- Ghanaian English.
- South African English.

Additional international languages must be supported through provider capability discovery.

## 8.3 Requirements

**VOICE-001:** Search and filter voices by language, accent, gender presentation, style, and provider.

**VOICE-002:** Save reusable character voice profiles.

**VOICE-003:** Support pronunciation dictionaries.

**VOICE-004:** Support multilingual dialogue and code-switching where verified.

**VOICE-005:** Allow provider comparison for the same line.

**VOICE-006:** Preserve accent selections across project sessions.

**VOICE-007:** Clearly label unverified accent claims.

**VOICE-008:** Require appropriate authorization for voice cloning and reference voice use.

Native-speaker evaluations must be used to assess accent authenticity.

# 9. Script Import and Analysis

## 9.1 Initial Formats

- TXT.
- Fountain.
- Text-based PDF.

Later formats:

- DOCX.
- Final Draft FDX.

## 9.2 Requirements

**SCR-001:** Preserve original script text.

**SCR-002:** Identify scenes and dialogue speakers.

**SCR-003:** Extract action descriptions.

**SCR-004:** Allow correction of incorrectly parsed elements.

**SCR-005:** Preserve original source references.

**SCR-006:** Generate an editable scene plan.

**SCR-007:** Never automatically overwrite user-corrected screenplay data.

Image-only PDFs require a separate OCR capability and must not be silently treated as successfully parsed text.

# 10. Cinematic Soundscape and Foley Engine

## 10.1 Sound Categories

- Environmental ambience.
- Weather.
- Room tone.
- Footsteps.
- Clothing movement.
- Object interactions.
- Doors and machinery.
- Vehicles.
- Impacts.
- Crowd activity.
- Animal sounds.
- Environmental transitions.
- Cinematic event effects.

## 10.2 Requirements

**SFX-001:** Generate suggested sound cues from scripts.

**SFX-002:** Generate sound effects through supported AI providers.

**SFX-003:** Import licensed or user-owned audio assets.

**SFX-004:** Position effects on a timeline.

**SFX-005:** Support looping and crossfades.

**SFX-006:** Support room-specific acoustic processing.

**SFX-007:** Track ambience continuity between shots.

**SFX-008:** Export Foley, ambience, and effects separately.

## 10.3 Acoustic Processing

Implement deterministic audio processing for:

- Gain.
- Panning.
- Fades.
- EQ.
- Reverb.
- Distance attenuation.
- Occlusion filtering.
- Stereo placement.

Do not use generative AI for operations better handled by reliable DSP.

# 11. Cinematic Music Engine

## 11.1 Purpose

Generate and arrange scene-aware musical scores.

## 11.2 Requirements

**MUS-001:** Analyze emotional and dramatic beats.

**MUS-002:** Generate music prompts and cue plans.

**MUS-003:** Support cloud and local music providers.

**MUS-004:** Support duration and timing constraints.

**MUS-005:** Allow music to be regenerated independently of dialogue.

**MUS-006:** Support fade, trim, gain, and placement.

**MUS-007:** Preserve music-generation metadata and license information.

**MUS-008:** Export music independently.

Initial model candidates are Eleven Music and ACE-Step 1.5.

# 12. Video Intelligence and Local Video Generation

## 12.1 P1: Video Import and Synchronization

**VID-001:** Import supported video files.

**VID-002:** Display a synchronized video preview.

**VID-003:** Associate dialogue and sound cues with video time positions.

**VID-004:** Support frame-rate-aware timing.

**VID-005:** Preserve synchronization during export.

**VID-006:** Allow manual correction of automatically suggested timings.

## 12.2 P2: Local Video Generation

Candidate models:

- Wan 2.2.
- LTX-2.3.
- HunyuanVideo 1.5.

Requirements:

- Hardware capability detection.
- Optional model downloads.
- Disk-space validation.
- VRAM-aware execution.
- Generation progress reporting.
- Cancellation where supported.
- Local asset storage.
- Clear licensing and model limitations.

Local video generation must run through a separately managed worker process and must not freeze the desktop UI.

# 13. Soundstage Model Lab

## 13.1 Purpose

Evaluate and compare AI providers using repeatable benchmark scripts.

## 13.2 Evaluation Dimensions

| Dimension | Initial weighting |
|---|---:|
| Emotional authenticity | 25% |
| Accent authenticity | 25% |
| Physical acting realism | 20% |
| Character consistency | 15% |
| Pronunciation | 10% |
| Cost and latency | 5% |

These weights are provisional and must be adjustable by benchmark category.

## 13.3 Requirements

**LAB-001:** Run identical test scripts across compatible models.

**LAB-002:** Preserve model settings and generated outputs.

**LAB-003:** Support blind listening comparisons.

**LAB-004:** Record human evaluator scores.

**LAB-005:** Record latency and estimated/actual cost where available.

**LAB-006:** Compare model strengths by task and language.

**LAB-007:** Export benchmark reports.

**LAB-008:** Never label a model as accent-verified solely from automated scoring.

## 13.4 Acceptance Benchmark Set

Include test scenarios for:

- Nigerian English dramatic dialogue.
- Nigerian Pidgin dialogue.
- Korean dialogue.
- Korean-accented English.
- Filipino/Tagalog dialogue.
- Taglish dialogue.
- Heavy lifting and exertion.
- Breathless running dialogue.
- Suppressed laughter.
- Crying while speaking.
- Speaking while chewing.
- Coughing and sneezing.
- Dialogue in different acoustic environments.

# 14. Desktop Application Architecture

## 14.1 Technology Stack

- Tauri 2.
- React.
- TypeScript.
- Tailwind CSS.
- Rust native services.
- Local project files and media assets.
- FFmpeg-based media operations.
- Native audio processing.
- Optional local AI worker processes.

## 14.2 Core Modules

1. Desktop Shell.
2. Project Manager.
3. Script Parser.
4. Scene Intelligence.
5. Director Engine.
6. Emotional Engine.
7. Physical Acting Engine.
8. Voice Provider Manager.
9. Model Capability Registry.
10. Model Lab.
11. Soundscape and Foley Manager.
12. Music Supervisor.
13. Timeline and Audio Engine.
14. Video Intelligence.
15. Export Manager.
16. Settings and Credential Manager.

## 14.3 Storage Strategy

Projects must remain local by default.

Each project must contain:

- Versioned project manifest.
- Original imported assets.
- Scene and character data.
- Performance plans.
- Generated takes.
- Sound and music assets.
- Timeline state.
- Export configuration.
- Generation provenance.

Use stable IDs and relative asset references.

Generated source media must be immutable unless the user explicitly deletes it.

A database is not required for the initial release.

# 15. Hardware-Adaptive AI Runtime

## 15.1 Hardware Profiles

Soundstage must support:

**Cloud-first:** CPU-only or limited GPU hardware.

**Local-light:** Suitable for selected lightweight or quantized speech models.

**Local-standard:** Supports broader speech and audio models.

**Local-advanced:** Supports larger multimodal and selected video models.

**Local-workstation:** High-memory GPU configurations for advanced video workflows.

These are capability profiles, not fixed guarantees tied to one VRAM number.

## 15.2 Runtime Requirements

**HW-001:** Detect CPU, RAM, GPU, and available VRAM where supported.

**HW-002:** Detect compatible inference backends.

**HW-003:** Display model compatibility before download.

**HW-004:** Estimate model storage requirements.

**HW-005:** Avoid starting incompatible models.

**HW-006:** Allow cloud fallback with user approval.

**HW-007:** Handle out-of-memory failures without corrupting projects.

**HW-008:** Allow removal of downloaded model files.

# 16. User Experience Requirements

Soundstage must use a premium dark cinematic design with strong visual hierarchy and consistent spacing.

Required workspaces:

1. Project Library.
2. Script Import and Review.
3. Director Workspace.
4. Character and Voice Library.
5. Performance Lab.
6. Soundscape Designer.
7. Music Studio.
8. Production Timeline.
9. Video Workspace.
10. Model Lab.
11. Export Center.
12. Settings and Model Manager.

Every workspace must define:

- Empty state.
- Loading state.
- Error state.
- Offline state.
- Success state.
- Keyboard interaction.
- Accessibility behavior.
- Recovery behavior where relevant.

The UI must remain responsive during generation.

# 17. Security, Privacy, and Licensing

**SEC-001:** Store API credentials using secure operating-system-backed storage.

**SEC-002:** Do not expose credentials in logs or project exports.

**SEC-003:** Require consent or authorization for voice cloning.

**SEC-004:** Record generation provenance.

**SEC-005:** Validate commercial-use rights for model checkpoints and generated assets.

**SEC-006:** Clearly disclose when script, voice, or media data is sent to cloud providers.

**SEC-007:** Allow users to select local-only generation when compatible models are available.

**SEC-008:** Restrict local worker access to approved project and model directories.

**SEC-009:** Avoid automatically executing imported scripts or downloaded model code.

**SEC-010:** Require explicit user approval for paid cloud generation when cost may be incurred.

# 18. Performance and Reliability

The desktop UI must remain interactive during AI generation, import, export, and media processing.

Long-running operations must expose status and progress where available.

Generation jobs must have stable IDs and explicit states:

- Queued.
- Running.
- Completed.
- Failed.
- Cancelled.
- Interrupted.

Interrupted operations must not damage project files.

Failed generations must preserve error information and allow retry.

Project saving must use an atomic or recoverable persistence strategy.

# 19. Acceptance Criteria

## AC-001 — Script-to-Performance

Given a supported screenplay, the user can import it, inspect parsed scenes and characters, correct errors, and generate an editable performance plan.

## AC-002 — Directed Acting

Given a dialogue line and performance instructions, the user can generate and audition multiple takes, select one, and preserve the original script.

## AC-003 — Vocal Reactions

The application can place laughter, coughing, gasps, or other supported vocal events within a performance without reading their direction text aloud.

## AC-004 — Physical Acting

The application can produce an editable hybrid performance containing speech, breathing, and effort sounds aligned to a physical action plan.

Realism must be evaluated through human listening tests.

## AC-005 — International Voices

The user can select supported language/accent combinations, generate speech, and inspect whether the combination has passed accent evaluation.

## AC-006 — Selective Regeneration

Regenerating one line creates a new take without modifying unrelated dialogue or audio assets.

## AC-007 — Soundscape

The application can construct a layered soundscape from ambience, Foley, and event effects, with each layer independently editable.

## AC-008 — Local Model Execution

On validated compatible hardware, a supported local model can generate output without transmitting generation content to a cloud provider.

## AC-009 — Provider Fallback

When a requested model is unavailable or incompatible, the application presents alternatives without silently incurring charges.

## AC-010 — Export

The user can export individual dialogue clips and supported stems, with correct durations, naming, and audio formats.

## AC-011 — Project Recovery

A project can be reopened after an interrupted generation without losing previously committed edits or assets.

## AC-012 — Video Synchronization

When video synchronization is enabled, audio cues maintain their intended frame/time positions after export within an explicitly tested tolerance.

# 20. Implementation Roadmap

## Phase 0 — Model Feasibility

- Validate cloud and local model availability.
- Check licensing and API access.
- Benchmark expressive voice acting.
- Benchmark international accents.
- Test physical acting effects.
- Assess local hardware requirements.
- Record provider capabilities.

**Exit gate:** Approved provider capability matrix and feasibility report.

## Phase 1 — Windows Foundation

- Tauri desktop shell.
- React UI and design system.
- Local project management.
- Secure settings.
- Model registry.
- Hardware detection.
- Background job management.

**Exit gate:** Stable desktop foundation and recoverable projects.

## Phase 2 — Script and Director Intelligence

- Script import.
- Scene and character parsing.
- Director Engine.
- Emotional Engine.
- PerformancePlan schema.
- Human review workflow.

**Exit gate:** Editable cinematic performance plans.

## Phase 3 — Voice Performance

- Cloud voice integration.
- Local TTS integration.
- Global voice library.
- Physical Acting Engine.
- Performance takes.
- Selective regeneration.
- Character continuity.

**Exit gate:** Validated expressive dialogue workflow.

## Phase 4 — Cinematic Sound Design

- Sound asset library.
- AI SFX generation.
- Foley cues.
- Environment profiles.
- Acoustic processing.
- Music generation.

**Exit gate:** Layered cinematic scene audio.

## Phase 5 — Production Timeline and Export

- Multitrack timeline.
- Audio mixing.
- Individual clip export.
- Character tracks.
- Scene stems.
- Export validation.

**Exit gate:** Production-ready audio delivery.

## Phase 6 — Video Intelligence

- Video import.
- Preview.
- Timeline synchronization.
- Visual scene analysis.
- Audio-to-picture alignment.

**Exit gate:** Reliable synchronized video/audio workflow.

## Phase 7 — Advanced Local Video

- Optional video model worker.
- Model management.
- Hardware-aware generation.
- Shot generation.
- Advanced synchronization experiments.

**Exit gate:** Separately validated advanced video functionality.

# 21. Claude Code — Golden Path Instructions

Claude must follow these rules when implementing Soundstage:

1. Read the approved master PRD, `dev_spec.md`, and `AGENTS.md`.
2. Treat this document as a proposed update until approved.
3. Preserve the existing architecture unless a documented technical reason requires change.
4. Do not introduce a cloud backend or database without justification and approval.
5. Implement one milestone at a time.
6. Use typed interfaces and versioned schemas.
7. Keep model integrations behind provider adapters.
8. Do not invent unsupported model capabilities.
9. Verify licenses, dependencies, and model identifiers.
10. Write tests for completed features.
11. Validate user-visible workflows, not just isolated functions.
12. Record architectural decisions in `DECISIONS.md`.
13. Record task progress and blockers in `PROGRESS.md`.
14. Do not expand the approved feature scope.
15. Stop at milestone approval gates when required.
16. Do not begin implementation until explicitly authorized.

# 22. Definition of Done

A feature is complete only when:

- Its acceptance criteria pass.
- Required automated tests pass.
- Errors and recovery states are handled.
- UI behavior is validated.
- Data persistence is verified.
- Security and licensing requirements are satisfied.
- Relevant documentation is updated.
- No critical regression is introduced.
- The implementation follows the approved architecture.

# 23. Final Product Principle

**Soundstage must operate as one intelligent cinematic production system.**

The Director Engine understands the story.

The Emotional Engine understands the character's internal state.

The Physical Acting Engine understands how actions affect vocal performance.

The Voice Engine produces the performance.

The Soundscape and Music Engines construct the surrounding world.

The Video Engine supplies visual context and synchronization.

The Timeline combines everything into an editable production.

Every module must share consistent scene, character, timing, and asset data.
