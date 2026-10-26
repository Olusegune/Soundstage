//! Listening scores and the report (`docs/spec/09-MS4-PLAN.md`, tasks T-4.5 and T-4.6).
//!
//! Reads the private key and the listeners' score sheets made by a listening pack, checks every row, and works out how
//! each version did against the pass levels in `docs/spec/07-LISTENING-TEST-PROTOCOL.md`. It never guesses: a bad row is
//! reported with its line and left out, a gate with too few listeners says "not enough listeners", and nothing here
//! changes the status of any voice. Only the owner does that.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_FILE_BYTES: usize = 1_000_000;
pub const MAX_SHEETS: usize = 24;

/* ------------------------------------------------------------------ CSV */

/// A small CSV reader: quotes, commas and line breaks inside quotes, a leading byte-order mark, CRLF.
pub fn parse_csv(text: &str) -> Vec<(usize, Vec<String>)> {
    let text = text.trim_start_matches('\u{feff}');
    let mut rows: Vec<(usize, Vec<String>)> = vec![];
    let (mut row, mut cell) = (Vec::<String>::new(), String::new());
    let (mut in_q, mut line, mut start) = (false, 1usize, 1usize);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, in_q) {
            ('"', true) if chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            ('"', _) => in_q = !in_q,
            (',', false) => row.push(std::mem::take(&mut cell)),
            ('\r', false) => {}
            ('\n', false) => {
                row.push(std::mem::take(&mut cell));
                if !(row.len() == 1 && row[0].trim().is_empty()) {
                    rows.push((start, std::mem::take(&mut row)));
                } else {
                    row.clear();
                }
                line += 1;
                start = line;
            }
            ('\n', true) => {
                cell.push('\n');
                line += 1;
            }
            (c, _) => cell.push(c),
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        if !(row.len() == 1 && row[0].trim().is_empty()) {
            rows.push((start, row));
        }
    }
    rows
}

