# Sound design: scenes, performance, picture sync and delivery

Status: Draft v0.1. Scope is the full sound side of a film or video: dialogue (voice, performance and direction), sound effects (see MUSIC_AND_SFX.md), music (see MUSIC_AND_SFX.md), and the mix that brings them together for delivery. Working title "Voice Studio" is provisional; the product name is pending.

## 1. What "sound for picture" means here
A finished scene has four kinds of sound:
1. **Dialogue**: spoken lines, performed by voices.
2. **Effects (SFX and foley)**: door slams, footsteps, ambience, impacts.
3. **Music**: score, source music, stingers.
4. **Mix**: balance, space, and loudness, delivered to a target platform.

The app treats these as one timeline, not three tools. A line, an effect and a music cue placed on the same frame are edited together, and the mix reflects that.

## 2. Core concepts

| Concept | Meaning |
|---|---|
| **Project** | One production (an episode, a short, a film). Contains scenes, cast, and settings. |
| **Scene** | A continuous piece of picture with its own location, time, and cast. Carries the scene's audio context (room, mood, action). |
| **Cue** | Any sound placed on the timeline at a time: a line, an effect, a music entry. |
| **Take** | One rendered or recorded version of a line. A line can have many takes; one is selected. |
| **Direction** | Instructions that shape a performance: emotion, intent, pace, pauses, non-verbal sounds, and scene context. |
| **Stem** | A mixed group of one kind of sound (dialogue, effects, music), exported separately. |

## 3. Performance and direction

### 3.1 Levels of direction
Direction is applied at four levels, from broad to fine. Each lower level overrides the one above it.
1. **Project style**: default voice character and delivery for the whole project (for example, "warm, slightly playful, children's educational").
2. **Character**: each speaker's baseline (age, energy, habits, accent, and a short note such as "tired after a long day").
3. **Scene**: the scene's emotional arc and setting ("kitchen, morning, rushed").
4. **Line**: the specific delivery for one line (emotion, pace, pauses, non-verbal sounds, stress on words).

Example: the project says "warm", the character says "energetic", the scene says "late night, tired", and one line says `[sigh] [pause:short] Fine.` The line wins for that line only.

