# PRD: Voice Studio

Status: Draft v0.1. Owner: Victor. Open questions are listed at the end.

## 1. Problem
Creators who make educational, animated, or narrative content need the whole sound side of a production: voices that sound authentic to their audience, performances that can be directed scene by scene, effects, music, and a mix delivered to the right platform. Existing tools fall short in one or more ways:
- Character voices often cannot laugh, cough, or sneeze convincingly.
- Many tools cannot produce authentic Nigerian, Filipino, Indian, or Korean accents, or cannot direct them.
- Dialogue, effects, music and mixing live in separate tools, so timing and loudness drift between them.
- Each tool has its own tags, voices and licences, so switching providers means rewriting the work.
- Music and effects are often unlicensed for commercial use, or unclear about it.

## 2. Users
Professional filmmakers and creators worldwide, across genres. Each persona has a primary workflow in the product:
- **Director:** breaks down scenes, gives notes, reviews and approves takes.
- **Sound designer or post supervisor:** builds the mix, manages stems, meets loudness and delivery specs.
- **Actor or voice talent:** records a guide performance (v2), hears the read-through, and controls use of their own voice and performance.
- **Writer:** imports or writes the script and keeps it in sync with the cues.
- **Producer:** checks cost, licence status, approvals and the delivery manifest.
- **Independent creator:** works alone through the golden path, with minimal setup.
- **Localisation lead (v3):** produces versions of one project in several languages.

Victor and the Wheelbarrow Studios team are the first users, and they are not the design centre.

## 3. Goals
1. Turn a script into finished, directed, multi-voice audio in under 30 minutes for a 5-minute episode, after setup.
2. Support performance direction at four levels (project, character, scene, line) and at word level: emotion, pace, pitch, pauses, stress, and non-verbal sounds.
3. Offer a voice library spanning accents and languages that is tested by native speakers, not marketed by demo clips.
4. Build a complete sound-for-picture mix: dialogue, effects, music, ducking, and stems, with a master delivered to a chosen loudness target.
5. Let users replace a line against picture (ADR) with its original timing kept.
6. Provide a licensed source for music and effects, library or generated, with the same licence gate as voices.
7. Stay provider-agnostic, so the best engine for each language, sound or need can be chosen and replaced.
8. Make licence status and consent visible and enforced, so exported work is safe to sell.
9. Deliver a sleek, fast Windows desktop experience with a guided golden path.
10. Give directors, actors, sound designers and producers tools that match how they already work: script import, scene breakdown, notes, take review, and performance direction.
11. Work reliably for professionals in any market, with language and rights coverage that grows by phase.

## 4. Non-goals (binding for v1)
- Training or fine-tuning our own speech or music model.
- Real-time voice conversion or live streaming voice changer.
- Lip-sync dubbing of video (possible later; out of scope for v1).
- Full video editing. The app places sound against picture; it does not cut or grade picture. A low-resolution picture reference is allowed (SOUND_DESIGN.md section 4).
- Cinema-grade print mastering (Dolby Atmos, object-based audio). Stereo and 5.1 stems are in scope later; immersive formats are not in v1.
- Cloning any real person's voice without a verified consent record.
- macOS and Linux builds (the Tauri core keeps them possible later).
- Mobile.
- Real-time multi-user editing in v1 (team review is v2; see section 4a).

## 4a. Product phasing
v1 is the first release. v2 and v3 are designed for in v1 and built later.

**v1 (first release, Windows)**
- Dialogue with cloud voices from at least two providers, plus designed voices.
- Direction at four levels, word-level control, canonical non-verbal tags.
- Scenes, takes, a basic character bible, and a plain-text cue format for scripts.
- Dialogue, effects and music buses, ducking, loudness per destination, stems.
- Import of licensed music and effects from libraries. No generated music or effects.
- Basic frame-based timecode and markers for picture-linked projects. No picture reference and no ADR.
- Licence- and consent-gated export, with a content rating recorded per project.
- Languages that pass the bake-off in PROVIDERS.md section 7. Proposed: English, Nigerian Pidgin, Yoruba and Hindi, plus one to be confirmed.
- Read-through preview of a scene in the cast's voices.

