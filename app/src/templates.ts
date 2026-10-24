// Starting scripts for new projects. They are plain scripts in Soundstage's own format, so anything here can be edited.
export interface Template { id: string; name: string; blurb: string; script: string }

export const TEMPLATES: Template[] = [
  {
    id: "podcast", name: "Podcast episode", blurb: "A host and a guest, with an intro and an outro.",
    script: `# Intro
HOST: [pause:short] Welcome to the show. I'm your host, and today we have a very special guest.
HOST: Before we begin, a quick word about what we will cover.

# Conversation
HOST: Thank you for joining us. Tell us a little about yourself.
GUEST: Thanks for having me. [pause:short] It's a pleasure to be here.
HOST: What is the one thing you wish more people knew about your work?
GUEST: That it starts with listening. [pause:short] Everything else follows from that.

# Outro
HOST: That's all for today. If you enjoyed this episode, tell a friend. [pause:long] See you next time.
`,
  },
  {
    id: "audiobook", name: "Audiobook chapter", blurb: "A narrator and a little dialogue.",
    script: `# Chapter One
NARRATOR: The rain had not stopped for three days. [pause:short] In the village, the river was rising.
NARRATOR: Ama stood at the window and watched the water creep toward the road.
AMA: [whisper] Kola, come and look at this.
KOLA: [pause:short] It's higher than yesterday.
NARRATOR: Neither of them spoke for a long time. [pause:long] Then the first bell began to ring.
`,
  },
  {
    id: "ad", name: "30-second ad", blurb: "One announcer, short and punchy.",
    script: `# Spot
ANNOUNCER: Tired of waiting? [pause:short] There is a better way.
ANNOUNCER: Meet Swift, the app that gets things done before you finish your coffee.
ANNOUNCER: [excited] Download it today. [pause:short] Swift. Done.
`,
  },
  {
    id: "explainer", name: "Explainer video narration", blurb: "A calm narrator walking through three steps.",
    script: `# Explainer
NARRATOR: Every great idea starts with a single step. [pause:short] Here is how it works.
NARRATOR: Step one. [pause:short] Write down what you want to say.
NARRATOR: Step two. [pause:short] Choose a voice that fits your story.
NARRATOR: Step three. [pause:short] Listen, adjust, and share.
NARRATOR: [pause:long] It really is that simple.
`,
  },
  {
    id: "dialogue", name: "Short drama scene", blurb: "Two characters, with feeling and silence.",
    script: `# Kitchen, morning
AMA: [sigh] You didn't sleep, did you?
KOLA: [pause:short] Not really.
AMA: [whisper] Tell me what's wrong.
KOLA: [pause:long] They called last night. [pause:short] It's about the house.
AMA: [pause:short] Then we'll face it together.
`,
  },
];
