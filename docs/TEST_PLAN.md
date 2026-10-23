# Test plan: Voice Studio

Testing is how we keep quality claims honest. A provider, accent or tag is "supported" only when the tests below say so.

## 1. Layers
| Layer | What it checks | Runs |
|---|---|---|
| Unit | Parsers, tag validation, gate logic, cache keys, loudness math | Every commit |
| Adapter contract | Each adapter meets the interface and error mapping | Every commit (with fakes) and nightly (live, budgeted) |
| Pipeline | Render, cache, mix, export produce the expected manifest and audio properties | Every commit |
| UI | Golden path end to end, keyboard operation, accessibility basics | Every PR touching UI |
| Listening (human) | Accent, language, non-verbal realism, direction | Per milestone and before each new supported claim |
| Consent and licence | Clone refusals, revocation, export blocks, music and effect licence checks | Every commit |
| Mix and delivery | Bus mixing, ducking, loudness per destination, stems alignment, master-vs-stems sum | Every commit (with fixtures) and per milestone |
| Picture sync and ADR | Timecode accuracy, marker placement, ADR timing tolerance, overrun flags | Every commit |

## 2. Adapter contract suite
Every adapter must pass these, run against a fake server in CI and against the live provider nightly.

1. `capabilities()` returns only tags and languages that the adapter actually renders in the listening test.
2. `render()` for a short line returns audio with the declared format and a duration within 20 percent of the text's expected duration.
3. Unknown canonical tags are rejected before any network call.
4. Tags the adapter does not support are rejected before any network call, with `unsupported_tag`.
5. Auth failure maps to `auth`, rate limit to `rate_limit`, timeouts to `network`, provider faults to `provider_error`. Never leaks the key.
6. `provider_version` is returned and non-empty.
7. Output is deterministic enough for caching: the cache key does not change between identical requests.
8. Max-length requests are split by the core, not sent oversize.
9. Adapter never writes project files (asserted by a read-only project fixture).

## 3. Core and pipeline tests
- Cache: identical request makes zero provider calls the second time. Changing one tag, the voice, or the provider version misses the cache.
- Stale marking: editing one line marks only that line and any dependent mix.
- Export gate: a fixture project with a `non-commercial` voice and an `unknown` voice blocks commercial export and lists both by name. Personal export shows the notice and proceeds.
- Licence staleness: a licence checked 91 days ago blocks commercial export until re-checked.
- Consent: a clone without a complete consent record is rejected. A revoked consent blocks new renders with that voice.
- Migration: an older project schema opens and migrates; a newer one is refused with a message.
- Loudness: a fixed test mix measures within 0.5 LU of the target, for each destination in SOUND_DESIGN.md section 5.2.
- Clipping and silence checks flag planted defects.

## 3a. Sound design and mix tests
- **Direction resolution:** a fixture with project, character, scene, and line directions resolves to the documented precedence. Changing one level changes only lines that inherit from it.
- **Takes:** a line with three takes keeps all three; selecting one changes what the timeline plays and exports, and nothing else.
- **Ducking:** a dialogue-over-music fixture drops the music by the configured depth within the configured attack and release (measured within 20 ms and 0.5 dB).
- **Stems alignment:** master and all three stems have identical start time and length (to the sample). Summing the stems reproduces the master within 0.1 dB at every sample window of 100 ms.
- **Stems re-import:** the master and stems imported into a standard editor (test in a headless or scripted environment where possible) re-align with no drift over a 10-minute fixture.
- **Timecode:** at each supported frame rate, a cue at frame N lands at N within one sample; drop-frame behaviour at 29.97 is tested separately.
- **Markers and overruns:** a cue that extends past its scene boundary or marker is flagged.
- **ADR:** a replacement take with duration within tolerance keeps the original start time. One outside tolerance offers rewrite and stretch; the applied option is recorded on the take.
- **Destination loudness:** each fixture mix measures within 0.5 LU and 1 dB true peak of its target, and the manifest records the target.
- **Source music as effect:** a fixture with a diegetic radio cue is not ducked by the score's settings.

## 3b. Music and effects tests
- **Licence gate:** a project with an `unknown` music source blocks commercial export and lists the asset by name. A `commercial` source with `uses_prohibited` matching the project's destination blocks export.
- **Generated assets:** generated effects and music have silence, clipping, and loudness checks on creation.
- **Loop seams:** a cue marked to loop is checked at its seam for click and level step (threshold set in fixtures).
- **Layered effects:** a preset rebuilt after one layer is replaced reproduces the other layers unchanged.
- **Contract (music, effect, library adapters):** each adapter declares licence, output format, sample rate, and maximum duration; returns a non-empty `provider_version`; maps errors to the common set; never writes project files.

## 4. Bake-off protocol (M0 and before any new provider)