**v2 (after the v1 pilot)**
- Consent-gated voice cloning (FR-5).
- Fountain screenplay import (FR-14).
- Performance reference capture and speech-to-speech direction, where verified (FR-15).
- Director review: notes, approvals, full take comparison, and a read-only review link.
- ADR, picture reference, generated effects and music.
- More languages as each passes the bake-off: Igbo, Hausa, Indian regional languages, Tagalog and Filipino English, Korean, Brazilian Portuguese.
- Local open-weight models through the sidecar.

**v3 (later)**
- Interchange with NLEs and DAWs (EDL, XML, AAF).
- Localisation: dub versions for each language from one project.
- Team accounts and roles.
- Regional settings for currency, payment, tax and data residency.
- Cinema-style deliverables (5.1 stems) and macOS.

## 5. The golden path
One linear flow that a first-time user can finish without reading docs:

1. **Script.** Paste or import text. The app parses speakers (`NAME:` lines or a simple cue format) and flags non-verbal tags.
2. **Cast.** Assign a voice to each speaker. The app suggests voices by language and accent, and plays a sample of each script line in that voice.
3. **Direct.** Adjust each line: emotion, pace, pauses, and non-verbal sounds, using the canonical tag set. Preview any line in one click.
4. **Render.** Generate audio. Jobs run in the background with per-line progress, caching and retry.
5. **Mix.** Place lines on a timeline with music and SFX. Set levels, ducking, and loudness target.
6. **Export.** Choose format (WAV, MP3, or stems). The licence check runs first and reports anything blocking.

Acceptance: a new user completes steps 1 to 6 on a 5-minute, two-speaker script with no documentation.

The golden path covers the first pass. Scene-based production, picture sync, effects and music follow the same flow for each scene, as described in SOUND_DESIGN.md and MUSIC_AND_SFX.md.

## 6. Functional requirements

### FR-1 Script editor
- Accepts plain text and a simple cue format. Round-trips without data loss.
- Shows speaker list and per-line status (drafted, previewed, rendered, stale).
- AC: Editing a line marks only that line and its render as stale.

### FR-2 Voice library
- Lists voices with provider, language, accent, gender, age range, licence tag, and a sample.
- Filter by language, accent, and non-verbal capability.
- AC: A voice missing a licence tag cannot be exported to a commercial project.

### FR-3 Direction
- Canonical tag set (docs/PROVIDERS.md, section 2). Tags render per provider capability.
- Non-verbal sounds: laugh, chuckle, sigh, cough, sneeze, gasp, cry, breath.
- Per-line controls for emotion, pace, and pause length.
- AC: A tag a provider does not support is shown as "not supported by this voice" before render, never silently dropped.

### FR-4 Voice design
- Create a voice from a text description (age, accent, pitch, pace, tone) where a provider supports it.
- Save designed voices to the library with their provider, parameters, and licence tag.
- AC: A designed voice's parameters are stored and can regenerate the same voice where the provider allows.

### FR-5 Voice cloning (gated)
- Available only after a consent record is verified (SAFETY_LICENSING.md).
- AC: Cloning without a complete consent record is impossible in the UI and rejected by the core.

### FR-6 Rendering
- Background job queue. Per-line caching keyed by text, voice, parameters and provider version.
- Retry with backoff. Clear error per line. Local-model fallback when the network is down, where a local voice exists.
- AC: Re-rendering an unchanged project makes zero provider calls.

