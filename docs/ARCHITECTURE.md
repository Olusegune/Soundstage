# Architecture

> **Note (2026-10-25):** this describes v0.1 and is partly out of date. The target design is in `docs/spec/03-ARCHITECTURE-UPDATE.md`; together they are the dev spec (decision D-02).

Status: describes what exists in v0.1. CLAUDE.md names this file; it was not in the supplied doc set, so this one is derived from the code. Review it.

## Components

```
UI (React + TypeScript, strict)
  |  Tauri commands (JSON)
Shell (app/src-tauri)           thin: file dialogs, credential manager, background threads
  |
soundstage-core (Rust)          every decision about what is allowed, rendered and exported
  script    parse NAME: lines and scenes; source text is the single source of truth
  tags      canonical vocabulary; unknown tags are errors and never reach a provider
  direction project -> character -> scene -> line, field by field, with the source recorded
  providers Adapter trait, capability checks before any work, request splitting, Registry
  adapters  windows-speech (SAPI via PowerShell), elevenlabs (cloud, pinned host), scratch-tone (test fake)
  render    analyze() for status and issues; render_line() for takes and the cache
  layout    places selected takes on one timeline
  mix       buses, ducking, one shared master gain so stems sum to the master
  loudness  BS.1770 integrated, short-term, range, estimated true peak
  licence   tags, staleness, the export gate
  consent   record model and the gate cloning must pass
  safety    refusal screen
  exporter  gate, mixdown, stems, manifest
  project   model, schema version, migration, atomic save, recovery
```

## Data

A project is a folder under `Documents\Soundstage\<id>\`:

```
project.json        human-readable, schema_version 1 (project.json.bak is the last good copy)
takes\*.wav         one mono 16-bit file per render
assets\*.wav        imported music and effects
```

Exports go to a folder the user picks: master, three stems, and `manifest.json`.

## Render and cache

A line's cache key is the hash of adapter id, provider version, voice id, canonical tokens, resolved emotion, pace and language. A line is **rendered** when its selected take has the current key, **stale** when it has a take with an old key, and **drafted** when it has none. Re-rendering an unchanged line makes no provider call.

## Mix

All buses are built at 48 kHz stereo float. Dialogue is built first; its activity drives a linear-ramp ducking envelope that effects and music follow unless a cue opts out (source music). One gain is applied to every bus to reach the destination's loudness target. If that gain would push the true peak over the ceiling, the gain is capped, the mix is flagged, and nothing is clipped or limited.

## Security boundaries

- Project ids and relative file paths are validated before use; exports never trust a path from the UI.
- Imported audio is size- and duration-limited and decoded in-process.
- The only subprocess is PowerShell for Windows speech. Its script is a constant sent as `-EncodedCommand`; user text travels in a JSON file in a private temp folder; the voice name must match the installed-voice list.
- Keys are stored with `keyring` (Windows Credential Manager) and are never returned to the UI. The shell passes the core a key provider; adapters read the key per call and never store or log it.
- The ElevenLabs adapter is pinned to `https://api.elevenlabs.io`, never follows redirects, validates voice ids before putting them in a URL, caps response sizes, and reports only short error codes. Cloned and professional voices are never listed.
- The UI asks before any text is sent over the network and says which provider receives it.
- The webview CSP allows no remote origins, and the asset protocol is scoped to the Soundstage documents folder.
