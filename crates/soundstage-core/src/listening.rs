//! Listening test packs (`docs/spec/07-LISTENING-TEST-PROTOCOL.md`, an early part of the Model Lab).
//!
//! Given a few audio clips grouped by scene, this makes one folder per listener with the clips renamed to random codes,
//! in a different random order for each listener, all matched to the same loudness, plus a score sheet to fill in. A
//! private key file says which code is which clip. Nothing in a listener's folder says what a clip is.
//!
//! Loudness is matched, never boosted into clipping: if a clip cannot reach the target without its peak passing the
//! ceiling it is left quieter, and that is written in the key so the results can be read fairly.

use crate::audio::{db_to_gain, to_mix_format, write_wav_24, Stereo};
use crate::decode::decode_file;
use crate::loudness::measure;
use crate::util::sha256_hex;
use std::path::{Path, PathBuf};

pub const MAX_CLIPS: usize = 30;
pub const MAX_LISTENERS: usize = 12;
pub const MAX_CLIP_SECONDS: usize = 120;
const PEAK_CEILING_DB: f64 = -1.0;
const CODE_CHARS: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";

#[derive(Clone, Debug)]
pub struct ClipIn {
    /// The scene or test item this clip belongs to. Clips in one group are compared with each other.
    pub group: String,
    /// What the clip is (for example `planned`). Only ever written to the private key.
    pub label: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct PackOptions {
    pub listeners: usize,
    pub target_lufs: f64,
    /// Makes the same pack again. `None` uses a fresh random seed.
    pub seed: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PackReport {
    pub folder: PathBuf,
    pub key_file: PathBuf,
    pub listeners: usize,
    pub groups: usize,
    pub clips: usize,
    pub files_written: usize,
    /// Plain-words notes, for example a clip that could not reach the loudness target.
    pub notes: Vec<String>,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            v.swap(i, self.below(i + 1));
        }
    }
    fn code(&mut self) -> String {
        (0..2).map(|_| CODE_CHARS[self.below(CODE_CHARS.len())] as char).collect()
    }
}

/// A cell that a spreadsheet will treat as text, never as a formula.
pub(crate) fn csv_cell(s: &str) -> String {
    let mut t: String = s.chars().filter(|c| !c.is_control()).collect();
    if t.starts_with(['=', '+', '-', '@']) {
        t.insert(0, '\'');
    }
    if t.contains([',', '"']) {
        format!("\"{}\"", t.replace('"', "\"\""))
    } else {
        t
    }
}

struct Prepared {
    group: usize,
    label: String,
    source: String,
    audio: Stereo,
    gain_db: f64,
    note: Option<String>,
}

fn prepare(c: &ClipIn, group: usize, target: f64) -> Result<Prepared, String> {
    let name = c.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let d = decode_file(&c.path).map_err(|e| format!("{name}: {e}"))?;
    let audio = to_mix_format(&d);
    if audio.is_empty() {
        return Err(format!("{name} has no audio."));
    }
    if audio.len() > MAX_CLIP_SECONDS * crate::audio::SR as usize {
        return Err(format!("{name} is longer than {MAX_CLIP_SECONDS} seconds. Listening clips should be short."));
    }
    let m = measure(&audio);
    let Some(integrated) = m.integrated_lufs else { return Err(format!("{name} is silent or nearly silent.")) };
    let mut gain = target - integrated;
    let mut note = None;
    // Do not let the loudest moment pass the ceiling: keep the peak at or below it and say so.
    let room = PEAK_CEILING_DB - m.true_peak_dbtp;
    if gain > room {
        note = Some(format!("{name} could not be raised all the way to {target:.0} LUFS without clipping, so it is {:.1} LU quieter than the others.", gain - room));
        gain = room;
    }
    let mut out = audio;
    out.scale(db_to_gain(gain as f32));
    Ok(Prepared { group, label: c.label.clone(), source: name, audio: out, gain_db: gain, note })
}

