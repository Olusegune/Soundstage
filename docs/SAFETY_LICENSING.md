# Safety, consent and licensing: Voice Studio

This document is binding. If code conflicts with it, the code is wrong. Changes to this document need Victor's approval.

## 1. Principles
1. A person's voice is theirs. No clone is made or used without verified, specific, revocable consent.
2. Licence status is data, not a guess. Every voice and model carries a licence tag with its source and check date.
3. Commercial export is blocked by default. A user must clear every blocking item explicitly.
4. Generated audio is identifiable. We mark it so it can be traced back to the project, provider and model.

## 2. Voice cloning and consent

### 2.1 Who may be cloned
- The person who gives consent, and only them, with a verified consent record.
- Not public figures, celebrities, politicians, or religious leaders, unless the person has given consent and the output is not presented as their real speech. Default: refuse these cases and explain why.
- Not children. Reference audio of a minor is rejected. Adults only.
- Not deceased people, even with family consent, in v1.

### 2.2 Consent record (`consent/<id>.json`)
```json
{
  "id": "consent-uuid",
  "voice_subject": "name of the person whose voice it is",
  "subject_is_self": true,
  "subject_age_confirmed_18plus": true,
  "purpose": "narration for [project name]",
  "permitted_uses": ["narration", "dialogue"],
  "prohibited_uses": ["political", "sexual", "impersonation", "endorsement without separate consent"],
  "commercial_use_permitted": true,
  "territory": "worldwide",
  "expires": "YYYY-MM-DD or null",
  "revocable": true,
  "evidence": {
    "method": "recorded statement | signed form | in-app attestation",
    "stored_at": "path to evidence (encrypted)"
  },
  "signed_at": "ISO 8601",
  "status": "active | revoked | expired"
}
```

Rules:
- The subject must attest in their own words, recorded or signed, that they consent to this specific use.
- Revoking a consent record immediately blocks new renders and exports that use the voice. Existing exports are listed in the revocation report so the owner can decide what to do.
- Consent evidence is stored encrypted, outside the project folder, and never sent to a provider.

### 2.2.1 Implementation status (v0.2.0)
- Cloning is built for ElevenLabs. A clone request cannot be made without a consent proof, which only the consent check can produce.
- The record is saved to `Documents\Soundstage\.consent` before any upload and is never sent to a provider. Evidence is an in-app attestation (the owner agrees to a fixed statement). Recorded-statement and signed-form evidence are not built.
- Evidence is **not yet encrypted** at rest (open item: DPAPI or a keyring-wrapped key).
- Every render and export re-checks the record; revoking blocks both at once. Exports list the consent ids used.
- Age, identity and public-figure status cannot be verified by the app; the owner attests.
- Not built: speech-to-speech and performance-reference use, retention policy, revocation reports listing past exports.

### 2.2.2 Performance reference and dubbing (v0.2.0)
- The voice changer uses a recording as a performance. It requires a consent record for that use (`performance_reference`), made from the same promises as a clone, saved before anything is sent. A record made for performance cannot be used to clone, and the reverse.
- A cloned target voice is checked as well.
- The dubbing helper never clones the original speakers: it translates the transcript and the user chooses the voices.
- Not built: automatic dubbing that copies the original voices, lip-sync, and talking-head avatars. These are deliberately left out for now because they are the easiest route to a convincing fake of a real person.

### 2.3 Reference audio
- Single speaker, minimum length (VERIFY per adapter), no music or background speech.
- The app runs a check and shows the result. A failed check blocks cloning and says why.

## 3. Licence tags

### 3.1 Values
- `commercial`: the provider's terms in the licence source allow commercial use and resale of output, as checked on the date recorded.
- `non-commercial`: terms forbid or restrict commercial use.
- `unknown`: no verified terms. Treated as blocked for commercial export.

### 3.2 Each voice and model records
`licence`, `licence_source` (URL), `licence_checked` (date), and `licence_notes` (any limits, such as attribution, revenue caps, or prohibited categories).

