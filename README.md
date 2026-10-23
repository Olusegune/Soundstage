# Soundstage

The whole sound side of a film or video on a Windows desktop: dialogue with performance direction, music and effects you have the rights to, a loudness-managed mix, and a licence- and consent-gated export of a master plus stems.

Soundstage orchestrates voice engines behind one consistent interface. It does not train its own speech or music models. The product brief lives in [`CLAUDE.md`](CLAUDE.md) and [`docs/`](docs).

## Status: v0.1 (first slice of v1)

| Area | In this build | Not yet (and why) |
|---|---|---|
| Golden path | Script, Cast, Direct, Render, Mix, Export, with step gating | |
| Script | `NAME:` lines, `#` / `INT.` scenes, canonical tag validation, import of text | Fountain import (v2) |
| Direction | Project, character, scene and line levels with the resolved source shown per line; pause, pace, emotion | Word-level emphasis markup (designed, not built) |
| Voices | Windows built-in voices (offline), ElevenLabs cloud voices (Eleven v3, needs your own key), test tones | ElevenLabs is tested against a fake server only; a live check with a real key is logged in `docs/PROVIDERS.md` section 9. Voice design and cloning are off. |
| Non-verbal sounds | Validated and **flagged as "not supported by this voice"** before render, never silently dropped. ElevenLabs renders laugh, sigh and cry, per its docs | Cough, sneeze, sniff and gasp are not documented by any shipped provider. They need a provider that renders them, or a licensed effects library. This is the open product decision in PROVIDERS.md section 2.2. |
| Takes | Every render is a take; side-by-side selection; approve, hold, reject; notes | Read-only review link (v2) |
| Mix | Dialogue, effects and music buses; ducking; loudness to a destination; preview mix; master plus three stems that sum to the master | Picture reference, ADR, generated music and effects (v2); MP3 export |
| Licensing | Licence tag per voice, engine, music and effect; commercial export blocks `unknown`, `non-commercial` and checks older than 90 days; user-recorded checks with source and date | Provenance marking such as C2PA (needs your decision, SAFETY_LICENSING 9.1) |
| Consent | Consent record model and enforcement in the core, tested | Cloning UI and adapters (v2) |
| Languages | Whatever voices Windows has installed | **No accent or language claim is made.** Nothing is listed as supported without a bake-off result. The bake-off has not been run. |

ElevenLabs voices and Windows voices both carry an `unknown` licence, so a project marked **Commercial** will not export with them until you read their terms and record the check. That is deliberate.

## Layout

```
crates/soundstage-core/   Rust core: script, tags, direction, licence gate, consent, safety,
                          adapters, render and cache, mix, loudness, export
app/                      Tauri 2 shell and React + TypeScript UI
  src-tauri/              Thin command layer, Windows Credential Manager access
  src/                    UI (steps, screens, shared logic)
docs/                     Product, provider, safety, sound design, test and UX docs
fixtures/                 Test scripts
```

## Develop

Requirements: Rust (stable), Node 20+, WebView2 (included in Windows 11), and the Visual Studio C++ build tools.

```bash
cargo test -p soundstage-core          # core tests, including the adapter contract suite
cd app && npm install
npm test                               # UI logic tests
npx tauri dev                          # run the app
npx tauri build                        # release build and NSIS installer
```

The Windows speech adapter runs against the real engine in tests when voices are installed, and skips itself otherwise.

## Rules the code enforces

1. No clone is created or used without a complete, active consent record.
2. Commercial export refuses anything not tagged `commercial` with a check in the last 90 days, and names each blocker.
3. The UI and project files only ever hold canonical tags and voice ids.
4. Provider keys live in the Windows Credential Manager only. The only network host is `api.elevenlabs.io`, and the app asks before sending any text.
5. A tag a voice cannot perform is shown before render, never dropped.
6. Loudness that cannot be reached without clipping is flagged, not silently limited.

Loudness targets and true-peak ceilings are starting points marked VERIFY in [`docs/SOUND_DESIGN.md`](docs/SOUND_DESIGN.md); confirm them for your destination before delivery. The true-peak figure is an estimate (4x oversampling), not a certified meter.

## Engines and export (v0.2)
- Voices: Windows voices, ElevenLabs, any OpenAI-compatible speech server (local or cloud), Piper, and your own ComfyUI workflows. No account is needed for the local ones.
- Music and effects: ElevenLabs, Suno through Kie, fal models, ComfyUI. Everything generated is labelled AI-generated and starts as licence unknown.
- Voice design from a text description (ElevenLabs).
- Export: WAV (16, 24, 32-bit float), and with a user-installed FFmpeg FLAC, MP3, AAC, Opus, Ogg Vorbis and MP4 (waveform video); SRT and WebVTT subtitles; stems; manifest.
See `docs/PROVIDERS.md` section 10 for what is and is not live-tested.

See also: `CHANGELOG.md`, `PRIVACY.md`, `THIRD_PARTY_NOTICES.md`, `docs/QA.md`, and the landing page draft in `docs/site/`.
