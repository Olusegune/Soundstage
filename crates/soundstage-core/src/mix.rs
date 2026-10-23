//! Dialogue, effects and music buses; ducking under dialogue; loudness to a target.
//! The same gain is applied to every bus, so the stems sum to the master.
//! A mix that cannot reach its target without exceeding the true-peak ceiling is
//! flagged, never silently limited or clipped.

use crate::audio::{db_to_gain, Stereo, SR};
use crate::loudness::{measure, true_peak_db, Loudness};

#[derive(Clone, Debug)]
pub struct Placed {
    pub start: usize,
    pub buf: Stereo,
    pub gain_db: f32,
    pub fade_in: usize,
    pub fade_out: usize,
    /// Whether this item ducks under dialogue. Source music in a scene opts out.
    pub duck: bool,
}

#[derive(Clone, Debug)]
pub struct MixParams {
    pub duck_depth_db: f32,
    pub attack_ms: u32,
    pub release_ms: u32,
    pub target_lufs: f64,
    pub true_peak_ceiling_dbtp: f64,
}

#[derive(Clone, Debug)]
pub struct MixOutput {
    pub dialogue: Stereo,
    pub effects: Stereo,
    pub music: Stereo,
    pub master: Stereo,
    pub loudness: Loudness,
    pub gain_db: f64,
    pub target_reached: bool,
    pub warnings: Vec<String>,
}

fn faded(p: &Placed) -> Stereo {
    let mut b = p.buf.clone();
    let n = b.len();
    for i in 0..n {
        let mut g = 1.0f32;
        if p.fade_in > 0 && i < p.fade_in {
            g = g.min(i as f32 / p.fade_in as f32);
        }
        if p.fade_out > 0 && n - i <= p.fade_out {
            g = g.min((n - i) as f32 / p.fade_out as f32);
        }
        b.l[i] *= g;
        b.r[i] *= g;
    }
    b
}

/// Per-sample ducking gain from the dialogue bus. Linear ramps reach the full depth
/// in exactly `attack_ms` and recover in `release_ms`. A short hold stops pumping between words.
pub fn duck_envelope(dialogue: &Stereo, len: usize, depth_db: f32, attack_ms: u32, release_ms: u32) -> Vec<f32> {
    let depth = db_to_gain(-depth_db.abs());
    let hop = (SR as usize / 100).max(1); // 10 ms
    let hold_hops = 25;
    let thresh = db_to_gain(-50.0);
    let mut active_until = 0usize;
    let mut env = vec![1.0f32; len];
    let down = (1.0 - depth) / ((attack_ms.max(1) as f32 / 1000.0) * SR as f32);
    let up = (1.0 - depth) / ((release_ms.max(1) as f32 / 1000.0) * SR as f32);
    let mut g = 1.0f32;
    for (h, start) in (0..len).step_by(hop).enumerate() {
        let end = (start + hop).min(len).min(dialogue.len());
        let rms = if start < end {
            let s: f32 = (start..end).map(|i| 0.5 * (dialogue.l[i] * dialogue.l[i] + dialogue.r[i] * dialogue.r[i])).sum();
            (s / (end - start) as f32).sqrt()
        } else {
            0.0
        };
        if rms > thresh {
            active_until = h + hold_hops;
        }
        let target = if h <= active_until && active_until > 0 { depth } else { 1.0 };
        for i in start..(start + hop).min(len) {
            if target < g {
                g = (g - down).max(target);
            } else {
                g = (g + up).min(target);
            }
            env[i] = g;
        }
    }
    env
}

fn bus(items: &[Placed], len: usize, env: Option<&[f32]>) -> Stereo {
    let mut out = Stereo::zeros(len);
    for p in items {
        let b = faded(p);
        out.add(&b, p.start, db_to_gain(p.gain_db), if p.duck { env } else { None });
    }
    out
}

