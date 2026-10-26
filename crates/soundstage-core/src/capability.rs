//! The model capability registry (PRD-002 section 4.4, `docs/spec/04` section 2): one entry per model
//! Soundstage can use, saying what it is for and how far that has been checked.
//!
//! The file ships inside the app (`data/models.json`). Two rules are enforced when it loads (D-09):
//! nothing may be `verified` unless it names the Model Lab runs that earned it, and an accent
//! or language may be `verified` only with evidence too. Until the Lab exists, everything is
//! `experimental` or `unverified`, and the app never says a language or accent is supported.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const REGISTRY_JSON: &str = include_str!("../data/models.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Verified,
    #[default]
    Experimental,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Evaluation {
    #[default]
    Unverified,
    Experimental,
    Verified,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Accent {
    pub name: String,
    pub language: String,
    pub status: Evaluation,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Language {
    pub code: String,
    pub status: Evaluation,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Controls {
    /// `none`, `tags`, `instructions` or `numeric`.
    pub emotion: String,
    pub pace: bool,
    pub pauses: bool,
    pub emphasis: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct NonVerbal {
    pub event: String,
    /// `native`, `tag` or `none`.
    pub support: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct RegistryLicence {
    pub name: String,
    /// `yes`, `no`, `conditional` or `unknown`.
    pub commercial: String,
    pub checked_on: Option<String>,
    pub conditions: Vec<crate::licence::LicenceCondition>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ModelEntry {
    pub id: String,
    pub provider: String,
    pub model: String,
    /// `tts`, `stt`, `sfx-music`, `llm`, `video`.
    pub kind: String,
    pub local: Option<bool>,
    pub sends_data: Option<bool>,
    pub host: Option<String>,
    pub languages: Vec<Language>,
    pub accents: Vec<Accent>,
    pub controls: Controls,
    pub non_verbal: Vec<NonVerbal>,
    pub cloning: Option<String>,
    pub needs_consent: bool,
    pub hardware_profile_min: String,
    pub licence: Option<RegistryLicence>,
    pub status: Status,
    /// Model Lab run ids that support a `verified` status.
    pub evidence: Vec<String>,
    pub last_verified: Option<String>,
    pub notes: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ModelRegistry {
    pub schema: u32,
    pub updated: String,
    pub models: Vec<ModelEntry>,
}

pub const PROFILES: [&str; 5] = ["cloud-first", "local-light", "local-standard", "local-advanced", "local-workstation"];

impl ModelRegistry {
    pub fn parse(json: &str) -> Result<ModelRegistry, String> {
        let r: ModelRegistry = serde_json::from_str(json).map_err(|e| format!("The model list is damaged: {e}"))?;
        r.validate()?;
        Ok(r)
    }

    /// The registry that ships with the app.
    pub fn builtin() -> ModelRegistry {
        Self::parse(REGISTRY_JSON).expect("the shipped model list is valid (checked by a test)")
    }

    pub fn get(&self, id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|m| m.id == id)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 {
            return Err(format!("Unknown model list format {}.", self.schema));
        }
        let mut seen = BTreeSet::new();
        for m in &self.models {
            let who = &m.id;
            if m.id.is_empty() || !seen.insert(m.id.clone()) {
                return Err(format!("Model id '{who}' is empty or repeated."));
            }
            if !matches!(m.kind.as_str(), "tts" | "stt" | "sfx-music" | "llm" | "video") {
                return Err(format!("{who}: unknown kind '{}'.", m.kind));
            }
            if !PROFILES.contains(&m.hardware_profile_min.as_str()) {
                return Err(format!("{who}: unknown hardware profile '{}'.", m.hardware_profile_min));
            }
            if m.sends_data == Some(true) && m.host.as_deref().is_none_or(str::is_empty) {
                return Err(format!("{who}: a model that sends data must name its host."));
            }
            if m.host.as_deref().is_some_and(|h| h.contains("://") || h.contains('/')) {
                return Err(format!("{who}: the host must be a bare name."));
            }
            // D-09: nothing is verified without the runs that verified it.
            if m.status == Status::Verified && (m.evidence.is_empty() || m.last_verified.is_none()) {
                return Err(format!("{who}: 'verified' needs evidence and a date."));
            }
            if m.languages.iter().any(|l| l.status == Evaluation::Verified && l.evidence.is_empty())
                || m.accents.iter().any(|a| a.status == Evaluation::Verified && a.evidence.is_empty())
            {
                return Err(format!("{who}: a verified language or accent needs evidence."));
            }
            if let Some(l) = &m.licence {
                if !matches!(l.commercial.as_str(), "yes" | "no" | "conditional" | "unknown") {
                    return Err(format!("{who}: unknown commercial value '{}'.", l.commercial));
                }
                if matches!(l.commercial.as_str(), "yes") && l.checked_on.is_none() {
                    return Err(format!("{who}: a commercial licence needs the date it was checked."));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_list_is_valid_and_claims_nothing_is_verified() {
        let r = ModelRegistry::builtin();
        assert!(r.models.len() >= 10);
        assert!(r.models.iter().all(|m| m.status != Status::Verified), "no model is verified before the Model Lab exists");
        assert!(r.models.iter().all(|m| m.languages.is_empty() && m.accents.is_empty()), "no language or accent claims");
    }

    #[test]
    fn every_adapter_in_the_app_has_an_entry() {
        let r = ModelRegistry::builtin();
        for id in ["windows-speech", "piper", "elevenlabs", "voicekiller", "whisper-local", "elevenlabs-audio", "openai-compatible", "kie", "fal", "comfyui"] {
            assert!(r.get(id).is_some(), "missing {id}");
        }
    }

    #[test]
    fn cloud_entries_name_their_host_and_say_they_send_data() {
        let r = ModelRegistry::builtin();
        for m in r.models.iter().filter(|m| m.local == Some(false)) {
            assert_eq!(m.sends_data, Some(true), "{}", m.id);
            assert!(m.host.is_some(), "{}", m.id);
        }
    }

    fn with(edit: impl FnOnce(&mut ModelEntry)) -> Result<(), String> {
        let mut r = ModelRegistry::builtin();
        edit(&mut r.models[0]);
        r.validate()
    }

    #[test]
    fn verified_needs_evidence_and_a_date() {
        assert!(with(|m| m.status = Status::Verified).is_err());
        assert!(with(|m| {
            m.status = Status::Verified;
            m.evidence = vec!["lab-2026-11-01-a".into()];
            m.last_verified = Some("2026-11-01".into());
        })
        .is_ok());
    }

    #[test]
    fn a_verified_accent_needs_evidence() {
        assert!(with(|m| m.accents.push(Accent { name: "Nigerian English".into(), language: "en".into(), status: Evaluation::Verified, evidence: vec![] })).is_err());
        assert!(with(|m| m.accents.push(Accent { name: "Nigerian English".into(), language: "en".into(), status: Evaluation::Unverified, evidence: vec![] })).is_ok());
    }

    #[test]
    fn repeated_ids_bad_hosts_profiles_and_unchecked_commercial_claims_are_refused() {
        let mut r = ModelRegistry::builtin();
        let dup = r.models[0].clone();
        r.models.push(dup);
        assert!(r.validate().is_err());
        assert!(with(|m| m.host = Some("https://evil.example/x".into())).is_err());
        assert!(with(|m| m.hardware_profile_min = "supercomputer".into()).is_err());
        assert!(with(|m| m.licence = Some(RegistryLicence { name: "x".into(), commercial: "yes".into(), checked_on: None, conditions: vec![] })).is_err());
        assert!(with(|m| m.sends_data = Some(true)).is_err(), "sends data but names no host");
    }

    #[test]
    fn damaged_files_give_a_plain_error() {
        assert!(ModelRegistry::parse("{nope").unwrap_err().starts_with("The model list is damaged"));
        assert!(ModelRegistry::parse(r#"{"schema":2,"updated":"x","models":[]}"#).is_err());
    }
}
