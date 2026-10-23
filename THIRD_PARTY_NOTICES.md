# Third-party notices

_A summary of the main components. Before a public release, generate the complete list with `cargo about` (Rust) and `npx license-checker --production` (JavaScript), and keep the output with the installer._

| Component | Used for | Licence (verify against the version in `Cargo.lock` / `package-lock.json`) |
|---|---|---|
| Tauri 2 and its plugins (dialog, opener) | Desktop shell | MIT or Apache-2.0 |
| WebView2 | Renders the interface (installed with Windows) | Microsoft terms |
| React, Vite | Interface | MIT |
| ureq | Network calls | MIT or Apache-2.0 |
| keyring | Windows Credential Manager | MIT or Apache-2.0 |
| symphonia | Reading MP3, FLAC, Ogg and M4A | MPL-2.0 |
| serde, serde_json and other Rust crates | Data handling | MIT or Apache-2.0 |
| flag-icons | Language flags | MIT |
| axe-core, Vitest (development only, not shipped) | Accessibility and unit tests | MPL-2.0, MIT |

## Not included, used if you install them
- **FFmpeg** is not bundled. Soundstage runs the `ffmpeg.exe` you install or choose. Its licence (LGPL or GPL, depending on the build) is yours to review. If Soundstage later bundles FFmpeg, use an LGPL build, ship its licence text and source offer, and update this file.
- **Piper, ComfyUI, Ollama and any speech or music models** are separate programs with their own licences. Every voice and model has its own terms, which Soundstage cannot read for you.

## Online services
ElevenLabs, Kie (including Suno output), fal and OpenRouter are used only with your own account and key, under their terms. Soundstage is not affiliated with them.
