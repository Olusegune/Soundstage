# CLAUDE.md: Voice Studio agent brief

Read this file first. Then read docs/PRD.md and docs/ARCHITECTURE.md. Then read the doc that matches your task (listed below).

## Mission
Build a Windows desktop app for the complete sound side of a film or video: dialogue (multi-voice, multi-accent, multi-language, with performance direction and non-verbal sounds such as laughs, sighs and sneezes), sound effects and foley, music (library and generated), picture sync, and the mix and delivery of a finished master and stems. The app orchestrates best-in-class voice, music and effects providers and local models behind one consistent interface. We do not train our own speech or music models.

Users are professional filmmakers and content creators worldwide, across every genre: drama, romance, comedy, sci-fi, animation, documentary, and children's content. The product must be reliable enough for professional production and approachable enough for a first-time creator.

Working title "Voice Studio"; the product name is pending. Do not hard-code the name into code, package IDs, or user-facing copy until it is confirmed.

## Owner
Victor (Wheelbarrow Studios) is the product owner. Commercial use is expected, so licensing is a hard requirement, not a footnote.

## Non-negotiables
1. **Consent gate.** No voice clone is created or used without a consent record (see docs/SAFETY_LICENSING.md). Never bypass this, including for testing with real people's voices.
2. **Licence gate.** Every voice, model, music track, sound effect and generated asset carries a licence tag: `commercial`, `non-commercial`, or `unknown`. Export of a project marked commercial must refuse non-commercial or unknown-licence assets and explain why.
3. **Provider-agnostic core.** UI code and project files never reference a provider's native tag syntax or voice IDs directly. Everything goes through the canonical layer in docs/PROVIDERS.md.
4. **Secrets stay out of the repo.** Provider keys live in the Windows Credential Manager. Never commit keys, log them, or put them in URLs.
5. **Verify provider facts.** Model names, tags, languages, prices and licences change often. Anything marked `VERIFY` in docs/PROVIDERS.md must be confirmed against the provider's current documentation before it is coded or promised to a user.
6. **No dark patterns.** No fake countdowns, no invented scarcity, no inconsistent pricing. Upgrade and pricing copy must match the billing system.
7. **Honest quality claims.** Do not claim an accent, language or non-verbal sound works unless the bake-off (docs/TEST_PLAN.md) shows it does for that provider.
8. **Content rules are per project.** Adult fiction with fictional adult characters is allowed in projects rated adult, with age-gating. Real-person, minor, and fraud rules apply to every project in every genre (docs/SAFETY_LICENSING.md).
9. **Performer rights.** A performer's voice, performance reference, or likeness is used only under that performer's consent record, and any AI use is disclosed in the project manifest (docs/SAFETY_LICENSING.md section 10).

## Working style
- Small, verifiable steps. Each task ends with tests passing and a short PR description covering what changed and what was verified.
- Prefer typed, boring code. TypeScript strict mode in the frontend. Rust for the core. Python only in the local inference sidecar.
- Write the contract test before the adapter. Every provider adapter must pass the shared suite in docs/TEST_PLAN.md.
- Build to the phase in docs/PRD.md section 4a. Features marked v2 or v3 are designed for, but not built, in v1.
- If a requirement conflicts with a non-negotiable, stop and ask Victor. Do not work around it silently.
- Ask before: adding a paid provider, adding a runtime dependency over about 50 MB, or sending user audio or scripts to a new network host.
- Never send Victor's email or other personal data to a provider, URL or service unless he asks.

## Repo layout (target)
```
voice-studio/
  CLAUDE.md          this brief
  docs/
    PRD.md           what we build and why, with acceptance criteria
    ARCHITECTURE.md  components, data model, pipeline, sidecar
    SOUND_DESIGN.md  scenes, takes, performance direction, ADR, picture sync, mix and delivery
    MUSIC_AND_SFX.md music and sound effect sourcing, cues, scoring, licensing
    PROVIDERS.md     canonical tags, provider matrix, routing, adapter contract
    SAFETY_LICENSING.md  consent, licence tags, provenance, refusals
    UX_GOLDEN_PATH.md    screens, flow, interaction rules, visual direction
    TEST_PLAN.md     contract tests, bake-off protocol, listening tests, mix tests
  app/               Tauri 2 shell
    src/             React + TypeScript UI
    src-tauri/       Rust core: project store, job queue, pipeline orchestration
  sidecar/           Python service for local models
  adapters/          one folder per provider, each with contract tests
  fixtures/          test scripts, reference clips (each with a consent record), golden audio
  tools/             bake-off harness, loudness checker, listening-test exporter
```

## Which doc to read for which task
- Anything touching the UI: UX_GOLDEN_PATH.md
- Any new voice provider or model: PROVIDERS.md, then TEST_PLAN.md
- Any scene, take, direction, picture sync, ADR, or mix and delivery work: SOUND_DESIGN.md
- Any music or sound effect source, cue, or scoring work: MUSIC_AND_SFX.md
- Any clone, export, or licence logic: SAFETY_LICENSING.md
- Pipeline, storage, jobs, sidecar: ARCHITECTURE.md
- Scope and phasing: PRD.md (non-goals and section 4a are binding)
- Directing and acting tools: PRD.md FR-14 and FR-15, SOUND_DESIGN.md section 9

## Definition of done (every feature)
- Acceptance criteria in the PRD are met, shown by a recorded clip or screenshot.
- Adapter contract tests pass for every adapter touched.
- Every new `VERIFY` item is logged in PROVIDERS.md with an owner.
- Consent and licence checks cover any new path that creates or exports audio.
- Operable by keyboard and passes the checks in UX_GOLDEN_PATH.md.

## Open questions for Victor (do not guess)
- Pulseframe visual reference (screenshots, tokens, or a description). The UI spec depends on it.
- Launch languages and accents, in priority order.
- Business model: sold, licensed, or internal first.
- Which providers have API access and budget.
- Native-speaker reviewers available for listening tests (Nigerian, Filipino, Indian, Korean).
- Confirm what "golden path" means. This brief assumes a linear flow: script, cast, direct, render, mix, export.
- Frame rates and whether picture reference import is needed in v1 (SOUND_DESIGN.md section 8).
- Which music and SFX libraries the team already has access to (MUSIC_AND_SFX.md section 9).
- Whether generated music is in scope for v1.
- Final product name.
- Target languages for v1 (proposed: English, Nigerian Pidgin, Yoruba, Hindi, plus one to confirm).
- Which regional Indian languages come first after v1.
- Whether performance reference capture should be in v1 (currently planned for v2).
