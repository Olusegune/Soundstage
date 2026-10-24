//! Voice cloning, behind the consent gate (SAFETY_LICENSING.md section 2).
//!
//! Three rules shape this module:
//! 1. A clone cannot be requested without a [`ConsentProof`], and a proof can only come from `consent::require`.
//! 2. The consent record is written to a folder outside every project (`.consent`) before anything is sent, and it is
//!    never sent to a provider. Only the reference audio and a display name are.
//! 3. A cloned voice is usable only while its consent record is clean. Render and export both re-check it, so
//!    revoking consent blocks new takes and new exports at once.

use crate::audio::{resample, Stereo, SR};
use crate::consent::{ConsentRecord, ConsentStatus, Evidence};
use crate::util::{iso_now, new_id};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::{Path, PathBuf};

pub const MIN_REFERENCE_S: f32 = 15.0;
pub const GOOD_REFERENCE_S: f32 = 60.0;
/// Longer recordings are trimmed to this much before upload, which also keeps the file small.
pub const MAX_REFERENCE_S: f32 = 180.0;
const UPLOAD_RATE: u32 = 24_000;

/// The statement the voice's owner must agree to. It is stored word for word in the record.
pub const STATEMENT: &str = "I am the owner of this voice, or I have the owner's clear permission. The owner is an adult, is alive, and is not a public figure. They agree that Soundstage may create a synthetic copy of this voice from the recording and use it for the purposes below, and they know they can withdraw this consent at any time.";

/// The statement for using someone's performance to drive another voice (the voice changer).
pub const PERFORMANCE_STATEMENT: &str = "I am the person speaking in this recording, or I have the speaker's clear permission. The speaker is an adult, is alive, and is not a public figure. They agree that Soundstage may use their performance (the words, timing and feeling) to drive a different synthetic voice for the purposes below, and they know they can withdraw this consent at any time.";

#[derive(Clone, Debug, Deserialize)]
pub struct Attestation {
    /// The name of the person whose voice it is.
    pub subject: String,
    pub subject_is_self: bool,
    pub adult: bool,
    pub alive: bool,
    pub not_public_figure: bool,
    /// Ticked when the person agreeing has read `STATEMENT`.
    pub agrees: bool,
    pub purpose: String,
    pub commercial: bool,
    /// Optional `YYYY-MM-DD`.
    pub expires: Option<String>,
}

fn clean_line(s: &str, max: usize) -> Option<String> {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    (!t.is_empty() && t.chars().count() <= max && !t.chars().any(|c| c.is_control())).then_some(t)
}

/// Turn a filled-in form into a consent record, or list everything that is missing.
pub fn build_record(a: &Attestation, id: &str) -> Result<ConsentRecord, Vec<String>> {
    let mut p = Vec::new();
    let subject = clean_line(&a.subject, 80).filter(|s| s.chars().count() >= 2);
    if subject.is_none() {
        p.push("Enter the name of the person whose voice this is (2 to 80 characters).".to_string());
    }
    let purpose = clean_line(&a.purpose, 200);
    if purpose.is_none() {
        p.push("Say what the voice will be used for (up to 200 characters).".to_string());
    }
    if !a.adult {
        p.push("The voice owner must be 18 or over. Children's voices are never cloned.".into());
    }
    if !a.alive {
        p.push("The voice owner must be alive. Voices of people who have died are not cloned.".into());
    }
    if !a.not_public_figure {
        p.push("Public figures, celebrities, politicians and religious leaders are not cloned.".into());
    }
    if !a.agrees {
        p.push("The voice owner must agree to the statement.".into());
    }
    let expires = match a.expires.as_deref().map(str::trim).filter(|e| !e.is_empty()) {
        Some(e) if crate::util::parse_date(e).is_some_and(|d| d >= crate::util::today_days()) => Some(e.to_string()),
        Some(_) => {
            p.push("The end date must be a real date in the future (YYYY-MM-DD), or left empty.".into());
            None
        }
        None => None,
    };
    if !p.is_empty() {
        return Err(p);
    }
    Ok(ConsentRecord {
        id: id.to_string(),
        voice_subject: subject.unwrap_or_default(),
        subject_is_self: a.subject_is_self,
        subject_age_confirmed_18plus: true,
        purpose: purpose.unwrap_or_default(),
        permitted_uses: vec!["dialogue".into(), "narration".into()],
        prohibited_uses: vec!["political".into(), "sexual".into(), "impersonation".into(), "endorsement without separate consent".into()],
        commercial_use_permitted: a.commercial,
        territory: "worldwide".into(),
        expires,
        revocable: true,
        evidence: Some(Evidence { method: "in-app attestation".into(), stored_at: format!(".consent/{id}.json") }),
        signed_at: Some(iso_now()),
        status: ConsentStatus::Active,
    })
}

