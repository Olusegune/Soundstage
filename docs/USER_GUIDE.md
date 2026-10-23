# Soundstage user guide

_Generated from the in-app Help centre (`app/src/help/content.ts`). Edit that file, then run `npm run docs` in `app/`._

## Contents

**Start here**
- [Quick start: your first scene in ten minutes](#quick-start-your-first-scene-in-ten-minutes)
- [The Golden Path](#the-golden-path)

**The steps**
- [Step 1: Script](#step-1-script)
- [Step 2: Cast](#step-2-cast)
- [Step 3: Direct](#step-3-direct)
- [Step 4: Render](#step-4-render)
- [Step 5: Mix](#step-5-mix)
- [Step 6: Export](#step-6-export)

**Understanding**
- [What is free and what costs money](#what-is-free-and-what-costs-money)
- [Voices and providers](#voices-and-providers)
- [Speak, transcribe and clean audio](#speak-transcribe-and-clean-audio)
- [The prompt helper](#the-prompt-helper)
- [Consent and voice cloning](#consent-and-voice-cloning)
- [Saving, undo and recovery](#saving-undo-and-recovery)
- [Simple and Advanced mode](#simple-and-advanced-mode)
- [Keyboard shortcuts](#keyboard-shortcuts)

**Fix a problem**
- [Troubleshooting](#troubleshooting)

- [Glossary](#glossary)

---

# Start here

## Quick start: your first scene in ten minutes

_Make a real two-person scene with free voices, from script to finished files._

This walks you through a complete scene using the free voices built into Windows. Nothing here costs money and nothing leaves your computer.

1. On Home, press Create project. Leave Start with a sample script ticked. You land on the Script step with a short kitchen scene.
2. Read the script. Lines start with a speaker name, like AMA: Good morning. Words in square brackets, like [sigh] or [pause:short], are directions.
3. Press the gold button at the bottom of the guide panel on the right. It always shows the next useful thing to do. Here it takes you to Cast.
4. Give AMA and KOLA a voice. Press Make a sample to hear it first.
5. Open Direct. KOLA's line has a [sigh], which Windows voices cannot perform. Soundstage tells you instead of silently dropping it. Press Remove [sigh] from this line.
6. Open Render and press Render. Each line becomes audio. Press Play to listen.
7. Open Mix and press Build a preview mix. Check the loudness readout.
8. Open Export. Your project is a personal project, so you will see a notice that Windows voices are not cleared for commercial use. Tick the box, choose a folder and export.

> **Tip.** You can always press Ctrl+K and type what you want to do, like render or export.

You now have a master recording, three stem files and a record of what was used.

## The Golden Path

_The recommended order, and how Soundstage keeps you on it without forcing you._

Making sound for a film goes best in a set order: Script, Cast, Direct, Render, Mix, Export. The sidebar shows these six steps and where you are.

- A tick means the step is finished. A highlighted step is the one to do now.
- Direct and Mix are optional. Soundstage only pulls Direct forward when a line has a problem that would stop it rendering.
- You can always go back to an earlier step. Going forward past an unfinished step is blocked, and the sidebar tells you why.
- The guide panel on the right explains why the step matters, what happens if you skip it, and whether it is free.

The gold button at the bottom of the guide panel (and Ctrl+Enter) does the next recommended action. When that action costs money, the button says so.

---

# The steps

## Step 1: Script

_How to write lines, scenes and directions._

Start every line with the speaker's name and a colon. Soundstage turns each new name into a character.

Start a new scene with a line beginning with # (for example # Kitchen, morning), or with INT. or EXT. like a screenplay heading.


### Directions in square brackets

- Sounds: [laugh] [chuckle] [sigh] [cough] [sneeze] [gasp] [cry] [breath] [sniff]
- Delivery: [whisper] [shout] [excited] [sad] [tired] [sarcastic] [curious] [serious]
- Pauses: [pause:short], [pause:long], or an exact length like [pause:350] (milliseconds).

Only these directions exist. Anything else in brackets is flagged so a typo never reaches a voice. Whether a voice can actually perform a direction depends on the voice; you will see that in Cast and Direct.

> **Tip.** Use the tag buttons above the editor to insert a direction at the cursor. You can also import a plain text file.

## Step 2: Cast

_Choosing a voice for each character, and hearing samples first._

Every speaker needs a voice. Pick one from the list and press Make a sample to hear it say that character's first line.

- Windows voices run on your computer, are free, and work offline.
- ElevenLabs voices are more expressive and can perform sighs, laughs and more. They need an ElevenLabs key (Settings) and use your ElevenLabs account.
- Test tones are beeps for checking timing. They are not speech.

> **Uses your provider account.** A sample from a cloud voice uses your provider account. Soundstage asks before sending anything.

Changing a voice after you have rendered makes those lines out of date so they can be rendered again. Earlier takes are kept.

Character notes (age, background, habits) stay with the project as context for you.

## Step 3: Direct

_Emotion, pace, pauses, and fixing what a voice cannot do._

Direction is set at four levels, from broad to fine: the whole project, a character, a scene, and a single line. A finer level overrides a broader one. Every line shows where its delivery comes from, for example emotion tired, from scene.

- Emotion changes how a line is spoken.
- Pace speeds up or slows down delivery.
- Pause before adds silence ahead of a line.
- A director's note stays with the line as a reminder.


### When a voice cannot do something

If a line asks for [sneeze] and its voice cannot sneeze, Soundstage shows Not supported by this voice before you render. Your choices are to remove the tag from that line, or choose another voice. Nothing is ever dropped silently.

> **Tip.** Press Preview line to hear just that line. A line that has not changed is reused, not rendered again.

## Step 4: Render

_Turning lines into audio, takes, and what out of date means._

Rendering makes the audio for each line. The wavy picture beside a line is its waveform: the shape of the sound over time. Flat means silence; a wall of full height means clipping.

- Every render makes a take. Open Takes to compare versions, choose one, and approve, hold or reject it.
- Out of date means you changed the words, voice or direction since the last render. Only those lines need rendering again.
- Skip leaves a line without audio on purpose. Include brings it back.
- A rejected take is left out of the mix and export.

> **Uses your provider account.** Rendering with a cloud voice is billed by the provider. Soundstage cannot show the price in advance, so it tells you which provider receives how much text and waits for your yes.

## Step 5: Mix

_Music, effects, ducking and loudness._

The mix has three groups of sound: dialogue, effects and music. Dialogue is laid out in script order with the gaps you choose.

- Import music or effects (WAV, MP3, FLAC, Ogg or M4A), or create them with AI: describe the sound under Create with AI and choose a generator. Created sounds are labelled AI-generated and start as licence unknown.
- Add to timeline places a file at a scene, with an offset, a level and fades. Loop to fill the scene repeats music for the scene's length.
- Ducking lowers music and effects while someone is speaking so dialogue stays clear. Turn it off for music that is part of the scene, like a radio.
- Where will this play? sets a loudness target. See the glossary for LUFS.

Build a preview mix to hear the result and see its loudness. If a target cannot be reached without distorting, Soundstage flags it instead of squashing your audio.

> **Tip.** Every imported file starts as licence unknown. Record where you checked its terms before a commercial export.

## Step 6: Export

_Licence checks, the files you get, and the manifest._

Before anything is written, Soundstage checks every voice, engine, music track and effect against your project's target.

- Personal projects can use anything. You see a notice for items not cleared for commercial use and must acknowledge it.
- Commercial projects can only use items tagged commercial and checked within the last 90 days. Anything else blocks export and is listed by name with the reason.
- Record my licence check lets you state what you found, where you read it and when. Soundstage cannot read terms for you.


### What you get

- A master recording in the format you choose: WAV (16, 24 or 32-bit float), or with FFmpeg installed FLAC, MP3, AAC, Opus, Ogg Vorbis, or an MP4 video with a waveform picture.
- Three stems (dialogue, effects, music) of exactly the same length, so they line up in an editor and add back up to the master.
- Optional subtitles (SRT or WebVTT) built from your script and its timings, with speaker names if you like.
- A manifest listing every voice, provider, licence decision and file fingerprint, and recording that the dialogue is AI-generated.

Every export goes into a new folder. Nothing is ever overwritten.

> **Tip.** Every format works out of the box: Soundstage includes FFmpeg (LGPL build) to write MP3, FLAC, AAC, Opus, Ogg Vorbis and MP4. To use your own copy instead, choose its ffmpeg.exe on the Export screen.

---

# Understanding

## What is free and what costs money

_A plain account of what can use your provider account._

Almost everything in Soundstage is free and runs on your computer: writing, casting, directing, rendering with Windows voices, mixing and exporting.

> **Uses your provider account.** The only things that can cost money are cloud services you connect (ElevenLabs, Kie, fal, OpenRouter): rendering a line, making a sample, designing a voice, or creating music or an effect. Each is billed to your own account at that provider.

- Soundstage always asks first and says which provider gets how much text.
- It does not show a price, because providers change them and Soundstage will not guess.
- A line that has not changed is reused, so you do not pay twice for the same audio.
- Nothing is sent until you press the confirming button.

## Voices and providers

_Windows voices, ElevenLabs, keys, and what each can do._

Soundstage works with several voice engines behind one set of directions, so changing engine does not mean rewriting your script.

- Windows voices: free, offline. Can change pace and loudness for whisper, shout, excited, sad and tired. Cannot laugh, sigh or cough.
- Your own engines: connect a speech server on your computer (Kokoro, LocalAI, Speaches and many others speak the same OpenAI-style language), or Piper. They are free and need no account.
- VoiceKiller: cloud, about two thousand voices in many languages, plus voices you cloned on voicekiller.com. Delivery and speed are passed as instructions. Cannot laugh, sigh or cough.
- ElevenLabs: cloud. Can perform laugh, sigh, whisper, excited, sarcastic, curious and cry, according to its documentation. Cough, sneeze, sniff, gasp, chuckle, breath, shout, sad, tired and serious are not offered.

No account? Open Voice engines and add a local engine or Piper. Ollama runs language models, not speech, so it cannot be a voice engine. ComfyUI works through a workflow you save from it in API format.

Music and effects: ElevenLabs makes both with the same key. Suno songs come through a Kie key. fal models and ComfyUI workflows can be added under Voice engines. Everything made starts as licence unknown.

Design a voice: in the Voice library, describe a character (age, tone, accent), listen to three candidates and keep one. Descriptions that aim at a real person are refused.

To use ElevenLabs, open Settings, paste your key, press Save key, then Test connection. Your key is stored in the Windows Credential Manager, not in the project, and Soundstage never shows it again.

Soundstage does not list cloned, professional or famous voices from your ElevenLabs account, because using someone's voice requires their recorded consent.

> **Tip.** Accent and language quality have not been tested by native speakers yet. Soundstage does not claim a language is supported until that is done.

## Speak, transcribe and clean audio

_Quick ways to use a voice or a recording without making a project._

Not everything needs a project. The Speak and Audio tools screens (in the sidebar, the Go menu and the command palette) do one job at a time.


### Speak

- Search the voice list, type up to 5000 characters, pick a delivery and speed, and press Speak it.
- Play the result, then Save as WAV, MP3, FLAC, M4A, Ogg, Opus or MP4. Choose the format by the file name's ending.
- Online voices ask before sending your text. Audio made here starts as licence unknown, so use a project for anything you will sell.


### Speech to text

- Choose a recording (up to 25 minutes) and press Transcribe. You get the text and, when the engine gives word times, SRT and WebVTT subtitles.
- Works with ElevenLabs, or with your own server: add a Whisper model name to a local engine under Voice engines.
- The recording is converted on your computer first and sent only to the engine you chose.


### Clean audio

Removes background noise and music and keeps the voice (ElevenLabs). Your original is not changed. Recordings up to 10 minutes.

## The prompt helper

_How to describe music, effects and voices, even if you have never written a prompt._

A prompt is just a short description of what you want. Wherever you describe music, a sound effect or a designed voice, press Help me write this.

- Tap the choices that fit (mood, style, speed, place, accent). Your description fills in, and you can keep editing it.
- A hint under the box tells you when the description is clear enough. It never blocks you.
- Start from an example if you are stuck.
- Improve with AI rewrites your description more clearly. It is optional: set a chat server such as Ollama in Settings. Ollama on your own computer sends nothing out. You read the result before anything is made.

> **Tip.** Describe the feeling, the sound and the moment in the film. For voices, describe a character (age, accent, tone), never a real person.

## Consent and voice cloning

_How cloning works, and the rules it follows._

A person's voice is theirs. In the Voice library, Clone a voice makes a synthetic copy from a recording, and only with the owner's consent, which you record in the app.

1. Say whose voice it is and what it will be used for. Tick that the owner is 18 or over, alive, and not a public figure, and that they agree to the statement.
2. Choose a recording of that one person speaking clearly. Soundstage checks the length, the level and the sound before anything is sent.
3. Confirm. The recording and the voice name go to the provider (ElevenLabs). The consent record stays on this computer.

- Children, people who have died, and public figures are refused.
- Soundstage cannot tell who is speaking or whether the owner really agreed. The record is your promise, and it is kept.
- Every render and every export re-checks the consent. If it is revoked, expired, or does not allow commercial use for a commercial project, the voice is blocked.
- Revoke consent any time from the Voice library. New takes and exports stop at once, and you can also delete the voice from the provider.
- Exports list the consent record of every cloned voice used.
- Scripts that look like fraud or impersonation of officials are not rendered, and Soundstage tells you which rule applied.

> **Tip.** Consent records are saved in Documents\Soundstage\.consent, outside your projects. They are not encrypted yet, so keep that folder private.

## Saving, undo and recovery

_Autosave, undo, and where your projects live._

- Soundstage saves automatically a moment after every change. The toolbar shows Saved, Saving or Not saved.
- Ctrl+Z undoes your last edits (about 30 steps). Undo never throws away rendered takes or imported audio.
- If a project file is damaged, Soundstage opens the last good copy.
- Projects are folders in Documents\Soundstage. You can copy a folder to back it up.
- Deleting a project asks you to confirm, naming the project, and removes its folder. Files you exported are not touched.

## Simple and Advanced mode

_The same project, with fewer or more controls._

Simple shows what most scenes need. Advanced adds finer controls: pace and pause per line, direction levels, ducking depth, fades and loops, and detailed licence tools.

It is one project in both modes. Switching hides controls without changing any value, and anything you set in Advanced still applies in Simple. Switch with Ctrl+Shift+M or the toggle in the toolbar.

## Keyboard shortcuts

_The shortcuts you will use most. Press Ctrl+/ for the full, searchable, editable list._

| Keys | Does |
|---|---|
| Ctrl+K | Command palette and search |
| Ctrl+N | New project |
| Ctrl+O | Open a project |
| Ctrl+S | Save now |
| Ctrl+Z / Ctrl+Shift+Z | Undo / redo |
| Ctrl+1 to Ctrl+6 | Go to a step |
| Alt+Left / Alt+Right | Previous / next step |
| Ctrl+Enter | Do the recommended next action |
| Space | Play or stop audio (outside text fields) |
| Ctrl+B | Show or hide the sidebar |
| Ctrl+J | Show or hide the guide panel |
| Ctrl+Shift+M | Switch Simple and Advanced |
| F1 | Help |
| Ctrl+/ | Shortcut reference |

Shortcuts never interfere with typing: inside a text box, undo and redo belong to the box. You can change any shortcut in the reference (Ctrl+/); Soundstage warns you if one is already used.

---

# Fix a problem

## Troubleshooting

_Common problems and what to do._


### No voices are listed

Windows has no speech voices installed. Open Windows Settings, Time and language, Speech, and add one. Then restart Soundstage.


### A line will not render

Open Direct. The line shows what is wrong: a voice that cannot do a tag, a tag Soundstage does not know, or a missing voice. Fix it or remove the tag.


### I cannot play audio

Check your Windows output device and volume. If one line will not play, render it again.


### The mix is quieter than my target

Soundstage will not distort your audio to reach a loudness target. It tells you how far short it is. Speech with a few loud peaks is the usual reason. Lower the loudest line, or accept a quieter master.


### Export is blocked

Your project is set to Commercial and something is not cleared. Each blocked item is listed with the reason. Record your licence check, replace the item, or mark the project personal.


### ElevenLabs says the key was rejected

The key is wrong, expired or revoked. Create a new key in your ElevenLabs account and save it in Settings.


### ElevenLabs is busy or limiting requests

Soundstage retries a few times on its own. If it keeps happening, wait a minute and try again.


### Windows warns about the installer

The installer is not code-signed yet, so Windows SmartScreen may warn. Choose More info, then Run anyway, if you trust where it came from.


### A project will not open

If it was saved by a newer version of Soundstage, update Soundstage. If its file is damaged, Soundstage tries the last good copy automatically.

---

## Screen gallery

_Captured from the real app by `tools/e2e/run.mjs`._

![The welcome screen on first launch.](design/screens/dark-welcome.png)
*The welcome screen on first launch.*

![Projects: start a new scene, or pick up where you left off.](design/screens/dark-1280x720-home.png)
*Projects: start a new scene, or pick up where you left off.*

![Step 1, Script: write or import dialogue with speaker names.](design/screens/dark-1280x720-script.png)
*Step 1, Script: write or import dialogue with speaker names.*

![Step 2, Cast: choose a voice for each speaker.](design/screens/dark-1280x720-cast.png)
*Step 2, Cast: choose a voice for each speaker.*

![Step 3, Direct: set the feeling and pauses for each line.](design/screens/dark-1280x720-direct.png)
*Step 3, Direct: set the feeling and pauses for each line.*

![Step 4, Render: make the audio. Cloud voices ask first.](design/screens/dark-1280x720-render.png)
*Step 4, Render: make the audio. Cloud voices ask first.*

![Step 5, Mix: dialogue, effects and music on one timeline.](design/screens/dark-1280x720-mix.png)
*Step 5, Mix: dialogue, effects and music on one timeline.*

![Create music or an effect with AI, with the prompt helper open.](design/screens/dark-1280x720-mix-create.png)
*Create music or an effect with AI, with the prompt helper open.*

![Step 6, Export: licence checks, then the files.](design/screens/dark-1280x720-export.png)
*Step 6, Export: licence checks, then the files.*

![Export formats, subtitles and the optional FFmpeg setting.](design/screens/dark-1280x720-export-formats.png)
*Export formats, subtitles and the optional FFmpeg setting.*

![A commercial export blocked until licences are recorded.](design/screens/dark-1280x720-export-blocked.png)
*A commercial export blocked until licences are recorded.*

![The voice library, with languages shown by flag.](design/screens/dark-1280x720-library.png)
*The voice library, with languages shown by flag.*

![Design a voice from a description, with the prompt helper.](design/screens/dark-1280x720-voice-design.png)
*Design a voice from a description, with the prompt helper.*

![Speak: type something, choose a voice, hear it and save it as WAV, MP3, FLAC and more.](design/screens/dark-1280x720-speak.png)
*Speak: type something, choose a voice, hear it and save it as WAV, MP3, FLAC and more.*

![Audio tools: speech to text with subtitles, and noise and music removal.](design/screens/dark-1280x720-audio-tools.png)
*Audio tools: speech to text with subtitles, and noise and music removal.*

![Voice engines: local engines, Piper, music and effects, ComfyUI.](design/screens/dark-1280x720-voice-engines.png)
*Voice engines: local engines, Piper, music and effects, ComfyUI.*

![Settings, including the optional AI prompt helper.](design/screens/dark-1280x720-settings.png)
*Settings, including the optional AI prompt helper.*

![The command palette (Ctrl+K).](design/screens/dark-1280x720-palette.png)
*The command palette (Ctrl+K).*

![The shortcut reference, with rebinding.](design/screens/dark-1280x720-shortcuts.png)
*The shortcut reference, with rebinding.*

![The built-in help centre.](design/screens/dark-1280x720-help.png)
*The built-in help centre.*

![The guided tour.](design/screens/dark-1280x720-tour.png)
*The guided tour.*

![The light theme.](design/screens/light-1280x720-mix.png)
*The light theme.*

---

## Glossary

**AI-generated.** Created by an AI model. Soundstage records this in every export manifest.

**Bus.** A group of sounds mixed together: dialogue, effects or music.

**Cache.** Soundstage remembers audio it has already made, so an unchanged line is reused instead of rendered again (and not paid for twice).

**Clipping.** Distortion that happens when audio is pushed past the loudest level a file can hold.

**Cloud voice.** A voice made on a provider's servers. Your text is sent over the internet, and your provider account is billed.

**Commercial use.** Using the result to make money, for example in a film you sell or a paid advertisement.

**Consent record.** A signed, revocable record that the owner of a voice agrees to its use. Required before any voice cloning.

**Credential Manager.** The part of Windows that stores passwords and keys securely. Soundstage keeps your provider key there.

**Cue.** A piece of music or an effect placed on the timeline.

**Direction.** Instructions that shape a performance: emotion, pace, pauses and sounds. Set for the project, a character, a scene or a line.

**Ducking.** Automatically lowering music and effects while someone speaks, so the words stay clear.

**Fade.** A gradual rise or fall in level at the start or end of a sound.

**Gain.** How loud something is, in decibels (dB). Lowering by 6 dB roughly halves the perceived level.

**Licence tag.** A note on every voice, music track and effect saying whether it may be used commercially: commercial, non-commercial or unknown.

**Loop.** Repeating a sound until it fills a scene.

**Loudness range.** How much the loudness varies through the piece. A small range is steady; a large one swings between quiet and loud.

**LUFS.** A measure of how loud audio feels over its whole length. Streaming platforms and broadcasters ask for different targets, for example around -14 LUFS for online video.

**Manifest.** A file saved with each export listing what was used, under which licence, and a fingerprint of every file.

**Master.** The finished mix of everything together.

**Non-verbal sound.** A human sound that is not a word: a laugh, sigh, cough, sneeze, gasp, cry, breath or sniff.

**Out of date.** A line whose words, voice or direction changed since it was last rendered. Also called stale.

**Provider.** A company or program that makes the voice audio, such as Windows or ElevenLabs.

**Render.** Turning the words of a line into audio with a voice.

**Scene.** A continuous piece of the story with its own place and mood.

**Stem.** One bus saved as its own file, so an editor can adjust it later. Soundstage's three stems add back up to the master.

**Tag.** A direction in square brackets, like [sigh] or [pause:short].

**Take.** One rendered version of a line. A line can have many takes; you choose which one is used.

**True peak.** The loudest instant of the audio, including peaks between samples. Soundstage's figure is an estimate, not a certified meter.

**Waveform.** A picture of a sound's loudness over time.
