# Providers: Voice Studio

Status: research snapshot dated 2026-10-23, from public search results. Anything marked `VERIFY` must be checked against the provider's current documentation and licence before it is coded or promised to users. Re-check every entry at least quarterly, and before each milestone.

## 1. Provider matrix (initial)

| Provider | Type | Expressive tags | Languages (claimed) | Licence | Status |
|---|---|---|---|---|---|
| ElevenLabs (Eleven v3) | Cloud | Yes: inline tags such as laughs, whispers, sighs, excited | 70+ | Commercial plan terms. VERIFY output rights and API resale terms | **Adapter shipped in v0.1** (`adapters/elevenlabs.rs`). Approved by Victor as a paid provider and network host. Tested against a fake server only; live check pending (section 9) |
| ElevenLabs Voice Design v3 | Cloud | Yes, via text | Multilingual. VERIFY | Same as above. VERIFY | Candidate, M0 |
| OmniVoice | Open model (self-hosted) | Yes: [laughter], [sigh], [sniff], [surprise-*], [question-*] (per its README) | Claims 600+ (treat as unverified until tested) | VERIFY. Non-commercial terms are common in this category | Candidate, M0 |
| Qwen3-TTS (Alibaba) | Open model | Style control via instructions | At least 10, including Chinese, English, Japanese, Korean | Apache 2.0 per announcement. VERIFY the specific checkpoints | Candidate, M0 |
| VoxCPM2 (OpenBMB) | Open model, local | Voice design via description | Multilingual (VERIFY list) | Apache 2.0 per guide. VERIFY | Candidate, M4 |
| Rime Coda | Cloud | Expressive; laughter mentioned for Mist | English, Spanish, French, Portuguese, German, Japanese | VERIFY | Watch list (no Nigerian, Filipino, Indian or Korean languages listed) |
| Voxtral TTS (Mistral) | Open weights | Not assessed | 9 languages, 3-second cloning | Reported as non-commercial. VERIFY | Excluded for commercial use unless licence changes |
| Magpie-Multilingual 357M (NVIDIA) | Open model | Not assessed | 9 languages | Reported as commercial-safe. VERIFY | Candidate, M4 |
| Cartesia (Sonic) | Cloud | Emotion control; check tag support | 42 | VERIFY | Watch list |

Notes:
- Coverage of **Nigerian (Yoruba, Igbo, Nigerian Pidgin, Nigerian-accented English), Filipino (Tagalog, Taglish, Filipino-accented English), Indian (Hindi, Tamil, Indian-accented English), and Korean** is the hardest test. No source reviewed so far confirms authentic Nigerian or Filipino accent support from any provider. Treat every claim as unproven until the bake-off says otherwise.
- Pricing is not recorded here because published figures conflict across sources. Record verified prices with a date when each provider is confirmed.

## 2. Canonical direction layer

The app uses one tag vocabulary. Adapters translate it. UI and project files never use provider-native syntax.

### 2.1 Canonical tags (v1)
Non-verbal: `[laugh]`, `[chuckle]`, `[sigh]`, `[cough]`, `[sneeze]`, `[gasp]`, `[cry]`, `[breath]`, `[sniff]`
Delivery: `[whisper]`, `[shout]`, `[excited]`, `[sad]`, `[tired]`, `[sarcastic]`, `[curious]`, `[serious]`
Timing: `[pause:short]` (about 250 ms), `[pause:long]` (about 800 ms), `[pause:<ms>]` (explicit)

Rules:
- Tags are case-insensitive in input and stored in lower case.
- Unknown tags are a validation error. They are never sent to a provider.
- A tag a voice cannot produce is flagged before render with "not supported by this voice". The user can pick another voice or drop the tag. Silent removal is not allowed.

### 2.2 Mapping table (initial, all entries VERIFY)

| Canonical | ElevenLabs v3 | OmniVoice | Other |
|---|---|---|---|
| laugh | `[laughs]` | `[laughter]` | VERIFY per provider |
| sigh | `[sighs]` | `[sigh]` | VERIFY |
| sneeze | not confirmed in sources reviewed. VERIFY | not confirmed | VERIFY |
| cough | not confirmed in sources reviewed. VERIFY | not confirmed | VERIFY |
| whisper | `[whispers]` | whisper attribute on voice design (VERIFY) | VERIFY |
| sniff | not confirmed | `[sniff]` | VERIFY |

The sneeze and cough rows are the critical gap. If no cloud or open provider produces them convincingly, the fallback is a licensed sound-effect library placed on the timeline, with the line split around it. That is a product decision (see PRD section 12), not a silent fallback.

### 2.3 ElevenLabs as implemented (verified from public docs, 2026-10-23)

Sources: `elevenlabs.io/docs/api-reference/text-to-speech/convert`, `.../voices/search`, `.../eleven-api/resources/errors`, `.../overview/models`, `.../overview/capabilities/text-to-speech/best-practices`.

