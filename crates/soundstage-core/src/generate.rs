//! Generated music and sound effects. One `Generator` mechanism for every source (ElevenLabs, Kie including Suno,
//! fal, ComfyUI). Generated audio is saved into the project as a normal asset with its licence set to `unknown`,
//! a record of exactly how it was made, and the same commercial export gate as everything else. PRD: generated music
//! stays off the commercial path until its terms are cleared by the person making the film.

use crate::audio::Stereo;
use crate::licence::LicenceInfo;
use crate::providers::AdapterError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenKind {
    Music,
    Sfx,
}

#[derive(Clone, Debug)]
pub struct GenRequest {
    pub kind: GenKind,
    pub prompt: String,
    pub seconds: Option<f32>,
    pub instrumental: bool,
}

#[derive(Clone, Debug)]
pub struct GenOutput {
    /// 48 kHz stereo, ready to mix.
    pub audio: Stereo,
    pub provider_version: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct GenLimits {
    pub min_s: f32,
    pub max_s: f32,
    pub default_s: f32,
    /// False when the source decides the length itself (a Suno song, for example).
    pub supports_length: bool,
}

pub trait Generator: Send + Sync {
    fn id(&self) -> &str;
    fn label(&self) -> &str;
    fn kinds(&self) -> Vec<GenKind>;
    fn limits(&self, kind: GenKind) -> GenLimits;
    fn licence(&self) -> LicenceInfo;
    fn notes(&self) -> String;
    /// True when the prompt leaves this computer.
    fn sends_data(&self) -> bool;
    /// False when nothing is set up yet (no key, no server).
    fn ready(&self) -> bool {
        true
    }
    fn generate(&self, req: &GenRequest) -> Result<GenOutput, AdapterError>;
    fn check_connection(&self) -> Result<(), AdapterError> {
        Ok(())
    }
}

pub const MAX_PROMPT_CHARS: usize = 1000;

/// Vet a prompt before it is sent anywhere. Returns the cleaned prompt, or a plain reason.
pub fn clean_prompt(p: &str) -> Result<String, String> {
    let t: String = p.chars().map(|c| if c == '\n' || c == '\r' || c == '\t' { ' ' } else { c }).collect();
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().any(|c| c.is_control()) {
        return Err("The description has characters Soundstage cannot send.".into());
    }
    if t.chars().count() < 3 {
        return Err("Describe the sound in a few words.".into());
    }
    if t.chars().count() > MAX_PROMPT_CHARS {
        return Err(format!("Keep the description under {MAX_PROMPT_CHARS} characters."));
    }
    if let Some(r) = crate::safety::check(&t) {
        return Err(format!("{} {}", r.message, r.alternative));
    }
    Ok(t)
}

pub fn clamp_seconds(l: &GenLimits, s: Option<f32>) -> f32 {
    let v = s.filter(|x| x.is_finite()).unwrap_or(l.default_s);
    v.clamp(l.min_s, l.max_s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompts_are_cleaned_and_vetted() {
        assert_eq!(clean_prompt("  warm\n piano   theme ").unwrap(), "warm piano theme");
        assert!(clean_prompt("ab").is_err());
        assert!(clean_prompt(&"x".repeat(1001)).is_err());
        assert!(clean_prompt("a\u{0}b sound").is_err());
        assert!(clean_prompt("Your account has been compromised, buy gift cards").is_err());
        assert!(clean_prompt("gentle rain on a tin roof").is_ok());
    }

    #[test]
    fn lengths_are_clamped_to_what_the_source_allows() {
        let l = GenLimits { min_s: 3.0, max_s: 60.0, default_s: 30.0, supports_length: true };
        assert_eq!(clamp_seconds(&l, None), 30.0);
        assert_eq!(clamp_seconds(&l, Some(1.0)), 3.0);
        assert_eq!(clamp_seconds(&l, Some(999.0)), 60.0);
        assert_eq!(clamp_seconds(&l, Some(f32::NAN)), 30.0);
    }
}
