# 03. Architecture update (target design)

Status: **Proposed.** Spec only; no code changes until the owner approves. Written 2026-10-25.

This file is the delta from the as-built design in `docs/ARCHITECTURE.md` (v0.1, now partly stale) to the design PRD-002 needs. Together they are the `dev_spec` (decision D-02). It keeps what works: local-first, Windows-first, provider independence, no database, plain files, the Golden Path.

## 1. As built (v0.2.0, summary)

- `crates/soundstage-core`: script parser, canonical tags, direction resolver, render planner with cache keys and takes, layout, mixer (ducking, BS.1770), licence and consent gates, exporter, adapters, pronunciation, project pack, OTIO, history.
- `app/src-tauri`: Tauri commands that call the core, run work on background threads, store keys in Windows Credential Manager.
- `app`: React 19 and TypeScript, plain CSS tokens, 7 main screens plus tools.
- Bundled: FFmpeg (LGPL), whisper.cpp. Models download on request, pinned by size and SHA-256.

## 2. Design rules that do not change

1. Local-first. Cloud use is opt-in, per action, with a plain statement of what is sent and the cost.
2. Provider independence. No feature depends on one vendor. Every AI call goes through a trait.
3. The script is the source of truth. Plans, takes and cues are derived and never overwrite user text.
4. Generated source media is written once and never edited in place. Edits are layers.
5. No database for the first release. JSON files with atomic writes and a `.bak`.
6. Every long operation is a job: cancellable, resumable or restartable, and never able to corrupt a project.

## 3. Stable identifiers (resolves C-06)

**Problem.** Lines are keyed `s{scene}-l{ordinal}` (`script.rs:125`). Inserting a line renumbers everything below it, so cache entries, directions, takes and stale marks attach to the wrong text.

**Design.**
- Each parsed element (scene, line, action) gets an immutable `id`: a short string stored in the `line_ids` table inside `project.json` (as built; a separate sidecar file was not needed), with the element's content hash and position.
- On re-parse, ids are matched to elements by this order: exact hash and position; exact hash anywhere nearby; same speaker and a text similarity above 0.85; otherwise a new id. Unmatched old ids become "orphaned" and their takes are kept under an Orphans list instead of deleted.
- The user can accept, reassign, or discard a match when it is uncertain (similarity 0.85 to 0.95).
- The cache key keeps its content hash parts; the line id replaces the ordinal.
- **Migration.** Project schema 1 to 2 (as built). On first open, ids are assigned to the existing ordinals in order, takes and directions are carried over by ordinal, a backup `project.v1.bak` is written, and the migration is tested against the fixture projects. A project opened in an older app still loads read-only with a warning.

**As built (MS1, 2026-10-25).** Lines only; scenes keep their positional index (scene inserts can still move scene-level directions and status, a follow-up). The first ids are the old positional keys, so no take, direction or note was rewritten. The matcher runs on every parse and save (`ids.rs`): exact text in order, then similar text (0.85, within 200 lines), then paired gaps between matched neighbours, then new ids `l1`, `l2`, and so on. Lines that leave the script are kept as orphans (up to 500) and come back if the same text returns. Uncertain matches are not yet shown to the user for confirmation (follow-up). Opening a version 1 project writes `project.v1.bak`; an older Soundstage refuses a version 2 project with a plain "newer version" message rather than opening it read-only.

## 4. Performance plan

A `PerformancePlan` is one versioned JSON document per project (`plan.json`), separate from the script.

```
PerformancePlan { schema: 1, source_hash, created_by: {kind: "director"|"user", provider?, model?, at},
  scenes: [ SceneIntent ], lines: { line_id: LinePlan }, cues: [ SoundCue ] }

LinePlan {
  line_id, objective?, subtext?, relationship?,
  emotion: [ { name, intensity 0..1, surface: bool } ],
  transition?: { from, to, curve },
  pacing: { pace, pauses: [ {at, ms} ] }, emphasis: [ {span, level} ],
  vocal_events: [ { kind, at, duration_ms?, intensity, source: "provider-tag"|"generated"|"library" } ],
  physical: [ { category, intensity, notes } ],
  language_spans: [ {span, language, accent?} ],
  alternatives: [ { label, plan_delta } ],
  confidence: 0..1, uncertainty_note?,
  user_locked: [ field names the user edited ]
}
```

Rules:
- Unknown fields are kept (forward compatible). A bad document is rejected with a plain error and the old plan is kept.
- `user_locked` fields are never changed by a re-run of the Director. A re-run produces a proposed diff the user accepts per line.
- The plan holds intent only. Provider-specific controls are derived at render time (`providers.rs`), so switching provider keeps the plan.
- The existing four-level direction (`direction.rs`) becomes the resolved view of the plan plus user overrides. Existing projects keep working: with no plan, behaviour is today's.

## 5. Director safety (resolves C-16)

The Director sends script text to a language model. That is both a privacy decision and an injection surface.

1. **Consent.** First use per project shows what is sent (the scene text and character names), to which host, and the estimated tokens and cost. The user confirms. A "local only" project setting restricts the Director to a loopback model or the rule-based fallback.
2. **Hosts.** Only hosts the owner has approved in `DECISIONS.md` (currently none for the Director). Redirects are refused. Keys follow existing rules.
3. **Untrusted text.** Script text is data. It goes inside a delimited block; the system prompt says instructions inside it are not to be followed. Output is requested as JSON against a schema.
4. **Output handling.** The reply is parsed, validated against the plan schema, size-capped, and every string is length-limited and stripped of control characters. Anything that fails validation is dropped, not repaired. No model output is ever executed, used as a path, a URL, a command or a provider name.
5. **Limited agency.** The Director can only propose a plan. It cannot render, spend, export, change settings or call tools. All spending stays behind the existing approval prompts.
6. **Fallback.** Without a model, a rule-based pass (punctuation, tags, stage directions) fills only what it is sure of and leaves the rest empty with a "not analysed" note.
7. **Safety rules.** The existing refusals (fraud, impersonation, minors) run on script text before and after the model call.
8. **Provenance.** The plan records which model produced it. Logs never contain script text.
9. **Cost bound.** Scene-by-scene calls with a token cap per call and a session total shown.

