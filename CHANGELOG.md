# Changelog

## 0.2.0 (2026-10-23)

### Added
- **FFmpeg is now included** (LGPL build), so MP3, FLAC, AAC, Opus, Ogg Vorbis and MP4 export work with nothing to install.
- **Voices:** any OpenAI-compatible speech server (Kokoro, LocalAI, Speaches, OpenRouter and others), Piper, and your own ComfyUI workflows. Local engines need no account.
- **Music and sound effects:** create them in the Mix step with ElevenLabs, Suno through Kie, fal models (including ACE-Step) or ComfyUI. Everything generated is labelled AI-generated and starts as licence unknown.
- **Voice cloning** (ElevenLabs), only with a recorded consent record. The record stays on your computer; every render and export re-checks it, and revoking blocks the voice at once.
- **VoiceKiller** voices (about two thousand, many languages) and your own VoiceKiller clones, behind the same consent record as clones made in the app.
- **Speak** screen: type, pick a voice, hear it, and save as WAV, MP3, FLAC, M4A, Ogg, Opus or MP4.
- **Audio tools:** speech to text with SRT and WebVTT subtitles (ElevenLabs or your own Whisper server), and noise and music removal (ElevenLabs).
- **Voice design:** describe a character and keep one of three candidate voices (ElevenLabs). Descriptions that aim at a real person are refused.
- **Prompt helper:** "Help me write this" guides beginners with taps and examples, and an optional "Improve with AI" button uses a chat server you choose, such as Ollama.
- **Export formats:** WAV (16, 24, 32-bit float), and with FFmpeg FLAC, MP3, AAC, Opus, Ogg Vorbis and MP4; SRT and WebVTT subtitles.
- **Import:** WAV, MP3, FLAC, Ogg and M4A.
- **Windows menu bar:** File, Edit, View, Go and Help.
- **System check and Report a problem** in Settings, with a report that holds no keys or script text.
- Language flags in the voice list; a redesigned, accessible interface with Simple and Advanced modes, a command palette, rebindable shortcuts, a guided tour and a built-in help centre.

### Fixed
- Welcome and Home banners no longer show the splash artwork's own wordmark through the text.
- Export folders created in the same second no longer collide.
- Timeline inputs are bounded.

### Known limits
- Kie, fal, OpenRouter, ComfyUI, Piper, Ollama and ElevenLabs music and design were tested against local fakes only. See `docs/PROVIDERS.md` section 10.
- The installer is not code-signed, so Windows SmartScreen shows a warning.
- MP4 video uses the encoder inside the bundled LGPL FFmpeg (OpenH264), which may differ slightly from x264 output.

## 0.1.0 (2026-10-23)
First release: script, cast, direct, render, mix and export with Windows voices and ElevenLabs, licence and consent gates, and an export manifest.