| Item | Value used |
|---|---|
| Host | `https://api.elevenlabs.io` only. Redirects are never followed |
| Auth | `xi-api-key` header, injected by the core from the Windows Credential Manager |
| Render | `POST /v1/text-to-speech/{voice_id}?output_format=pcm_24000`, body `text`, `model_id: eleven_v3`, optional `voice_settings.speed` (0.7 to 1.2) |
| Voices | `GET /v2/voices?page_size=100`, paged with `next_page_token` while `has_more` |
| Limit | Eleven v3: 5,000 characters per request. The adapter splits at 4,500 |
| Errors | `detail.code`: `invalid_api_key` and `missing_api_key` (401), `rate_limit_exceeded`, `concurrent_limit_exceeded`, `system_busy` (429), `voice_not_found` (404), `insufficient_credits` (402), 422 for rejected input. Mapped to the common error set. 429 retries with backoff, 3 times |
| Pauses | Eleven v3 does not support SSML `<break>`. The adapter renders segments and inserts exact silence |
| Voices listed | `premade` and `generated` only. `cloned`, `professional`, `famous` and other categories are never listed, because they need a consent record |

Canonical tag mapping (only tags ElevenLabs documents):

| Canonical | Eleven v3 | Canonical | Eleven v3 |
|---|---|---|---|
| laugh | `[laughs]` | curious | `[curious]` |
| sigh | `[sighs]` | cry | `[crying]` |
| whisper | `[whispers]` | sarcastic | `[sarcastic]` |
| excited | `[excited]` | | |

Not mapped, so reported as "not supported by this voice": cough, sneeze, sniff, gasp, chuckle, breath, shout, sad, tired, serious. The docs list `[exhales]` and `[happy gasp]` but not as general tags, so they are left out until the bake-off shows they work. **The sneeze and cough gap remains open** (section 2.2).

## 3. Routing policy

Choose the engine per line, based on the first match:
1. **Locked voice.** If a voice is cast, use its provider. Never switch providers mid-character.
2. **Language and accent support.** Use only providers that passed the listening test for that language and accent.
3. **Tags needed.** Use only providers that support every tag in the line, or split the line.
4. **Licence.** Use only voices whose licence matches the project's `licence_target`.
5. **Cost and speed.** Prefer local or cheaper engines when quality passes.
6. **Offline.** If offline, use local voices only.

The routing decision is recorded on each rendered line so it can be explained and reproduced.

## 4. Adapter contract

Every adapter implements:

```
id: string                       // "elevenlabs-v3", "omnivoice-local"
licence: "commercial" | "non-commercial" | "unknown"
capabilities(): {
  languages: string[],           // BCP 47
  tags: CanonicalTag[],          // the canonical tags this adapter can render
  voice_design: boolean,
  cloning: boolean,
  max_chars_per_request: number,
  streaming: boolean
}
list_voices(): Voice[]
design_voice(params): Voice                 // only if voice_design
clone_voice(consent_ref, reference_audio): Voice   // only if cloning, gated by core
render(request: { text, voice, direction, language }): { audio: bytes, format, duration_ms, provider_version }
estimate_cost(request): { units, amount, currency } | null
```

Rules:
- Adapters never persist project state and never read the keys store directly. The core injects credentials.
- Adapters must map errors into the common error set (auth, rate_limit, unsupported_tag, unsupported_language, quota, network, provider_error) so the UI can explain them.
- `provider_version` (model id and date) is stored with every rendered line.

## 5. Adding a provider (checklist)
1. Create `adapters/<id>/` with README.md (verified facts, dated, with source URLs).
2. Record licence and terms in this file, and the licence tag used in the voice registry.
3. Implement the contract. Write contract tests first (TEST_PLAN.md section 2).
4. Map every canonical tag the adapter supports; mark the rest as unsupported.
5. Run the bake-off on the fixture scripts. Record results in `tools/bakeoff/results/<date>-<id>.md`.
6. Get listening-test sign-off for each language and accent before marking it "supported".
7. Add the network host to the allowlist.

## 6. Watch list and exclusions
- **Exclude for commercial use** until licence terms are confirmed: any model whose published terms are non-commercial (for example, Voxtral TTS as currently reported).
- **Do not promise** any accent or language until the bake-off passes it.

## 7. Language coverage roadmap
A language is listed as supported only after the bake-off passes it (TEST_PLAN.md section 4). Candidate providers are filled in during M0 and are not yet verified.

