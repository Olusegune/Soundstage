// The user guide. One source for the in-app Help centre and docs/USER_GUIDE.md (npm run docs).
// Written for intelligent people who are not sound engineers. No jargon without a plain explanation.

export type Block =
  | { t: "p"; text: string }
  | { t: "h"; text: string }
  | { t: "ul"; items: string[] }
  | { t: "steps"; items: string[] }
  | { t: "tip"; text: string }
  | { t: "paid"; text: string }
  | { t: "keys"; rows: [string, string][] };

export interface Article {
  id: string;
  title: string;
  group: "Start here" | "The steps" | "Understanding" | "Fix a problem";
  summary: string;
  tags: string[];
  body: Block[];
}

export const ARTICLES: Article[] = [
  {
    id: "quick-start", title: "Quick start: your first scene in ten minutes", group: "Start here",
    summary: "Make a real two-person scene with free voices, from script to finished files.",
    tags: ["begin", "first", "tutorial", "sample", "new"],
    body: [
      { t: "p", text: "This walks you through a complete scene using the free voices built into Windows. Nothing here costs money and nothing leaves your computer." },
      { t: "steps", items: [
        "On Home, press Create project. Leave Start with a sample script ticked. You land on the Script step with a short kitchen scene.",
        "Read the script. Lines start with a speaker name, like AMA: Good morning. Words in square brackets, like [sigh] or [pause:short], are directions.",
        "Press the gold button at the bottom of the guide panel on the right. It always shows the next useful thing to do. Here it takes you to Cast.",
        "Give AMA and KOLA a voice. Press Make a sample to hear it first.",
        "Open Direct. KOLA's line has a [sigh], which Windows voices cannot perform. Soundstage tells you instead of silently dropping it. Press Remove [sigh] from this line.",
        "Open Render and press Render. Each line becomes audio. Press Play to listen.",
        "Open Mix and press Build a preview mix. Check the loudness readout.",
        "Open Export. Your project is a personal project, so you will see a notice that Windows voices are not cleared for commercial use. Tick the box, choose a folder and export.",
      ] },
      { t: "tip", text: "You can always press Ctrl+K and type what you want to do, like render or export." },
      { t: "p", text: "You now have a master recording, three stem files and a record of what was used." },
    ],
  },
  {
    id: "golden-path", title: "The Golden Path", group: "Start here",
    summary: "The recommended order, and how Soundstage keeps you on it without forcing you.",
    tags: ["steps", "workflow", "guide", "order", "next"],
    body: [
      { t: "p", text: "Making sound for a film goes best in a set order: Script, Cast, Direct, Render, Mix, Export. The sidebar shows these six steps and where you are." },
      { t: "ul", items: [
        "A tick means the step is finished. A highlighted step is the one to do now.",
        "Direct and Mix are optional. Soundstage only pulls Direct forward when a line has a problem that would stop it rendering.",
        "You can always go back to an earlier step. Going forward past an unfinished step is blocked, and the sidebar tells you why.",
        "The guide panel on the right explains why the step matters, what happens if you skip it, and whether it is free.",
      ] },
      { t: "p", text: "The gold button at the bottom of the guide panel (and Ctrl+Enter) does the next recommended action. When that action costs money, the button says so." },
    ],
  },
  {
    id: "writing-script", title: "Step 1: Script", group: "The steps",
    summary: "How to write lines, scenes and directions.",
    tags: ["script", "speaker", "scene", "tags", "pause", "format", "import"],
    body: [
      { t: "p", text: "Start every line with the speaker's name and a colon. Soundstage turns each new name into a character." },
      { t: "p", text: "Start a new scene with a line beginning with # (for example # Kitchen, morning), or with INT. or EXT. like a screenplay heading." },
      { t: "h", text: "Directions in square brackets" },
      { t: "ul", items: [
        "Sounds: [laugh] [chuckle] [sigh] [cough] [sneeze] [gasp] [cry] [breath] [sniff]",
        "Delivery: [whisper] [shout] [excited] [sad] [tired] [sarcastic] [curious] [serious]",
        "Pauses: [pause:short], [pause:long], or an exact length like [pause:350] (milliseconds).",
      ] },
      { t: "p", text: "Only these directions exist. Anything else in brackets is flagged so a typo never reaches a voice. Whether a voice can actually perform a direction depends on the voice; you will see that in Cast and Direct." },
      { t: "tip", text: "Use the tag buttons above the editor to insert a direction at the cursor. You can also import a plain text file." },
    ],
  },
  {
    id: "casting-voices", title: "Step 2: Cast", group: "The steps",
    summary: "Choosing a voice for each character, and hearing samples first.",
    tags: ["voice", "cast", "sample", "character", "elevenlabs", "windows"],
    body: [
      { t: "p", text: "Every speaker needs a voice. Pick one from the list and press Make a sample to hear it say that character's first line." },
      { t: "ul", items: [
        "Windows voices run on your computer, are free, and work offline.",
        "ElevenLabs voices are more expressive and can perform sighs, laughs and more. They need an ElevenLabs key (Settings) and use your ElevenLabs account.",
        "Test tones are beeps for checking timing. They are not speech.",
      ] },
      { t: "paid", text: "A sample from a cloud voice uses your provider account. Soundstage asks before sending anything." },
      { t: "p", text: "Changing a voice after you have rendered makes those lines out of date so they can be rendered again. Earlier takes are kept." },
      { t: "p", text: "Character notes (age, background, habits) stay with the project as context for you." },
    ],
  },
  {
    id: "directing-performance", title: "Step 3: Direct", group: "The steps",
    summary: "Emotion, pace, pauses, and fixing what a voice cannot do.",
    tags: ["direction", "emotion", "pace", "pause", "unsupported", "performance", "notes"],
    body: [
      { t: "p", text: "Direction is set at four levels, from broad to fine: the whole project, a character, a scene, and a single line. A finer level overrides a broader one. Every line shows where its delivery comes from, for example emotion tired, from scene." },
      { t: "ul", items: [
        "Emotion changes how a line is spoken.",
        "Pace speeds up or slows down delivery.",
        "Pause before adds silence ahead of a line.",
        "A director's note stays with the line as a reminder.",
      ] },
      { t: "h", text: "When a voice cannot do something" },
      { t: "p", text: "If a line asks for [sneeze] and its voice cannot sneeze, Soundstage shows Not supported by this voice before you render. Your choices are to remove the tag from that line, or choose another voice. Nothing is ever dropped silently." },
      { t: "tip", text: "Press Preview line to hear just that line. A line that has not changed is reused, not rendered again." },
    ],
  },
  {
    id: "rendering", title: "Step 4: Render", group: "The steps",
    summary: "Turning lines into audio, takes, and what out of date means.",
    tags: ["render", "take", "stale", "cache", "approve", "skip", "waveform"],
    body: [
      { t: "p", text: "Rendering makes the audio for each line. The wavy picture beside a line is its waveform: the shape of the sound over time. Flat means silence; a wall of full height means clipping." },
      { t: "ul", items: [
        "Every render makes a take. Open Takes to compare versions, choose one, and approve, hold or reject it.",
        "Out of date means you changed the words, voice or direction since the last render. Only those lines need rendering again.",
        "Skip leaves a line without audio on purpose. Include brings it back.",
        "A rejected take is left out of the mix and export.",
      ] },
      { t: "paid", text: "Rendering with a cloud voice is billed by the provider. Soundstage cannot show the price in advance, so it tells you which provider receives how much text and waits for your yes." },
    ],
  },
  {
    id: "mixing", title: "Step 5: Mix", group: "The steps",
    summary: "Music, effects, ducking and loudness.",
    tags: ["mix", "music", "effects", "ducking", "loudness", "lufs", "timeline", "import"],
    body: [
      { t: "p", text: "The mix has three groups of sound: dialogue, effects and music. Dialogue is laid out in script order with the gaps you choose." },
      { t: "ul", items: [
        "Import music or effects (WAV, MP3, FLAC, Ogg or M4A), or create them with AI: describe the sound under Create with AI and choose a generator. Created sounds are labelled AI-generated and start as licence unknown.",
        "Add to timeline places a file at a scene, with an offset, a level and fades. Loop to fill the scene repeats music for the scene's length.",
        "Ducking lowers music and effects while someone is speaking so dialogue stays clear. Turn it off for music that is part of the scene, like a radio.",
        "Where will this play? sets a loudness target. See the glossary for LUFS.",
      ] },
      { t: "p", text: "Build a preview mix to hear the result and see its loudness. If a target cannot be reached without distorting, Soundstage flags it instead of squashing your audio." },
      { t: "tip", text: "Every imported file starts as licence unknown. Record where you checked its terms before a commercial export." },
    ],
  },
  {
    id: "exporting", title: "Step 6: Export", group: "The steps",
    summary: "Licence checks, the files you get, and the manifest.",
    tags: ["format", "mp3", "mp4", "flac", "subtitles", "srt", "ffmpeg", "export", "licence", "license", "commercial", "personal", "stems", "manifest", "master"],
    body: [
      { t: "p", text: "Before anything is written, Soundstage checks every voice, engine, music track and effect against your project's target." },
      { t: "ul", items: [
        "Personal projects can use anything. You see a notice for items not cleared for commercial use and must acknowledge it.",
        "Commercial projects can only use items tagged commercial and checked within the last 90 days. Anything else blocks export and is listed by name with the reason.",
        "Record my licence check lets you state what you found, where you read it and when. Soundstage cannot read terms for you.",
      ] },
      { t: "h", text: "What you get" },
      { t: "ul", items: [
        "A master recording in the format you choose: WAV (16, 24 or 32-bit float), or with FFmpeg installed FLAC, MP3, AAC, Opus, Ogg Vorbis, or an MP4 video with a waveform picture.",
        "Three stems (dialogue, effects, music) of exactly the same length, so they line up in an editor and add back up to the master.",
        "Optional subtitles (SRT or WebVTT) built from your script and its timings, with speaker names if you like.",
        "A manifest listing every voice, provider, licence decision and file fingerprint, and recording that the dialogue is AI-generated.",
      ] },
      { t: "p", text: "Every export goes into a new folder. Nothing is ever overwritten." },
      { t: "tip", text: "WAV needs nothing extra. The other formats use FFmpeg, a free program Soundstage does not include. Install it once (for example winget install Gyan.FFmpeg) or choose ffmpeg.exe on the Export screen." },
    ],
  },
  {
    id: "free-vs-paid", title: "What is free and what costs money", group: "Understanding",
    summary: "A plain account of what can use your provider account.",
    tags: ["cost", "price", "paid", "free", "billing", "credits", "elevenlabs"],
    body: [
      { t: "p", text: "Almost everything in Soundstage is free and runs on your computer: writing, casting, directing, rendering with Windows voices, mixing and exporting." },
      { t: "paid", text: "The only things that can cost money are cloud services you connect (ElevenLabs, Kie, fal, OpenRouter): rendering a line, making a sample, designing a voice, or creating music or an effect. Each is billed to your own account at that provider." },
      { t: "ul", items: [
        "Soundstage always asks first and says which provider gets how much text.",
        "It does not show a price, because providers change them and Soundstage will not guess.",
        "A line that has not changed is reused, so you do not pay twice for the same audio.",
        "Nothing is sent until you press the confirming button.",
      ] },
    ],
  },
  {
    id: "providers", title: "Voices and providers", group: "Understanding",
    summary: "Windows voices, ElevenLabs, keys, and what each can do.",
    tags: ["provider", "elevenlabs", "key", "windows", "api", "settings", "voices", "local", "piper", "comfyui", "suno", "kie", "fal", "openrouter", "ollama", "design"],
    body: [
      { t: "p", text: "Soundstage works with several voice engines behind one set of directions, so changing engine does not mean rewriting your script." },
      { t: "ul", items: [
        "Windows voices: free, offline. Can change pace and loudness for whisper, shout, excited, sad and tired. Cannot laugh, sigh or cough.",
        "Your own engines: connect a speech server on your computer (Kokoro, LocalAI, Speaches and many others speak the same OpenAI-style language), or Piper. They are free and need no account.",
        "ElevenLabs: cloud. Can perform laugh, sigh, whisper, excited, sarcastic, curious and cry, according to its documentation. Cough, sneeze, sniff, gasp, chuckle, breath, shout, sad, tired and serious are not offered.",
      ] },
      { t: "p", text: "No account? Open Voice engines and add a local engine or Piper. Ollama runs language models, not speech, so it cannot be a voice engine. ComfyUI works through a workflow you save from it in API format." },
      { t: "p", text: "Music and effects: ElevenLabs makes both with the same key. Suno songs come through a Kie key. fal models and ComfyUI workflows can be added under Voice engines. Everything made starts as licence unknown." },
      { t: "p", text: "Design a voice: in the Voice library, describe a character (age, tone, accent), listen to three candidates and keep one. Descriptions that aim at a real person are refused." },
      { t: "p", text: "To use ElevenLabs, open Settings, paste your key, press Save key, then Test connection. Your key is stored in the Windows Credential Manager, not in the project, and Soundstage never shows it again." },
      { t: "p", text: "Soundstage does not list cloned, professional or famous voices from your ElevenLabs account, because using someone's voice requires their recorded consent." },
      { t: "tip", text: "Accent and language quality have not been tested by native speakers yet. Soundstage does not claim a language is supported until that is done." },
    ],
  },
  {
    id: "consent", title: "Consent and voice cloning", group: "Understanding",
    summary: "Why cloning is off, and the rules it will follow.",
    tags: ["consent", "clone", "cloning", "safety", "rights", "voice", "permission"],
    body: [
      { t: "p", text: "A person's voice is theirs. Voice cloning is not in this version. When it arrives it will only work with a complete, signed, revocable consent record from the voice's owner, and it will be impossible without one." },
      { t: "ul", items: [
        "Public figures, children and deceased people are refused.",
        "Revoking consent immediately blocks new renders and exports that use the voice.",
        "Scripts that look like fraud or impersonation of officials are not rendered, and Soundstage tells you which rule applied.",
      ] },
    ],
  },
  {
    id: "saving", title: "Saving, undo and recovery", group: "Understanding",
    summary: "Autosave, undo, and where your projects live.",
    tags: ["save", "autosave", "undo", "redo", "backup", "recovery", "folder", "delete"],
    body: [
      { t: "ul", items: [
        "Soundstage saves automatically a moment after every change. The toolbar shows Saved, Saving or Not saved.",
        "Ctrl+Z undoes your last edits (about 30 steps). Undo never throws away rendered takes or imported audio.",
        "If a project file is damaged, Soundstage opens the last good copy.",
        "Projects are folders in Documents\\Soundstage. You can copy a folder to back it up.",
        "Deleting a project asks you to confirm, naming the project, and removes its folder. Files you exported are not touched.",
      ] },
    ],
  },
  {
    id: "simple-advanced", title: "Simple and Advanced mode", group: "Understanding",
    summary: "The same project, with fewer or more controls.",
    tags: ["simple", "advanced", "mode", "controls", "beginner", "expert"],
    body: [
      { t: "p", text: "Simple shows what most scenes need. Advanced adds finer controls: pace and pause per line, direction levels, ducking depth, fades and loops, and detailed licence tools." },
      { t: "p", text: "It is one project in both modes. Switching hides controls without changing any value, and anything you set in Advanced still applies in Simple. Switch with Ctrl+Shift+M or the toggle in the toolbar." },
    ],
  },
  {
    id: "keyboard", title: "Keyboard shortcuts", group: "Understanding",
    summary: "The shortcuts you will use most. Press Ctrl+/ for the full, searchable, editable list.",
    tags: ["keys", "shortcut", "hotkey", "keyboard", "ctrl"],
    body: [
      { t: "keys", rows: [
        ["Ctrl+K", "Command palette and search"], ["Ctrl+N", "New project"], ["Ctrl+O", "Open a project"], ["Ctrl+S", "Save now"],
        ["Ctrl+Z / Ctrl+Shift+Z", "Undo / redo"], ["Ctrl+1 to Ctrl+6", "Go to a step"], ["Alt+Left / Alt+Right", "Previous / next step"],
        ["Ctrl+Enter", "Do the recommended next action"], ["Space", "Play or stop audio (outside text fields)"], ["Ctrl+B", "Show or hide the sidebar"],
        ["Ctrl+J", "Show or hide the guide panel"], ["Ctrl+Shift+M", "Switch Simple and Advanced"], ["F1", "Help"], ["Ctrl+/", "Shortcut reference"],
      ] },
      { t: "p", text: "Shortcuts never interfere with typing: inside a text box, undo and redo belong to the box. You can change any shortcut in the reference (Ctrl+/); Soundstage warns you if one is already used." },
    ],
  },
  {
    id: "troubleshooting", title: "Troubleshooting", group: "Fix a problem",
    summary: "Common problems and what to do.",
    tags: ["problem", "error", "fix", "help", "not working", "silent", "blocked"],
    body: [
      { t: "h", text: "No voices are listed" },
      { t: "p", text: "Windows has no speech voices installed. Open Windows Settings, Time and language, Speech, and add one. Then restart Soundstage." },
      { t: "h", text: "A line will not render" },
      { t: "p", text: "Open Direct. The line shows what is wrong: a voice that cannot do a tag, a tag Soundstage does not know, or a missing voice. Fix it or remove the tag." },
      { t: "h", text: "I cannot play audio" },
      { t: "p", text: "Check your Windows output device and volume. If one line will not play, render it again." },
      { t: "h", text: "The mix is quieter than my target" },
      { t: "p", text: "Soundstage will not distort your audio to reach a loudness target. It tells you how far short it is. Speech with a few loud peaks is the usual reason. Lower the loudest line, or accept a quieter master." },
      { t: "h", text: "Export is blocked" },
      { t: "p", text: "Your project is set to Commercial and something is not cleared. Each blocked item is listed with the reason. Record your licence check, replace the item, or mark the project personal." },
      { t: "h", text: "ElevenLabs says the key was rejected" },
      { t: "p", text: "The key is wrong, expired or revoked. Create a new key in your ElevenLabs account and save it in Settings." },
      { t: "h", text: "ElevenLabs is busy or limiting requests" },
      { t: "p", text: "Soundstage retries a few times on its own. If it keeps happening, wait a minute and try again." },
      { t: "h", text: "Windows warns about the installer" },
      { t: "p", text: "The installer is not code-signed yet, so Windows SmartScreen may warn. Choose More info, then Run anyway, if you trust where it came from." },
      { t: "h", text: "A project will not open" },
      { t: "p", text: "If it was saved by a newer version of Soundstage, update Soundstage. If its file is damaged, Soundstage tries the last good copy automatically." },
    ],
  },
];

