# Soundstage privacy notice

_Draft for review by the product owner and a lawyer before public release._

## What stays on your computer
Your projects, scripts, takes, mixes and exports are saved in `Documents\Soundstage` on your computer. Preferences are stored in the app's local storage. Soundstage has no account system and **no telemetry**: it does not report usage, crashes or errors anywhere.

## What can leave your computer, and only when you choose it
Soundstage asks before sending anything. Depending on what you set up:
- **Online voices (ElevenLabs, OpenRouter, other online engines you add):** the text of the lines you render, and the voice you chose.
- **Voice cloning:** the recording of the voice (trimmed to three minutes), and the name you give the voice, sent to the provider you chose. The consent record is saved only on your computer and is never sent.
- **Speech to text and audio cleaning:** the recording you choose, converted on your computer, sent to the engine you chose.
- **VoiceKiller:** the text of the lines you render and the voice name, sent to voicekiller.com.
- **Music, effects and voice design:** the description you typed, sent to the service you chose (ElevenLabs, Kie, fal).
- **AI prompt helper:** the description you typed, sent to the chat server you set in Settings. If that is Ollama on your computer, nothing leaves it.
Local engines, Piper, ComfyUI on this computer and Windows voices send nothing.

Each service has its own privacy terms and billing. Soundstage cannot see or change how they store or use what you send.

## Keys
API keys are stored in the Windows Credential Manager, never in a project, an export or a log. Soundstage does not show a saved key again.

## Reports
"Report a problem" writes a text file on your computer with versions and the System check results. It contains no keys, script text or project names. Nothing is sent unless you attach it to a message yourself.

## Generated audio
Dialogue, music and effects made with AI are labelled as AI-generated in the export manifest. Voices and generated audio start as "licence unknown" until you record where you checked the provider's terms.

## Contact
_Add the support address here before release._
