# 08. MS3 Performance v1: detailed plan

Status: **Proposed, planning only. No code until the owner approves it.** Written 2026-10-25 after MS1 (foundations) and the first MS2 slice (Director) shipped. This expands section 6 of `06-MILESTONES-AND-TESTS.md`.

## 1. Goal

Turn the plan the Director (or the person) wrote into **better takes**, let the person **audition the same line on several voices and providers**, and make **vocal events** (a laugh, a gasp, a cough) reliable. Everything stays optional: a project with no plan renders exactly as it does today.

## 2. What the code does today (so the plan starts from facts)

- A line's delivery is resolved from four levels (`direction.rs`): emotion, pace and a pause before. `render.rs` turns the line into tokens (text, canonical tags, pauses) and a cache key built from the voice, the provider version, the tokens, the emotion and the pace.
- Adapters declare which canonical tags they can speak (`Capabilities.tags`). A tag a voice cannot speak is reported before rendering, with a fix button (PHY-005 already holds).
- A `Voice` already carries language, accent, gender and age range. The Voice library filters by search, language and licence only.
- Takes are kept per line, with one selected. There is no side-by-side audition and no per-take notes beyond approval status.
- Events exist only as inline tags the provider renders itself, if it can.
- The plan (MS1/MS2) is stored in `plan.json` and can fill empty line directions. It is not otherwise used when rendering.

## 3. Design decisions to settle first (proposed D-17 to D-20)

| ID | Question | Proposal | Why |
|---|---|---|---|
| D-17 | How does the plan reach a render? | **Precedence: the person's direction beats the plan, the plan beats defaults.** The plan is resolved into the same emotion and pace fields, plus pauses and emphasis, before the cache key is made. A plan change makes only the lines it touches stale. | Keeps the existing four-level rule and the "never throw away paid audio" promise. |
| D-18 | Where does emphasis and a mid-line pause go for providers that cannot take them? | Turn pauses into canonical pause tags (already supported everywhere). Emphasis is sent only to providers that declare it; otherwise it is listed in the "not honoured" report, never faked. | Honest about limits (PHY-005, EMO-007). |
| D-19 | Where do hybrid event sounds come from? | Three sources, in this order of preference: the **provider's own tag** (already works where supported); a **library clip** the person imported or generated through the existing sound-effect providers (ElevenLabs, fal, Kie, ComfyUI); nothing else. No new model. Hybrid events are a separate track under the speech take, never mixed into it. | Reuses approved providers and licences; keeps the speech take pure so it can be regenerated alone (PHY-006). |
| D-20 | How are multi-provider auditions paid for? | Every extra take is its own approved send (existing confirm prompt, now with a running cost estimate where a provider publishes prices, and a hard cap the person sets per audition, default 3 takes). | No silent charges (AC-009). |

## 4. Two releases inside MS3

Splitting lowers risk and gets value out earlier.

- **MS3a, "Perform":** T-3.1, T-3.2, T-3.4, T-3.5, T-3.6. Plan reaches render, tags and events are reliable, voices are easier to choose, auditions work, fallbacks are honest.
- **MS3b, "Hybrid":** T-3.3 and T-3.7. Event sounds as their own track, and performance reference tied to a line. These are the heavy parts and depend on what the owner learns from MS3a listening.

## 5. Tasks in dependency order

### MS3a

**T-3.1 Plan to controls, and support reports (L)**
- Resolve each line: person's direction, else plan, else default (D-17). Emit emotion, pace, pause tokens and emphasis ranges.
- New adapter method `plan_support(line_plan) -> SupportReport { honoured, approximated, ignored }`. Default implementation reads `Capabilities`; adapters override where they know better. The Render step shows "this voice will ignore: emphasis" before the person pays.
- Cache key includes pauses and emphasis only when the voice honours them, so changing something a voice ignores never makes a take stale.
- Tests: precedence table (all 8 combinations); cache key unchanged when only an ignored field changes; a plan-only change makes exactly the planned line stale; support report for every adapter in the contract suite; projects without a plan render byte-identically to before (golden test).

**T-3.2 Vocal events, provider-native (L)**
- Event list per line in the plan (already in the schema): kind, position, intensity. The Direct step gets an inline event editor: insert a laugh or gasp at a position in the line, choose intensity where the provider allows it.
- Mapping to canonical tags with intensity expressed as repetition or wording only where a provider documents it; otherwise one tag, and intensity is stored but marked "not used by this voice".
- PHY-001 regression: a contract test over every adapter proving a tag, however it is written, is never read out as text. Includes hostile cases (`[laugh]` inside quotes, a tag next to punctuation, a tag the provider does not know).
- Tests: event round trip (plan to tokens to request); no adapter request body contains the literal word of an event; unsupported event reported before render with a fix.

