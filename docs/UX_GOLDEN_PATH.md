# UX: Golden path and interface

Status: Draft v0.1. The visual direction is blocked on the Pulseframe reference. Until it arrives, use the tokens in section 7 as placeholders, and do not present them as final.

## 1. The flow
One linear path, six steps. A persistent step bar at the top shows where the user is and lets them jump back, never forward past an incomplete step.

| Step | Purpose | Done when |
|---|---|---|
| 1. Script | Write or import the text | At least one speaker line exists |
| 2. Cast | Assign voices to speakers | Every speaker has a voice |
| 3. Direct | Set emotion, pace, pauses, non-verbal sounds | Optional, but the user sees the controls |
| 4. Render | Generate audio | All lines rendered or explicitly skipped |
| 5. Mix | Arrange timeline, music, SFX | Optional. Default is voice track only |
| 6. Export | Choose format, pass the licence gate, write files | Export manifest written |

A new user should be able to go from an empty project to an exported file in one session, using only the step bar and on-screen hints.

## 2. Screens

### 2.1 Project home
- Recent projects (name, last edited, status), a "New project" action, and an "Import script" action.
- Show a licence summary per project: "Personal" or "Commercial target, 2 items need attention".

### 2.2 Script (step 1)
- Left: script editor, one line per row. Speaker shown as a chip. Typing `NAME:` creates a speaker.
- Right: speaker list, line count, and a validation panel.
- Tags typed as `[laugh]` are highlighted. Unknown tags underlined with a plain message.
- Keyboard: Enter for a new line, Tab to change speaker, Ctrl+Enter to preview the current line (available from step 2).

### 2.3 Cast (step 2)
- Speaker cards. Each card: current voice, language, accent, a play button for a sample line from the script.
- "Find a voice" opens the voice library, filtered by language and accent, with licence tag visible on every voice.
- Voices that cannot render the script's tags show a warning icon with the reason.

### 2.4 Direct (step 3)
- Line list with per-line controls: emotion, pace, pause before, non-verbal insert.
- Preview button on each line. Preview renders just that line, and is cached.
- A "Sound check" view plays the whole scene at the current settings.

### 2.5 Render (step 4)
- Job list with per-line status: queued, rendering, rendered, failed, stale.
- Shows estimated cost where the provider gives an estimate, before the user confirms a batch.
- Failed lines show the error in plain language and offer retry or a different voice.

### 2.6 Mix (step 5)
- Timeline with voice, music, and SFX tracks. Zoom, snap to lines, ducking control on music.
- Loudness meter visible.

### 2.7 Export (step 6)
- Format and quality options. Destination folder.
- "Check before export" runs the licence gate and lists items. Blocking items each have an action: replace voice, mark as personal target, or open the licence source.
- The export button stays disabled while blocking items remain, with the reason next to it.

### 2.8 Voice library
- Grid or list. Each voice: sample, provider, language, accent, gender, age, licence tag with check date.
- Filter bar: language, accent, non-verbal capability, licence.
- Voice design panel: text description and sliders for age, pitch, pace, tone. Generate three variants, keep one.

### 2.9 Settings
- Provider keys, each with a test-connection button and last-checked time.
- Local models: install, size on disk, licence shown before download.
- Consent records and cloning (behind a clear consent flow, section 4).

## 3. Interaction rules
- Every action that spends money or sends data says so before it runs.
- Nothing is deleted without a confirmation that names the thing being deleted.
- Undo for line edits, direction changes, and timeline moves (at least 20 steps).
- Long jobs never block the UI. Progress is visible from any screen.
- Errors say what happened, what it means for the project, and what to do next. No raw stack traces in the UI; they go to the log.
- Empty states offer one next action.
- Keyboard: every primary action is reachable without a mouse. Visible focus ring.
- Tooltips never hide required information.

## 4. Consent flow (cloning)
1. User chooses "Clone a voice" and is told it requires the voice owner's consent.
2. The app shows the consent statement and asks the subject to record their attestation in their own words, or to sign the form.
3. Reference audio is uploaded and checked. Results shown plainly.
4. The consent record is created and the clone job runs only after all steps pass.
5. A "Revoke consent" control is visible on the voice at all times.

## 5. Accessibility
- WCAG 2.1 AA contrast for all text and controls (4.5:1 body, 3:1 large text and UI components).
- Full keyboard operation. Screen-reader labels on all controls. No information carried by colour alone (licence and error states include text or an icon).
- Respect Windows reduced-motion setting.
- Minimum target size for controls: 24 by 24 px.

## 6. Writing voice in the UI
- Plain and direct. Short sentences. No jargon in primary text; technical detail goes behind "Details".
- No urgency tricks, countdowns, or scarcity language.
- Pricing and upgrade copy is generated from the billing source, not typed by hand in screens.

## 7. Visual direction (placeholder until Pulseframe is supplied)
- Dark theme default, light theme available. Both meet contrast requirements.
- One accent colour for primary actions. Neutral greys for everything else. Colour is never the only signal.
- Typography: one sans-serif family for UI, one monospace for tags and timing.
- Spacing scale: 4 px base. Radius: 8 px cards, 6 px controls.
- Motion: short (150 to 200 ms), purposeful, disabled under reduced motion.
- Waveforms and the timeline are the visual signature. Keep them clean and legible at small sizes.

Design tokens will live in `app/src/styles/tokens.ts` and be derived from the Pulseframe reference once supplied.

## 8. Directing, acting and review screens
(v1 unless marked v2)

- **Script import (v1 text, v2 Fountain):** paste or open a file, see detected scenes and speakers, fix them, then continue.
- **Scene breakdown (v1):** a list of scenes with cast, location, time of day, and status. Clicking a scene opens its lines.
- **Character bible (v1):** one card per character. Fields: age, background, speech habits, emotional range, notes.
- **Read-through (v1 preview, v2 full):** plays a scene in the cast's voices, labelled "preview".
- **Take review (v1 basic, v2 full):** side-by-side takes with the direction shown beside each. Approve, hold, or reject.
- **Performance reference (v2):** records a guide take. Shows consent status before recording starts. Refuses to use a reference without a consent record.
- **Director review link (v2):** a read-only link to a scene's takes and notes. Viewers can comment but cannot change takes.

## 9. Screens to review before any build sign-off
- Golden path, first run, with no prior knowledge.
- Direction with an unsupported tag on screen.
- Export with one blocking licence item.
- Consent flow with a refused reference clip.
- Each screen at 1280 by 720 and at 1920 by 1080, in light and dark themes.