### 4.1 Fixture scripts
Create 10 to 15 scripts in `fixtures/scripts/`, each 30 to 90 seconds, covering:
1. Plain narration, English, neutral.
2. Children's educational line with a warm tone.
3. Two-speaker dialogue, with overlap-free turns.
4. Non-verbal moments: a laugh, a sigh, a sneeze, a cough, a gasp, a sniff.
5. Direction: a whisper, a shout, a sad line, a sarcastic line.
6. Pauses: short and long, explicit ms.
7. Code-switching: English with Nigerian Pidgin.
8. Nigerian-accented English.
9. Yoruba or Igbo, with a native speaker's reference recording.
10. Filipino (Tagalog) and Taglish.
11. Indian-accented English.
12. Hindi.
13. Korean.
14. A tricky proper-noun list (names, places, brands).
15. A long paragraph (about 400 words) to test stability.
16. Nigerian Pidgin comedy: rhythm, interjections, fast exchanges.
17. Yoruba romance drama: tender and angry lines in one scene.
18. Hindi-English code-switching (Hinglish) dialogue.
19. Sci-fi narration and a processed or synthetic AI character voice.
20. A non-human or creature voice, for animation.
21. A comedy scene with interruptions and quick turn-taking (true crosstalk is v2).
22. A children's animation line, as one category among many.

Each script has a `notes.md` with direction intent and the reference recording, if one exists. Reference recordings need a consent record (SAFETY_LICENSING.md).

### 4.2 Run
- Same text, same tags (mapped per provider), same target loudness, same post-processing.
- Each provider renders each script, using its best available voice for the target language and accent, and its own designed voice where relevant.
- Record: provider, model version, voice id, date, cost, time to render, licence tag.

### 4.3 Score
Native speakers score each clip blind (provider not shown), on a 1 to 5 scale:
- **Accent authenticity:** does it sound like a native speaker of that accent?
- **Intelligibility:** can a listener follow every word?
- **Naturalness:** does it sound like a person or a machine?
- **Direction:** does the requested emotion or tag come through?
- **Non-verbal realism:** does the laugh, sneeze, or sigh sound human and fit the line?
- **Artefacts:** clicks, repeats, mispronunciations (count).
- **Emotional truth:** does the line land as the character's emotion (tenderness, anger, grief, joy)?
- **Comic timing (comedy scripts only):** do the pauses and rhythm land the joke?

A provider passes a language or accent when:
- Median score is 4 or higher on accent authenticity and intelligibility, and
- No artefact count is above the agreed threshold on the reviewer's own script, and
- Reviewers are at least two per language, from different backgrounds where possible.

Record the results in `tools/bakeoff/results/<date>-<provider>.md`. Update PROVIDERS.md with the outcome.

### 4.4 Reviewers
- Recruit native speakers for: Nigerian English and Pidgin, Yoruba, Igbo, Filipino, Indian English, Hindi, Tamil, Korean.
- Pay reviewers for their time. Store reviewer details separately from results; reviewers are identified by code only.

## 4a. Directing and acting tests (v2 features)
- **Fountain import:** a round trip keeps every scene heading, character, and line. The scene count matches the source.
- **Breakdown and notes:** notes attached to scenes, lines and takes persist across save and reload.
- **Approvals:** an approved take appears in the export manifest with the approver's identifier.
- **Performance reference:** a reference without a matching performer consent record cannot be used, and the feature refuses. With consent, extracted timing matches the reference within a stated tolerance.
- **Speech-to-speech:** output duration within 20 percent of the reference, and intelligible under the bake-off rubric.
- **Read-through:** a scene preview plays in the cast's voices with the same gaps as the script.

## 5. UI tests
- Golden path, scripted end to end, on a fixture project, in under 30 minutes of simulated use.
- Keyboard-only run of the golden path with no mouse.
- Contrast check on every token pair (automated).
- Screens at 1280 by 720 and 1920 by 1080, light and dark (visual regression).
- Reduced-motion setting honoured.

## 6. Performance targets
- App cold start under 4 seconds on a mid-range Windows laptop (VERIFY on pilot hardware).
- Render of a 5-minute, two-speaker script under 1x real time for cloud voices (VERIFY).
- Timeline scroll and zoom at 60 fps with 200 clips.

## 7. Test data and safety
- Fixtures contain no real person's voice without a consent record.
- Live provider tests use a budget cap per run. A run that exceeds it stops and reports.
- No live provider calls from unit or pipeline tests; use the fake adapter.
- Test keys are separate from production keys and are never committed.

## 7a. Sound-for-picture listening tests
Per milestone, reviewers score a full scene mix (not isolated clips) on:
- Dialogue intelligibility at the target loudness on laptop speakers and headphones.
- Music does not mask dialogue beyond the ducking setting.
- Effects land on their marks (a human judgement, with markers shown).
- The scene reads as one piece, not as separately rendered parts.

Record results in `tools/listening/results/<date>-<scene>.md`.

## 8. Release checklist
- [ ] Contract suite green for every adapter in the release, including music, effect, and library adapters.
- [ ] Mix and delivery suite green: stems alignment, ducking, loudness per destination.
- [ ] Sound-for-picture listening test passed for the reference scene.
- [ ] Bake-off results current (under 90 days) for every "supported" claim in the release notes.
- [ ] Licence tags re-checked for every voice and model shipped.
- [ ] Export gate test suite green.
- [ ] Consent flow reviewed against SAFETY_LICENSING.md.
- [ ] Golden path passes on the reference machine.
- [ ] Release notes contain no claim without a bake-off result behind it.
