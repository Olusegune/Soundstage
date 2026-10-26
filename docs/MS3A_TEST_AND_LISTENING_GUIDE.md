# MS3a: what to test, and how to run the listening session

For the product owner. Written 2026-10-25. About 45 minutes for your own test, and about 40 minutes per listening session. Nothing here needs you to read code.

## Part 1. Your own test (45 minutes)

Set up first: install the latest build (`target\release\bundle\nsis\Soundstage_0.2.0_x64-setup.exe`, or run `target\release\soundstage.exe`). Use a project with at least two speakers and four to six lines. Write one stage direction in brackets, for example `(She slams the cupboard.)`. Have at least two voices on this computer, and your ElevenLabs key saved if you want to test online voices. **Online steps cost a little money. Each one asks you first.**

Tick each box. If something differs from "You should see", note the screen and what happened, and send me a screenshot.

### A. The Director and the plan (needs your chat model; skip if you are doing that report separately)
- [ ] **Direct step → Director → "Simple rules only".** You should see suggestions only for lines with `...`, dashes, ALL-CAPS words or a delivery tag. Nothing is sent. Accept one.
- [ ] **"Suggest a plan".** You should see a screen naming the server, how many lines, and a rough token count. Cancel once, then run it. Nothing is sent before you confirm.
- [ ] **Edit the pace or feeling of one planned line.** You should see "Yours: pace" appear on it. Run the Director again and accept. Your edited field must not change.

### B. The plan reaches the take
- [ ] **Render everything first.** All lines show "Rendered".
- [ ] **Put a pause in the plan of one line** (use Simple rules on a line with `...`, or accept a Director suggestion that has a pause). Go to Render. You should see **only that line** as "Out of date", and the others still "Rendered".
- [ ] **Look under that line** for "This voice will not use from the plan: …". That is expected for voices that cannot do a feeling or emphasis.
- [ ] **Render the line again.** It must render, even though the plan asked for something the voice cannot do.
- [ ] **Direct the same line yourself** (choose a feeling chip). Your choice must beat the plan.

### C. Sounds on lines
- [ ] **Direct step → "Add a sound to".** Choose a line, "laugh", "At the start", Add sound. A chip appears on the line.
- [ ] **Render with a voice that cannot laugh** (Windows voices cannot). The line must still render, and the Render step says the voice will not use the sound.
- [ ] **Render with an ElevenLabs voice** (this costs a little). Listen: you should **hear a laugh sound, not the word "laugh"**. If you ever hear the word spoken, tell me at once.
- [ ] **Type `[laugh` with no closing bracket in a line.** You should be told it has no closing bracket and the line will not render until fixed.

### D. Try other voices (audition)
- [ ] **Render step → "Try other voices" on a rendered line.** You should see "Pick up to 4".
- [ ] **Tick one local voice → "Make 1 take".** A take appears with "Use this take". The line's original take and the Cast step are unchanged until you press it.
- [ ] **Tick four voices.** The fifth checkbox must be disabled.
- [ ] **Tick "Blind".** The takes show as Take A, B, C. Choose one, and the names are revealed.
- [ ] **Tick one online voice.** You should be asked before anything is sent, with the characters and provider named. Cancel once to confirm nothing happens.
- [ ] **Turn on Local-only mode** (Settings). The online voice must now fail in plain words, and "Suggest other voices" must not list online voices.

### E. Suggest other voices
- [ ] **Make a line fail**, for example put `[sneeze]` on a line cast with a Windows voice, then press "Render" (or use a voice with no credit). Press **"Suggest other voices"**. You should see voices that can really do it, free ones marked "free, on this computer".
- [ ] **Choose one.** The cast and plan must not change.

### F. Voice library
- [ ] **Voice library → filters.** Use Gender, Age, Accent, Provider, "Can make". Results narrow; "Clear filters" resets. Accents say "not rated".

### G. Other things
- [ ] **Make the window short** (drag the bottom edge up). The project name at the top of the left panel must not be cut in half.
- [ ] **Closing mid-render.** Start rendering six or more lines, close Soundstage after two finish, reopen the project. You should see "Rendering was interrupted" offering to keep what was done.

## Part 2. The listening session

### What we are trying to find out
1. **Do planned takes sound better than unplanned ones?** (feeling, pace, pauses)
2. **Are sounds (laughs, sighs) believable and not spoken as words?**
3. **Is the same voice recognisably the same voice before and after a sound?**