### 3.2 Word-level control
- Stress, pitch, and emphasis on a single word, using a canonical markup (proposed `*word*` for emphasis, with the mapping to each provider's native control in PROVIDERS.md).
- Pauses in milliseconds (`[pause:350]`).
- Non-verbal sounds as inline tags (PROVIDERS.md section 2).
- A tag the voice cannot perform is flagged before render, never dropped silently.

### 3.3 Takes and selection
- Every render creates a take. Takes are kept until the user deletes them.
- The user can audition takes side by side, pick one, and keep the others for later.
- The selected take is what the timeline plays and what export uses.
- Takes record the direction and provider version used, so a take can be re-created.

### 3.4 Performance consistency across a scene
- A "scene preview" plays all lines in order at current settings, with the correct gaps and overlaps.
- A "consistency check" flags a character whose delivery shifts sharply between adjacent lines without a direction change (for example, a sudden change in pitch range). Advisory only.
- Speakers can be locked to a voice for the whole project. Changing voice mid-project is an explicit action, with a warning.

### 3.5 Replacing a line (ADR)
Automated dialogue replacement: re-render one line against the same picture timing.
- The replacement keeps the original line's start time and total duration (within a tolerance the user sets).
- The app shows the original and new take side by side, with the timing difference in milliseconds.
- If the replacement runs long, the app offers a rewrite of the line or a timing stretch, and says which one it applied.

## 4. Picture sync
- **Timecode:** the timeline uses SMPTE timecode (frames) for picture-linked projects, and milliseconds for audio-only projects. The user chooses at project creation. Frame rates supported: 23.976, 24, 25, 29.97, 30 (PROPOSED).
- **Markers:** the user can place markers on the timeline ("shot 12 starts", "door opens") and attach cues to them.
- **Picture reference:** the app can import a reference video (proxy, low resolution) so the user can place and check sound against picture. The reference is never exported. PROPOSED for M3.
- **Sync checks:** a cue that overruns its marker or scene boundary is flagged.
- **Edit decision list (EDL) and XML exchange:** import and export of timeline cues to common NLEs. PROPOSED for a later milestone, once the core timeline is stable.

## 5. Delivery and mixing

### 5.1 Stems and mix buses
Standard mix structure, which the export always follows:
- **Dialogue bus**: all voice takes.
- **Effects bus**: SFX and foley.
- **Music bus**: score and source music.
- **Master**: the sum, with loudness processing.

Ducking: music and effects duck under dialogue by a configurable amount, with attack and release times. Defaults are set per target (section 5.2).

### 5.2 Loudness targets by destination
Each export is measured against a target chosen by the user. Values are starting points and must be verified against the current standard for each destination before the feature ships.

| Destination | Integrated target (starting point) | True peak | Note |
|---|---|---|---|
| Streaming and online video | around -14 LUFS | VERIFY | Platform normalisation varies |
| Broadcast (EBU) | -23 LUFS | VERIFY | EBU R128 |
| Broadcast (US, dialogue-gated) | -24 LKFS | VERIFY | ATSC A/85 |
| Podcast | around -16 LUFS | VERIFY | Common practice; confirm per distributor |
| Cinema | VERIFY per format | VERIFY | Mixed by a cinema standard, not a single loudness number |

Rules:
- The app measures and reports integrated loudness, short-term loudness, true peak, and loudness range.
- A mix that cannot reach the target without clipping is flagged, not silently limited.
- The export manifest records the target used.

### 5.3 Export formats
- Mixed master: WAV (24-bit, 48 kHz default), MP3 for delivery.
- Stems: dialogue, effects, music, each as WAV, with the same start time and length so they re-align in an NLE.
- Full-scene and per-cue export.

## 6. Quality bar for sound design
- Dialogue is intelligible at target loudness on a laptop speaker and in a headphone mix (a reference check on both).
- No unintended clicks at cue boundaries (checked automatically).
- Effects and music never mask dialogue beyond the ducking setting.
- Silence is intentional: gaps longer than 3 seconds are flagged for review, not removed.

## 7. Acceptance criteria (summary)
- A two-speaker, two-scene episode with dialogue, effects and music can be built, mixed, and exported as a master plus three stems.
- A single line can be replaced against picture with the original timing kept within the user's tolerance.
- Direction set at project, character, scene and line level resolves in the documented order, and the resolved direction is visible for each line.
- The mix meets the chosen loudness target, and the export manifest records it.

## 8. Open questions
1. Frame rates to support at launch.
2. Whether picture reference import is needed in v1 or later.
3. Loudness defaults for the first destinations we ship for.
4. Whether to include EDL and XML exchange in v1.

## 9. Directing and acting workflows
These tools sit on top of the sound model in sections 2 to 5. Phasing is in PRD section 4a.

### 9.1 Directing (director and script supervisor)
- **Script in:** plain-text cue format in v1; Fountain screenplay import in v2. Scene headings become scenes, and character names become speakers.
- **Breakdown:** a scene list with cast, location, time of day, and status (not started, in progress, approved).
- **Notes:** director's notes attach to a scene, a line, or a take, and persist with the project.
- **Take review:** takes are compared side by side, with the direction and provider version shown for each. Each take is approved, held, or rejected. Approvals appear in the export manifest.

### 9.2 Acting (actor and voice talent)
- **Character bible:** age, background, speech habits, emotional range, and notes for each character. In v1 it is text used as context, and direction uses it where the provider accepts context.
- **Read-through:** a scene plays in the cast's voices with correct gaps, so actors hear it in context before recording. Preview in v1; full read-through with stage notes in v2.
- **Performance reference (v2):** an actor records a guide take. The app uses its timing, stress and emotional contour as direction for the chosen voice. Speech-to-speech output is used only where verified (PROVIDERS.md section 8), and only under that performer's consent record (SAFETY_LICENSING.md section 10).
- **Performer control:** a performer can see every take made from their reference and revoke its use at any time.

### 9.3 Both
- Notes, approvals and direction history are part of the project. Any collaborator can see why a take was chosen.