/* ------------------------------------------------------------------ the key and the sheets */

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KeyRow {
    pub listener: String,
    pub scene: String,
    pub code: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Issue {
    pub file: String,
    pub line: usize,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ScoreRow {
    pub listener: String,
    pub scene: String,
    pub code: String,
    pub label: String,
    pub natural: Option<u8>,
    pub emotion: Option<u8>,
    /// The word the listener heard spoken that should have been a sound, if any.
    pub spoken_tag: Option<String>,
    pub same_voice: Option<bool>,
    pub understood: Option<bool>,
    pub rank: Option<u8>,
    pub comment: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ScoreTable {
    pub rows: Vec<ScoreRow>,
    pub issues: Vec<Issue>,
    pub listeners: Vec<String>,
}

fn col(header: &[String], name: &str) -> Option<usize> {
    header.iter().position(|h| h.trim().eq_ignore_ascii_case(name))
}

pub fn read_key(text: &str) -> Result<Vec<KeyRow>, String> {
    let rows = parse_csv(text);
    let Some((_, header)) = rows.first() else { return Err("The key file is empty.".into()) };
    let (Some(l), Some(s), Some(c), Some(lab)) = (col(header, "listener"), col(header, "scene"), col(header, "code"), col(header, "clip_label")) else {
        return Err("That is not a private key from a listening pack (it needs listener, scene, code and clip_label columns).".into());
    };
    let mut out: Vec<KeyRow> = vec![];
    for (n, r) in rows.iter().skip(1) {
        let get = |i: usize| r.get(i).map(|x| x.trim().trim_start_matches('\'').to_string()).unwrap_or_default();
        let k = KeyRow { listener: get(l), scene: get(s), code: get(c), label: get(lab) };
        if k.listener.is_empty() || k.code.is_empty() {
            return Err(format!("The key file has an incomplete row at line {n}."));
        }
        if out.iter().any(|o| o.listener == k.listener && o.code == k.code) {
            return Err(format!("The key file repeats the code {} for {} at line {n}.", k.code, k.listener));
        }
        out.push(k);
    }
    if out.is_empty() {
        return Err("The key file has no clips in it.".into());
    }
    Ok(out)
}

fn score(s: &str) -> Result<Option<u8>, String> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(None);
    }
    match t.parse::<u8>() {
        Ok(v) if (1..=5).contains(&v) => Ok(Some(v)),
        _ => Err(format!("\"{t}\" is not a score from 1 to 5.")),
    }
}

fn yes_no(s: &str) -> Result<Option<bool>, String> {
    match s.trim().to_lowercase().as_str() {
        "" => Ok(None),
        "yes" | "y" | "true" | "1" => Ok(Some(true)),
        "no" | "n" | "false" | "0" => Ok(Some(false)),
        other => Err(format!("\"{other}\" is not yes or no.")),
    }
}

fn clean_text(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(max).collect::<String>().trim().trim_start_matches('\'').to_string()
}

/// Read one listener's score sheet against the key. Bad rows go into `issues` and are left out.
pub fn read_sheet(file: &str, text: &str, key: &[KeyRow], table: &mut ScoreTable) {
    let rows = parse_csv(text);
    let Some((_, header)) = rows.first().cloned() else {
        table.issues.push(Issue { file: file.into(), line: 1, message: "The score sheet is empty.".into() });
        return;
    };
    let need = ["listener", "scene", "code", "natural_1to5", "emotion_1to5", "spoken_tag_word", "same_voice", "understood_all", "preference_rank"];
    let idx: Vec<Option<usize>> = need.iter().map(|n| col(&header, n)).collect();
    if idx.iter().any(Option::is_none) {
        table.issues.push(Issue { file: file.into(), line: 1, message: "That does not look like a score sheet from a listening pack (a column is missing).".into() });
        return;
    }
    let ix: Vec<usize> = idx.into_iter().flatten().collect();
    let comment_ix = col(&header, "comment");
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    let mut staged: Vec<ScoreRow> = vec![];
    let mut local_issues: Vec<Issue> = vec![];
    for (n, r) in rows.iter().skip(1) {
        let get = |i: usize| r.get(i).map(String::as_str).unwrap_or("");
        let (listener, scene, code) = (get(ix[0]).trim().to_string(), get(ix[1]).trim().trim_start_matches('\'').to_string(), get(ix[2]).trim().to_string());
        let mut err = |m: String| local_issues.push(Issue { file: file.to_string(), line: *n, message: m });
        let Some(k) = key.iter().find(|k| k.listener == listener && k.code == code) else {
            err(format!("{code} is not a code the key gives to {listener}."));
            continue;
        };
        if !k.scene.eq_ignore_ascii_case(&scene) {
            err(format!("{code} belongs to scene \"{}\", not \"{scene}\".", k.scene));
            continue;
        }
        if !seen.insert((listener.clone(), code.clone())) {
            err(format!("{code} is scored twice for {listener}."));
            continue;
        }
        let parsed = (|| -> Result<ScoreRow, String> {
            let tag = clean_text(get(ix[5]), 40);
            let spoken = if tag.is_empty() || ["no", "none", "n", "-"].contains(&tag.to_lowercase().as_str()) { None } else { Some(tag) };
            Ok(ScoreRow {
                listener: listener.clone(),
                scene: k.scene.clone(),
                code: code.clone(),
                label: k.label.clone(),
                natural: score(get(ix[3])).map_err(|e| format!("Natural: {e}"))?,
                emotion: score(get(ix[4])).map_err(|e| format!("Emotion: {e}"))?,
                spoken_tag: spoken,
                same_voice: yes_no(get(ix[6])).map_err(|e| format!("Same voice: {e}"))?,
                understood: yes_no(get(ix[7])).map_err(|e| format!("Understood: {e}"))?,
                rank: match get(ix[8]).trim() {
                    "" => None,
                    t => match t.parse::<u8>() {
                        Ok(v) if v >= 1 && (v as usize) <= key.iter().filter(|x| x.listener == listener && x.scene == k.scene).count() => Some(v),
                        _ => return Err(format!("Preference rank: \"{t}\" is not a rank from 1 to the number of clips in the scene.")),
                    },
                },
                comment: comment_ix.map(|i| clean_text(get(i), 300)).unwrap_or_default(),
            })
        })();
        match parsed {
            Ok(row) => staged.push(row),
            Err(m) => err(m),
        }
    }
    // Ranks must not repeat within one listener's scene (a tie is entered by leaving the rank blank).
    let mut ranks: BTreeMap<(String, String, u8), usize> = BTreeMap::new();
    for r in &staged {
        if let Some(v) = r.rank {
            *ranks.entry((r.listener.clone(), r.scene.clone(), v)).or_default() += 1;
        }
    }
    for r in &mut staged {
        if let Some(v) = r.rank {
            if ranks[&(r.listener.clone(), r.scene.clone(), v)] > 1 {
                local_issues.push(Issue { file: file.to_string(), line: 0, message: format!("{} gave rank {v} to more than one clip in {}. Those ranks were left out.", r.listener, r.scene) });
                r.rank = None;
            }
        }
    }
    table.issues.extend(local_issues);
    for r in staged {
        if !table.listeners.contains(&r.listener) {
            table.listeners.push(r.listener.clone());
        }
        table.rows.push(r);
    }
}

/// Read the key and every score sheet. `sheets` are `(file name, contents)`.
pub fn import(key_text: &str, sheets: &[(String, String)]) -> Result<ScoreTable, String> {
    if sheets.is_empty() || sheets.len() > MAX_SHEETS {
        return Err(format!("Choose between 1 and {MAX_SHEETS} score sheets."));
    }
    if key_text.len() > MAX_FILE_BYTES || sheets.iter().any(|(_, t)| t.len() > MAX_FILE_BYTES) {
        return Err("A file is too large to be a score sheet.".into());
    }
    let key = read_key(key_text)?;
    let mut table = ScoreTable { rows: vec![], issues: vec![], listeners: vec![] };
    for (name, text) in sheets {
        read_sheet(name, text, &key, &mut table);
    }
    // The same listener and code in two files: keep the first, say so.
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    let mut kept = vec![];
    for r in table.rows.drain(..) {
        if seen.insert((r.listener.clone(), r.code.clone())) {
            kept.push(r);
        } else {
            table.issues.push(Issue { file: String::new(), line: 0, message: format!("{} scored {} in more than one file. The first was kept.", r.listener, r.code) });
        }
    }
    table.rows = kept;
    table.listeners.sort();
    Ok(table)
}

/* ------------------------------------------------------------------ the report */

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Options {
    /// The version everything else is compared with for preference, for example `plain`.
    pub reference_label: Option<String>,
    /// Versions that contain sounds (laughs, sighs), which get the stricter sound checks.
    pub sound_labels: Vec<String>,
    /// The fewest listeners for a gate to count. Five by the protocol.
    pub min_listeners: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options { reference_label: None, sound_labels: vec![], min_listeners: 5 }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Met,
    NotMet,
    NotEnough,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Gate {
    pub name: String,
    pub verdict: Verdict,
    /// The number, in plain words, so the person sees why.
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LabelReport {
    pub label: String,
    pub listeners: usize,
    pub ratings: usize,
    pub natural_median: Option<f64>,
    pub emotion_median: Option<f64>,
    pub low_share: Option<f64>,
    pub spoken_tag_words: Vec<String>,
    pub same_voice_rate: Option<f64>,
    pub understood_rate: Option<f64>,
    pub preferred_or_tied: Option<f64>,
    pub comparisons: usize,
    pub gates: Vec<Gate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Report {
    pub listeners: usize,
    pub rows_used: usize,
    pub issues: Vec<Issue>,
    pub labels: Vec<LabelReport>,
    pub notes: Vec<String>,
}

pub fn median(v: &mut [f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let m = v.len() / 2;
    Some(if v.len() % 2 == 1 { v[m] } else { (v[m - 1] + v[m]) / 2.0 })
}

fn rate(yes: usize, all: usize) -> Option<f64> {
    (all > 0).then(|| yes as f64 / all as f64)
}

fn pct(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

pub fn report(table: &ScoreTable, opts: &Options) -> Report {
    let labels: BTreeSet<&String> = table.rows.iter().map(|r| &r.label).collect();
    let mut out = vec![];
    for label in labels {
        let rows: Vec<&ScoreRow> = table.rows.iter().filter(|r| &r.label == label).collect();
        let listeners: BTreeSet<&String> = rows.iter().map(|r| &r.listener).collect();
        let mut nat: Vec<f64> = rows.iter().filter_map(|r| r.natural.map(f64::from)).collect();
        let mut emo: Vec<f64> = rows.iter().filter_map(|r| r.emotion.map(f64::from)).collect();
        let low = rate(nat.iter().filter(|&&x| x < 3.0).count(), nat.len());
        let words: Vec<String> = rows.iter().filter_map(|r| r.spoken_tag.clone()).collect();
        let sv: Vec<bool> = rows.iter().filter_map(|r| r.same_voice).collect();
        let un: Vec<bool> = rows.iter().filter_map(|r| r.understood).collect();
        let same_voice = rate(sv.iter().filter(|&&b| b).count(), sv.len());
        let understood = rate(un.iter().filter(|&&b| b).count(), un.len());
        // Preference: in each listener's scene where this version and the reference were both ranked.
        let (mut wins, mut cmp) = (0usize, 0usize);
        let is_ref = opts.reference_label.as_deref() == Some(label.as_str());
        if let (Some(rf), false) = (&opts.reference_label, is_ref) {
            for r in &rows {
                let Some(mine) = r.rank else { continue };
                if let Some(theirs) = table.rows.iter().find(|x| x.listener == r.listener && x.scene == r.scene && &x.label == rf).and_then(|x| x.rank) {
                    cmp += 1;
                    if mine <= theirs {
                        wins += 1;
                    }
                }
            }
        }
        let pref = rate(wins, cmp);
        let enough = listeners.len() >= opts.min_listeners;
        let nm = median(&mut nat);
        let em = median(&mut emo);
        let mut gates = vec![];
        let mut gate = |name: &str, ok: Option<bool>, detail: String| {
            let verdict = match ok {
                None => Verdict::NotEnough,
                Some(_) if !enough => Verdict::NotEnough,
                Some(true) => Verdict::Met,
                Some(false) => Verdict::NotMet,
            };
            let detail = if enough || ok.is_none() { detail } else { format!("{detail} Only {} listener{}; at least {} are needed.", listeners.len(), if listeners.len() == 1 { "" } else { "s" }, opts.min_listeners) };
            gates.push(Gate { name: name.into(), verdict, detail });
        };
        gate("Natural: median at least 4, and under one clip in ten below 3", nm.zip(low).map(|(m, l)| m >= 4.0 && l <= 0.10), format!("Median {}, {} below 3.", nm.map_or("n/a".into(), |m| format!("{m:.1}")), low.map_or("n/a".into(), pct)));
        gate("Understood: at least 95% of clips fully understood", understood.map(|u| u >= 0.95), format!("{} understood.", understood.map_or("n/a".into(), pct)));
        if opts.sound_labels.contains(label) {
            gate("Sounds believable: median at least 3.5", nm.map(|m| m >= 3.5), format!("Median {}.", nm.map_or("n/a".into(), |m| format!("{m:.1}"))));
            gate("No spoken tag words: none heard", Some(words.is_empty()), if words.is_empty() { "None heard.".into() } else { format!("Heard: {}.", words.join(", ")) });
            gate("Same voice throughout: at least 90% of clips", same_voice.map(|s| s >= 0.9), format!("{} said yes.", same_voice.map_or("n/a".into(), pct)));
        }
        if opts.reference_label.is_some() && !is_ref {
            gate("Preferred or tied against the reference in at least half the comparisons", pref.map(|p| p >= 0.5), format!("{} of {} comparisons.", wins, cmp));
        }
        out.push(LabelReport { label: label.clone(), listeners: listeners.len(), ratings: rows.len(), natural_median: nm, emotion_median: em, low_share: low, spoken_tag_words: words, same_voice_rate: same_voice, understood_rate: understood, preferred_or_tied: pref, comparisons: cmp, gates });
    }
    let mut notes = vec!["A result belongs to the voices and settings used in this test only. It is not a claim about a language or an accent.".to_string()];
    if table.listeners.len() < opts.min_listeners {
        notes.push(format!("Only {} listener{} took part, fewer than the {} the protocol asks for, so no pass level can count as met.", table.listeners.len(), if table.listeners.len() == 1 { "" } else { "s" }, opts.min_listeners));
    }
    if !table.issues.is_empty() {
        notes.push(format!("{} problem{} found in the score sheets; those rows were left out.", table.issues.len(), if table.issues.len() == 1 { "" } else { "s" }));
    }
    Report { listeners: table.listeners.len(), rows_used: table.rows.len(), issues: table.issues.clone(), labels: out, notes }
}

fn n(x: Option<f64>) -> String {
    x.map_or(String::new(), |v| format!("{v:.2}"))
}

pub fn to_csv(r: &Report) -> String {
    use crate::listening::csv_cell;
    let mut s = String::from("version,listeners,ratings,natural_median,emotion_median,share_below_3,same_voice_rate,understood_rate,preferred_or_tied,comparisons,spoken_tag_words,gate,verdict,detail\n");
    for l in &r.labels {
        for g in &l.gates {
            s.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                csv_cell(&l.label), l.listeners, l.ratings, n(l.natural_median), n(l.emotion_median), n(l.low_share), n(l.same_voice_rate), n(l.understood_rate), n(l.preferred_or_tied), l.comparisons,
                csv_cell(&l.spoken_tag_words.join("; ")), csv_cell(&g.name), format!("{:?}", g.verdict).to_lowercase(), csv_cell(&g.detail)
            ));
        }
    }
    s
}

pub fn to_markdown(r: &Report, title: &str) -> String {
    let mut s = format!("# {}\n\n{} listener{}, {} scored clips.\n\n", title.chars().filter(|c| !c.is_control()).take(100).collect::<String>(), r.listeners, if r.listeners == 1 { "" } else { "s" }, r.rows_used);
    for l in &r.labels {
        s.push_str(&format!("## {}\n\n- Listeners: {}; ratings: {}\n- Natural median: {}; emotion median: {}\n- Below 3: {}; same voice: {}; understood: {}\n", l.label, l.listeners, l.ratings, n(l.natural_median), n(l.emotion_median), l.low_share.map_or("n/a".into(), pct), l.same_voice_rate.map_or("n/a".into(), pct), l.understood_rate.map_or("n/a".into(), pct)));
        if !l.spoken_tag_words.is_empty() {
            s.push_str(&format!("- Spoken tag words heard: {}\n", l.spoken_tag_words.join(", ")));
        }
        s.push('\n');
        for g in &l.gates {
            let mark = match g.verdict {
                Verdict::Met => "MET",
                Verdict::NotMet => "NOT MET",
                Verdict::NotEnough => "NOT ENOUGH LISTENERS",
            };
            s.push_str(&format!("- **{mark}**: {} ({})\n", g.name, g.detail));
        }
        s.push('\n');
    }
    for note in &r.notes {
        s.push_str(&format!("> {note}\n"));
    }
    if !r.issues.is_empty() {
        s.push_str("\n## Problems in the score sheets\n\n");
        for i in &r.issues {
            s.push_str(&format!("- {} line {}: {}\n", if i.file.is_empty() { "(files)" } else { &i.file }, i.line, i.message));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_for(listeners: usize) -> String {
        let mut k = String::from("listener,scene,play_order,code,clip_label,source_file,gain_db_applied,note\n");
        for l in 1..=listeners {
            for (i, (scene, label)) in [("Kitchen", "plain"), ("Kitchen", "planned"), ("Kitchen", "planned+sounds")].iter().enumerate() {
                k.push_str(&format!("L{l},{scene},{},S1-{}{l},{label},f.wav,0.0,\n", i + 1, ["A", "B", "C"][i]));
            }
        }
        k
    }

    /// One listener's sheet: scores for plain, planned, planned+sounds, in that order.
    fn sheet(l: usize, plain: [&str; 5], planned: [&str; 5], sounds: [&str; 5]) -> String {
        let mut s = String::from("listener,scene,code,natural_1to5,emotion_1to5,spoken_tag_word,same_voice,understood_all,preference_rank,comment\n");
        for (i, v) in [plain, planned, sounds].iter().enumerate() {
            s.push_str(&format!("L{l},Kitchen,S1-{}{l},{},{},{},{},{},{},fine\n", ["A", "B", "C"][i], v[0], v[1], v[2], v[3], v[4], [3, 1, 2][i]));
        }
        s
    }

    fn good(l: usize) -> String {
        sheet(l, ["3", "3", "", "yes", "yes"], ["4", "4", "", "yes", "yes"], ["4", "4", "", "yes", "yes"])
    }

    #[test]
    fn csv_handles_quotes_commas_line_breaks_and_a_byte_order_mark() {
        let rows = parse_csv("\u{feff}a,b\r\n\"x, y\",\"he said \"\"hi\"\"\"\r\n\"two\nlines\",z\r\n\r\n");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1].1, vec!["x, y", "he said \"hi\""]);
        assert_eq!(rows[2].1, vec!["two\nlines", "z"]);
        assert_eq!(rows[2].0, 3, "the line a row starts on");
    }

    #[test]
    fn six_good_sheets_give_a_report_and_the_gates_are_judged_per_version() {
        let sheets: Vec<(String, String)> = (1..=6).map(|l| (format!("L{l}.csv"), good(l))).collect();
        let t = import(&key_for(6), &sheets).unwrap();
        assert!(t.issues.is_empty(), "{:?}", t.issues);
        assert_eq!((t.rows.len(), t.listeners.len()), (18, 6));
        let r = report(&t, &Options { reference_label: Some("plain".into()), sound_labels: vec!["planned+sounds".into()], min_listeners: 5 });
        assert_eq!(r.labels.len(), 3);
        let by = |name: &str| r.labels.iter().find(|l| l.label == name).unwrap();
        let verdict = |l: &LabelReport, start: &str| l.gates.iter().find(|g| g.name.starts_with(start)).unwrap().verdict;
        assert_eq!(by("plain").natural_median, Some(3.0));
        assert_eq!(verdict(by("plain"), "Natural"), Verdict::NotMet, "median 3 is below 4");
        assert_eq!(verdict(by("planned"), "Natural"), Verdict::Met);
        assert_eq!(verdict(by("planned"), "Preferred"), Verdict::Met, "planned was ranked above plain every time");
        assert_eq!(by("planned").preferred_or_tied, Some(1.0));
        assert_eq!(verdict(by("planned+sounds"), "No spoken tag"), Verdict::Met);
        assert_eq!(verdict(by("planned+sounds"), "Same voice"), Verdict::Met);
        assert!(by("plain").gates.iter().all(|g| !g.name.starts_with("Preferred")), "the reference is not compared with itself");
    }

    #[test]
    fn too_few_listeners_can_never_meet_a_gate() {
        let sheets: Vec<(String, String)> = (1..=3).map(|l| (format!("L{l}.csv"), good(l))).collect();
        let t = import(&key_for(3), &sheets).unwrap();
        let r = report(&t, &Options::default());
        assert!(r.labels.iter().flat_map(|l| &l.gates).all(|g| g.verdict == Verdict::NotEnough), "{:?}", r.labels[1].gates);
        assert!(r.notes.iter().any(|n| n.contains("fewer than the 5")));
    }

    #[test]
    fn a_spoken_tag_word_fails_the_sound_checks_and_a_low_score_share_is_counted() {
        let mut sheets = vec![];
        for l in 1..=5 {
            let sounds = if l == 2 { ["2", "3", "laugh", "no", "yes"] } else { ["4", "4", "", "yes", "yes"] };
            sheets.push((format!("L{l}.csv"), sheet(l, ["3", "3", "", "yes", "yes"], ["4", "4", "", "yes", "yes"], sounds)));
        }
        let t = import(&key_for(5), &sheets).unwrap();
        let r = report(&t, &Options { reference_label: None, sound_labels: vec!["planned+sounds".into()], min_listeners: 5 });
        let s = r.labels.iter().find(|l| l.label == "planned+sounds").unwrap();
        assert_eq!(s.spoken_tag_words, vec!["laugh"]);
        assert!(s.gates.iter().any(|g| g.name.starts_with("No spoken") && g.verdict == Verdict::NotMet && g.detail.contains("laugh")));
        assert_eq!(s.low_share, Some(0.2));
        assert!(s.gates.iter().any(|g| g.name.starts_with("Natural") && g.verdict == Verdict::NotMet), "one in five below 3 is more than one in ten");
        assert!(s.gates.iter().any(|g| g.name.starts_with("Same voice") && g.verdict == Verdict::NotMet), "4 of 5 said same voice");
    }

    #[test]
    fn bad_rows_are_reported_with_their_line_and_left_out() {
        let hdr = "listener,scene,code,natural_1to5,emotion_1to5,spoken_tag_word,same_voice,understood_all,preference_rank,comment
";
        let s = format!("{hdr}L1,Kitchen,S1-A1,3,3,,yes,yes,,ok
L1,Kitchen,S1-A1,3,3,,yes,yes,,again
L1,Kitchen,S1-ZZ,3,3,,yes,yes,,x
L1,Market,S1-B1,3,3,,yes,yes,,x
L1,Kitchen,S1-B1,9,3,,yes,yes,,x
L1,Kitchen,S1-C1,3,3,,maybe,yes,,x
");
        let t = import(&key_for(1), &[("a.csv".into(), s)]).unwrap();
        assert_eq!(t.rows.len(), 1, "only the one good row is kept");
        let msgs: Vec<&str> = t.issues.iter().map(|i| i.message.as_str()).collect();
        assert!(msgs.iter().any(|m| m.contains("S1-ZZ is not a code")));
        assert!(msgs.iter().any(|m| m.contains("belongs to scene")));
        assert!(msgs.iter().any(|m| m.contains("scored twice")));
        assert!(msgs.iter().any(|m| m.contains("not a score from 1 to 5")));
        assert!(msgs.iter().any(|m| m.contains("not yes or no")));
        assert_eq!(t.issues.iter().find(|i| i.message.contains("S1-ZZ")).unwrap().line, 4);
    }

    #[test]
    fn a_listener_scored_in_two_files_is_counted_once_and_repeated_ranks_are_dropped() {
        let t = import(&key_for(1), &[("a.csv".into(), good(1)), ("b.csv".into(), good(1))]).unwrap();
        assert_eq!(t.rows.len(), 3);
        assert!(t.issues.iter().any(|i| i.message.contains("more than one file")));
        let mut s = good(1);
        s = s.replace(",,yes,yes,2,fine", ",,yes,yes,1,fine");
        let t2 = import(&key_for(1), &[("a.csv".into(), s)]).unwrap();
        assert!(t2.issues.iter().any(|i| i.message.contains("rank 1 to more than one clip")));
        assert!(t2.rows.iter().all(|r| r.rank != Some(1)));
    }

    #[test]
    fn wrong_files_are_refused_in_plain_words() {
        assert!(import("not,a,key\n1,2,3\n", &[("a.csv".into(), good(1))]).unwrap_err().contains("private key"));
        assert!(import(&key_for(1), &[]).is_err());
        let t = import(&key_for(1), &[("a.csv".into(), "a,b\n1,2\n".into())]).unwrap();
        assert!(t.rows.is_empty() && t.issues[0].message.contains("does not look like a score sheet"));
        let t2 = import(&key_for(1), &[("a.csv".into(), String::new())]).unwrap();
        assert!(t2.issues[0].message.contains("empty"));
        assert!(import(&key_for(1), &[("a.csv".into(), "x".repeat(MAX_FILE_BYTES + 1))]).is_err());
    }

    #[test]
    fn exports_are_readable_and_formula_cells_are_neutralised() {
        let mut s = good(1);
        s = s.replace(",fine", ",=HYPERLINK(\"http://x\")");
        let sheets = vec![("a.csv".to_string(), s)];
        let t = import(&key_for(1), &sheets).unwrap();
        assert!(t.rows[0].comment.starts_with("=HYPERLINK") || !t.rows[0].comment.starts_with('\''), "stored without the guard quote");
        let r = report(&t, &Options { reference_label: Some("plain".into()), sound_labels: vec!["planned+sounds".into()], min_listeners: 5 });
        let csv = to_csv(&r);
        assert!(csv.lines().next().unwrap().starts_with("version,listeners"));
        assert!(csv.contains("planned+sounds") && csv.contains("notenough"));
        let md = to_markdown(&r, "MS3a \u{7}listening");
        assert!(md.starts_with("# MS3a listening") && md.contains("NOT ENOUGH LISTENERS") && md.contains("not a claim about a language"));
        assert_eq!(median(&mut [1.0, 4.0, 2.0, 3.0]), Some(2.5));
        assert_eq!(median(&mut []), None);
    }
}