**T-3.4 Voice profiles and filters (M)**
- A `Character` can hold a **profile**: preferred accent, style words, pronunciation notes, and the voices it has been cast with before. Profiles live in the project and are remembered by speaker name like voices are today.
- Voice library filters: gender, age range, accent, style, provider, "verified for my language" (reads the registry; shows `unverified` honestly, never as supported).
- Tests: filter combinations; the registry state is shown and never inferred; saved profiles survive reopen and migration.

**T-3.5 Take audition (L)**
- On a line, "Try this line with...": pick up to the cap (D-20) of other voices or providers, render each as a take, play them side by side with the same loudness, keep one, reject the rest. Optional blind mode hides which voice is which until you choose.
- Takes keep what made them (voice, provider version, plan snapshot hash) so the choice can be explained later.
- Tests: audition makes exactly the takes requested and no others; cap enforced; rejecting a take never deletes the file (it stays recoverable); blind mode never reveals names in the accessibility tree before the choice; loudness matching within 0.5 LU.

**T-3.6 Fallback alternatives (M)**
- When a provider fails, runs out of credit, or cannot honour the plan, list alternatives that can, each with: what it would drop (from `plan_support`), cost or "free, runs here", whether data leaves the computer. Choosing one asks for a fresh approval. The plan is untouched.
- Tests: no call is made to an alternative without an approval event; the plan hash is unchanged after a fallback; a local-only project is never offered a cloud alternative.

**MS3a exit gate:** AC-002 and AC-009 pass automatically; PHY-001 holds across all adapters; the golden test shows old projects render identically; real-window run covers audition, filters, event insert and fallback; first listening run on events and on accent labels done with at least five listeners (protocol 07) and reported honestly, even if the answer is "not good enough yet".

### MS3b

**T-3.3 Hybrid events (XL)**
- An event with source `library` or `generated` is a separate clip on its own track under the speech take, with an offset from the line start, gain, and a fade. It is created through the sound-effect providers or chosen from imported audio, with the licence recorded like any other asset.
- Timeline and exports treat it as an asset in a new "vocal events" stem group; the speech take is never altered.
- Tests: speech take byte-identical with and without events; event offset within one frame (about 20 ms) of the plan; stems still sum to the master; licence gate covers event clips; regenerate one event without touching the speech.
- **Risk:** it may sound pasted-on. The listening protocol decides whether it ships on by default or stays an advanced option.

**T-3.7 Performance reference tied to a line (M)**
- The existing voice changer (performer-consent gated) can be launched from a line, with the result stored as a take for that line and marked "from reference".
- Tests: consent record checked on every use; revoking blocks it at once; the take is traceable to the reference file and consent id in the export manifest.

**MS3b exit gate:** AC-004 passes; listening run on hybrid events; owner demo.

## 6. Order of work and rough effort

| Step | Tasks | Effort |
|---|---|---|
| 1 | T-3.1 | 1 week |
| 2 | T-3.2 (can overlap with step 3) | 1 week |
| 3 | T-3.4 | 3 days |
| 4 | T-3.5, T-3.6 | 1.5 weeks |
| 5 | MS3a gate, listening run, fixes | 1 week |
| 6 | T-3.3, T-3.7 (MS3b) | 2 to 3 weeks |

MS3a is about 4 to 5 weeks, MS3b a further 2 to 3. These are estimates for one developer with this assistant, not promises.

## 7. What can be done before approval

Nothing in code. If approved, the first step is T-3.1 because everything else reads its resolved output.

## 8. Risks specific to MS3

| Risk | Mitigation |
|---|---|
| Plan changes cause unexpected re-renders and charges | Cache key only includes fields the voice honours; stale preview ("3 lines would re-render, about N characters sent") before any paid action |
| Providers differ in how they treat the same tag | Per-adapter contract tests against recorded examples; listening runs, not assumptions |
| Auditions run up a bill | Cap and per-take approval (D-20); a session total shown next to the cap |
| Hybrid events sound fake | Separate track, off by default until the listening gate passes |
| Blind mode leaks names to screen readers | Explicit accessibility test |
| Voice filters imply quality claims | Filters only use stored facts; the registry's evaluation state is always shown |

## 9. Questions for the owner

1. Approve D-17 to D-20, or change them?
2. Ship MS3a first and decide on MS3b after listening, as proposed?
3. Audition cap: is 3 takes right, or do you want more?
4. Who can take part in listening runs (needs at least five people, and native speakers for any language or accent you want marked verified)?
