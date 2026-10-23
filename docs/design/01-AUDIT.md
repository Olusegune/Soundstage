# UI/UX audit of Soundstage v0.1

Method: I ran the built v0.1 app through the golden path in its real window (screenshots in `docs/design/before/`), read the code, and compared it with FilmCrafter's workflow and docs. This is a heuristic audit by the author of v0.1. It is not user research.

## What v0.1 gets right (kept)
- The licence and consent rules are visible where decisions happen, with plain-language reasons. Do not lose this.
- Every consequential action says what it does first (send text, delete, change voice).
- Step gating that explains itself, undo, autosave, and honest "not supported by this voice" messages.
- One idea per screen for Script, Cast, Render and Export.

## Shortcomings, by impact

| # | Finding | Evidence | Why it matters |
|---|---|---|---|
| 1 | **The Golden Path is a gated tab bar, not guidance.** It says "step 3 of 6" and greys out later steps. It never says why a step matters, what skipping costs, or what to do next. | `App.tsx` step bar; blocked steps only show a tooltip. | A beginner can finish a step and still not know the next action. The brief calls this non-negotiable. |
| 2 | **Audio is invisible.** An audio app with no waveforms. A take is a line of text and a Play button; the mix is blue rectangles. | Render and Mix screens. | The output is the hero and it is not shown. Hard to judge a take, spot silence or clipping, or trust the mix. |
| 3 | **Generic look.** Default blue accent borrowed from Pulseframe, flat grey cards, default `<select>` and range controls, 6 px everywhere. No sense of place or brand. | All screens. | Reads as a dev tool, not a premium creative environment. The supplied brand art (gold and teal on near-black) is not reflected anywhere. |
| 4 | **Cluttered top bar.** Project name input, licence target, rating, save state, Voices and Settings share one row with no hierarchy. | Header in every project screen. | Project-level decisions compete with navigation. Rating and target are rarely changed but always shown. |
| 5 | **No command palette or shortcut system.** Only Ctrl+Z, and Ctrl+Enter in one field. No reference, no customisation. | `App.tsx` key handler. | Power users must mouse everywhere. Nothing is discoverable. |
| 6 | **No help, onboarding or glossary.** The first screen is a form. Terms like LUFS, ducking, true peak, stem, take and cache are unexplained. | Home, Mix, Export. | A first-time user has nowhere to learn. Mix and Export are jargon-heavy. |
| 7 | **Same layout everywhere.** Every step is a stack of cards on one centred column. | All steps. | Tasks differ (writing, casting, reviewing, mixing) but the workspace never adapts. |
| 8 | **Direct and Mix expose every control at once.** Pace, pause, notes, ducking depth, fades and loops are all visible to a first-time user. | Direct line cards, Mix cue cards. | Overwhelms beginners. There is no Simple mode. |
| 9 | **Render is a dense table.** Line, text, status, four small buttons, and a hidden takes drawer. | Render step. | Takes (the creative choice) are buried; status chips are small and low-contrast. |
| 10 | **Empty states and home are thin.** Home is a form plus a card grid. No quick start that produces a real result. | Home. | The first minute decides whether a user continues. |
| 11 | **Light theme missing.** Only dark. | Global. | The brief asks for an equally considered light mode. |
| 12 | **Contrast and targets are unchecked.** Muted text (`#98a2b1` on `#171b22`) passes, but some chip and border pairs were never measured. Small buttons are 24 px. | CSS. | WCAG 2.2 AA needs measuring, not hoping (target size 24 px is the minimum, so some are borderline). |
| 13 | **Decorative-free is not the same as designed.** Spacing is uniform; there is no hierarchy between primary, secondary and tertiary actions. Several equal-weight buttons per row. | Render rows, Cast cards. | The eye has nowhere to go. |

## Severity summary
Serious: 1, 2, 3, 5, 6. Moderate: 4, 7, 8, 9, 10, 11. Minor: 12, 13.

## Constraints that shape the fix
- Preserve the data model and every gate (licence, consent, send confirmation). The redesign changes presentation and guidance, not rules.
- The app is a Tauri webview: no external fonts or CDNs (CSP). Fonts are system fonts.
- Windows only. Minimum window 1000 x 680; must work at 1280 x 720 and 1920 x 1080.
