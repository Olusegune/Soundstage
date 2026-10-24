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

## Bundled and separate programs
- **FFmpeg** is bundled as a separate program (`resources\ffmpeg.exe`), an LGPL v3 build (n8.1, from BtbN/FFmpeg-Builds, no GPL or non-free parts). Its licence text and source pointers ship beside it (`FFMPEG-LICENSE.txt`, `FFMPEG-NOTICE.txt`). You may replace it with your own build on the Export screen. H.264 video in MP4 uses OpenH264 inside this build; codec patents are the user's responsibility where they apply. When updating the bundled build, keep it an LGPL build, ship its licence text and source pointers, and update this file.
- **whisper.cpp** (offline speech to text, MIT licence) is bundled as a separate program (`resources\whisper\whisper-cli.exe` with its libraries; release v1.9.2 from github.com/ggml-org/whisper.cpp). Its licence text ships beside it (`WHISPER-LICENSE.txt`). The Whisper speech models (OpenAI, MIT licence) are not bundled: people download one inside the app from huggingface.co, and each is checked against a fixed size and SHA-256.
- **Piper, ComfyUI, Ollama and any speech or music models** are separate programs with their own licences. Every voice and model has its own terms, which Soundstage cannot read for you.

## Online services
ElevenLabs, Kie (including Suno output), fal and OpenRouter are used only with your own account and key, under their terms. Soundstage is not affiliated with them.
