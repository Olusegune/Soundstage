//! Pronunciation rules. A rule says "when the script has this word, have the voice say that spelling instead", for names
//! and terms a voice gets wrong (`Okafor` as `Oh-kah-for`).
//!
//! Rules change only what is sent to the voice. The script, the subtitles and the exports keep the original spelling.
//! Rules are applied in one pass, longest match first, so one rule's output is never rewritten by another.

use crate::tags::Token;
use serde::{Deserialize, Serialize};

pub const MAX_RULES: usize = 500;
const MAX_OUTPUT_CHARS: usize = 200_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Rule {
    /// The word or phrase as written in the script.
    pub from: String,
    /// What the voice should be given instead.
    pub to: String,
    /// Match only whole words, so `Ade` does not change `Adelaide`.
    pub whole_word: bool,
    pub ignore_case: bool,
}

impl Default for Rule {
    fn default() -> Self {
        Rule { from: String::new(), to: String::new(), whole_word: true, ignore_case: true }
    }
}

fn plain(s: &str, max: usize) -> bool {
    let n = s.chars().count();
    n >= 1 && n <= max && !s.chars().any(|c| c.is_control())
}

impl Rule {
    pub fn validate(&self) -> Result<(), String> {
        let from = self.from.trim();
        let to = self.to.trim();
        if !plain(from, 60) {
            return Err("A rule needs a word or phrase of up to 60 characters.".into());
        }
        if !plain(to, 120) {
            return Err(format!("“{from}” needs a spelling for the voice, up to 120 characters."));
        }
        if to.contains('[') || to.contains(']') {
            return Err(format!("The spelling for “{from}” cannot contain square brackets, which are used for directions like [pause:short]."));
        }
        if from.eq_ignore_ascii_case(to) && from == to {
            return Err(format!("“{from}” is already spelled that way."));
        }
        Ok(())
    }
}

