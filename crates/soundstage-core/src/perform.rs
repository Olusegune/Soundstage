//! From plan to performance (MS3, decisions D-17 and D-18): how a line's plan reaches a render, and what a
//! given voice will and will not do with it.
//!
//! The rules, in short:
//! * Anything the person set (project, character, scene or line direction) beats the plan; the plan beats the defaults.
//! * A plan can never stop a render. If a voice cannot do something the plan asks for (an emotion, a laugh), that part
//!   is left out and listed as "not used by this voice", instead of failing the line.
//! * Pauses become ordinary pause tags. Emphasis is passed on only to voices that declare they can use it.

use crate::capability::ModelRegistry;
use crate::direction::Direction;
use crate::plan::LinePlan;
use crate::providers::Capabilities;
use crate::tags::{DELIVERY, NON_VERBAL};
use serde::{Deserialize, Serialize};

/// What a voice will do with a line's plan.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SupportReport {
    /// Parts of the plan the voice will use, in plain words.
    pub used: Vec<String>,
    /// Parts it will not use, so the person is told before paying for the take.
    pub ignored: Vec<String>,
}

/// The delivery the plan asks for: the strongest emotion shown on the surface that a voice can be told, and the pace.
pub fn plan_direction(lp: &LinePlan) -> Direction {
    let emotion = lp
        .emotion
        .iter()
        .filter(|e| e.surface)
        .max_by(|a, b| a.intensity.total_cmp(&b.intensity))
        .map(|e| e.name.to_lowercase())
        .filter(|e| DELIVERY.contains(&e.as_str()));
    Direction { emotion, pace: lp.pace, pause_before_ms: None, note: None }
}

/// Character ranges that are inside a `[tag]`, where nothing may be inserted.
fn tag_spans(chars: &[char]) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut open = None;
    for (i, c) in chars.iter().enumerate() {
        match (c, open) {
            ('[', None) => open = Some(i),
            (']', Some(s)) => {
                out.push((s, i));
                open = None;
            }
            _ => {}
        }
    }
    out
}