## 6. Jobs and workers (resolves C-07, C-08)

**Job record** (`jobs/{job_id}.json`, in the project):
`{ id, kind, state, created, updated, progress 0..1, step_label, inputs_hash, provider, cost_estimate, error?, result_refs[] }`

**States:** `queued → running → (succeeded | failed | cancelled)`, plus `interrupted`, set on startup for any job found `running` (the app died). Interrupted jobs offer Resume or Discard; partial outputs are written to a temp name and renamed only on success.

**Queue rules:** one running job per resource class (cloud network, local GPU, local CPU), a cap per provider, cancel at any time, no job starts a paid call without the approval recorded on the job.

**Workers.** Heavy local models run in a separate process, never in the app process.
- Protocol: newline-delimited JSON over the child's stdin/stdout; messages `hello`, `load`, `run`, `progress`, `result`, `error`, `cancel`, `unload`, `bye`. No network listener. Messages are size-capped.
- Packaging: a worker is an optional **model pack** installed on request: runtime plus weights, each file pinned by size and SHA-256, shown with size and licence before download, removable from a manager. The installer itself stays small.
- Format rule (D-10): only safetensors and GGUF weights. No pickle or arbitrary code loaded from a download.
- Sandbox: the worker is started with a working directory inside the project cache, with paths passed by the app; it gets no keys and its output is validated (format, duration, sample rate) before use.
- Failure: on out-of-memory or crash the app reports the reason, unloads, offers a smaller setting or a cloud alternative (with approval), and the project is untouched.
- The runtime (Python or native) is a decision for the pack author per model and needs owner approval per pack (C-08, D-11). No Python is bundled by default.

## 7. Capability registry (resolves C-23 groundwork)

A registry file, `registry/models.json`, shipped with the app and updatable, one entry per provider-model. Fields (PRD section 4.4): provider, model, kind, languages, accents (each with a verification state), expressive controls, non-verbal support, cloning support, hardware needs, licence (structured), cost and latency, status (`verified` | `experimental` | `unsupported`), evidence ids, last verified date.

- Status can be raised to `verified` only by a Model Lab result that meets the gates in `07-LISTENING-TEST-PROTOCOL.md` (D-09).
- The UI reads the registry for filters, warnings and the "not verified" labels. Nothing in code hardcodes a language or accent claim.
- Existing hardcoded capability knowledge (tag support per adapter) moves into the registry with the same tests.

## 8. Timeline v2 (MS7)

- Today the timeline is derived from the script each time (`layout.rs`). Target: a `timeline.json` of tracks and clips that the user can edit (move, trim, split, fade, gain, mute) with the derived layout as the starting point.
- Clips reference immutable source files plus edit parameters; undo and redo are a stack of parameter changes.
- "Re-layout from script" is an explicit action that shows what would move and never silently overwrites edits.
- Export reads the timeline. Existing exports stay valid when no timeline edits exist.

## 9. Video (MS8, MS9)

- MS8: import a video, probe with FFmpeg, show a proxy preview, pin cues to timecode with frame-rate-aware math, export an audio mix that is aligned to the video within the tolerance in AC-012. No generation.
- MS9: generation is a worker pack like any other model, only after the video likeness safety document (C-17) is written and approved, and after the scope decision (C-01).

## 10. Storage layout (project)

```
MyProject/
  project.json          format version, ids of other files
  script.txt            user text, never rewritten by the app
  project.json          also holds `line_ids`, the stable id table (as built)
  plan.json             PerformancePlan (new)
  timeline.json         tracks and clips (MS7)
  takes/                immutable audio
  jobs/                 job records (new)
  cache/                derived and regenerable
  export/               output and manifest
```
Paths in files are relative and validated. Anything under `cache/` can be deleted without losing user work.

## 11. Module map

| PRD module | Where it lives | Status |
|---|---|---|
| Project and media manager | `project.rs`, `projectpack.rs` | exists |
| Script parser | `script.rs` | extend |
| Director | `director/` (new, core) | new |
| Emotional | inside the plan and `direction.rs` | new |
| Physical Acting | `acting/` (new, core) | new |
| Voice and provider registry | `providers.rs`, `registry` | extend |
| Model Lab | `lab/` plus `tools/bakeoff` | new |
| Audio engine and mix | `mixer.rs`, `layout.rs` | exists |
| Foley and soundscape | `sound/` (new) | partial |
| Music | adapters, `generate.rs` | exists |
| Video | `video/` (new) | new |
| Job and worker runtime | `jobs/`, `worker/` (new) | new |
| Hardware detection | `hardware/` (new) | new |
| Export | `exporter.rs`, `otio.rs` | exists |
| Settings and credentials | `credentials`, `prefs` | exists |
| Safety and provenance | `consent.rs`, `licence`, manifest | extend |

## 12. Dependencies to be approved before use

None are installed. Each needs a yes from the owner at the milestone that uses it (D-11):
- Hardware detection (MS1): a Rust crate for CPU/RAM and a GPU probe (NVML or DXGI). Alternative: call `nvidia-smi` and Windows APIs directly without a crate.
- PDF text extraction (MS2): a permissively licensed crate.
- Video probing (MS8): FFmpeg already bundled.
- Worker runtimes (MS5): per model pack.