/// A consent record for using a recording as a performance reference. It permits that use and nothing else.
pub fn build_performance_record(a: &Attestation, id: &str) -> Result<ConsentRecord, Vec<String>> {
    let mut rec = build_record(a, id)?;
    rec.permitted_uses = vec!["performance_reference".into()];
    Ok(rec)
}

#[derive(Clone, Debug, Serialize)]
pub struct RefReport {
    pub seconds: f32,
    pub used_seconds: f32,
    pub peak: f32,
    pub clipped_pct: f32,
    pub silence_pct: f32,
    /// Reasons the recording cannot be used. Empty means it passed.
    pub problems: Vec<String>,
    pub warnings: Vec<String>,
}

/// Judge a reference recording before anything is uploaded. It cannot prove there is one speaker, so it says so.
pub fn check_reference(audio: &Stereo) -> RefReport {
    let n = audio.len();
    let seconds = n as f32 / SR as f32;
    let used = (seconds).min(MAX_REFERENCE_S);
    let used_n = (used * SR as f32) as usize;
    let mono = |i: usize| (audio.l[i] + audio.r[i]) * 0.5;
    let (mut peak, mut clipped, mut quiet) = (0f32, 0usize, 0usize);
    for i in 0..used_n.min(n) {
        let a = mono(i).abs();
        peak = peak.max(a);
        if a >= 0.995 {
            clipped += 1;
        }
        if a < 0.003 {
            quiet += 1;
        }
    }
    let denom = used_n.max(1) as f32;
    let (clipped_pct, silence_pct) = (100.0 * clipped as f32 / denom, 100.0 * quiet as f32 / denom);
    let mut problems = Vec::new();
    let mut warnings = Vec::new();
    if seconds < MIN_REFERENCE_S {
        problems.push(format!("The recording is {seconds:.0} seconds long. At least {MIN_REFERENCE_S:.0} seconds of clear speech is needed."));
    } else if seconds < GOOD_REFERENCE_S {
        warnings.push("One minute or more of speech usually gives a better copy.".to_string());
    }
    if peak < 0.02 {
        problems.push("The recording is almost silent. Record again closer to the microphone.".into());
    }
    if clipped_pct > 1.0 {
        problems.push("The recording is distorted (clipped). Record again at a lower input level.".into());
    }
    if silence_pct > 60.0 {
        problems.push("More than half of the recording is silence. Use a recording where the person speaks throughout.".into());
    }
    if seconds > MAX_REFERENCE_S {
        warnings.push(format!("Only the first {MAX_REFERENCE_S:.0} seconds will be used."));
    }
    warnings.push("Soundstage cannot tell how many people are speaking. Use a recording of the one person, with no music or other voices.".into());
    RefReport { seconds, used_seconds: used, peak, clipped_pct, silence_pct, problems, warnings }
}