pub fn make_pack(clips: &[ClipIn], dest_dir: &Path, opts: &PackOptions) -> Result<PackReport, String> {
    if opts.listeners == 0 || opts.listeners > MAX_LISTENERS {
        return Err(format!("Choose between 1 and {MAX_LISTENERS} listeners."));
    }
    if !(-30.0..=-10.0).contains(&opts.target_lufs) {
        return Err("The loudness target must be between -30 and -10 LUFS.".into());
    }
    if clips.is_empty() || clips.len() > MAX_CLIPS {
        return Err(format!("Use between 2 and {MAX_CLIPS} clips."));
    }
    let mut groups: Vec<String> = vec![];
    for c in clips {
        let g = c.group.trim();
        if g.is_empty() || g.chars().count() > 60 {
            return Err("Every clip needs a scene name (up to 60 characters).".into());
        }
        if !groups.iter().any(|x| x == g) {
            groups.push(g.to_string());
        }
        if c.label.trim().is_empty() || c.label.chars().count() > 80 {
            return Err("Every clip needs a short label for the private key.".into());
        }
    }
    for g in &groups {
        if clips.iter().filter(|c| c.group.trim() == g).count() < 2 {
            return Err(format!("Scene \"{g}\" needs at least two clips to compare."));
        }
    }
    if !dest_dir.is_dir() {
        return Err("That folder does not exist.".into());
    }

    let mut prepared = Vec::with_capacity(clips.len());
    for c in clips {
        let gi = groups.iter().position(|x| x == c.group.trim()).unwrap_or(0);
        prepared.push(prepare(c, gi, opts.target_lufs)?);
    }

    // A new folder every time, so an earlier pack is never overwritten.
    let stamp = crate::util::iso_now().replace([':', 'T', 'Z'], "").replace('-', "");
    let mut folder = dest_dir.join(format!("listening-pack-{}", stamp.chars().take(14).collect::<String>()));
    let mut n = 1;
    while folder.exists() {
        n += 1;
        folder = dest_dir.join(format!("listening-pack-{}-{n}", stamp.chars().take(14).collect::<String>()));
    }
    std::fs::create_dir_all(&folder).map_err(|e| format!("Could not create the folder: {e}"))?;

    let seed = opts.seed.unwrap_or_else(|| {
        let h = sha256_hex(format!("{}{}{}", crate::util::new_id("lp"), std::process::id(), clips.len()).as_bytes());
        u64::from_str_radix(&h[..16], 16).unwrap_or(0x9E37_79B9_7F4A_7C15)
    });
    let mut rng = Rng::new(seed);

    let mut key = String::from("listener,scene,play_order,code,clip_label,source_file,gain_db_applied,note\n");
    let mut files = 0usize;
    let mut notes: Vec<String> = prepared.iter().filter_map(|p| p.note.clone()).collect();
    for li in 1..=opts.listeners {
        let ldir = folder.join(format!("listener-L{li}"));
        std::fs::create_dir_all(&ldir).map_err(|e| format!("Could not create the folder: {e}"))?;
        let mut sheet = String::from("listener,scene,code,natural_1to5,emotion_1to5,spoken_tag_word,same_voice,understood_all,preference_rank,comment\n");
        let mut used: Vec<String> = vec![];
        for (gi, g) in groups.iter().enumerate() {
            let mut order: Vec<usize> = (0..prepared.len()).filter(|&i| prepared[i].group == gi).collect();
            rng.shuffle(&mut order);
            for (pos, &i) in order.iter().enumerate() {
                let code = loop {
                    let c = format!("S{}-{}", gi + 1, rng.code());
                    if !used.contains(&c) {
                        used.push(c.clone());
                        break c;
                    }
                };
                write_wav_24(&ldir.join(format!("{code}.wav")), &prepared[i].audio)?;
                files += 1;
                sheet.push_str(&format!("L{li},{},{code},,,,,,,\n", csv_cell(g)));
                let p = &prepared[i];
                key.push_str(&format!("L{li},{},{},{code},{},{},{:.2},{}\n", csv_cell(g), pos + 1, csv_cell(&p.label), csv_cell(&p.source), p.gain_db, csv_cell(p.note.as_deref().unwrap_or(""))));
            }
        }
        std::fs::write(ldir.join("score-sheet.csv"), sheet).map_err(|e| format!("Could not write the score sheet: {e}"))?;
        std::fs::write(ldir.join("READ-ME-FIRST.txt"), LISTENER_README).map_err(|e| e.to_string())?;
    }
    let key_file = folder.join("PRIVATE-KEY-do-not-share.csv");
    std::fs::write(&key_file, key).map_err(|e| format!("Could not write the key: {e}"))?;
    std::fs::write(folder.join("OWNER-README.txt"), OWNER_README).map_err(|e| e.to_string())?;
    notes.sort();
    notes.dedup();
    Ok(PackReport { folder, key_file, listeners: opts.listeners, groups: groups.len(), clips: clips.len(), files_written: files, notes })
}

