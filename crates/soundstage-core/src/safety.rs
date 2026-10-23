//! Content refusals (SAFETY_LICENSING.md section 5). This is a small keyword
//! screen. A pass means no rule fired. It does not mean the content is allowed.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Refusal {
    pub rule: String,
    pub message: String,
    pub alternative: String,
}

const FRAUD_PHRASES: &[&str] = &[
    "send me your password",
    "tell me your pin",
    "read me the code we just sent",
    "your account has been compromised",
    "pay with gift cards",
    "buy gift cards",
    "wire the money to",
    "send your otp",
    "share your otp",
    "mom, i'm in jail",
    "grandma, i need bail",
];

const ELECTION_PHRASES: &[&str] = &["vote for me, your president", "this is your president speaking"];

const CHILD_SEXUAL: &[&str] = &["sexual", "explicit", "nude", "erotic"];
const MINOR_WORDS: &[&str] = &["child", "minor", "underage", "schoolgirl", "schoolboy", "year-old girl", "year-old boy"];

pub fn check(text: &str) -> Option<Refusal> {
    let t = text.to_lowercase();
    if FRAUD_PHRASES.iter().any(|p| t.contains(p)) {
        return Some(Refusal {
            rule: "fraud".into(),
            message: "This script looks like it is meant to trick someone into paying or sharing private details, which Soundstage does not render.".into(),
            alternative: "Rewrite it as an openly fictional scene that does not ask the listener for money or credentials.".into(),
        });
    }
    if ELECTION_PHRASES.iter().any(|p| t.contains(p)) {
        return Some(Refusal {
            rule: "impersonation".into(),
            message: "This script speaks as a real public official, which Soundstage does not render.".into(),
            alternative: "Use a clearly fictional character with a library voice.".into(),
        });
    }
    if MINOR_WORDS.iter().any(|w| t.contains(w)) && CHILD_SEXUAL.iter().any(|w| t.contains(w)) {
        return Some(Refusal {
            rule: "minors".into(),
            message: "Sexual content involving minors is never rendered.".into(),
            alternative: "Remove the content. Nothing else can be offered here.".into(),
        });
    }
    None
}

const DESIGN_PHRASES: &[&str] = &["impersonat", "imitat", "clone of", "sound exactly like", "sounds exactly like", "in the voice of", "identical to the voice of", "deepfake", "voice of the real"];

/// A designed voice describes a character (age, tone, accent), never a real person. Real-person rules apply to every
/// project (SAFETY_LICENSING.md section 5), so descriptions that aim at someone are refused before anything is sent.
pub fn check_design(description: &str) -> Option<Refusal> {
    let lower = description.to_lowercase();
    let names_someone = ["sounds like ", "sound like ", "sounding like "].iter().any(|lead| {
        lower.match_indices(lead).any(|(i, _)| {
            // Offsets come from the lower-cased copy, so slice the original only where that is valid text.
            let rest = description.get((i + lead.len()).min(description.len())..).unwrap_or("");
            rest.split_whitespace().next().is_some_and(|w| w.chars().next().is_some_and(|c| c.is_uppercase()))
        })
    });
    if DESIGN_PHRASES.iter().any(|p| lower.contains(p)) || names_someone {
        return Some(Refusal {
            rule: "impersonation".into(),
            message: "A designed voice cannot be modelled on a real person.".into(),
            alternative: "Describe the character instead: age, tone, pace and accent, for example \"a calm older woman with a warm Lagos accent\".".into(),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fraud_script_is_refused_with_rule_and_alternative() {
        let r = check("Hello, your account has been compromised. Buy gift cards now.").unwrap();
        assert_eq!(r.rule, "fraud");
        assert!(!r.alternative.is_empty());
    }

    #[test]
    fn minor_sexual_content_is_refused() {
        assert_eq!(check("an explicit scene with a minor").unwrap().rule, "minors");
    }

    #[test]
    fn voice_design_refuses_real_people_and_allows_characters() {
        for bad in ["sounds like Morgan Freeman", "a voice that sounds exactly like my boss", "Impersonate the president", "in the voice of a famous actor", "clone of her voice", "make a deepfake of him"] {
            assert_eq!(check_design(bad).unwrap().rule, "impersonation", "{bad}");
        }
        for ok in ["a calm older woman with a warm Lagos accent", "a gruff pirate captain, slow and gravelly", "bright young narrator, sounds like a friendly teacher", "energetic child-like robot sidekick", "\u{130}stanbul style: sounds like x"] {
            assert!(check_design(ok).is_none(), "{ok}");
        }
    }

    #[test]
    fn ordinary_dialogue_passes() {
        assert!(check("Good morning. Did you sleep well? The child laughed.").is_none());
    }
}
