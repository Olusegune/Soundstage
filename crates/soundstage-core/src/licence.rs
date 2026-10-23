//! Licence tags and the export gate (SAFETY_LICENSING.md section 3).
//! `unknown` is data, not a guess: it blocks commercial export.

use crate::util::{parse_date, today_days};
use serde::{Deserialize, Serialize};

pub const STALE_AFTER_DAYS: i64 = 90;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Licence {
    Commercial,
    NonCommercial,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct LicenceInfo {
    pub licence: Licence,
    pub source: Option<String>,
    /// `YYYY-MM-DD` of the last time someone checked the terms.
    pub checked: Option<String>,
    pub notes: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LicenceTarget {
    Commercial,
    #[default]
    Personal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Blocking,
    Notice,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Subject {
    /// `voice`, `model`, `music` or `effect`.
    pub kind: String,
    pub id: String,
    pub name: String,
    pub info: LicenceInfo,
    /// Destinations this asset's terms prohibit (for example `broadcast`).
    pub uses_prohibited: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GateItem {
    pub kind: String,
    pub id: String,
    pub name: String,
    pub reason: String,
    pub severity: Severity,
    pub source: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GateReport {
    pub target: LicenceTarget,
    pub checked_on: String,
    pub blocking: Vec<GateItem>,
    pub notices: Vec<GateItem>,
    pub allowed: bool,
}

/// Age in days of a recorded check, or `None` when the date is missing or invalid.
pub fn check_age_days(info: &LicenceInfo, today: i64) -> Option<i64> {
    info.checked.as_deref().and_then(parse_date).map(|d| today - d)
}

pub fn gate(target: LicenceTarget, subjects: &[Subject], destination: &str, today: i64) -> GateReport {
    let mut blocking = Vec::new();
    let mut notices = Vec::new();
    let mut seen: Vec<(String, String)> = Vec::new();
    for s in subjects {
        let key = (s.kind.clone(), s.id.clone());
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let mk = |reason: String, severity| GateItem {
            kind: s.kind.clone(),
            id: s.id.clone(),
            name: s.name.clone(),
            reason,
            severity,
            source: s.info.source.clone(),
        };
        let age = check_age_days(&s.info, today);
        let problem: Option<String> = match s.info.licence {
            Licence::NonCommercial => Some("Its terms do not allow commercial use.".into()),
            Licence::Unknown => Some("Its licence is unknown. Check the terms and record the result.".into()),
            Licence::Commercial => match age {
                None => Some("It is tagged commercial but has no check date. Record when the terms were checked.".into()),
                Some(a) if a > STALE_AFTER_DAYS => Some(format!(
                    "Its licence was last checked {a} days ago (over {STALE_AFTER_DAYS}). It needs a re-check."
                )),
                _ => None,
            },
        };
        let prohibited = s.uses_prohibited.iter().any(|u| u.eq_ignore_ascii_case(destination));
        match target {
            LicenceTarget::Commercial => {
                if let Some(p) = problem {
                    blocking.push(mk(p, Severity::Blocking));
                }
                if prohibited {
                    blocking.push(mk(format!("Its terms prohibit use for {destination}."), Severity::Blocking));
                }
            }
            LicenceTarget::Personal => {
                if matches!(s.info.licence, Licence::NonCommercial | Licence::Unknown) {
                    notices.push(mk(
                        "Not cleared for commercial use. Fine for a personal project.".into(),
                        Severity::Notice,
                    ));
                }
            }
        }
    }
    GateReport {
        target,
        checked_on: crate::util::date_string(today),
        allowed: blocking.is_empty(),
        blocking,
        notices,
    }
}

pub fn gate_today(target: LicenceTarget, subjects: &[Subject], destination: &str) -> GateReport {
    gate(target, subjects, destination, today_days())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::date_string;

    fn subj(id: &str, lic: Licence, checked_days_ago: Option<i64>, today: i64) -> Subject {
        Subject {
            kind: "voice".into(),
            id: id.into(),
            name: format!("Voice {id}"),
            info: LicenceInfo {
                licence: lic,
                source: Some("https://example.com/terms".into()),
                checked: checked_days_ago.map(|d| date_string(today - d)),
                notes: String::new(),
            },
            uses_prohibited: vec![],
        }
    }

    #[test]
    fn commercial_export_blocks_non_commercial_and_unknown_by_name() {
        let today = 20_000;
        let s = vec![
            subj("a", Licence::Commercial, Some(1), today),
            subj("b", Licence::NonCommercial, Some(1), today),
            subj("c", Licence::Unknown, None, today),
        ];
        let r = gate(LicenceTarget::Commercial, &s, "streaming", today);
        assert!(!r.allowed);
        let names: Vec<_> = r.blocking.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["Voice b", "Voice c"]);
    }

    #[test]
    fn licence_checked_91_days_ago_blocks_until_rechecked() {
        let today = 20_000;
        let old = vec![subj("a", Licence::Commercial, Some(91), today)];
        assert!(!gate(LicenceTarget::Commercial, &old, "streaming", today).allowed);
        let ok = vec![subj("a", Licence::Commercial, Some(90), today)];
        assert!(gate(LicenceTarget::Commercial, &ok, "streaming", today).allowed);
    }

    #[test]
    fn commercial_without_a_check_date_blocks() {
        let today = 20_000;
        let s = vec![subj("a", Licence::Commercial, None, today)];
        assert!(!gate(LicenceTarget::Commercial, &s, "streaming", today).allowed);
    }

    #[test]
    fn personal_target_shows_notices_and_still_allows() {
        let today = 20_000;
        let s = vec![subj("b", Licence::NonCommercial, Some(1), today), subj("c", Licence::Unknown, None, today)];
        let r = gate(LicenceTarget::Personal, &s, "streaming", today);
        assert!(r.allowed);
        assert_eq!(r.notices.len(), 2);
    }

    #[test]
    fn prohibited_destination_blocks_a_commercial_asset() {
        let today = 20_000;
        let mut s = subj("m", Licence::Commercial, Some(1), today);
        s.kind = "music".into();
        s.uses_prohibited = vec!["broadcast".into()];
        assert!(gate(LicenceTarget::Commercial, &[s.clone()], "broadcast", today).blocking.len() == 1);
        assert!(gate(LicenceTarget::Commercial, &[s], "streaming", today).allowed);
    }

    #[test]
    fn duplicate_subjects_are_listed_once() {
        let today = 20_000;
        let s = subj("b", Licence::Unknown, None, today);
        let r = gate(LicenceTarget::Commercial, &[s.clone(), s], "streaming", today);
        assert_eq!(r.blocking.len(), 1);
    }
}