const LISTENER_README: &str = "Thank you for listening.\n\nYou will hear short scenes. Each scene has several clips with a code such as S1-4F.\nPlay every clip in a scene once without scoring, then play them again and fill in score-sheet.csv, one row per clip.\n\nnatural_1to5: 1 = unusable, 3 = fine for a rough draft, 5 = could be a good human recording.\nemotion_1to5: how well the feeling fits the words, same scale.\nspoken_tag_word: write the word if you heard something read out that should be a sound or an instruction (for example the word \"laugh\"), otherwise leave empty.\nsame_voice: yes or no, did the voice stay the same person all the way through.\nunderstood_all: yes or no.\npreference_rank: after each scene, 1 for the clip you would rather listen to for a whole episode, then 2, 3.\n\nThere are no right answers. Please do not look for or ask what the versions are until everyone has finished.\n";

const OWNER_README: &str = "Do not give listeners anything in this folder except their own listener-L# folder.\nPRIVATE-KEY-do-not-share.csv says which code is which clip and how much each was adjusted. Keep it until you have collected all score sheets, then use it to turn codes back into versions.\nEach listener has a different random order and different codes. Clips were matched to the same loudness. A note in the key says if any clip could not reach it.\nCombine the score sheets (one row per clip) and join them to the key on listener and code.\n";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{write_wav_24 as w24, Stereo};
    use crate::util::new_id;

    fn tone(dir: &Path, name: &str, amp: f32, secs: f32) -> PathBuf {
        let n = (secs * 48_000.0) as usize;
        let m: Vec<f32> = (0..n).map(|i| amp * (2.0 * std::f32::consts::PI * 300.0 * i as f32 / 48_000.0).sin() * (1.0 + 0.3 * (i as f32 / 4000.0).sin())).collect();
        let p = dir.join(name);
        w24(&p, &Stereo::from_mono(&m)).unwrap();
        p
    }

    fn work() -> PathBuf {
        let d = std::env::temp_dir().join(new_id("ss-listen"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn clips(d: &Path) -> Vec<ClipIn> {
        let mk = |g: &str, l: &str, f: &str, a: f32, s: f32| ClipIn { group: g.into(), label: l.into(), path: tone(d, f, a, s) };
        vec![
            mk("Kitchen", "plain", "a1.wav", 0.05, 3.0), mk("Kitchen", "planned", "a2.wav", 0.2, 3.5), mk("Kitchen", "planned+sounds", "a3.wav", 0.4, 4.0),
            mk("Market", "plain", "b1.wav", 0.1, 3.0), mk("Market", "planned", "b2.wav", 0.02, 3.2), mk("Market", "planned+sounds", "b3.wav", 0.3, 3.4),
        ]
    }

    fn read_key(report: &PackReport) -> Vec<Vec<String>> {
        std::fs::read_to_string(&report.key_file).unwrap().lines().skip(1).map(|l| l.split(',').map(String::from).collect()).collect()
    }

    #[test]
    fn every_clip_reaches_the_same_loudness_and_listeners_get_different_codes_and_orders() {
        let d = work();
        let out = d.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let r = make_pack(&clips(&d), &out, &PackOptions { listeners: 4, target_lufs: -23.0, seed: Some(7) }).unwrap();
        assert_eq!((r.listeners, r.groups, r.clips, r.files_written), (4, 2, 6, 24));
        let key = read_key(&r);
        assert_eq!(key.len(), 24);
        for row in &key {
            let wav = r.folder.join(format!("listener-{}", row[0])).join(format!("{}.wav", row[3]));
            let lufs = measure(&to_mix_format(&decode_file(&wav).unwrap())).integrated_lufs.unwrap();
            assert!((lufs + 23.0).abs() < 0.3 || !row[7].is_empty(), "{} is {lufs} LUFS", row[3]);
        }
        let codes_of = |l: &str| -> Vec<String> { key.iter().filter(|r| r[0] == l).map(|r| r[3].clone()).collect() };
        let (a, b) = (codes_of("L1"), codes_of("L2"));
        assert_ne!(a, b, "different codes for each listener");
        let order_of = |l: &str| -> Vec<String> { key.iter().filter(|r| r[0] == l && r[1] == "Kitchen").map(|r| r[4].clone()).collect() };
        assert!((1..=4).any(|i| order_of(&format!("L{i}")) != order_of("L1")), "at least one listener has a different order");
        // Codes are unique within a listener and never contain a label.
        for l in ["L1", "L2", "L3", "L4"] {
            let c = codes_of(l);
            let mut u = c.clone();
            u.sort();
            u.dedup();
            assert_eq!(u.len(), c.len());
        }
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn nothing_a_listener_sees_says_what_a_clip_is() {
        let d = work();
        let out = d.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let r = make_pack(&clips(&d), &out, &PackOptions { listeners: 2, target_lufs: -23.0, seed: Some(1) }).unwrap();
        for entry in std::fs::read_dir(r.folder.join("listener-L1")).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string().to_lowercase();
            for secret in ["plain", "planned", "sounds", "kitchen", "market", "a1", "b3"] {
                assert!(!name.contains(secret), "{name} gives away {secret}");
            }
        }
        let sheet = std::fs::read_to_string(r.folder.join("listener-L1").join("score-sheet.csv")).unwrap();
        assert!(!sheet.to_lowercase().contains("planned") && !sheet.to_lowercase().contains("plain"));
        assert!(sheet.lines().count() == 7, "a header and one row per clip");
        assert!(std::fs::read_to_string(&r.key_file).unwrap().contains("planned+sounds"));
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn the_same_seed_makes_the_same_pack_and_a_new_one_never_overwrites() {
        let d = work();
        let out = d.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let c = clips(&d);
        let o = PackOptions { listeners: 2, target_lufs: -23.0, seed: Some(99) };
        let a = make_pack(&c, &out, &o).unwrap();
        let b = make_pack(&c, &out, &o).unwrap();
        assert_ne!(a.folder, b.folder);
        assert_eq!(std::fs::read_to_string(&a.key_file).unwrap(), std::fs::read_to_string(&b.key_file).unwrap());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn a_clip_that_cannot_reach_the_target_without_clipping_is_left_quieter_and_noted() {
        let d = work();
        let out = d.join("out");
        std::fs::create_dir_all(&out).unwrap();
        // Very peaky: a short loud click on near silence has low loudness but a high peak.
        let n = 48_000 * 3;
        let mut m = vec![0.0005f32; n];
        m[n / 2] = 0.95;
        let p = d.join("click.wav");
        w24(&p, &Stereo::from_mono(&m)).unwrap();
        let mut c = clips(&d);
        c.push(ClipIn { group: "Kitchen".into(), label: "peaky".into(), path: p });
        let r = make_pack(&c, &out, &PackOptions { listeners: 1, target_lufs: -14.0, seed: Some(3) }).unwrap();
        assert!(r.notes.iter().any(|n| n.contains("clipping")), "{:?}", r.notes);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn bad_input_is_refused_in_plain_words() {
        let d = work();
        let out = d.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let o = PackOptions { listeners: 2, target_lufs: -23.0, seed: Some(1) };
        let c = clips(&d);
        assert!(make_pack(&c, &out, &PackOptions { listeners: 0, ..o.clone() }).is_err());
        assert!(make_pack(&c, &out, &PackOptions { listeners: 13, ..o.clone() }).is_err());
        assert!(make_pack(&c, &out, &PackOptions { target_lufs: 0.0, ..o.clone() }).is_err());
        assert!(make_pack(&[], &out, &o).is_err());
        assert!(make_pack(&c[..1], &out, &o).unwrap_err().contains("at least two"));
        assert!(make_pack(&c, &d.join("nope"), &o).is_err());
        let mut missing = c.clone();
        missing[0].path = d.join("missing.wav");
        assert!(make_pack(&missing, &out, &o).is_err());
        let mut unnamed = c.clone();
        unnamed[0].group = "  ".into();
        assert!(make_pack(&unnamed, &out, &o).is_err());
        let silent = d.join("silent.wav");
        w24(&silent, &Stereo::zeros(48_000 * 2)).unwrap();
        let mut s = c.clone();
        s[0].path = silent;
        assert!(make_pack(&s, &out, &o).unwrap_err().contains("silent"));
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn labels_that_look_like_spreadsheet_formulas_are_written_as_text() {
        assert_eq!(csv_cell("=HYPERLINK(\"x\")"), "\"'=HYPERLINK(\"\"x\"\")\"");
        assert_eq!(csv_cell("plain"), "plain");
        assert_eq!(csv_cell("a,b"), "\"a,b\"");
        assert_eq!(csv_cell("@x"), "'@x");
    }
}