### FR-7 Mix and export
- Timeline with dialogue, effects, and music buses. Gain, fade, ducking under dialogue (SOUND_DESIGN.md section 5).
- Loudness normalisation to a user-selected destination target (SOUND_DESIGN.md section 5.2). Targets are VERIFY before shipping.
- Export WAV, MP3, a master, and per-bus stems at matching lengths.
- AC: Export refuses blocked licences and lists each blocking asset.
- AC: A mix that cannot reach the target without clipping is flagged, not silently limited.

### FR-10 Scenes, takes and performance direction
- Projects are organised into scenes. Each scene has location, time, cast, and an emotional note.
- Direction resolves at four levels (project, character, scene, line). The resolved direction is shown for each line.
- Each render creates a take. The user can audition takes side by side and select one.
- AC: Changing a scene note changes the resolved direction of its lines and marks them stale, and nothing else.

### FR-11 Picture sync and ADR
- Timecode in frames for picture-linked projects; markers on the timeline; cues attached to markers.
- Replace a line against picture: the original start time and duration are kept within a user tolerance, with the timing difference shown in ms.
- AC: A replaced line that runs long offers a rewrite or a stretch, and states which one was applied.

### FR-12 Music and sound effects
- Library and generated sources, each with a licence tag and recorded terms (MUSIC_AND_SFX.md section 2).
- Effects: search, placement on markers, gain, fade, pan, ducking exemption, layering into presets.
- Music: cues with scene and hit-point markers, trim and loop, ducking under dialogue, and source music treated as an effect.
- AC: A music or effect asset with `unknown` licence blocks commercial export and is listed by name.

### FR-13 Stems and delivery
- Stems for dialogue, effects, and music, each the same length as the master and aligned to the scene start.
- AC: Importing the master and all stems into a standard editor and summing them reproduces the master within a stated tolerance (TEST_PLAN.md section 3a).

### FR-8 Projects and storage
- Project file is human-readable JSON plus an assets folder. Versioned schema.
- Autosave. Crash recovery.
- AC: Opening a project saved by an older version migrates it or says why it cannot.

### FR-9 Settings and keys
- Provider keys stored in Windows Credential Manager. Test-connection button per provider.
- AC: Keys never appear in logs, project files, or error reports.

### FR-14 Directing tools (Phase: v1 basic, v2 full)
- Script in: plain-text cue format (v1); Fountain screenplay import (v2). Scene headings become scenes, character names become speakers.
- Breakdown view: scenes with cast, location, time of day, and status (not started, in progress, approved).
- Director's notes attach to a scene, a line, or a take, and persist with the project.
- Take review: side-by-side comparison showing the direction and provider version used. Each take is approved, held, or rejected.
- AC: An approval and its note history persist across save and reload, and appear in the export manifest with the approver's identifier.

### FR-15 Acting tools (Phase: v1 basic, v2 full)
- Character bible per speaker (age, background, speech habits, emotional range, notes). In v1 it is context for direction, where the provider accepts it.
- Read-through: a scene plays in the cast's voices with correct gaps (preview in v1; full read-through with stage notes in v2).
- Performance reference (v2): an actor records a guide take. The app extracts timing, stress and emotional contour and uses them as direction. Speech-to-speech is used only where verified per provider.
- AC: A performance reference is used only if the performer's consent record covers that use. Without it, the feature is disabled for that performer.

### FR-16 Global scope and language (Phase: v1 to v3)
- Languages follow section 4a. A language is listed as supported only after it passes the listening test for at least one voice.
- Content category and rating per project (SAFETY_LICENSING.md section 7).
- Currency, payment and tax settings per region (v3). Data residency options (v3).
- AC: The UI never lists a language as supported without a bake-off result behind it.

## 7. Experience principles
- The golden path is the default. Advanced panels are one click away, never required.
- Show cost and licence status before work is spent, not after.
- Every generated line can be previewed, changed, and re-rendered alone.
- Dark, calm, high-contrast interface with a single accent colour. Match Pulseframe once its reference is supplied.

