# Providers: Voice Studio

Status: research snapshot dated 2026-10-23, from public search results. Anything marked `VERIFY` must be checked against the provider's current documentation and licence before it is coded or promised to users. Re-check every entry at least quarterly, and before each milestone.

## 1. Provider matrix (initial)

| Provider | Type | Expressive tags | Languages (claimed) | Licence | Status |
|---|---|---|---|---|---|
| ElevenLabs (Eleven v3) | Cloud | Yes: inline tags such as laughs, whispers, sighs, excited | 70+ | Commercial plan terms. VERIFY output rights and API resale terms | Candidate, M0 |
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
