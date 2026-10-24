# Soundstage QA report

Last full run: 2026-10-23, Windows 11, release build of v0.1.0 plus the v0.2 additions.

## How to run everything

| Layer | Command (from the repo root unless noted) | Result at last run |
|---|---|---|
| Core and shell unit and integration tests (Rust) | `cargo test --workspace` | 219 passed, 0 failed |
| Front-end unit tests (Vitest) | `npm run test` in `app/` (`npx vitest run`) | 79 passed, 0 failed |
| Types | `npx tsc --noEmit` in `app/` | clean |
| Lint | `cargo clippy --workspace --all-targets` | style warnings only, no errors |
| Real-window end-to-end, accessibility and screenshots | `node tools/e2e/run.mjs` (needs a release build: `npx tauri build` in `app/`) | 173 of 173 passed |
| Secrets | `rafter secrets .` | none found |
| Dependencies | `npm audit --omit=dev` in `app/` | 0 vulnerabilities |

The end-to-end run drives the built app through the Chrome DevTools protocol in a real WebView2 window. It walks the Golden Path, checks every screen in light and dark at 1000x680, 1280x720 and 1920x1080 for horizontal overflow, runs axe-core (WCAG 2.2 AA) on the main screens, and saves the screenshots used in `docs/USER_GUIDE.md` to `docs/design/screens/`. It deletes the projects it created and resets preferences when it finishes.

## What is covered

- **Script, tags and directions:** parsing, canonical tags, four-level direction, unsupported-tag refusal.
- **Render:** cache keys, takes, stale detection, Windows speech, scratch tones, and a shared adapter contract suite that every adapter must pass.
- **Mix and loudness:** layout, ducking, stems that add up to the master, BS.1770 loudness.
- **Licence and consent gates:** unknown licences block commercial export, 90-day staleness, consent refusals, export manifest.
- **Export:** WAV 16, 24 and 32-bit float; FLAC, MP3, AAC, Opus, Vorbis and MP4 through a real FFmpeg when present; SRT and VTT subtitles; unique output folders.
- **Engines:** OpenAI-compatible speech, Piper, ElevenLabs (voices, render, music, effects, voice design), Kie, fal and ComfyUI against a local fake HTTP server, including failure, slow-job and missing-key cases.
- **Network safety:** pinned hosts, no redirects, loopback-only `http`, public-address-only downloads, size caps, header-injection refusal, provider error bodies reduced to short codes.
- **Safety rules:** fraud, impersonation and minors refusals for scripts; real-person refusal for voice design, including text a prompt helper writes.
- **Prompt helper:** the guided builder (choices to sentence, strength hints), and the AI rewrite (fenced input, bearer key handling, vetted output, plain errors).
- **Front end:** preferences sanitising, colour-contrast ratios asserted from the real design tokens, flags, shortcuts, help search, engine presets, prompt builder.
- **Accessibility:** axe-core on the Home, Script, Cast, Direct, Render, Mix, Export, Voice library, Voice engines screens and the welcome flow, plus keyboard-only paths and focus handling in dialogs.

## Defects the QA found and fixed during this work

- Selected prompt-helper chips failed the contrast check (found by axe); fixed with the gold button colours.
- Welcome and Home banners showed the splash artwork's own wordmark through the text; the welcome screen now uses a CSS glow and the Home banner is cropped above the wordmark.
- Export folders created in the same second collided; a unique suffix is added.
- Timeline inputs were unbounded; they are clamped.
- Voice-design safety slicing could split a multi-byte character; it now slices safely.

## Not covered, and why

- **Live services.** Kie, fal, OpenRouter, ElevenLabs (render, music, design), ComfyUI, Piper and Ollama were tested only against local fakes built from public documentation. Run each once with a real account or install and record the result in `docs/PROVIDERS.md` section 9 or 10.
- **Listening tests.** Accent quality, tag performance and language support are not rated, and the app does not claim them.
- **Other machines.** Only this Windows 11 machine was used. The installer is unsigned, so Windows SmartScreen will warn.
- **Automated visual regression.** Screenshots are saved and reviewed by eye. They are not compared pixel by pixel.
- **Rafter remote scan.** It cannot read the private repository, so only the local secrets scan and a manual review of the new network and subprocess code were done.

## Reporting a bug

Say which screen, what you did, what you expected and what happened. A screenshot helps. For an engine problem, say which engine and whether it is local or online. Never paste a key.
