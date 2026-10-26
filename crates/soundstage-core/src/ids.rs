//! Stable line identifiers (decision D-03).
//!
//! Lines used to be keyed by scene and position (`s0-l2`), so inserting a line renumbered every line
//! below it and takes, directions and stale marks attached to the wrong text. Each line now has an id
//! that survives edits, inserts, deletes and reorders. The ids live in an `IdTable` saved with the
//! project. Whenever the script changes, the new parse is matched to the table:
//!
//! 1. Exact speaker and text, in order, preferring the same scene. Lines removed earlier can come back
//!    (undo) by matching the orphan list.
//! 2. Same speaker and very similar text (an edited line), at least 0.85 similarity.
//! 3. Between two matched neighbours, equal numbers of unmatched old and new lines by the same
//!    speakers are paired in order (a line rewritten in place).
//! 4. Anything left gets a new id. Old lines left over are kept as orphans, never deleted, so their
//!    takes stay reachable.
//!
//! Existing projects are migrated by using their old positional keys as the first ids, so no take,
//! direction or note has to be rewritten.

use crate::script::{self, Parsed};
use crate::util::sha256_hex;
use serde::{Deserialize, Serialize};

const MAX_ORPHANS: usize = 500;
const SIMILARITY: f32 = 0.85;
/// How far from its old position an edited line is looked for. Long moves are matched exactly instead.
const SEARCH_WINDOW: usize = 200;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LineRecord {
    pub id: String,
    pub speaker: String,
    pub text: String,
    pub scene: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct IdTable {
    /// Next number for a new id. Only ever grows, so an id is never reused.
    pub seq: u64,
    /// Hash of the script this table was last matched to.
    pub script_hash: String,
    /// One record per line of that script, in order.
    pub lines: Vec<LineRecord>,
    /// Lines that left the script. Kept so their takes can be reached or restored.
    pub orphans: Vec<LineRecord>,
}

pub fn script_hash(script: &str) -> String {
    sha256_hex(script.as_bytes())
}

fn records(parsed: &Parsed, ids: &[String]) -> Vec<LineRecord> {
    parsed
        .lines
        .iter()
        .zip(ids)
        .map(|(l, id)| LineRecord {
            id: id.clone(),
            speaker: l.speaker.clone(),
            text: l.text.clone(),
            scene: parsed.scenes.get(l.scene).map(|s| s.title.clone()).unwrap_or_default(),
        })
        .collect()
}

fn bigrams(s: &str) -> Vec<(char, char)> {
    let c: Vec<char> = s.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
    c.windows(2).map(|w| (w[0], w[1])).collect()
}

/// Dice coefficient on character pairs: 1.0 identical, 0.0 nothing shared.
pub fn similarity(a: &str, b: &str) -> f32 {
    if a == b {
        return 1.0;
    }
    let (x, mut y) = (bigrams(a), bigrams(b));
    if x.is_empty() || y.is_empty() {
        return 0.0;
    }
    let total = x.len() + y.len();
    let mut shared = 0;
    for g in &x {
        if let Some(p) = y.iter().position(|h| h == g) {
            y.swap_remove(p);
            shared += 1;
        }
    }
    2.0 * shared as f32 / total as f32
}

/// Match a fresh parse to the table. Returns the new table and one id per parsed line.
/// Pure: the same table and script always give the same ids.
pub fn reconcile(table: &IdTable, script_text: &str, parsed: &Parsed) -> (IdTable, Vec<String>) {
    let hash = script_hash(script_text);
    let n = parsed.lines.len();

    // Fast path: nothing changed since the last match.
    if table.script_hash == hash && table.lines.len() == n && (n > 0 || !table.script_hash.is_empty()) {
        let ids = table.lines.iter().map(|r| r.id.clone()).collect();
        return (table.clone(), ids);
    }

    // First time for this project: the old positional keys become the ids.
    if table.script_hash.is_empty() && table.lines.is_empty() && table.orphans.is_empty() && table.seq == 0 {
        let ids: Vec<String> = parsed.lines.iter().map(|l| l.key.clone()).collect();
        let t = IdTable { seq: 0, script_hash: hash, lines: records(parsed, &ids), orphans: vec![] };
        return (t, ids);
    }

    let new_recs = records(parsed, &vec![String::new(); n]);
    let old = &table.lines;
    let mut assigned: Vec<Option<String>> = vec![None; n];
    let mut old_used = vec![false; old.len()];
    let mut orph_used = vec![false; table.orphans.len()];
    let mut matched_old: Vec<Option<usize>> = vec![None; n];

    // 1. Exact matches, in order: the first unused copy at or after the previous match.
    // An index by speaker and text keeps this fast on very long scripts.
    let mut by_text: std::collections::HashMap<(&str, &str), Vec<usize>> = std::collections::HashMap::new();
    for (j, r) in old.iter().enumerate() {
        by_text.entry((r.speaker.as_str(), r.text.as_str())).or_default().push(j);
    }
    let mut last_j = 0usize;
    for i in 0..n {
        let best = by_text
            .get(&(new_recs[i].speaker.as_str(), new_recs[i].text.as_str()))
            .into_iter()
            .flatten()
            .copied()
            .filter(|&j| !old_used[j])
            .min_by_key(|&j| (old[j].scene != new_recs[i].scene, j < last_j, j));
        if let Some(j) = best {
            old_used[j] = true;
            assigned[i] = Some(old[j].id.clone());
            matched_old[i] = Some(j);
            last_j = j;
        }
    }
    for i in 0..n {
        if assigned[i].is_some() {
            continue;
        }
        let best = (0..table.orphans.len()).find(|&k| {
            !orph_used[k] && table.orphans[k].speaker == new_recs[i].speaker && table.orphans[k].text == new_recs[i].text
        });
        if let Some(k) = best {
            orph_used[k] = true;
            assigned[i] = Some(table.orphans[k].id.clone());
        }
    }

    // 2. Edited lines: same speaker, similar text.
    for i in 0..n {
        if assigned[i].is_some() {
            continue;
        }
        let mut best: Option<(usize, f32)> = None;
        // Only lines near the old position are compared, which keeps this fast on very long scripts.
        for j in i.saturating_sub(SEARCH_WINDOW)..(i + SEARCH_WINDOW).min(old.len()) {
            if old_used[j] || old[j].speaker != new_recs[i].speaker {
                continue;
            }
            let s = similarity(&old[j].text, &new_recs[i].text);
            if s >= SIMILARITY && best.is_none_or(|(_, bs)| s > bs) {
                best = Some((j, s));
            }
        }
        if let Some((j, _)) = best {
            old_used[j] = true;
            assigned[i] = Some(old[j].id.clone());
            matched_old[i] = Some(j);
        }
    }

    // 3. Lines rewritten in place: pair the gaps between matched neighbours.
    let mut anchors: Vec<(usize, usize)> = vec![];
    for (i, m) in matched_old.iter().enumerate() {
        if let Some(j) = *m {
            if anchors.last().is_none_or(|&(_, lj)| j > lj) {
                anchors.push((i, j));
            }
        }
    }
    // Gaps are ((new_start, new_end), (old_start, old_end)), end exclusive.
    let mut gaps: Vec<((usize, usize), (usize, usize))> = vec![];
    let (mut pi, mut pj) = (0usize, 0usize);
    for &(i, j) in &anchors {
        gaps.push(((pi, i), (pj, j)));
        pi = i + 1;
        pj = j + 1;
    }
    gaps.push(((pi, n), (pj, old.len())));
    for ((ns, ne), (os, oe)) in gaps {
        let new_idx: Vec<usize> = (ns..ne).filter(|&i| assigned[i].is_none()).collect();
        let old_idx: Vec<usize> = (os..oe).filter(|&j| !old_used[j]).collect();
        if new_idx.len() == old_idx.len() && !new_idx.is_empty() && new_idx.iter().zip(&old_idx).all(|(&i, &j)| new_recs[i].speaker == old[j].speaker) {
            for (&i, &j) in new_idx.iter().zip(&old_idx) {
                old_used[j] = true;
                assigned[i] = Some(old[j].id.clone());
            }
        }
    }

    // 4. New ids for what is left; leftovers from the old table become orphans.
    let mut seq = table.seq;
    let ids: Vec<String> = assigned
        .into_iter()
        .map(|a| {
            a.unwrap_or_else(|| {
                seq += 1;
                format!("l{seq}")
            })
        })
        .collect();
    let mut orphans: Vec<LineRecord> = old.iter().enumerate().filter(|(j, _)| !old_used[*j]).map(|(_, r)| r.clone()).collect();
    orphans.extend(table.orphans.iter().enumerate().filter(|(k, _)| !orph_used[*k]).map(|(_, r)| r.clone()));
    // A restored id must not also stay in the orphan list.
    orphans.retain(|o| !ids.contains(&o.id));
    orphans.truncate(MAX_ORPHANS);
    (IdTable { seq, script_hash: hash, lines: records(parsed, &ids), orphans }, ids)
}

/// Parse a script and give every line its stable id in `key`.
pub fn parse_with(table: &IdTable, script_text: &str) -> (Parsed, IdTable) {
    let mut parsed = script::parse(script_text);
    let (t, ids) = reconcile(table, script_text, &parsed);
    for (l, id) in parsed.lines.iter_mut().zip(ids) {
        l.key = id;
    }
    (parsed, t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &str = "# One\nAMA: Good morning.\nKOLA: I know.\nAMA: Come on.\n";

    fn keys(table: &IdTable, text: &str) -> Vec<String> {
        parse_with(table, text).0.lines.iter().map(|l| l.key.clone()).collect()
    }

    fn seeded() -> IdTable {
        parse_with(&IdTable::default(), S).1
    }

    #[test]
    fn first_parse_uses_the_old_positional_keys() {
        assert_eq!(keys(&IdTable::default(), S), vec!["s0-l0", "s0-l1", "s0-l2"]);
    }

    #[test]
    fn inserting_a_line_keeps_the_other_ids() {
        let t = seeded();
        let edited = "# One\nAMA: Good morning.\nBOLA: Who is there?\nKOLA: I know.\nAMA: Come on.\n";
        let (p, t2) = parse_with(&t, edited);
        let k: Vec<_> = p.lines.iter().map(|l| l.key.as_str()).collect();
        assert_eq!(k[0], "s0-l0");
        assert_eq!(k[1], "l1");
        assert_eq!(k[2], "s0-l1");
        assert_eq!(k[3], "s0-l2");
        assert!(t2.orphans.is_empty());
    }

    #[test]
    fn ids_are_stable_across_repeated_parses() {
        let t = seeded();
        let edited = "# One\nBOLA: Hi.\nAMA: Good morning.\nKOLA: I know.\nAMA: Come on.\n";
        assert_eq!(keys(&t, edited), keys(&t, edited));
        let t2 = parse_with(&t, edited).1;
        assert_eq!(keys(&t2, edited), keys(&t, edited));
    }

    #[test]
    fn deleting_a_line_orphans_it_and_undo_restores_the_id() {
        let t = seeded();
        let without = "# One\nAMA: Good morning.\nAMA: Come on.\n";
        let (p, t2) = parse_with(&t, without);
        assert_eq!(p.lines[1].key, "s0-l2");
        assert_eq!(t2.orphans.len(), 1);
        assert_eq!(t2.orphans[0].id, "s0-l1");
        let (p3, t3) = parse_with(&t2, S);
        assert_eq!(p3.lines[1].key, "s0-l1");
        assert!(t3.orphans.is_empty());
    }

    #[test]
    fn editing_a_line_slightly_keeps_its_id() {
        let t = seeded();
        let edited = "# One\nAMA: Good morning!\nKOLA: I know, I know.\nAMA: Come on.\n";
        assert_eq!(keys(&t, edited), vec!["s0-l0", "s0-l1", "s0-l2"]);
    }

    #[test]
    fn rewriting_a_line_in_place_keeps_its_id() {
        let t = seeded();
        let edited = "# One\nAMA: Good morning.\nKOLA: Completely different words here now.\nAMA: Come on.\n";
        assert_eq!(keys(&t, edited), vec!["s0-l0", "s0-l1", "s0-l2"]);
    }

    #[test]
    fn reordering_keeps_ids_with_their_text() {
        let t = seeded();
        let edited = "# One\nAMA: Come on.\nKOLA: I know.\nAMA: Good morning.\n";
        assert_eq!(keys(&t, edited), vec!["s0-l2", "s0-l1", "s0-l0"]);
    }

    #[test]
    fn duplicate_lines_pair_in_order() {
        let t = parse_with(&IdTable::default(), "# A\nAMA: Yes.\nAMA: Yes.\n").1;
        let (p, _) = parse_with(&t, "# A\nBOLA: Hm.\nAMA: Yes.\nAMA: Yes.\n");
        assert_eq!(p.lines[1].key, "s0-l0");
        assert_eq!(p.lines[2].key, "s0-l1");
    }

    #[test]
    fn new_ids_are_never_reused() {
        let t = seeded();
        let (_, t2) = parse_with(&t, "# One\nBOLA: Who is there?\n");
        let (p, _) = parse_with(&t2, "# One\nBOLA: Who is there?\nZED: Another new line altogether.\n");
        assert_ne!(p.lines[1].key, p.lines[0].key);
        assert!(t2.seq >= 1);
    }

    #[test]
    fn a_very_long_script_is_matched_quickly() {
        let big: String = (0..30_000).map(|i| format!("AMA: This is line number {i} of a very long script.
")).collect();
        let t = parse_with(&IdTable::default(), &big).1;
        let edited = format!("BOLA: A brand new first line.
{big}").replace("line number 15000 ", "line number fifteen thousand ");
        let start = std::time::Instant::now();
        let (p, t2) = parse_with(&t, &edited);
        assert!(start.elapsed().as_secs() < 8, "took {:?}", start.elapsed());
        assert_eq!(p.lines.len(), 30_001);
        assert_eq!(p.lines[15_001].key, "s0-l15000", "the edited line keeps its id");
        assert!(t2.orphans.is_empty());
    }

    #[test]
    fn similarity_is_sensible() {
        assert_eq!(similarity("hello there", "hello there"), 1.0);
        assert!(similarity("Good morning.", "Good morning!") > 0.85);
        assert!(similarity("Good morning.", "Completely other") < 0.4);
        assert_eq!(similarity("", "x"), 0.0);
    }

    #[test]
    fn empty_script_gives_empty_table() {
        let (p, t) = parse_with(&IdTable::default(), "");
        assert!(p.lines.is_empty());
        assert!(t.lines.is_empty());
    }
}