/// The upload: 16-bit mono WAV at 24 kHz, trimmed to `MAX_REFERENCE_S`. Held in memory, never written next to a project.
pub fn encode_sample(audio: &Stereo) -> Result<Vec<u8>, String> {
    let keep = ((MAX_REFERENCE_S * SR as f32) as usize).min(audio.len());
    let mono: Vec<f32> = (0..keep).map(|i| (audio.l[i] + audio.r[i]) * 0.5).collect();
    let low = resample(&mono, SR, UPLOAD_RATE);
    let spec = hound::WavSpec { channels: 1, sample_rate: UPLOAD_RATE, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut cur = Cursor::new(Vec::new());
    {
        let mut w = hound::WavWriter::new(&mut cur, spec).map_err(|e| e.to_string())?;
        for s in low {
            w.write_sample((s.clamp(-1.0, 1.0) * 32_767.0).round() as i16).map_err(|e| e.to_string())?;
        }
        w.finalize().map_err(|e| e.to_string())?;
    }
    Ok(cur.into_inner())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClonedVoice {
    pub voice_id: String,
    pub adapter_id: String,
    pub name: String,
    pub consent_id: String,
    pub created: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CloneEntry {
    pub voice: ClonedVoice,
    pub consent: ConsentRecord,
}

/// Consent records and the voices they cover, kept in a folder outside every project.
pub struct ConsentStore {
    dir: PathBuf,
}

fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

impl ConsentStore {
    pub fn new(dir: PathBuf) -> Self {
        ConsentStore { dir }
    }

    fn record_path(&self, id: &str) -> Result<PathBuf, String> {
        if !safe_id(id) {
            return Err("That consent id is not valid.".into());
        }
        Ok(self.dir.join(format!("{id}.json")))
    }

    fn voices_path(&self) -> PathBuf {
        self.dir.join("voices.json")
    }

    pub fn new_record_id() -> String {
        new_id("consent")
    }

    /// Save a record before any upload, so a crash can never leave a clone without its consent on file.
    pub fn save_record(&self, rec: &ConsentRecord) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("Could not create the consent folder: {e}"))?;
        let json = serde_json::to_string_pretty(rec).map_err(|e| e.to_string())?;
        // Records name a real person and what their voice may be used for, so they are encrypted for this Windows user.
        write_atomic(&self.record_path(&rec.id)?, &crate::secure::seal(json.as_bytes())?)
    }

    /// Read a record. A record saved by an older version (plain text) is read, then saved again encrypted.
    pub fn record(&self, id: &str) -> Option<ConsentRecord> {
        let path = self.record_path(id).ok()?;
        let (bytes, was_protected) = crate::secure::open(&std::fs::read(&path).ok()?).ok()?;
        let rec: ConsentRecord = serde_json::from_slice(&bytes).ok()?;
        if !was_protected {
            let _ = self.save_record(&rec);
        }
        Some(rec)
    }

    fn voices(&self) -> Vec<ClonedVoice> {
        std::fs::read(self.voices_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn add_voice(&self, v: ClonedVoice) -> Result<(), String> {
        let mut all = self.voices();
        all.retain(|x| x.voice_id != v.voice_id);
        all.push(v);
        write_atomic(&self.voices_path(), serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?.as_bytes())
    }

    pub fn list(&self) -> Vec<CloneEntry> {
        self.voices().into_iter().filter_map(|voice| self.record(&voice.consent_id).map(|consent| CloneEntry { voice, consent })).collect()
    }

    /// Revoke consent. The voice stays listed (so old takes say why they are blocked) but can no longer be used.
    pub fn revoke(&self, consent_id: &str) -> Result<ClonedVoice, String> {
        let mut rec = self.record(consent_id).ok_or("No consent record with that id.")?;
        crate::consent::revoke(&mut rec)?;
        self.save_record(&rec)?;
        self.voices().into_iter().find(|v| v.consent_id == consent_id).ok_or_else(|| "No voice uses that record.".to_string())
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("Could not save: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Could not save: {e}"))
}

/// Create a clone. The record is saved first, the upload happens second, and the voice is listed only if it worked.
pub fn run_clone(adapter: &dyn crate::providers::Adapter, store: &ConsentStore, rec: ConsentRecord, name: &str, sample_wav: Vec<u8>) -> Result<ClonedVoice, String> {
    let proof = crate::consent::proof(&rec, &crate::consent::Use::Dialogue, rec.commercial_use_permitted).map_err(|p| p.join(" "))?;
    let name = clean_line(name, 60).ok_or("Give the voice a name of up to 60 characters.")?;
    if !adapter.capabilities().cloning {
        return Err(format!("{} cannot clone voices.", adapter.label()));
    }
    store.save_record(&rec)?;
    let voice_id = adapter.clone_voice(&crate::providers::CloneRequest { name: name.clone(), sample_wav, proof }).map_err(|e| e.to_string())?;
    let voice = ClonedVoice { voice_id, adapter_id: adapter.id().to_string(), name, consent_id: rec.id.clone(), created: iso_now() };
    store.add_voice(voice.clone())?;
    Ok(voice)
}

/// Register a voice cloned on the provider's own website. The consent record is made and saved first, exactly as for a
/// clone made here. Nothing is uploaded: the provider already holds the voice, and we only remember its name.
pub fn register_clone(adapter: &dyn crate::providers::Adapter, store: &ConsentStore, rec: ConsentRecord, name: &str) -> Result<ClonedVoice, String> {
    crate::consent::proof(&rec, &crate::consent::Use::Dialogue, rec.commercial_use_permitted).map_err(|p| p.join(" "))?;
    let name = clean_line(name, 80).ok_or("Enter the voice's name exactly as it is saved in your library.")?;
    if !adapter.can_register_clone() {
        return Err(format!("{} does not support registering a cloned voice by name.", adapter.label()));
    }
    store.save_record(&rec)?;
    let voice = ClonedVoice { voice_id: format!("{}-clone:{name}", adapter.id()), adapter_id: adapter.id().to_string(), name, consent_id: rec.id.clone(), created: iso_now() };
    store.add_voice(voice.clone())?;
    Ok(voice)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form() -> Attestation {
        Attestation { subject: "Ama Okafor".into(), subject_is_self: true, adult: true, alive: true, not_public_figure: true, agrees: true, purpose: "Narration for my short film".into(), commercial: true, expires: None }
    }

    fn tone(secs: f32, amp: f32) -> Stereo {
        let n = (secs * SR as f32) as usize;
        let s: Vec<f32> = (0..n).map(|i| amp * (i as f32 * 0.05).sin()).collect();
        Stereo { l: s.clone(), r: s }
    }

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(new_id("consent-test"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_complete_form_makes_a_record_that_passes_the_gate() {
        let rec = build_record(&form(), "consent-1").unwrap();
        assert!(crate::consent::require(Some(&rec), &crate::consent::Use::Dialogue, true).is_ok());
        assert_eq!(rec.evidence.as_ref().unwrap().method, "in-app attestation");
        assert!(rec.revocable && rec.permitted_uses.contains(&"narration".to_string()));
    }

    #[test]
    fn a_performance_record_permits_only_that_use() {
        let rec = build_performance_record(&form(), "consent-p").unwrap();
        assert!(crate::consent::proof(&rec, &crate::consent::Use::PerformanceReference, false).is_ok());
        assert!(crate::consent::proof(&rec, &crate::consent::Use::Dialogue, false).is_err(), "it cannot be reused to clone the voice");
        let mut f = form();
        f.agrees = false;
        assert!(build_performance_record(&f, "x").is_err());
    }

    #[test]
    fn every_missing_promise_is_listed_and_nothing_is_made() {
        let mut f = form();
        f.adult = false;
        f.alive = false;
        f.not_public_figure = false;
        f.agrees = false;
        f.subject = " ".into();
        f.purpose = "".into();
        f.expires = Some("2020-01-01".into());
        assert_eq!(build_record(&f, "x").unwrap_err().len(), 7);
    }

    #[test]
    fn control_characters_and_long_text_are_refused() {
        let mut f = form();
        f.subject = "Ama\u{0}".into();
        assert!(build_record(&f, "x").is_err());
        f.subject = "A".repeat(81);
        assert!(build_record(&f, "x").is_err());
    }

    #[test]
    fn reference_audio_is_judged_before_upload() {
        assert!(check_reference(&tone(30.0, 0.4)).problems.is_empty());
        assert!(!check_reference(&tone(5.0, 0.4)).problems.is_empty(), "too short");
        assert!(!check_reference(&tone(30.0, 0.005)).problems.is_empty(), "too quiet");
        assert!(!check_reference(&tone(30.0, 3.0)).problems.is_empty(), "clipped");
        let silent = Stereo { l: vec![0.0; 48_000 * 30], r: vec![0.0; 48_000 * 30] };
        assert!(!check_reference(&silent).problems.is_empty());
        let r = check_reference(&tone(30.0, 0.4));
        assert!(r.warnings.iter().any(|w| w.contains("one person")), "always warns that one speaker cannot be verified");
    }

    #[test]
    fn the_upload_is_small_mono_and_trimmed() {
        let wav = encode_sample(&tone(240.0, 0.4)).unwrap();
        let r = hound::WavReader::new(Cursor::new(&wav)).unwrap();
        assert_eq!((r.spec().channels, r.spec().sample_rate, r.spec().bits_per_sample), (1, 24_000, 16));
        assert_eq!(r.duration(), 180 * 24_000);
        assert!(wav.len() < 9_000_000);
    }

    #[test]
    fn records_are_encrypted_on_disk_and_older_plain_records_are_upgraded() {
        let d = tmp();
        let st = ConsentStore::new(d.join(".consent"));
        let rec = build_record(&form(), "consent-enc").unwrap();
        st.save_record(&rec).unwrap();
        let on_disk = std::fs::read(d.join(".consent").join("consent-enc.json")).unwrap();
        if cfg!(windows) {
            assert!(!String::from_utf8_lossy(&on_disk).contains("Okafor"), "the owner's name must not be readable in the file");
        }
        assert_eq!(st.record("consent-enc").unwrap(), rec);
        // A record written by version 0.2.0 or earlier is plain JSON.
        std::fs::write(d.join(".consent").join("consent-old.json"), serde_json::to_vec_pretty(&build_record(&form(), "consent-old").unwrap()).unwrap()).unwrap();
        assert_eq!(st.record("consent-old").unwrap().id, "consent-old");
        if cfg!(windows) {
            let upgraded = std::fs::read(d.join(".consent").join("consent-old.json")).unwrap();
            assert!(!String::from_utf8_lossy(&upgraded).contains("Okafor"), "reading an old record encrypts it");
        }
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn records_are_stored_outside_projects_and_revocation_sticks() {
        let d = tmp();
        let st = ConsentStore::new(d.join(".consent"));
        let rec = build_record(&form(), "consent-9").unwrap();
        st.save_record(&rec).unwrap();
        st.add_voice(ClonedVoice { voice_id: "Abc123".into(), adapter_id: "elevenlabs".into(), name: "Ama".into(), consent_id: "consent-9".into(), created: iso_now() }).unwrap();
        assert_eq!(st.list().len(), 1);
        st.revoke("consent-9").unwrap();
        let after = st.list();
        assert_eq!(after[0].consent.status, ConsentStatus::Revoked);
        assert!(crate::consent::require(Some(&after[0].consent), &crate::consent::Use::Dialogue, false).is_err());
        assert!(st.record("../etc/passwd").is_none() && st.revoke("..\\evil").is_err(), "ids cannot escape the folder");
        std::fs::remove_dir_all(d).ok();
    }
}
