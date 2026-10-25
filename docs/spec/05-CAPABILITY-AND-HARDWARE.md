# 05. Model capability and hardware matrices

Status: **Proposed.** Written 2026-10-25. Nothing here is a quality claim. "Reported" means a third-party page said so on the date given and it has not been tested in Soundstage. The Model Lab (MS4) is the only thing that can change a status to `verified`.

## 1. Capability matrix (what each candidate is for)

Legend: **I** integrated and tested against fakes; **C** candidate; **-** not applicable; **V** VERIFY before use.

| Candidate | Kind | Where | Status in app | Languages | Expressive control | Non-verbal events | Cloning | Reported licence | Pack milestone |
|---|---|---|---|---|---|---|---|---|---|
| Windows SAPI | TTS | local | I, `verified` for basic use | system voices | rate, volume | none | no | OS | - |
| Piper | TTS | local | I, `experimental` | per voice | rate only | none | no | MIT engine, per-voice | - |
| ElevenLabs v3 | TTS | cloud | I, `experimental` | many (V) | audio tags | tags (laugh, sigh and others) | yes, consent gate | commercial terms | - |
| ElevenLabs v4 | TTS | cloud | C | V | V | V | V | V | after V |
| VoiceKiller | TTS | cloud | I, `experimental` | per voice | instructions | via instructions | registration | terms | - |
| OpenAI-compatible | TTS | either | I, `experimental` | server dependent | server dependent | none | no | server's own | - |
| Chatterbox | TTS | local | C | V (reported English first) | reported exaggeration control | reported paralinguistic tags (V) | zero-shot (consent gate) | MIT (reported) | MS5 |
| Qwen3-TTS | TTS | local | C | reported 10 | reported instruction control | V | reported (V) | Apache-2.0 (reported) | MS5 |
| Whisper (whisper.cpp) | STT | local | I | 99 | - | - | - | MIT | - |
| ElevenLabs Scribe v2 | STT | cloud | I | many | - | - | - | terms | - |
| ElevenLabs sound and music | SFX, music | cloud | I | - | prompt | - | - | terms | - |
| Kie / Suno | music | cloud | I | - | prompt | - | - | terms | - |
| fal models | SFX, music | cloud | I | - | prompt | - | - | per model | - |
| ComfyUI | any | local | I | - | workflow | - | - | per workflow | - |
| ACE-Step 1.5 | music | local | C | - | prompt, lyrics | - | - | MIT (reported) | MS5 |
| Stable Audio Open | SFX, music | local | C | - | prompt | - | - | **unclear (V)** | after V |
| Wan 2.2 | video | local | C | - | prompt | - | - | Apache-2.0 (reported) | MS9 |
| LTX-2.3 | video | local | C | - | prompt | - | - | community licence, revenue threshold (reported) | MS9 |
| HunyuanVideo | video | local | C | - | prompt | - | - | territory exclusions (reported) | MS9 |

Rows with `V` in a language or control column stay blank in the registry (not "supported") until verified.

## 2. Language and accent status

All 13 priority languages/accents in PRD section 8 start as `unverified` for every provider. Status moves like this:

`unverified` → `experimental` (a Lab run exists, gate not met) → `verified` (gate in `07-LISTENING-TEST-PROTOCOL.md` met, per provider per language).

The UI shows the state next to every voice and never says "supports" for an `unverified` one. Automated scores may rank candidates for listening but can never set `verified` (LAB-008).

## 3. Source conflicts to resolve before relying on any row

| Topic | Disagreement | Action |
|---|---|---|
| ElevenLabs v4 | only third-party pages (aggregators, blog summaries) mention a late-September 2026 release; the official API reference was not confirmed | check elevenlabs.io docs and the API model list in T-0.4 |
| Stable Audio Open licence | sources describe a community licence with a revenue limit, others call it open without limits | read the Hugging Face model card and licence text |
| Qwen3-TTS languages | "10 languages" in three summaries; per-language quality unknown | Lab |
| ACE-Step VRAM | "under 4 GB" is for a reduced setting; full quality needs more | Lab on real hardware |
| LTX-2.3 threshold | reported as a $10M revenue threshold | read the licence; encode as `revenue_cap` |

## 4. Hardware profiles (HW-PROF)

Detection reads CPU model and cores, total RAM, free disk on the model drive, each GPU's name, VRAM and driver, and which backends respond (CUDA, Vulkan, DirectML). Detection never sends anything off the computer.

| Profile | Typical machine | Minimum for the profile | What it can run |
|---|---|---|---|
| Cloud-first | any Windows 10/11 PC | 4 GB RAM | cloud providers, SAPI, Piper, Whisper tiny/base |
| Local-light | laptop, integrated GPU | 8 GB RAM, 4 CPU cores | above plus Whisper small, small music models in CPU mode (slow) |
| Local-standard | mid gaming PC | 16 GB RAM, GPU with 8 GB VRAM (NVIDIA first) | local TTS packs (Chatterbox, Qwen3-TTS), Whisper medium, ACE-Step |
| Local-advanced | enthusiast PC | 32 GB RAM, 16 GB VRAM | larger TTS, short video at low resolution (Wan 2.2 5B class, reported to need about 24 GB at full settings) |
| Local-workstation | workstation | 64 GB RAM, 24 GB or more VRAM | video packs at higher settings (LTX class reported to want 32 GB or more) |

Thresholds are starting points from reported figures. They are measured in T-1.6 and MS5 on the owner's machine and then adjusted. They are recorded as `hardware.profile_min` per registry entry, not hardcoded.

## 5. Verdict shown before a download or a run

| Verdict | Meaning | Action offered |
|---|---|---|
| Runs well | meets the model's recommended needs | download |
| Runs slowly | meets the minimum only (for example CPU mode) | download with a warning and an estimate |
| Will not run | below the minimum | no download; offer a cloud alternative with cost and data notice |
| Unknown | detection incomplete | explain what is missing; allow manual override with a warning |

Also checked: free disk (needs size plus 20%), and the licence summary. A model that would not fit is never started (HW-005).

## 6. Order of vendor support

NVIDIA with CUDA first (D-12), because most open models target it. Vulkan or DirectML paths are accepted only when a pack proves them in the Lab. AMD and Intel GPUs fall back to Cloud-first or CPU until then, and the verdict says so honestly. Apple and Linux are out of scope for this release.
