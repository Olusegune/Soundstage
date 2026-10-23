# Music and sound effects

Status: stub. CLAUDE.md names this file; it was not in the supplied doc set. This records only what v0.1 does. Victor to supply or approve the full spec.

## What v0.1 does
- Import WAV files as **music** or **effect** assets (500 MB and 30 minute limits).
- Every asset starts with licence `unknown`. A user can record the licence, the web address of the terms they read, the date, and notes.
- Place assets on the timeline as cues: scene and offset, level, fades, loop to fill the scene, and whether it ducks under dialogue. Music that exists inside the scene (a radio) opts out of ducking.
- Commercial export blocks any asset that is `unknown`, `non-commercial`, unchecked, or last checked over 90 days ago, and lists it by name. Assets can also list destinations their terms prohibit.

## What it does not do yet
- No generated music or effects (v2, off until each source's terms are cleared).
- No library browsing or search; no MP3 or other formats on import.
- No hit-point markers or layered effect presets.

## Open questions (CLAUDE.md)
Which libraries the team already has, and whether generated music is in v1.