| Language | Market relevance | Target phase | Candidate providers | Native reviewers needed |
|---|---|---|---|---|
| English (Nigerian, Indian, Filipino, US, UK) | Core | v1 | To fill in (VERIFY) | Yes, per accent |
| Nigerian Pidgin | Comedy, drama | v1 | To fill in (VERIFY) | Yes |
| Yoruba | Drama, comedy | v1 | To fill in (VERIFY) | Yes |
| Hindi | Film, TV | v1 | To fill in (VERIFY) | Yes |
| Igbo, Hausa | Nigerian film | v2 | To fill in (VERIFY) | Yes |
| Tamil, Telugu, Bengali, Marathi, Punjabi, Kannada, Malayalam | Indian regional film | v2 | To fill in (VERIFY) | Yes |
| Tagalog and Filipino English | Film, TV, comedy | v2 | To fill in (VERIFY) | Yes |
| Korean | Film, drama | v2 | To fill in (VERIFY) | Yes |
| Brazilian Portuguese, Spanish, French, Arabic | International film | v2 or v3 | To fill in (VERIFY) | Yes |

## 8. Speech-to-speech and performance transfer
Used for the v2 performance reference feature (PRD FR-15). Nothing here is verified yet.

- Requirement: take an actor's guide take and output the chosen voice with the actor's timing, stress, and emotional contour.
- Candidates, to be checked per provider (VERIFY): whether speech-to-speech conversion is offered, which languages it covers, its licence for commercial output, and whether it needs the source performer's consent record.
- Fallback if no provider passes: direction through tags and the performer's notes, with the reference used only for timing.

## 9. VERIFY log (owner: Victor unless stated)
Logged 2026-10-23 when the ElevenLabs adapter shipped.
1. **Live call.** The adapter has only run against a local fake server built from the docs. Run a real render and voice list with a real key, and record the result here.
2. **`pcm_24000` and plan.** The docs name plan limits for 192 kbps MP3 and 44.1 kHz PCM but none for `pcm_24000`. Confirm it works on the plan in use.
3. **Tag list for v3.** The tag list comes from the current prompting page, which says it also applies to v3. Confirm each mapped tag on `eleven_v3` by listening. Until the bake-off passes, the UI says tags are "not yet listening-tested".
4. **`eleven_v4`.** The docs now list `eleven_v4` (90+ languages, 10,000 characters). Decide whether to move to it.
5. **Commercial terms per plan.** Voices are `licence unknown` until a user records their own check. Read the current terms and decide whether Soundstage should ship a default.
6. **Retention.** `enable_logging` defaults to true (zero-retention is Enterprise only). The UI says text is sent and billed. Decide whether to say more about retention.
7. **Pricing.** Not recorded and not estimated. Record verified prices with a date.
8. **Regional hosts.** The docs list US, EU, India and Singapore residency hosts. Only the default host is allowed. Adding one needs approval.

## 10. Engines added after v0.1 (2026-10-23)
Shipped but **not live-tested** unless stated. Each one is built from public documentation and tested against a local fake server. Run a real call with a real account or install and record the result here.
| Engine | Where it runs | Host | What was built | VERIFY |
|---|---|---|---|---|
| OpenAI-compatible speech (Kokoro-FastAPI, LocalAI, Speaches, OpenRouter, others) | Local, LAN or cloud | Whatever address the user enters; `http` only for loopback and private addresses | `/audio/speech`, optional voice discovery, tag map per engine | Presets are typical addresses, not promises. OpenRouter's model id and voices need a live check |
| Piper | Local | None (subprocess, argument list only) | Reads `.onnx` voices from a folder; uses `--output_file` and `--length-scale` | Confirm the CLI flags against the installed Piper release; each voice has its own licence |
| ElevenLabs music and sound effects | Cloud | api.elevenlabs.io (already approved) | `/v1/music`, `/v1/sound-generation` | Live call, plan limits, commercial terms |
| ElevenLabs voice design | Cloud | api.elevenlabs.io | `/v1/text-to-voice/design`, `/v1/text-to-voice` | Live call; credits per preview |
| Kie (Suno and other jobs) | Cloud | api.kie.ai (approved by Victor 2026-10-23; not yet live-tested) | Create-task then poll; template per model; Suno template `{"custom_mode":false,"instrumental":"{{instrumental}}","model":"V6","prompt":"{{prompt}}"}` on `ai-music-api/generate` | Endpoint, model names and output terms |
| fal | Cloud | queue.fal.run (approved by Victor 2026-10-23; not yet live-tested) | Queue submit, poll, fetch result; template per model | Field names differ per model; presets are starting points |
| ComfyUI | Local | Loopback or private address | Submit a user-supplied API-format workflow, poll history, fetch the output file | Behaviour differs by node pack and version |

Downloads of generated audio from a provider go through a hardened fetcher: https only, public addresses only, at most three validated redirects, a size cap, and no credentials sent.

Not bundled: FFmpeg (user-installed, LGPL/GPL by build), Piper, ComfyUI, any model weights. Bundling an LGPL FFmpeg build (about 70 to 100 MB) is a decision for the project owner.
