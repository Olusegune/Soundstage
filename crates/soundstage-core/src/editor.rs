//! A small audio editor: trim, place and layer clips, set levels and fades, and render one file.
//!
//! Everything is done on copies in memory. The original files are never changed, and limits keep a mistake from using
//! all the computer's memory or producing an enormous file.

use crate::audio::{Stereo, SR};
use serde::Deserialize;

pub const MAX_CLIPS: usize = 24;
pub const MAX_OUTPUT_S: f32 = 30.0 * 60.0;
const PEAK_CEILING: f32 = 0.891; // -1 dBFS

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ClipParams {
    /// Where in the source to start, in seconds.
    pub start_s: f32,
    /// Where in the source to stop. `None` means the end.
    pub end_s: Option<f32>,
    /// Where on the timeline the clip begins.
    pub offset_s: f32,
    pub gain_db: f32,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
}

impl Default for ClipParams {
    fn default() -> Self {
        ClipParams { start_s: 0.0, end_s: None, offset_s: 0.0, gain_db: 0.0, fade_in_ms: 0, fade_out_ms: 0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EditResult {
    pub audio: Stereo,
    /// Level change applied at the end so the result does not clip (0 when none was needed).
    pub gain_applied_db: f32,
}

fn finite(x: f32) -> bool {
    x.is_finite()
}

/// Mix the clips into one stereo file. Clips overlap freely (that is layering), and are summed.
pub fn render(clips: &[(Stereo, ClipParams)], normalize: bool) -> Result<EditResult, String> {
    if clips.is_empty() {
        return Err("Add at least one clip.".into());
    }
    if clips.len() > MAX_CLIPS {
        return Err(format!("Use at most {MAX_CLIPS} clips at a time."));
    }
    // Work out every clip's range first, so nothing is allocated until the whole edit is known to be valid.
    struct Cut {
        from: usize,
        to: usize,
        at: usize,
        gain: f32,
        fi: usize,
        fo: usize,
    }
    let mut cuts = Vec::with_capacity(clips.len());
    let mut total = 0usize;
    for (i, (audio, p)) in clips.iter().enumerate() {
        let n = i + 1;
        if ![p.start_s, p.offset_s, p.gain_db].iter().all(|x| finite(*x)) || p.end_s.is_some_and(|e| !finite(e)) {
            return Err(format!("Clip {n} has a setting that is not a number."));
        }
        if p.start_s < 0.0 || p.offset_s < 0.0 {
            return Err(format!("Clip {n}: times cannot be negative."));
        }
        let len = audio.len();
        let from = ((p.start_s * SR as f32) as usize).min(len);
        let to = p.end_s.map_or(len, |e| ((e.max(0.0) * SR as f32) as usize).min(len));
        if to <= from {
            return Err(format!("Clip {n}: the end must come after the start."));
        }
        let at = (p.offset_s * SR as f32) as usize;
        let end = at.checked_add(to - from).ok_or("The edit is too long.")?;
        if end as f32 / SR as f32 > MAX_OUTPUT_S {
            return Err(format!("The result would be longer than {:.0} minutes.", MAX_OUTPUT_S / 60.0));
        }
        total = total.max(end);
        let gain = 10f32.powf(p.gain_db.clamp(-48.0, 24.0) / 20.0);
        let ms = |ms: u32| (ms.min(60_000) as usize * SR as usize) / 1000;
        cuts.push(Cut { from, to, at, gain, fi: ms(p.fade_in_ms), fo: ms(p.fade_out_ms) });
    }
    let mut out = Stereo { l: vec![0.0; total], r: vec![0.0; total] };
    for ((audio, _), c) in clips.iter().zip(&cuts) {
        let len = c.to - c.from;
        for k in 0..len {
            let mut g = c.gain;
            if k < c.fi {
                g *= k as f32 / c.fi as f32;
            }
            if c.fo > 0 && len - k <= c.fo {
                g *= (len - k) as f32 / c.fo as f32;
            }
            out.l[c.at + k] += audio.l[c.from + k] * g;
            out.r[c.at + k] += audio.r[c.from + k] * g;
        }
    }
    let peak = out.l.iter().chain(out.r.iter()).fold(0f32, |m, x| m.max(x.abs()));
    let mut gain_applied_db = 0.0;
    if peak > 0.0 && (normalize || peak > 0.999) {
        let g = PEAK_CEILING / peak;
        // Without normalising, only turn down when needed to avoid clipping; never turn quiet audio up.
        if normalize || g < 1.0 {
            for s in out.l.iter_mut().chain(out.r.iter_mut()) {
                *s *= g;
            }
            gain_applied_db = 20.0 * g.log10();
        }
    }
    Ok(EditResult { audio: out, gain_applied_db })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(secs: f32, amp: f32) -> Stereo {
        let n = (secs * SR as f32) as usize;
        let s: Vec<f32> = (0..n).map(|i| amp * (i as f32 * 0.05).sin()).collect();
        Stereo { l: s.clone(), r: s }
    }

    fn p(f: impl FnOnce(&mut ClipParams)) -> ClipParams {
        let mut c = ClipParams::default();
        f(&mut c);
        c
    }

    #[test]
    fn trimming_and_merging_place_clips_one_after_another() {
        let a = tone(2.0, 0.3);
        let b = tone(1.0, 0.3);
        let r = render(&[(a, p(|c| { c.start_s = 0.5; c.end_s = Some(1.5); })), (b, p(|c| c.offset_s = 1.0))], false).unwrap();
        assert_eq!(r.audio.len(), 2 * SR as usize, "a one second cut, then one second more");
        assert_eq!(r.gain_applied_db, 0.0);
    }

    #[test]
    fn layering_adds_clips_and_gain_and_fades_apply() {
        let a = tone(1.0, 0.2);
        let layered = render(&[(a.clone(), ClipParams::default()), (a.clone(), ClipParams::default())], false).unwrap();
        let single = render(&[(a.clone(), ClipParams::default())], false).unwrap();
        let k = 24_000;
        assert!((layered.audio.l[k] - 2.0 * single.audio.l[k]).abs() < 1e-5, "two layers add");
        let quiet = render(&[(a.clone(), p(|c| c.gain_db = -6.0))], false).unwrap();
        assert!((quiet.audio.l[k] / single.audio.l[k] - 0.501).abs() < 0.01);
        let faded = render(&[(a, p(|c| { c.fade_in_ms = 200; c.fade_out_ms = 200; }))], false).unwrap();
        assert_eq!(faded.audio.l[0], 0.0);
        assert!(faded.audio.l.last().unwrap().abs() < 0.001 && faded.audio.l[k].abs() > 0.0);
    }

    #[test]
    fn clipping_is_prevented_and_normalising_raises_quiet_audio() {
        let loud = render(&[(tone(1.0, 0.9), ClipParams::default()), (tone(1.0, 0.9), ClipParams::default())], false).unwrap();
        let peak = loud.audio.l.iter().fold(0f32, |m, x| m.max(x.abs()));
        assert!(peak <= 0.9 && loud.gain_applied_db < 0.0, "layers that would clip are turned down");
        let quiet = render(&[(tone(1.0, 0.05), ClipParams::default())], false).unwrap();
        assert_eq!(quiet.gain_applied_db, 0.0, "quiet audio is left alone");
        let up = render(&[(tone(1.0, 0.05), ClipParams::default())], true).unwrap();
        assert!(up.gain_applied_db > 20.0 && up.audio.l.iter().fold(0f32, |m, x| m.max(x.abs())) <= 0.9);
    }

    #[test]
    fn nonsense_and_oversized_edits_are_refused_before_any_work() {
        let a = tone(1.0, 0.3);
        assert!(render(&[], false).is_err());
        assert!(render(&vec![(a.clone(), ClipParams::default()); MAX_CLIPS + 1], false).is_err());
        assert!(render(&[(a.clone(), p(|c| c.start_s = f32::NAN))], false).is_err());
        assert!(render(&[(a.clone(), p(|c| c.start_s = -1.0))], false).is_err());
        assert!(render(&[(a.clone(), p(|c| { c.start_s = 0.8; c.end_s = Some(0.2); }))], false).is_err());
        assert!(render(&[(a.clone(), p(|c| c.start_s = 5.0))], false).is_err(), "starting after the end leaves nothing");
        assert!(render(&[(a.clone(), p(|c| c.offset_s = 4000.0))], false).unwrap_err().contains("longer than 30 minutes"));
        assert!(render(&[(a, p(|c| c.offset_s = f32::INFINITY))], false).is_err());
    }
}