/// Check a whole list: each rule, the count, and repeated words.
pub fn validate_all(rules: &[Rule]) -> Result<(), String> {
    if rules.len() > MAX_RULES {
        return Err(format!("Too many pronunciation rules (limit {MAX_RULES})."));
    }
    let mut seen = std::collections::BTreeSet::new();
    for r in rules {
        r.validate()?;
        let key = (r.from.trim().to_lowercase(), r.ignore_case, r.whole_word);
        if !seen.insert(key) {
            return Err(format!("“{}” is listed twice.", r.from.trim()));
        }
    }
    Ok(())
}

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Apply rules to plain text. The result depends only on the text and the rules, so cache keys stay stable.
pub fn apply(text: &str, rules: &[Rule]) -> String {
    if rules.is_empty() || text.is_empty() {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let folded: Vec<char> = chars.iter().map(|c| fold(*c)).collect();
    // Longest `from` first; the original order breaks ties, so earlier rules win.
    let mut order: Vec<(usize, Vec<char>, Vec<char>)> = rules
        .iter()
        .enumerate()
        .filter(|(_, r)| r.validate().is_ok())
        .map(|(i, r)| {
            let f = r.from.trim();
            (i, f.chars().collect::<Vec<_>>(), f.chars().map(fold).collect::<Vec<_>>())
        })
        .collect();
    order.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    let mut out = String::with_capacity(text.len() + 16);
    let mut i = 0;
    while i < chars.len() {
        let mut hit = None;
        for (ri, exact, lower) in &order {
            let r = &rules[*ri];
            let n = exact.len();
            if i + n > chars.len() {
                continue;
            }
            let matches = if r.ignore_case { folded[i..i + n] == lower[..] } else { chars[i..i + n] == exact[..] };
            if !matches {
                continue;
            }
            if r.whole_word {
                let before_ok = i == 0 || !is_word(chars[i - 1]);
                let after_ok = i + n == chars.len() || !is_word(chars[i + n]);
                if !(before_ok && after_ok) {
                    continue;
                }
            }
            hit = Some((n, r.to.trim()));
            break;
        }
        match hit {
            Some((n, to)) => {
                out.push_str(to);
                i += n;
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
        if out.len() > MAX_OUTPUT_CHARS {
            return text.to_string();
        }
    }
    out
}

/// Apply rules to the spoken text of a line. Directions and pauses are left exactly as they are.
pub fn apply_tokens(tokens: Vec<Token>, rules: &[Rule]) -> Vec<Token> {
    if rules.is_empty() {
        return tokens;
    }
    tokens.into_iter().map(|t| if let Token::Text(s) = t { Token::Text(apply(&s, rules)) } else { t }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(from: &str, to: &str) -> Rule {
        Rule { from: from.into(), to: to.into(), ..Default::default() }
    }

    #[test]
    fn whole_words_only_and_case_is_ignored_by_default() {
        let rules = [r("Ade", "Ah-day")];
        assert_eq!(apply("Ade met ade in Adelaide, said Ade's friend.", &rules), "Ah-day met Ah-day in Adelaide, said Ah-day's friend.");
        let exact = Rule { ignore_case: false, ..r("Ade", "Ah-day") };
        assert_eq!(apply("Ade and ade", &[exact]), "Ah-day and ade");
        let inside = Rule { whole_word: false, ..r("Ade", "Ah-day") };
        assert_eq!(apply("Adelaide", &[inside]), "Ah-dayLaide".replace("Laide", "laide"));
    }

    #[test]
    fn longest_match_wins_and_output_is_never_rewritten_again() {
        let rules = [r("Okafor", "Oh-kah-for"), r("Ama Okafor", "Ah-ma Oh-kah-for"), r("kah", "KAH")];
        assert_eq!(apply("Ama Okafor spoke. Okafor left.", &rules), "Ah-ma Oh-kah-for spoke. Oh-kah-for left.");
    }

    #[test]
    fn phrases_punctuation_and_unicode_work() {
        assert_eq!(apply("Visit Ọ̀yọ́ today.", &[r("Ọ̀yọ́", "Oh-yo")]), "Visit Oh-yo today.");
        assert_eq!(apply("The U.N. met.", &[r("U.N.", "United Nations")]), "The United Nations met.");
        assert_eq!(apply("İstanbul İSTANBUL", &[r("İstanbul", "Istanbul")]), "Istanbul Istanbul");
        assert_eq!(apply("nothing here", &[r("x", "y")]), "nothing here");
        assert_eq!(apply("", &[r("x", "y")]), "");
    }

    #[test]
    fn only_spoken_text_changes_and_directions_survive() {
        let (tokens, _) = crate::tags::tokenize("Okafor [whisper] hello Okafor [pause:long] bye");
        let out = apply_tokens(tokens.clone(), &[r("Okafor", "Oh-kah-for")]);
        assert_eq!(out.len(), tokens.len());
        assert!(matches!(out[1], Token::Delivery(_)) && out.iter().any(|t| matches!(t, Token::Pause(_))));
        let spoken: Vec<&str> = out.iter().filter_map(|t| if let Token::Text(s) = t { Some(s.as_str()) } else { None }).collect();
        assert!(spoken.iter().any(|s| s.contains("Oh-kah-for")) && spoken.iter().all(|s| !s.contains("Okafor")));
        assert_eq!(apply_tokens(tokens.clone(), &[]), tokens);
    }

    #[test]
    fn bad_rules_are_refused_and_ignored_when_applied() {
        assert!(r("", "x").validate().is_err());
        assert!(r("x", "").validate().is_err());
        assert!(r("x", "[pause:long]").validate().is_err(), "a rule cannot smuggle in a direction");
        assert!(r("a\u{7}b", "x").validate().is_err());
        assert!(r(&"a".repeat(61), "x").validate().is_err());
        assert!(r("same", "same").validate().is_err());
        assert!(validate_all(&[r("a", "b"), r("A", "c")]).is_err(), "the same word twice");
        assert!(validate_all(&vec![r("a", "b"); 501]).is_err());
        assert_eq!(apply("x y", &[r("x", "[pause:long]")]), "x y", "an invalid rule is skipped, not applied");
    }
}
