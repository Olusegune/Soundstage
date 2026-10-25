# 04. Model and provider specification update

Status: **Proposed.** Written 2026-10-25. Extends `docs/PROVIDERS.md`; it does not replace it. Facts about third-party models are marked **VERIFY** until checked against the provider's own page on the date shown. See `05-CAPABILITY-AND-HARDWARE.md` for the matrix.

## 1. Provider kinds

| Kind | Trait | Exists | Notes |
|---|---|---|---|
| Speech (TTS) | `Adapter` | yes | Windows SAPI, ElevenLabs, VoiceKiller, OpenAI-compatible, Piper |
| Speech to text | `transcribe` | yes | ElevenLabs `scribe_v2`, offline whisper.cpp |
| Voice change | `change_voice` | yes | ElevenLabs |
| Sound and music | `Generator` | yes | ElevenLabs, Kie, fal, ComfyUI |
| Language model (Director) | `Planner` | **new** | Chat-completion style; cloud or local; rule-based fallback |
| Video generation | `VideoGenerator` | **new** (MS9) | worker pack or cloud, behind the safety document |
| Lip-sync | `LipSync` | **new** (MS9) | same gate |

## 2. Registry entry (schema 1)

```
{ id, provider, model, kind,
  display_name,
  languages: [ {code, status} ],
  accents:   [ {name, language, status, evidence_id?} ],
  controls:  { emotion: none|tags|instructions|numeric, pace, pauses, emphasis },
  non_verbal: [ {event, support: native|tag|none} ],
  cloning: none|zero-shot|fine-tune, needs_consent: bool,
  hardware: { profile_min, vram_gb?, ram_gb?, disk_gb?, backends[] },
  licence: { name, url, checked_on, commercial: yes|no|conditional|unknown,
             conditions: [ {kind: "revenue_cap"|"territory"|"attribution"|"field_of_use"|"other", text, value?} ] },
  cost: { unit, estimate?, currency? }, latency: { typical_ms? },
  sends_data: bool, host?,
  status: verified|experimental|unsupported,
  evidence: [ lab_run_id ], last_verified }
```
`status` and every `accents[].status` default to `unverified` or `experimental`. Only the Model Lab can promote them (D-09). The structured `licence.conditions` resolve C-18: the export gate can then reason about a revenue cap or territory exclusion instead of showing a note.

## 3. Adapter contract extensions

All new capabilities are optional and discoverable, so existing adapters keep passing the shared contract suite.

1. `capabilities()` returns the registry entry for the adapter (replaces scattered checks).
2. `plan_support(plan_line) -> SupportReport`: which parts of a `LinePlan` this model can honour (emotion, pace, events, languages), and what would be dropped. Used for PHY-005, EMO-007 and the provider comparison.
3. `estimate(request) -> {cost, duration}` before any paid call, shown in the approval prompt.
4. Errors keep the existing rule: short codes, never provider bodies.
5. Every adapter must pass: no spoken tags (PHY-001), no keys in requests or logs (SEC-002), cancel within a bound, size caps, pinned hosts.

## 4. Selection and fallback (ARCH-4.1b, ARCH-4.5, HW-006)

Ranking inputs, in order: user choice (always wins), language and accent status, required controls, hardware fit, licence for the intended use, cost, latency. The app proposes; it never switches by itself.

Fallback when a provider fails or is unsuitable:
1. Show alternatives that can honour the plan, with what each would drop.
2. Show cost (or "free, runs on this computer") and whether data leaves the computer.
3. Switching to a paid or cloud alternative needs a fresh approval each time.
4. The plan is preserved; only the derived controls change.

## 5. Cloud candidates (PRD section 4.2)

| Candidate | State today | Spec action |
|---|---|---|
| ElevenLabs v3 (`eleven_v3`) | integrated, tested against fakes | keep default |
| ElevenLabs v4 | reported by third-party pages around 28 Sep 2026; not in code | **VERIFY** on elevenlabs.io before adding; add as a model choice, never silently replace v3 (C-21) |
| VoiceKiller | integrated | unchanged |
| OpenAI-compatible speech | integrated, any host the user sets | host approval stays per user setting |
| Google Gemini speech/LLM | not integrated | **VERIFY** terms and capabilities; host needs owner approval (C-22) |
| OpenAI GPT models (Director, speech) | not integrated | **VERIFY**; host approval (C-22) |
| Kie, fal | integrated for music, effects | video models there are candidates for MS9 only |
| OpenRouter | integrated for chat | candidate Director backend; needs consent flow (section 5 of `03`) |
| Cloud video APIs | none | **VERIFY**, gated by C-01 and C-17 |

## 6. Local candidates (PRD section 4.3)

Reported facts, to be re-checked at the pack milestone. Dates are when they were last looked up (2026-10-25).

| Candidate | Use | Reported licence | Note |
|---|---|---|---|
| Piper | TTS | MIT (engine); each voice has its own licence | integrated; check per-voice licence in the registry |
| whisper.cpp / Whisper | STT | MIT | integrated |
| Chatterbox | expressive TTS, cloning | MIT | MS5 pack candidate; **VERIFY** hardware and watermark behaviour |
| Qwen3-TTS | multilingual TTS | Apache-2.0 | reported 10 languages; **VERIFY** per-language quality in the Lab |
| ACE-Step 1.5 | music | MIT | reported to run in under 4 GB VRAM with limits; **VERIFY** |
| Stable Audio Open | effects, music | unclear in the sources found | **VERIFY** the exact licence before any use; do not bundle |
| Wan 2.2 | video | Apache-2.0 | MS9 only |
| LTX-2.3 | video | community licence with a revenue threshold | MS9 only; structured condition `revenue_cap` |
| HunyuanVideo | video | territory exclusions | MS9 only; structured condition `territory` |
| WhisperX, Demucs | alignment, separation | **VERIFY** | not required for any milestone gate |

Where sources disagreed or were thin, the matrix says so rather than choosing one.

## 7. Safety and licensing rules for providers

- A provider with licence `unknown` can be used to draft but blocks commercial export (existing behaviour, kept).
- A `conditional` licence shows its conditions at selection and again at export, and the user must confirm each condition applies or not.
- Cloning and reference voices keep the consent gate for every provider, including local ones.
- A new host needs a line in `DECISIONS.md` and the pinned-host list before code may call it.
- No model whose weights load through pickle or arbitrary code is allowed in a pack (D-10).
