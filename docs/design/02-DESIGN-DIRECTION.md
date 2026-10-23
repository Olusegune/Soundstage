# Design direction

Philosophy: **make sophisticated sound design feel effortless.** The magic comes from clarity, fluidity and craft, not from effects.

## 1. Identity (from the supplied brand art)
The splash, icon and file icon share one world: a dark stage lit from above, a warm gold light, deep teal, thin concentric rings, and a waveform at the centre. The UI borrows that restraint.

- **Stage black:** `#05080B` window, `#0A1015` surfaces. Never pure black panels; depth comes from a faint top-down glow.
- **Gold** is the *action* colour: the one primary button per view, the current step, the playhead. Warm and rare.
- **Teal** is the *information* colour: waveforms, selection, links, the "free" badge.
- **Cream** text (`#F2ECE0`) rather than blue-white, matching the wordmark.
- Status uses the same family: success is teal-green, warning is amber, error is coral. Colour is never the only signal.
- **Type:** system Segoe UI Variable. Headings and the wordmark use a light weight with wide tracking (like the logo); body is regular. Numbers are tabular.
- **Shape:** restrained radii (4 px controls, 6 px cards, 10 px dialogs). Rectangular cards, confident 36 px buttons, no pills except status badges.
- **Light mode:** warm paper (`#F5F1E8`) with deepened gold and teal so AA contrast holds. Same layout, same hierarchy.
- **Motion:** 120 to 240 ms, ease-out, used for state change only (panel slide, step completion, toast). Off under reduced motion.

### Avoid (and how it is enforced)
Emoji as icons: none, one custom stroke icon family. Neon, heavy gradients: gold and teal appear only as small accents and thin lines. Modals for simple choices: inline panels first; dialogs only for confirmation of consequences.

## 2. Application shell
```
+--------+-----------------------------------------------+------------+
| rail   |  toolbar: project / step - save - undo redo -  | guide      |
|        |  Simple|Advanced - search (Ctrl+K)             | panel      |
| Home   +-----------------------------------------------+            |
| Path:  |                                               | Why        |
|  1..6  |   central workspace (changes by step)         | Next       |
| Voices |                                               | If skipped |
| Guide  |                                               | Checklist  |
| Prefs  |                                               | Help link  |
+--------+-----------------------------------------------+------------+
```
- **Left rail** (collapsible to icons): Home, the six Golden Path steps with live status, Voices, Guide, Settings.
- **Toolbar:** breadcrumb, save state, undo and redo, Simple | Advanced, command palette button. Project-level settings (licence target, rating) live in a Project details popover, not the bar.
- **Guide panel** (right, collapsible, resizable): always answers the six questions of the brief for the current step. It is the Golden Path made functional.
- **Command palette** (Ctrl+K): commands, steps, projects, script lines, voices, help articles.

### Workspace per task (not one template)
| Step | Layout | Hero |
|---|---|---|
| Script | Editor with tag palette; speakers and checks beside it | The words |
| Cast | Large voice cards in a grid, side-by-side sample playback | The voices |
| Direct | Scene by scene; line cards with delivery chips; one-click preview | The performance |
| Render | Take wall: each line with its waveform, status, and takes drawer | The audio |
| Mix | Timeline with waveforms across dialogue, effects and music lanes; loudness meter | The mix |
| Export | Readiness checklist, then a delivery summary | The licence clearance and the files |

## 3. Golden Path specification
The path is computed from real project state (`golden.ts`), never stored. For each step the app derives a **status**, **progress**, **next action**, and fixed teaching copy.

| Step | Done when | Why it matters | If you skip it | Cost |
|---|---|---|---|---|
| Script | at least one line has a speaker | Everything is made from the words | Nothing can be cast or rendered | Free |
| Cast | every speaker has a voice | Voices decide the sound of the whole film | Lines cannot be rendered | Free to choose; cloud samples are paid |
| Direct | optional; counts open notes | Emotion, pauses and non-verbal sounds make it acting, not reading | Lines are spoken neutrally | Free |
| Render | every line rendered or skipped | Turns text into audio you can judge | Nothing to mix or export | Free (Windows) or paid (cloud, asked first) |
| Mix | optional; preview built | Balances voice, music and effects to a loudness target | Export is the voice track alone | Free |
| Export | manifest written | Licence clearance and delivery | Nothing leaves the app | Free |

Statuses: `done`, `current` (the first required step not done), `ready`, `blocked` (with the reason and the step that unblocks it), `optional`. Going back is always allowed. The **Next** button in the guide panel and toolbar is the first useful action of the current step ("Choose a voice for KOLA", "Render 3 lines", "Fix 2 notes"). Paid actions say so on the button.

Simple | Advanced is a UI preference, not a different project. Simple hides pace and pause sliders, ducking depth, fades and loops, and licence-editor detail; everything it hides keeps its value.

## 4. Design system
Tokens in `app/src/styles/tokens.css` (colour, type, space 4 px base, radii, elevation, motion, z-index) for dark and light. Components in `components.css`: Button (primary, secondary, quiet, danger, icon), Field, Select, Switch, Segmented, Tabs, Card, Badge, Callout, Tooltip, Dialog, Drawer, Toast, Progress, Skeleton, Kbd, Waveform, Meter. Every interactive state (hover, focus-visible, selected, disabled, error) is defined once. Contrast of every token pair is asserted by a test.

## 5. Keyboard map
Defaults (all rebindable in the shortcut dialog, conflicts detected, text fields keep native editing):

| Action | Keys | Action | Keys |
|---|---|---|---|
| Command palette / search | Ctrl+K | New project | Ctrl+N |
| Open project (home) | Ctrl+O | Save now | Ctrl+S |
| Undo / redo | Ctrl+Z / Ctrl+Shift+Z, Ctrl+Y | Step 1 to 6 | Ctrl+1 to Ctrl+6 |
| Previous / next step | Alt+Left / Alt+Right | Do the next recommended action | Ctrl+Enter |
| Toggle sidebar | Ctrl+B | Toggle guide panel | Ctrl+J |
| Simple / Advanced | Ctrl+Shift+M | Voice library | Ctrl+Shift+V |
| Preview current line or mix | Space (outside text fields) | Settings | Ctrl+, |
| Help / user guide | F1 | Shortcut reference | Ctrl+/ |

## 6. Onboarding and documentation
- **Splash** (the supplied art) while voices load, then **Welcome** on first launch: three short panels (what Soundstage does, the Golden Path, free versus paid).
- **Guided tour**: six anchored steps over the real interface (rail, guide panel, next action, palette, help).
- **Quick Start**: one button creates the sample scene and the guide panel walks it to a real exported result with free Windows voices.
- **Help centre** (F1): searchable guide, glossary, free-versus-paid, troubleshooting, shortcuts. Contextual "?" on every step opens the matching article in a drawer.
- Plain-language tooltips on advanced controls (ducking, LUFS, true peak, stems).

## 7. Phased roadmap
1. Tokens, icons, base components, light and dark. 2. Shell, Golden Path guide panel, toolbar, rail. 3. Command palette, shortcuts, preferences. 4. Screens: Home, Script, Cast, Direct, Render (with waveforms), Mix, Export, Voices, Settings. 5. Help centre, glossary, onboarding, tour, splash. 6. Tests: golden path logic, shortcuts, contrast, help search, axe accessibility, screenshots at two sizes in both themes, regression of all prior tests.

Known limits are recorded in `04-VERIFICATION.md`.
