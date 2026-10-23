//! Canonical direction tags (PROVIDERS.md section 2). Adapters translate these;
//! UI code and project files never carry provider-native syntax.

use serde::{Deserialize, Serialize};

pub const NON_VERBAL: &[&str] =
    &["laugh", "chuckle", "sigh", "cough", "sneeze", "gasp", "cry", "breath", "sniff"];
pub const DELIVERY: &[&str] =
    &["whisper", "shout", "excited", "sad", "tired", "sarcastic", "curious", "serious"];
pub const PAUSE_SHORT_MS: u32 = 250;
pub const PAUSE_LONG_MS: u32 = 800;
pub const MAX_PAUSE_MS: u32 = 10_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Token {
    Text(String),
    NonVerbal(String),
    Delivery(String),
    Pause(u32),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TagIssue {
    pub tag: String,
    pub message: String,
}

fn classify(norm: &str) -> Option<Token> {
    if NON_VERBAL.contains(&norm) {
        return Some(Token::NonVerbal(norm.to_string()));
    }
    if DELIVERY.contains(&norm) {
        return Some(Token::Delivery(norm.to_string()));
    }
    let arg = norm.strip_prefix("pause:")?;
    let ms = match arg {
        "short" => PAUSE_SHORT_MS,
        "long" => PAUSE_LONG_MS,
        n if !n.is_empty() && n.len() <= 6 && n.bytes().all(|b| b.is_ascii_digit()) => {
            let v: u32 = n.parse().ok()?;
            if v == 0 || v > MAX_PAUSE_MS {
                return None;
            }
            v
        }
        _ => return None,
    };
    Some(Token::Pause(ms))
}

/// Split a line into text and canonical tags. Unknown tags are reported, never emitted.
pub fn tokenize(input: &str) -> (Vec<Token>, Vec<TagIssue>) {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut issues = Vec::new();
    let mut text = String::new();
    let flush = |text: &mut String, tokens: &mut Vec<Token>| {
        if !text.trim().is_empty() {
            let words: Vec<&str> = text.split_whitespace().collect();
            tokens.push(Token::Text(words.join(" ")));
        }
        text.clear();
    };
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            let mut j = i + 1;
            let mut close = None;
            while j < chars.len() {
                match chars[j] {
                    ']' => {
                        close = Some(j);
                        break;
                    }
                    '[' => break,
                    _ => j += 1,
                }
            }
            if let Some(j) = close {
                let inner: String = chars[i + 1..j].iter().collect();
                let norm = inner.trim().to_lowercase();
                match classify(&norm) {
                    Some(tok) => {
                        flush(&mut text, &mut tokens);
                        tokens.push(tok);
                    }
                    None => issues.push(TagIssue {
                        tag: format!("[{}]", inner.trim()),
                        message: format!("[{}] is not a Soundstage tag.", inner.trim()),
                    }),
                }
                i = j + 1;
                continue;
            }
        }
        text.push(chars[i]);
        i += 1;
    }
    flush(&mut text, &mut tokens);
    (tokens, issues)
}

pub fn token_canonical(t: &Token) -> String {
    match t {
        Token::Text(s) => s.clone(),
        Token::NonVerbal(s) | Token::Delivery(s) => format!("[{s}]"),
        Token::Pause(ms) => format!("[pause:{ms}]"),
    }
}

/// The words a voice would say, with tags removed.
pub fn spoken_text(tokens: &[Token]) -> String {
    tokens
        .iter()
        .filter_map(|t| if let Token::Text(s) = t { Some(s.as_str()) } else { None })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Canonical tag names a line needs (non-verbal and delivery), without duplicates.
pub fn required_tags(tokens: &[Token]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tokens {
        if let Token::NonVerbal(s) | Token::Delivery(s) = t {
            if !out.contains(s) {
                out.push(s.clone());
            }
        }
    }
    out
}

/// Rough expected speaking time, used for contract tests and layout hints.
pub fn expected_duration_ms(tokens: &[Token]) -> u64 {
    let words = spoken_text(tokens).split_whitespace().count() as u64;
    let pauses: u64 = tokens.iter().map(|t| if let Token::Pause(ms) = t { *ms as u64 } else { 0 }).sum();
    words * 350 + pauses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_tags_case_insensitively() {
        let (t, issues) = tokenize("Well [Sigh] [PAUSE:short] fine [pause:350] ok");
        assert!(issues.is_empty());
        assert_eq!(
            t,
            vec![
                Token::Text("Well".into()),
                Token::NonVerbal("sigh".into()),
                Token::Pause(250),
                Token::Text("fine".into()),
                Token::Pause(350),
                Token::Text("ok".into()),
            ]
        );
    }

    #[test]
    fn unknown_tags_are_errors_and_never_emitted() {
        let (t, issues) = tokenize("Hi [laughs] there [pause:0] [pause:99999] [pause:long]");
        assert_eq!(issues.len(), 3);
        assert_eq!(t, vec![Token::Text("Hi there".into()), Token::Pause(800)]);
    }

    #[test]
    fn unclosed_bracket_is_plain_text() {
        let (t, issues) = tokenize("a [b c");
        assert!(issues.is_empty());
        assert_eq!(t, vec![Token::Text("a [b c".into())]);
    }

    #[test]
    fn required_tags_dedupes() {
        let (t, _) = tokenize("[laugh] a [laugh] b [whisper] c");
        assert_eq!(required_tags(&t), vec!["laugh", "whisper"]);
    }
}
