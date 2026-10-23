//! ITU-R BS.1770 loudness at 48 kHz: integrated (gated), short-term max, loudness
//! range, and an estimated true peak. Destination targets are VERIFY items
//! (SOUND_DESIGN.md section 5.2) and are shown as such in the UI.

use crate::audio::{cubic, Stereo, SR};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}

impl Biquad {
    fn new(b: [f64; 3], a1: f64, a2: f64) -> Self {
        Biquad { b, a: [a1, a2], z: [0.0; 2] }
    }
    fn run(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

/// K-weighting for 48 kHz (BS.1770-4 table coefficients).
fn k_weight(x: &[f32]) -> Vec<f64> {
    let mut shelf = Biquad::new([1.53512485958697, -2.69169618940638, 1.19839281085285], -1.69065929318241, 0.73248077421585);
    let mut high = Biquad::new([1.0, -2.0, 1.0], -1.99004745483398, 0.99007225036621);
    x.iter().map(|&v| high.run(shelf.run(v as f64))).collect()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Loudness {
    pub integrated_lufs: Option<f64>,
    pub short_term_max_lufs: Option<f64>,
    pub loudness_range_lu: Option<f64>,
    /// Estimated with 4x cubic oversampling, not a certified meter.
    pub true_peak_dbtp: f64,
}

fn lufs(energy: f64) -> f64 {
    -0.691 + 10.0 * energy.max(1e-20).log10()
}

fn window_energies(l: &[f64], r: &[f64], win: usize, step: usize) -> Vec<f64> {
    if l.len() < win {
        return vec![];
    }
    let cum = |x: &[f64]| {
        let mut c = Vec::with_capacity(x.len() + 1);
        c.push(0.0);
        let mut s = 0.0;
        for v in x {
            s += v * v;
            c.push(s);
        }
        c
    };
    let (cl, cr) = (cum(l), cum(r));
    (0..=(l.len() - win))
        .step_by(step)
        .map(|i| ((cl[i + win] - cl[i]) + (cr[i + win] - cr[i])) / win as f64)
        .collect()
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

pub fn true_peak_db(s: &Stereo) -> f64 {
    let mut peak = 0.0f32;
    for ch in [&s.l, &s.r] {
        for i in 0..ch.len() {
            peak = peak.max(ch[i].abs());
            if i >= 1 && i + 2 < ch.len() {
                for k in 1..4 {
                    let v = cubic(ch[i - 1], ch[i], ch[i + 1], ch[i + 2], k as f32 / 4.0);
                    peak = peak.max(v.abs());
                }
            }
        }
    }
    crate::audio::gain_to_db(peak as f64)
}

pub fn measure(s: &Stereo) -> Loudness {
    let (kl, kr) = (k_weight(&s.l), k_weight(&s.r));
    let block = (SR as usize * 4) / 10;
    let blocks = window_energies(&kl, &kr, block, SR as usize / 10);
    let integrated = {
        let above: Vec<f64> = blocks.iter().copied().filter(|&e| lufs(e) > -70.0).collect();
        if above.is_empty() {
            None
        } else {
            let rel = lufs(above.iter().sum::<f64>() / above.len() as f64) - 10.0;
            let kept: Vec<f64> = above.into_iter().filter(|&e| lufs(e) > rel).collect();
            if kept.is_empty() { None } else { Some(lufs(kept.iter().sum::<f64>() / kept.len() as f64)) }
        }
    };
    let short = window_energies(&kl, &kr, SR as usize * 3, SR as usize / 10);
    let short_lufs: Vec<f64> = short.iter().map(|&e| lufs(e)).filter(|&l| l > -70.0).collect();
    let short_max = short_lufs.iter().cloned().fold(None, |m: Option<f64>, v| Some(m.map_or(v, |x| x.max(v))));
    let lra = if short_lufs.len() >= 2 {
        let mean_e = short.iter().copied().filter(|&e| lufs(e) > -70.0).sum::<f64>() / short_lufs.len() as f64;
        let rel = lufs(mean_e) - 20.0;
        let mut kept: Vec<f64> = short_lufs.into_iter().filter(|&l| l > rel).collect();
        kept.sort_by(|a, b| a.partial_cmp(b).unwrap());
        if kept.len() >= 2 { Some(percentile(&kept, 0.95) - percentile(&kept, 0.10)) } else { None }
    } else {
        None
    };
    Loudness { integrated_lufs: integrated, short_term_max_lufs: short_max, loudness_range_lu: lra, true_peak_dbtp: true_peak_db(s) }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Destination {
    pub id: &'static str,
    pub label: &'static str,
    pub target_lufs: f64,
    pub true_peak_ceiling_dbtp: f64,
    pub note: &'static str,
}

/// Starting points from SOUND_DESIGN.md 5.2. All values are VERIFY before shipping to users.
pub const DESTINATIONS: &[Destination] = &[
    Destination { id: "streaming", label: "Streaming and online video", target_lufs: -14.0, true_peak_ceiling_dbtp: -1.0, note: "VERIFY: platform normalisation varies" },
    Destination { id: "ebu", label: "Broadcast (EBU R128)", target_lufs: -23.0, true_peak_ceiling_dbtp: -1.0, note: "VERIFY: true-peak limit per broadcaster" },
    Destination { id: "atsc", label: "Broadcast (US, ATSC A/85)", target_lufs: -24.0, true_peak_ceiling_dbtp: -2.0, note: "VERIFY: dialogue-gated measurement is not implemented" },
    Destination { id: "podcast", label: "Podcast", target_lufs: -16.0, true_peak_ceiling_dbtp: -1.0, note: "VERIFY: confirm per distributor" },
];

pub fn destination(id: &str) -> Option<&'static Destination> {
    DESTINATIONS.iter().find(|d| d.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(amp: f32, secs: f32) -> Stereo {
        let m: Vec<f32> = (0..(secs * SR as f32) as usize)
            .map(|n| amp * (2.0 * std::f32::consts::PI * 1000.0 * n as f32 / SR as f32).sin())
            .collect();
        Stereo::from_mono(&m)
    }

    #[test]
    fn stereo_1khz_sine_measures_its_own_level() {
        // A stereo 1 kHz sine at -20 dBFS peak reads about -20 LUFS (EBU Tech 3341 style check).
        let m = measure(&tone(0.1, 10.0));
        let l = m.integrated_lufs.unwrap();
        assert!((l + 20.0).abs() < 0.3, "got {l}");
        assert!((m.true_peak_dbtp + 20.0).abs() < 0.3);
    }

    #[test]
    fn six_db_louder_reads_six_lu_louder() {
        let a = measure(&tone(0.05, 6.0)).integrated_lufs.unwrap();
        let b = measure(&tone(0.1, 6.0)).integrated_lufs.unwrap();
        assert!((b - a - 6.02).abs() < 0.1);
    }

    #[test]
    fn silence_has_no_integrated_loudness() {
        assert_eq!(measure(&Stereo::zeros(SR as usize * 2)).integrated_lufs, None);
    }

    #[test]
    fn gating_ignores_long_silence() {
        let mut s = tone(0.1, 5.0);
        let quiet = Stereo::zeros(SR as usize * 20);
        s.l.extend(quiet.l);
        s.r.extend(quiet.r);
        let l = measure(&s).integrated_lufs.unwrap();
        assert!((l + 20.0).abs() < 0.3, "got {l}");
    }

    #[test]
    fn loudness_range_is_small_for_a_steady_tone() {
        let m = measure(&tone(0.1, 12.0));
        assert!(m.loudness_range_lu.unwrap() < 0.5);
    }
}
