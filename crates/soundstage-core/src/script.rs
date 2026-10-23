//! Plain-text cue format: `# Scene title` or `INT./EXT.` headings start scenes,
//! `NAME: line` creates a speaker line. The source text stays the single source
//! of truth, so the format round-trips without data loss.

use crate::tags::{tokenize, TagIssue};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ParsedScene {
    pub index: usize,
    pub title: String,
    pub location: String,
    pub time_of_day: String,
    pub cast: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ParsedLine {
    /// Stable within a script as long as scene and ordinal do not change: `s0-l2`.
    pub key: String,
    pub scene: usize,
    pub speaker: String,
    pub text: String,
    pub row: usize,
    pub tag_issues: Vec<TagIssue>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ScriptIssue {
    pub row: usize,
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Parsed {
    pub scenes: Vec<ParsedScene>,
    pub speakers: Vec<String>,
    pub lines: Vec<ParsedLine>,
    pub issues: Vec<ScriptIssue>,
}

pub const MAX_SCRIPT_BYTES: usize = 2_000_000;

fn speaker_split(line: &str) -> Option<(String, &str)> {
    let (name, rest) = line.split_once(':')?;
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 40 {
        return None;
    }
    let ok = name.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '\'' | '-' | '.' | '_'));
    if !ok || !name.chars().next().is_some_and(|c| c.is_alphabetic()) {
        return None;
    }
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    Some((name.to_uppercase(), rest))
}

fn heading(line: &str) -> Option<(String, String, String)> {
    let t = line.trim();
    if let Some(title) = t.strip_prefix('#') {
        let title = title.trim_start_matches('#').trim();
        return Some((title.to_string(), String::new(), String::new()));
    }
    let up = t.to_uppercase();
    for p in ["INT./EXT.", "INT/EXT", "INT.", "EXT.", "INT ", "EXT "] {
        if up.starts_with(p) {
            let rest = t[p.len()..].trim();
            let (loc, time) = match rest.rsplit_once(" - ") {
                Some((l, tm)) => (l.trim().to_string(), tm.trim().to_string()),
                None => (rest.to_string(), String::new()),
            };
            return Some((t.to_string(), loc, time));
        }
    }
    None
}

pub fn parse(source: &str) -> Parsed {
    let mut out = Parsed::default();
    if source.len() > MAX_SCRIPT_BYTES {
        out.issues.push(ScriptIssue { row: 0, message: "The script is too large (over 2 MB).".into() });
        return out;
    }
    let mut ordinal = 0usize;
    for (i, raw) in source.lines().enumerate() {
        let row = i + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((title, location, time_of_day)) = heading(line) {
            let index = out.scenes.len();
            out.scenes.push(ParsedScene { index, title, location, time_of_day, cast: vec![] });
            ordinal = 0;
            continue;
        }
        let Some((speaker, text)) = speaker_split(line) else {
            out.issues.push(ScriptIssue {
                row,
                message: "This line has no speaker. Start it with NAME: so Soundstage knows who says it.".into(),
            });
            continue;
        };
        if out.scenes.is_empty() {
            out.scenes.push(ParsedScene {
                index: 0,
                title: "Scene 1".into(),
                location: String::new(),
                time_of_day: String::new(),
                cast: vec![],
            });
        }
        let scene = out.scenes.len() - 1;
        let (_, tag_issues) = tokenize(text);
        if !out.speakers.contains(&speaker) {
            out.speakers.push(speaker.clone());
        }
        if !out.scenes[scene].cast.contains(&speaker) {
            out.scenes[scene].cast.push(speaker.clone());
        }
        out.lines.push(ParsedLine {
            key: format!("s{scene}-l{ordinal}"),
            scene,
            speaker,
            text: text.to_string(),
            row,
            tag_issues,
        });
        ordinal += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Kitchen\nAMA: Good morning. [pause:short] You're late!\nKOLA: [sigh] I know.\n\nINT. MARKET - NIGHT\nAMA: Come on.\nstray line\n";

    #[test]
    fn parses_scenes_speakers_and_lines() {
        let p = parse(SAMPLE);
        assert_eq!(p.scenes.len(), 2);
        assert_eq!(p.scenes[1].location, "MARKET");
        assert_eq!(p.scenes[1].time_of_day, "NIGHT");
        assert_eq!(p.speakers, vec!["AMA", "KOLA"]);
        assert_eq!(p.lines.len(), 3);
        assert_eq!(p.lines[2].key, "s1-l0");
        assert_eq!(p.issues.len(), 1);
        assert_eq!(p.issues[0].row, 7);
    }

    #[test]
    fn lines_before_any_heading_get_a_default_scene() {
        let p = parse("AMA: hi");
        assert_eq!(p.scenes[0].title, "Scene 1");
    }

    #[test]
    fn editing_one_line_leaves_other_keys_and_text_alone() {
        let a = parse(SAMPLE);
        let b = parse(&SAMPLE.replace("I know.", "I know, I know."));
        for (x, y) in a.lines.iter().zip(&b.lines) {
            assert_eq!(x.key, y.key);
            assert_eq!(x.text == y.text, x.key != "s0-l1");
        }
    }

    #[test]
    fn flags_unknown_tags_on_the_line() {
        let p = parse("AMA: hello [laughs]");
        assert_eq!(p.lines[0].tag_issues.len(), 1);
    }

    #[test]
    fn oversized_scripts_are_refused() {
        let big = "A: x\n".repeat(MAX_SCRIPT_BYTES / 4);
        assert_eq!(parse(&big).lines.len(), 0);
    }
}
