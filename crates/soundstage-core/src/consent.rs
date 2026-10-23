//! Consent records (SAFETY_LICENSING.md section 2.2 and 10). Cloning is a v2
//! feature, but the gate exists now so that no later code path can skip it.

use crate::util::{parse_date, today_days};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentStatus {
    Active,
    Revoked,
    Expired,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Evidence {
    /// `recorded statement`, `signed form` or `in-app attestation`.
    pub method: String,
    pub stored_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ConsentRecord {
    pub id: String,
    pub voice_subject: String,
    pub subject_is_self: bool,
    pub subject_age_confirmed_18plus: bool,
    pub purpose: String,
    pub permitted_uses: Vec<String>,
    pub prohibited_uses: Vec<String>,
    pub commercial_use_permitted: bool,
    pub territory: String,
    pub expires: Option<String>,
    pub revocable: bool,
    pub evidence: Option<Evidence>,
    pub signed_at: Option<String>,
    pub status: ConsentStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Use {
    Dialogue,
    Narration,
    PerformanceReference,
}

impl Use {
    fn label(&self) -> &'static str {
        match self {
            Use::Dialogue => "dialogue",
            Use::Narration => "narration",
            Use::PerformanceReference => "performance_reference",
        }
    }
}

/// Every reason a record cannot be used, so the UI can list them all.
pub fn problems(rec: &ConsentRecord, use_: &Use, commercial: bool, today: i64) -> Vec<String> {
    let mut p = Vec::new();
    match rec.status {
        ConsentStatus::Revoked => p.push("Consent for this voice was revoked.".to_string()),
        ConsentStatus::Expired => p.push("Consent for this voice has expired.".to_string()),
        ConsentStatus::Active => {}
    }
    if let Some(exp) = rec.expires.as_deref() {
        match parse_date(exp) {
            Some(d) if d < today => p.push("The consent record has passed its expiry date.".into()),
            None => p.push("The consent record has an unreadable expiry date.".into()),
            _ => {}
        }
    }
    if !rec.subject_age_confirmed_18plus {
        p.push("The voice owner's age (18 or over) is not confirmed.".into());
    }
    if rec.voice_subject.trim().is_empty() {
        p.push("The record does not name the voice owner.".into());
    }
    if rec.evidence.as_ref().map_or(true, |e| e.method.trim().is_empty() || e.stored_at.trim().is_empty()) {
        p.push("There is no stored evidence of consent.".into());
    }
    if rec.signed_at.as_deref().map_or(true, |s| s.trim().is_empty()) {
        p.push("The record is not signed.".into());
    }
    if rec.purpose.trim().is_empty() {
        p.push("The record does not state a purpose.".into());
    }
    if !rec.permitted_uses.iter().any(|u| u == use_.label()) {
        p.push(format!("The record does not permit {}.", use_.label().replace('_', " ")));
    }
    if commercial && !rec.commercial_use_permitted {
        p.push("The record does not permit commercial use.".into());
    }
    p
}

/// Cloning and performance-reference use are impossible without a clean record.
pub fn require(rec: Option<&ConsentRecord>, use_: &Use, commercial: bool) -> Result<(), Vec<String>> {
    match rec {
        None => Err(vec!["No consent record. A voice is cloned only with its owner's recorded consent.".into()]),
        Some(r) => {
            let p = problems(r, use_, commercial, today_days());
            if p.is_empty() { Ok(()) } else { Err(p) }
        }
    }
}

pub fn revoke(rec: &mut ConsentRecord) -> Result<(), String> {
    if !rec.revocable {
        return Err("This record is marked not revocable, which Soundstage does not accept.".into());
    }
    rec.status = ConsentStatus::Revoked;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn good() -> ConsentRecord {
        ConsentRecord {
            id: "consent-1".into(),
            voice_subject: "Ama Okafor".into(),
            subject_is_self: true,
            subject_age_confirmed_18plus: true,
            purpose: "narration for a short".into(),
            permitted_uses: vec!["narration".into(), "dialogue".into()],
            prohibited_uses: vec!["impersonation".into()],
            commercial_use_permitted: true,
            territory: "worldwide".into(),
            expires: None,
            revocable: true,
            evidence: Some(Evidence { method: "recorded statement".into(), stored_at: "consent/evidence-1.enc".into() }),
            signed_at: Some("2026-10-01T10:00:00Z".into()),
            status: ConsentStatus::Active,
        }
    }

    #[test]
    fn complete_record_is_accepted() {
        assert!(require(Some(&good()), &Use::Dialogue, true).is_ok());
    }

    #[test]
    fn missing_or_incomplete_record_is_rejected() {
        assert!(require(None, &Use::Dialogue, false).is_err());
        let mut r = good();
        r.evidence = None;
        r.subject_age_confirmed_18plus = false;
        let e = require(Some(&r), &Use::Dialogue, false).unwrap_err();
        assert_eq!(e.len(), 2);
    }

    #[test]
    fn revoked_record_blocks_use() {
        let mut r = good();
        revoke(&mut r).unwrap();
        assert!(require(Some(&r), &Use::Dialogue, false).is_err());
    }

    #[test]
    fn use_and_commercial_scope_are_enforced() {
        let mut r = good();
        r.commercial_use_permitted = false;
        assert!(require(Some(&r), &Use::Dialogue, true).is_err());
        assert!(require(Some(&r), &Use::Dialogue, false).is_ok());
        assert!(require(Some(&r), &Use::PerformanceReference, false).is_err());
    }

    #[test]
    fn expired_record_is_rejected() {
        let mut r = good();
        r.expires = Some("2020-01-01".into());
        assert!(require(Some(&r), &Use::Dialogue, false).is_err());
    }
}