export interface GlossaryEntry { term: string; meaning: string; aka?: string[] }

export const GLOSSARY: GlossaryEntry[] = [
  { term: "Take", meaning: "One rendered version of a line. A line can have many takes; you choose which one is used." },
  { term: "Render", meaning: "Turning the words of a line into audio with a voice." },
  { term: "Out of date", meaning: "A line whose words, voice or direction changed since it was last rendered. Also called stale.", aka: ["stale"] },
  { term: "Cache", meaning: "Soundstage remembers audio it has already made, so an unchanged line is reused instead of rendered again (and not paid for twice)." },
  { term: "Direction", meaning: "Instructions that shape a performance: emotion, pace, pauses and sounds. Set for the project, a character, a scene or a line." },
  { term: "Tag", meaning: "A direction in square brackets, like [sigh] or [pause:short]." },
  { term: "Non-verbal sound", meaning: "A human sound that is not a word: a laugh, sigh, cough, sneeze, gasp, cry, breath or sniff." },
  { term: "Scene", meaning: "A continuous piece of the story with its own place and mood." },
  { term: "Cue", meaning: "A piece of music or an effect placed on the timeline." },
  { term: "Bus", meaning: "A group of sounds mixed together: dialogue, effects or music." },
  { term: "Stem", meaning: "One bus saved as its own file, so an editor can adjust it later. Soundstage's three stems add back up to the master.", aka: ["stems"] },
  { term: "Master", meaning: "The finished mix of everything together." },
  { term: "Ducking", meaning: "Automatically lowering music and effects while someone speaks, so the words stay clear." },
  { term: "Gain", meaning: "How loud something is, in decibels (dB). Lowering by 6 dB roughly halves the perceived level." },
  { term: "Fade", meaning: "A gradual rise or fall in level at the start or end of a sound." },
  { term: "Loop", meaning: "Repeating a sound until it fills a scene." },
  { term: "Waveform", meaning: "A picture of a sound's loudness over time." },
  { term: "Clipping", meaning: "Distortion that happens when audio is pushed past the loudest level a file can hold." },
  { term: "LUFS", meaning: "A measure of how loud audio feels over its whole length. Streaming platforms and broadcasters ask for different targets, for example around -14 LUFS for online video.", aka: ["loudness"] },
  { term: "True peak", meaning: "The loudest instant of the audio, including peaks between samples. Soundstage's figure is an estimate, not a certified meter.", aka: ["dBTP"] },
  { term: "Loudness range", meaning: "How much the loudness varies through the piece. A small range is steady; a large one swings between quiet and loud." },
  { term: "Provider", meaning: "A company or program that makes the voice audio, such as Windows or ElevenLabs." },
  { term: "Cloud voice", meaning: "A voice made on a provider's servers. Your text is sent over the internet, and your provider account is billed." },
  { term: "Licence tag", meaning: "A note on every voice, music track and effect saying whether it may be used commercially: commercial, non-commercial or unknown." },
  { term: "Commercial use", meaning: "Using the result to make money, for example in a film you sell or a paid advertisement." },
  { term: "Consent record", meaning: "A signed, revocable record that the owner of a voice agrees to its use. Required before any voice cloning." },
  { term: "Manifest", meaning: "A file saved with each export listing what was used, under which licence, and a fingerprint of every file." },
  { term: "AI-generated", meaning: "Created by an AI model. Soundstage records this in every export manifest." },
  { term: "Credential Manager", meaning: "The part of Windows that stores passwords and keys securely. Soundstage keeps your provider key there." },
];

export const TOUR_STEPS = [
  { target: "rail", title: "Your path through the scene", body: "These six steps are the Golden Path. A tick means done, the highlighted one is what to do now, and you can always go back." },
  { target: "guide", title: "Always know what is next", body: "This panel explains why the step matters, what skipping it does, and whether it is free." },
  { target: "next", title: "One button for the next move", body: "It always does the next useful thing. When something costs money, it says so before you press it." },
  { target: "palette", title: "Find anything fast", body: "Press Ctrl+K to jump to a step, a project, a line of your script, a voice or a help article." },
  { target: "mode", title: "Simple or Advanced", body: "Start simple. Switch to Advanced for finer control. It is the same project either way." },
  { target: "help", title: "Help is always one key away", body: "Press F1 for the guide, the glossary and troubleshooting. Every step also has its own help link." },
] as const;