## 8. Quality bar
- Accent and language quality is judged by native-speaker listening tests (TEST_PLAN.md). A provider is listed as "supported" for an accent only after it passes.
- Non-verbal sounds judged for realism, timing, and whether they fit the line.
- Clipping, silence gaps, and loudness are checked automatically on every render.

## 9. Success metrics (first 90 days)
- Golden path completed without help by 3 of 3 pilot users.
- The v1 pilot includes users in at least three countries and at least two genres (for example drama, comedy, animation or documentary).
- Median render time under 1x real-time for a 5-minute script, for cloud voices (VERIFY on pilot hardware).
- Zero exports shipped with a blocked licence, measured by the export gate's logs.
- Listening-test pass rate reported per language and accent, published internally.

## 10. Milestones
- **M0: Bake-off.** 3 to 5 voice providers tested on the same 10 to 15 scripts. Music and effects sources identified and licence-checked. Output: PROVIDERS.md and MUSIC_AND_SFX.md filled in with evidence.
- **M1: Core.** Project store, script parser, scenes and takes, one cloud voice adapter, render queue, WAV export.
- **M2: Direction and casting.** Four-level direction, word-level control, voice library, golden path screens 1 to 4.
- **M3: Mix and licence gate.** Dialogue, effects and music buses, ducking, loudness targets, licence-gated export, stems.
- **M4: Sound for picture.** Timecode and markers, ADR, effects library and placement, music cues with hit points, picture reference.
- **M5: Consent, cloning and local models.** Consent records, gated cloning, second voice adapter, local sidecar with one open-weight model, generated effects.
- **M6: Pilot.** Three internal users on a real episode with scenes, music and effects. Feedback, polish, packaging.

Phase mapping: M0 to M3 deliver v1, and M4 and M5 are v2. M6 is the v1 pilot, run with users across at least two genres. The v2 pilot follows M5.

## 11. Risks
| Risk | Mitigation |
|---|---|
| Provider licence forbids commercial resale of output | Licence tag per voice and model, export gate, read terms before promising anything |
| Accent quality is weak for a target language | Bake-off with native speakers; say "not yet supported" honestly |
| Voice clone misuse or impersonation | Consent gate, no public-figure cloning, provenance marking |
| Provider tags or models change without notice | Adapter contract tests run on a schedule; PROVIDERS.md dated |
| Cost overruns from re-rendering | Caching, cost preview before render, per-project budget |
| Non-commercial open models mistaken for commercial | Licence field required; "unknown" blocks commercial export |
| Scope creep into dubbing, picture editing, or real-time | Non-goals are binding for v1 |
| Generated music or effects carry unclear ownership or terms | Licence check per source before commercial use; generated audio off by default until cleared |
| Music and dialogue mixes fail on different playback systems | Reference checks on speakers and headphones; loudness reported per destination |
| Timing drift between stems in the editor | Stems share start time and length; sync test in TEST_PLAN.md |
| Performer consent or union rules block the reference-performance feature in some markets | Performer consent records; per-market rights check (VERIFY) before v2; feature off where unresolved |
| Language coverage costs more than planned | Phase languages through the bake-off; never list a language without evidence |
| Speech-to-speech output is weak or narrowly licensed | Verify per provider before v2; fall back to tags and direction |

## 12. Open questions
1. Pulseframe visual reference.
2. Launch languages and accents, in priority order.
3. Business model and pricing (sold, licensed, or internal first).
4. Provider budget and access.
5. Native-speaker reviewers.
6. Exact meaning of "golden path" (this PRD assumes a linear flow).
7. Frame rates and picture reference import in v1 (SOUND_DESIGN.md section 8).
8. Music and effects libraries the team already has (MUSIC_AND_SFX.md section 9).
9. Whether generated music is in v1.
10. Final product name.
11. Target languages for v1 (section 4a).
12. Whether v1 includes basic frame timecode, as section 4a plans.
13. Per-market performer rights rules before v2 (SAFETY_LICENSING.md section 10).