We are not rating accents or languages in this round unless you bring native speakers (see the end).

### Who and how many
- **Five to six listeners.** Different from you, so you are not judging your own work. Ages and backgrounds mixed is good. They do not need to be experts.
- Each person listens alone on **the same kind of headphones** if possible.
- A session is about 40 minutes. Offer a break halfway.

### Prepare the material (about an hour)
Use **three short scenes of 10 to 25 seconds each**, written by you or taken from your own projects (not copyrighted text). Each scene has two speakers and at least one emotional moment. For each scene make **three versions** of the same lines with the same voices:

| Version | How to make it |
|---|---|
| **A: plain** | No plan, no sounds. Render as before. |
| **B: planned** | Accept the Director's suggestions (or your own edits) for pace, pauses and feeling. No sounds. |
| **C: planned plus sounds** | B, and add one or two sounds (for example a sigh before a line, a laugh in a joke) with a voice that supports them. |

Then:
1. **Export each version** as WAV (Export step, WAV 24-bit, master only).
2. **Match the loudness.** The app targets a loudness setting for each destination. Use the same destination (for example "Streaming") for every export so they come out about equally loud. If you can, open the three files in the Audio editor, play one after another, and fix any that are clearly louder.
3. **Rename the files with codes, not names:** `S1-4F.wav`, `S1-9K.wav`, `S1-2M.wav`, and so on. Keep a private key (a table: code to version) that listeners never see. **Use a different random order and different codes for each listener.**
4. **Do not tell listeners which version is which, or which voice is which.**

(A built-in blind package and score sheet arrive with MS4, the Model Lab. For this round it is done by hand.)

### Running a session (read this out)
> "Thank you for helping. You will hear short scenes. For each one you will hear three versions, in a random order. There are no right answers; I want your honest reaction. You can replay each clip as often as you like but please do not skip ahead. I am recording only your score and short comments next to a code, not your name."

For each scene, in the order on the sheet:
1. Listener plays all three clips once through without scoring.
2. For each clip, they fill in the sheet below.
3. Then they answer **"Which one would you rather listen to for a whole episode?"** (1st, 2nd, 3rd).

### The score sheet (one row per clip)

| Code | Natural and believable (1 to 5) | Emotion fits the words (1 to 5) | Did you hear any word that should not be spoken, like "laugh" or "sigh"? (yes/no, which) | Was it the same voice throughout? (yes/no) | Could you understand every word? (yes/no) | Comments |
|---|---|---|---|---|---|---|

Scale for the 1 to 5 scores: **1** unusable, **3** acceptable for a rough draft, **5** could be a good human recording.

A blank copy as a table you can paste into a spreadsheet:

```
listener,scene,code,natural_1to5,emotion_1to5,spoken_tag_word,same_voice,understood_all,preference_rank,comment
```

### What counts as a pass (from the listening protocol)
Counted per version across all listeners:
- **Natural:** median at least 4, and no more than one clip in ten scored below 3.
- **Sounds (version C):** believability median at least 3.5, **zero** "heard a spoken tag word", and "same voice" yes for at least 9 clips in 10.
- **Understood:** at least 95 percent of words understood.
- **Planned (B) beats plain (A):** B is preferred, or tied, in at least half the comparisons.

If a gate is not met, it is not a failure of you: the result tells us what to fix before building more. If fewer than five listeners take part, we mark it "experimental" and say how many there were.

### Afterwards
1. **Remove names from the sheets.** Keep only the listener code (L1 to L6).
2. **Send me the combined table** (paste it here or save it as `listening-ms3a.csv` in the project folder), the private key (code to version), the list of voices used, and any comments.
3. I will compute the numbers, compare them with the gates above, and write a short honest report. If any version passes, MS3b (sounds on their own track, voice-changer reference) is worth building. If not, we fix MS3a first.

### If you want to rate accents or languages too
This needs **at least two native speakers per accent or language**, listening to the same short lines read by the voices you want to test, scoring "Does this sound like [accent]? 1 to 5" and flagging anything that sounds like the wrong accent. Do this as a separate session. A voice is marked "verified" for an accent only when both reviewers score it 4 or more and no more than one clip in ten is flagged wrong. Until then Soundstage keeps saying "not rated".

### Fairness and privacy for your listeners
- Tell them what the clips are and that they are computer-generated.
- Ask their agreement to record scores and comments. Do not record their voice or name.
- Do not play clips that clone a real person's voice. Use library or designed voices only.