pub fn mix(dialogue: &[Placed], effects: &[Placed], music: &[Placed], p: &MixParams, tail: usize) -> MixOutput {
    let end = dialogue.iter().chain(effects).chain(music).map(|x| x.start + x.buf.len()).max().unwrap_or(0);
    let len = end + tail;
    let d = bus(dialogue, len, None);
    let env = duck_envelope(&d, len, p.duck_depth_db, p.attack_ms, p.release_ms);
    let mut e = bus(effects, len, Some(&env));
    let mut m = bus(music, len, Some(&env));
    let mut d = d;
    let mut warnings = Vec::new();

    let pre = Stereo::sum(&d, &e, &m);
    let before = measure(&pre);
    let mut gain_db = 0.0f64;
    let mut reached = true;
    match before.integrated_lufs {
        None => warnings.push("The mix is silent or too quiet to measure, so no loudness target was applied.".to_string()),
        Some(lufs) => {
            let need = p.target_lufs - lufs;
            let headroom = p.true_peak_ceiling_dbtp - true_peak_db(&pre);
            if need > headroom {
                gain_db = headroom;
                reached = false;
                warnings.push(format!(
                    "Reaching {:.1} LUFS would push the true peak above {:.1} dBTP. The mix is {:.1} LU below target. Nothing was limited or clipped.",
                    p.target_lufs, p.true_peak_ceiling_dbtp, need - headroom
                ));
            } else {
                gain_db = need;
            }
        }
    }
    let g = db_to_gain(gain_db as f32);
    d.scale(g);
    e.scale(g);
    m.scale(g);
    let master = Stereo::sum(&d, &e, &m);
    let loudness = measure(&master);
    MixOutput { dialogue: d, effects: e, music: m, master, loudness, gain_db, target_reached: reached, warnings }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(amp: f32, secs: f32, freq: f32) -> Stereo {
        let m: Vec<f32> = (0..(secs * SR as f32) as usize)
            .map(|n| amp * (2.0 * std::f32::consts::PI * freq * n as f32 / SR as f32).sin())
            .collect();
        Stereo::from_mono(&m)
    }
    fn item(buf: Stereo, start: usize, duck: bool) -> Placed {
        Placed { start, buf, gain_db: 0.0, fade_in: 0, fade_out: 0, duck }
    }
    fn params(target: f64) -> MixParams {
        MixParams { duck_depth_db: 12.0, attack_ms: 40, release_ms: 300, target_lufs: target, true_peak_ceiling_dbtp: -1.0 }
    }
    fn rms(s: &Stereo, a: usize, b: usize) -> f32 {
        ((a..b).map(|i| s.l[i] * s.l[i]).sum::<f32>() / (b - a) as f32).sqrt()
    }

    #[test]
    fn music_ducks_by_the_configured_depth_under_dialogue() {
        let sr = SR as usize;
        let dialogue = vec![item(tone(0.3, 1.0, 300.0), sr, false)]; // speech from 1.0 s to 2.0 s
        let music = vec![item(tone(0.2, 4.0, 440.0), 0, true)];
        let out = mix(&dialogue, &[], &music, &params(-70.0), 0);
        // Remove the master gain so levels compare with the source.
        let g = db_to_gain(out.gain_db as f32);
        let before = rms(&out.music, sr / 4, sr / 2) / g;
        let during = rms(&out.music, sr + sr / 4, sr + sr / 2) / g;
        let after = rms(&out.music, 3 * sr + sr / 2, 4 * sr - 100) / g;
        let depth_db = 20.0 * (during / before).log10();
        assert!((depth_db + 12.0).abs() < 0.5, "ducked by {depth_db} dB");
        assert!((20.0 * (after / before).log10()).abs() < 0.5, "did not recover");
    }

    #[test]
    fn duck_ramp_reaches_depth_within_the_attack_time() {
        let sr = SR as usize;
        let d = tone(0.3, 1.0, 300.0);
        let mut dd = Stereo::zeros(sr);
        dd.add(&d, 0, 1.0, None);
        let env = duck_envelope(&dd, sr, 12.0, 40, 300);
        let depth = db_to_gain(-12.0);
        let reach = env.iter().position(|&g| g <= depth + 1e-4).unwrap();
        let ms = reach as f32 * 1000.0 / SR as f32;
        assert!((ms - 40.0).abs() <= 20.0, "reached depth after {ms} ms");
    }

    #[test]
    fn source_music_that_opts_out_is_not_ducked() {
        let sr = SR as usize;
        let dialogue = vec![item(tone(0.3, 1.0, 300.0), 0, false)];
        let radio = vec![item(tone(0.2, 1.0, 440.0), 0, false)];
        let out = mix(&dialogue, &[], &radio, &params(-70.0), 0);
        let g = db_to_gain(out.gain_db as f32);
        assert!((rms(&out.music, sr / 2, sr / 2 + 2000) / g - 0.2 / 2f32.sqrt()).abs() < 0.01);
    }

    #[test]
    fn stems_sum_to_the_master_and_share_length() {
        let sr = SR as usize;
        let out = mix(
            &[item(tone(0.3, 1.0, 300.0), sr / 2, false)],
            &[item(tone(0.2, 0.5, 900.0), sr, true)],
            &[item(tone(0.2, 3.0, 440.0), 0, true)],
            &params(-23.0),
            sr / 4,
        );
        assert_eq!(out.dialogue.len(), out.master.len());
        assert_eq!(out.effects.len(), out.master.len());
        assert_eq!(out.music.len(), out.master.len());
        for i in 0..out.master.len() {
            let s = out.dialogue.l[i] + out.effects.l[i] + out.music.l[i];
            assert!((s - out.master.l[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn master_reaches_the_target_within_half_a_lu() {
        for target in [-14.0, -16.0, -23.0, -24.0] {
            let out = mix(&[item(tone(0.05, 6.0, 1000.0), 0, false)], &[], &[], &params(target), 0);
            assert!(out.target_reached);
            let l = out.loudness.integrated_lufs.unwrap();
            assert!((l - target).abs() < 0.5, "target {target} got {l}");
        }
    }

    #[test]
    fn unreachable_target_is_flagged_not_clipped() {
        // A very peaky signal: a short loud burst in a long quiet bed.
        let mut buf = tone(0.001, 6.0, 1000.0);
        for i in 1000..1100 {
            buf.l[i] = 0.9;
            buf.r[i] = 0.9;
        }
        let out = mix(&[item(buf, 0, false)], &[], &[], &params(-5.0), 0);
        assert!(!out.target_reached);
        assert!(!out.warnings.is_empty());
        assert!(out.loudness.true_peak_dbtp <= -1.0 + 0.01, "peak {}", out.loudness.true_peak_dbtp);
    }

    #[test]
    fn silent_mix_warns() {
        let out = mix(&[], &[], &[], &params(-14.0), 48000);
        assert!(!out.warnings.is_empty());
        assert_eq!(out.master.len(), 48000);
    }

    #[test]
    fn fades_remove_edge_clicks() {
        let p = Placed { start: 0, buf: tone(0.5, 0.2, 440.0), gain_db: 0.0, fade_in: 480, fade_out: 480, duck: false };
        let f = faded(&p);
        assert!(f.l[0].abs() < 1e-6 && f.l[f.len() - 1].abs() < 0.01);
    }
}
