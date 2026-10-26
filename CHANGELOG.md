# Changelog

## Unreleased (MS1 foundations)

### Added
- **Lines keep their identity.** Adding, deleting, editing or moving a line no longer marks the other lines as needing a re-render, and their takes, directions and notes stay with the right text. Older projects are upgraded on first open; a copy of the old file is kept as `project.v1.bak`.
- **Interrupted renders can be picked up again.** If Soundstage closes or crashes during a batch, reopening the project offers to keep the audio already made and finish the rest. A new **Stop after this line** button ends a batch early.
- **Licence conditions.** A licence can now carry conditions (revenue limit, country limit, limited uses). A commercial export asks you to confirm each one; a credit line shows as a notice. Conditions a provider declares cannot be removed by recording your own check.
- **Local-only mode** (Settings): refuses everything that would use the internet, including model downloads, until you turn it off.
- **This computer** appears in the System check: memory, graphics card, free disk space, and a level that later decides which local models are offered.
- **Director** (Direct step): suggests how each line should be played (what the character wants, what they hold back, pace, pauses, emphasis) using the chat server you set in Settings. It shows what will be sent and where before it sends anything, only suggests, and lets you accept line by line. What you edit is locked and never overwritten. "Simple rules only" works with no server. Action lines like `(She slams the door.)` are now understood and never spoken.
- **The plan now shapes the take.** Pauses and sounds from the plan are written into the line; a feeling or pace the plan asks for is used only where you have not directed the line yourself. A plan never stops a line from rendering: what a voice cannot do is left out and listed ("This voice will not use from the plan: emphasis").
- **Add a sound to a line** (laugh, sigh, gasp and others) on the Direct step. An unclosed tag such as `[laugh` is now reported instead of being read aloud.
- **Try other voices:** audition one line with up to four voices side by side, optionally blind. Your cast and chosen take change only when you choose. Online voices ask first.
- **Suggest other voices** when a line cannot be rendered: voices that can really do it, free ones first, never online ones in local-only mode.
- **Voice library filters:** gender, age, accent, provider and what a voice can make. Accents are marked "not rated" because none have been listened to yet.
- **Listening pack** (Audio tools): make a fair, blind listening test from a few clips: matched loudness, random codes and order for each listener, score sheets, and a private key for you.
- **Listening results** (Audio tools, Listening pack): choose the private key and the listeners' score sheets. Soundstage checks every row, reports any it cannot use, and judges each version against the pass levels (natural, understood, sounds believable, no spoken tag words, same voice, preferred over a reference). Too few listeners can never count as a pass. Save the report as a document or spreadsheet.
- **Model Lab runner** (behind the scenes for now): renders a built-in English test set with several voices, with an estimate, exact approval and a spending cap for online voices, resume, and a record of characters and time per clip.
- The side panel no longer squashes the project name in a short window.
- Behind the scenes: a performance-plan format with checks, and a model list that cannot call anything verified without evidence.


## 0.2.0 (2026-10-23)

### Added
- **FFmpeg is now included** (LGPL build), so MP3, FLAC, AAC, Opus, Ogg Vorbis and MP4 export work with nothing to install.
- **Voices:** any OpenAI-compatible speech server (Kokoro, LocalAI, Speaches, OpenRouter and others), Piper, and your own ComfyUI workflows. Local engines need no account.
- **Music and sound effects:** create them in the Mix step with ElevenLabs, Suno through Kie, fal models (including ACE-Step) or ComfyUI. Everything generated is labelled AI-generated and starts as licence unknown.
- **Back up and open projects as one file** (`.soundstage`): the script, takes, imported audio and pronunciation rules. Opening a file always makes a new project and never overwrites. Keys and consent records are not included.
- **Offline speech to text** with Whisper, built in. Download a model once (78 to 148 MB) and transcribe on your own computer with no account, no internet and nothing sent anywhere. Works for subtitles and the dubbing helper too.
- **Timeline for video editors** (OpenTimelineIO): every line, effect and music clip as its own file, placed exactly, with silences as gaps. Opens in DaVinci Resolve and other OTIO tools.
- **Sound for your scenes** on the Mix step: a drafted background sound and music for each scene, which you edit and make with one press.
- The welcome screen asks **what you are making** (podcast, audiobook, ad, explainer, drama) and starts from that template.
- **Do all the steps for me** shows its progress as a list. A running total of characters **sent to online services** appears in the send prompt and in Settings (kept only while the app is open).
- Problems on a line now have a **fix next to them**: remove a sound the voice cannot make, or pick another voice.
- After an export: **what next**, add subtitles and export again, export in another format, start another project. Soundstage **remembers your export choices** and **each speaker name's voice** for the next project.
- **Cast for me**, **Do all the steps for me** (cast, render and preview mix in one press) and goal tiles on the Projects screen, to make the first scene quicker.
- Consent records are encrypted for your Windows account.
- **Voice cloning** (ElevenLabs), only with a recorded consent record. The record stays on your computer; every render and export re-checks it, and revoking blocks the voice at once.
- **Voice changer** (ElevenLabs) behind a performer consent record, and a **dubbing helper**: transcribe, translate, save translated subtitles or start a project.
- **Audio editor**: trim, join and layer clips with levels and fades.
- **Starter templates** (podcast, audiobook, ad, explainer, drama scene) and a **Latest creations** list on the Projects screen.
- **Pronunciation** rules (all projects, and per project) that change only what the voice is given, with a Hear it test.
- **History** for Speak and Audio tools: play, use again, save as, delete, clear; stored on your computer.
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