### 3.3 Export gate
- A project's `licence_target` is `commercial` or `personal`.
- For `commercial`, every voice, model, music track and SFX file in the timeline must be `commercial`. Anything else blocks export and is listed by name with its reason.
- For `personal`, the export shows a notice of any non-commercial items, and the user must acknowledge it. Export is still allowed.
- Stale checks: a licence checked more than 90 days ago is shown as "needs re-check" and blocks commercial export until re-checked.
- Gate decisions are logged in the export manifest.

## 4. Provenance
- Every export includes a manifest: project id, export time, app version, providers and model versions per line, voice ids, licence tags, consent references, and a hash of each audio file.
- PROPOSED: embed C2PA content credentials or an equivalent provenance marker in exports, and check whether the chosen providers already do so. Needs Victor's decision.
- Do not claim exported audio is "human" or "real". The UI may say it was AI-generated.

## 5. Content refusals
The app and its adapters refuse these requests, and say so plainly:
- Making a real person say something they did not say, without consent, including in a clone made from public recordings.
- Impersonating a real person, company, official, or public body.
- Sexual content involving minors, or content that sexualises any real person.
- Political persuasion or election material using a real person's voice.
- Fraud material: scripts meant to deceive people into paying, giving credentials, or sharing personal data (for example, a "bank" call or a fake relative in distress).
- Hate speech or incitement to violence.
- Deceptive medical, legal, or financial advice presented as authoritative.

Refusal behaviour: state which rule applied, offer the nearest allowed alternative (for example, a clearly fictional character voice from the library), and do not render. Do not hide a refusal behind a generic error.

Classifier note: text checks use a keyword list and, where a provider offers moderation, that provider's check. A pass does not mean the content is allowed; it only means no rule fired.

## 6. Data handling
- Scripts and audio stay local unless sent to a cloud provider for rendering. The UI shows which provider will receive each line before render.
- No audio or script is sent to a host outside the adapter's allowlist.
- Logs contain job ids, durations, errors, and provider ids. They never contain keys, consent evidence, scripts, or audio.
- Users can delete a project, which removes its assets and cache. Provider-side copies follow the provider's policy, and the UI says so.
- No analytics on script or audio content.

## 7. Content categories, ratings and adult fiction
The product serves every genre. Each project declares a content category and rating, and the checks follow it.

### 7.1 Ratings
- Each project has a rating field (for example: all ages, teen, adult), set by the owner and recorded in the export manifest.
- Ratings are declarations, not automatic classifications. The app does not claim to certify a rating.

### 7.2 Adult fiction
- Romance, drama, and adult themes with fictional adult characters are allowed in projects rated adult.
- Sexual content is allowed only with fictional adult characters, is age-gated, and is rated accordingly. It is never made with any real person's voice or likeness.
- Providers may restrict sexually explicit content under their own policies. The app shows that restriction before render, not as a silent failure.

### 7.3 Children's content
Additional rules apply to projects rated all ages or marked as children's:
- Voices marked child-directed by the provider are used only where the provider permits.
- Tone checks flag scripts with content unsuitable for children when the project is marked for children. This is advisory and the user decides.
- No cloning of children's voices (section 2.1).

## 8. Incident response
- If a clone is misused or a refusal rule is bypassed, revoke the voice, notify the affected person and the provider, and log the incident.
- Keep a record of incidents in `docs/incidents/` (no personal data in the record beyond what is needed).

## 9. Open decisions for Victor
1. Provenance marking (C2PA or equivalent): required in v1 or later?
2. Minimum age for consent signers (proposed: 18).
3. Whether `personal` exports should carry a visible notice.
4. Who holds consent evidence storage and retention period.

## 10. Performer rights and AI use
- A performer's voice, guide performance (reference), or likeness is used only under that performer's written consent record.
- The consent record states the project, the permitted uses, the territory, the term, and whether AI-generated output is allowed.
- Performers can see every take made from their material, and they can revoke use. Revocation blocks new renders and exports. The revocation report lists existing ones.
- A performer's reference is never used in another performer's project.
- Where a contract or union rule governs AI use of performance in a market, the app records that rule and the performer's acknowledgement. Per-market rules are VERIFY before the feature ships in that market.
- Disclosure: where a destination or contract requires it, the export manifest marks AI-generated voice and performance.