/// The line's text with the plan's pauses and sounds written in as ordinary tags. Sounds the voice cannot make are
/// left out (they are reported by `support`), so a plan can never make a line unrenderable.
pub fn plan_text(text: &str, lp: &LinePlan, caps: &Capabilities) -> String {
    let chars: Vec<char> = text.chars().collect();
    let spans = tag_spans(&chars);
    let free = |at: usize| at <= chars.len() && !spans.iter().any(|&(s, e)| at > s && at <= e);
    let mut adds: Vec<(usize, String)> = vec![];
    for p in &lp.pauses {
        if free(p.at) && p.ms > 0 {
            adds.push((p.at, format!(" [pause:{}] ", p.ms.min(crate::tags::MAX_PAUSE_MS))));
        }
    }
    for e in &lp.events {
        let kind = e.kind.to_lowercase();
        if e.source == "provider-tag" && NON_VERBAL.contains(&kind.as_str()) && caps.tags.contains(&kind) && free(e.at) {
            adds.push((e.at, format!(" [{kind}] ")));
        }
    }
    if adds.is_empty() {
        return text.to_string();
    }
    adds.sort_by(|a, b| b.0.cmp(&a.0));
    let mut out = chars;
    for (at, s) in adds {
        let ins: Vec<char> = s.chars().collect();
        out.splice(at..at, ins);
    }
    out.into_iter().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn put(r: &mut SupportReport, ok: bool, what: String) {
    if ok { r.used.push(what) } else { r.ignored.push(what) }
}

/// What this voice will and will not use from the plan. `adapter_id` selects the voice's declared controls in the
/// model list; a voice that is not listed is assumed to take pauses and nothing else beyond its declared tags.
pub fn support(adapter_id: &str, caps: &Capabilities, lp: &LinePlan) -> SupportReport {
    let reg = ModelRegistry::builtin();
    let controls = reg.get(adapter_id).map(|m| m.controls.clone());
    let (can_pace, can_emphasis) = controls.as_ref().map_or((false, false), |c| (c.pace, c.emphasis));
    let mut r = SupportReport::default();
    let d = plan_direction(lp);
    if let Some(e) = &d.emotion {
        put(&mut r, caps.tags.contains(e), format!("feeling: {e}"));
    }
    for e in lp.emotion.iter().filter(|e| e.surface) {
        let n = e.name.to_lowercase();
        if !DELIVERY.contains(&n.as_str()) {
            r.ignored.push(format!("feeling: {n} (voices can only be told: {})", DELIVERY.join(", ")));
        }
    }
    if let Some(p) = lp.pace {
        put(&mut r, can_pace, format!("pace {p:.2}x"));
    }
    if !lp.pauses.is_empty() {
        r.used.push(format!("{} pause{}", lp.pauses.len(), if lp.pauses.len() == 1 { "" } else { "s" }));
    }
    if !lp.emphasis.is_empty() {
        put(&mut r, can_emphasis, format!("emphasis on {} part{}", lp.emphasis.len(), if lp.emphasis.len() == 1 { "" } else { "s" }));
    }
    for e in &lp.events {
        let kind = e.kind.to_lowercase();
        let ok = e.source == "provider-tag" && NON_VERBAL.contains(&kind.as_str()) && caps.tags.contains(&kind);
        put(&mut r, ok, format!("sound: {kind}"));
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{EmotionPart, Emphasis, Pause, VocalEvent};

    fn caps(tags: &[&str]) -> Capabilities {
        Capabilities { languages: vec!["en".into()], tags: tags.iter().map(|s| s.to_string()).collect(), voice_design: false, cloning: false, max_chars_per_request: 1000, streaming: false }
    }

    fn lp() -> LinePlan {
        LinePlan {
            emotion: vec![EmotionPart { name: "Sad".into(), intensity: 0.8, surface: true }, EmotionPart { name: "angry".into(), intensity: 0.3, surface: false }],
            pace: Some(0.9),
            pauses: vec![Pause { at: 5, ms: 400 }],
            emphasis: vec![Emphasis { start: 0, end: 4, level: 0.8 }],
            events: vec![VocalEvent { kind: "laugh".into(), at: 0, duration_ms: None, intensity: 0.5, source: "provider-tag".into() }],
            ..Default::default()
        }
    }

    #[test]
    fn the_plan_becomes_a_direction_with_only_what_a_voice_can_be_told() {
        let d = plan_direction(&lp());
        assert_eq!(d.emotion.as_deref(), Some("sad"));
        assert_eq!(d.pace, Some(0.9));
        let odd = LinePlan { emotion: vec![EmotionPart { name: "melancholic".into(), intensity: 1.0, surface: true }], ..Default::default() };
        assert_eq!(plan_direction(&odd).emotion, None);
        let hidden = LinePlan { emotion: vec![EmotionPart { name: "sad".into(), intensity: 1.0, surface: false }], ..Default::default() };
        assert_eq!(plan_direction(&hidden).emotion, None, "an emotion held back is not played on the surface");
    }

    #[test]
    fn pauses_and_supported_sounds_are_written_in_and_unsupported_ones_are_left_out() {
        let t = plan_text("Wait a moment please", &lp(), &caps(&["laugh", "sad"]));
        assert_eq!(t, "[laugh] Wait [pause:400] a moment please");
        let t2 = plan_text("Wait a moment please", &lp(), &caps(&[]));
        assert_eq!(t2, "Wait [pause:400] a moment please", "no laugh when the voice cannot laugh, and the line still renders");
    }

    #[test]
    fn nothing_is_inserted_inside_an_existing_tag_or_past_the_end() {
        let plan = LinePlan { pauses: vec![Pause { at: 3, ms: 300 }, Pause { at: 999, ms: 300 }], ..Default::default() };
        assert_eq!(plan_text("a [sigh] b", &plan, &caps(&[])), "a [sigh] b");
        let after = LinePlan { pauses: vec![Pause { at: 8, ms: 300 }], ..Default::default() };
        assert_eq!(plan_text("a [sigh] b", &after, &caps(&[])), "a [sigh] [pause:300] b");
    }

    #[test]
    fn a_plan_with_no_pauses_or_sounds_leaves_the_text_exactly_as_written() {
        let plan = LinePlan { pace: Some(1.2), ..Default::default() };
        let text = "  odd   spacing stays ";
        assert_eq!(plan_text(text, &plan, &caps(&[])), text);
    }

    #[test]
    fn the_report_says_what_each_voice_will_ignore() {
        let r = support("elevenlabs", &caps(&["sad", "laugh"]), &lp());
        assert!(r.used.iter().any(|u| u.starts_with("feeling: sad")));
        assert!(r.used.iter().any(|u| u == "sound: laugh"));
        assert!(r.ignored.iter().any(|u| u.starts_with("pace")), "ElevenLabs v3 has no pace control");
        assert!(r.ignored.iter().any(|u| u.starts_with("emphasis")));
        let none = support("windows-speech", &caps(&[]), &lp());
        assert!(none.used.iter().any(|u| u.starts_with("pace")), "Windows voices take a speaking rate");
        assert!(none.ignored.iter().any(|u| u == "feeling: sad") && none.ignored.iter().any(|u| u == "sound: laugh"));
    }
}
