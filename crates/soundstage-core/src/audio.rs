//! Audio buffers, WAV I/O and resampling. The mix runs at 48 kHz stereo float.

use std::path::Path;

pub const SR: u32 = 48_000;
/// Longest file the app will load into memory (30 minutes at 48 kHz).
pub const MAX_IMPORT_SECONDS: u64 = 30 * 60;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stereo {
    pub l: Vec<f32>,
    pub r: Vec<f32>,
}

impl Stereo {
    pub fn zeros(n: usize) -> Self {
        Stereo { l: vec![0.0; n], r: vec![0.0; n] }
    }
    pub fn len(&self) -> usize {
        self.l.len()
    }
    pub fn is_empty(&self) -> bool {
        self.l.is_empty()
    }
    pub fn resize(&mut self, n: usize) {
        self.l.resize(n, 0.0);
        self.r.resize(n, 0.0);
    }
    pub fn from_mono(m: &[f32]) -> Self {
        Stereo { l: m.to_vec(), r: m.to_vec() }
    }
    /// Add `src` at `start`, scaled by `gain` and optionally a per-sample envelope
    /// (indexed by absolute position in `self`). Grows `self` if needed.
    pub fn add(&mut self, src: &Stereo, start: usize, gain: f32, env: Option<&[f32]>) {
        let end = start + src.len();
        if end > self.len() {
            self.resize(end);
        }
        for i in 0..src.len() {
            let e = env.map_or(1.0, |e| e.get(start + i).copied().unwrap_or(1.0));
            self.l[start + i] += src.l[i] * gain * e;
            self.r[start + i] += src.r[i] * gain * e;
        }
    }
    pub fn scale(&mut self, g: f32) {
        self.l.iter_mut().for_each(|x| *x *= g);
        self.r.iter_mut().for_each(|x| *x *= g);
    }
    pub fn sum(a: &Stereo, b: &Stereo, c: &Stereo) -> Stereo {
        let n = a.len().max(b.len()).max(c.len());
        let mut out = Stereo::zeros(n);
        for s in [a, b, c] {
            for i in 0..s.len() {
                out.l[i] += s.l[i];
                out.r[i] += s.r[i];
            }
        }
        out
    }
    pub fn peak(&self) -> f32 {
        self.l.iter().chain(self.r.iter()).fold(0.0f32, |m, x| m.max(x.abs()))
    }
}

pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

pub fn gain_to_db(g: f64) -> f64 {
    if g <= 1e-9 { -180.0 } else { 20.0 * g.log10() }
}

/// Catmull-Rom interpolation between `p1` and `p2` at `t` in 0..1.
pub fn cubic(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1) + (-p0 + p2) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let out_len = ((input.len() as f64) / ratio).round() as usize;
    let get = |i: isize| input[i.clamp(0, input.len() as isize - 1) as usize];
    (0..out_len)
        .map(|n| {
            let pos = n as f64 * ratio;
            let i = pos.floor() as isize;
            let t = (pos - i as f64) as f32;
            cubic(get(i - 1), get(i), get(i + 1), get(i + 2), t)
        })
        .collect()
}

/// Decoded WAV: one `Vec<f32>` per channel, plus the sample rate.
pub struct Decoded {
    pub channels: Vec<Vec<f32>>,
    pub rate: u32,
}

pub fn read_wav(path: &Path) -> Result<Decoded, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| format!("Could not read the WAV file: {e}"))?;
    let spec = reader.spec();
    if spec.channels == 0 || spec.sample_rate == 0 {
        return Err("The WAV file has an invalid header.".into());
    }
    let seconds = reader.duration() as u64 / spec.sample_rate as u64;
    if seconds > MAX_IMPORT_SECONDS {
        return Err("The audio is longer than 30 minutes, which Soundstage does not load.".into());
    }
    let ch = spec.channels as usize;
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Could not decode the WAV data: {e}"))?,
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 * scale))
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Could not decode the WAV data: {e}"))?
        }
    };
    let frames = interleaved.len() / ch;
    let channels = (0..ch).map(|c| (0..frames).map(|f| interleaved[f * ch + c]).collect()).collect();
    Ok(Decoded { channels, rate: spec.sample_rate })
}

pub fn probe_duration_ms(path: &Path) -> Result<u64, String> {
    let r = hound::WavReader::open(path).map_err(|e| format!("Could not read the WAV file: {e}"))?;
    let s = r.spec();
    if s.sample_rate == 0 {
        return Err("The WAV file has an invalid header.".into());
    }
    Ok(r.duration() as u64 * 1000 / s.sample_rate as u64)
}

/// Convert any decoded file to 48 kHz stereo (mono is centred, extra channels are ignored).
pub fn to_mix_format(d: &Decoded) -> Stereo {
    let l = resample(&d.channels[0], d.rate, SR);
    let r = if d.channels.len() > 1 { resample(&d.channels[1], d.rate, SR) } else { l.clone() };
    Stereo { l, r }
}

fn quantise(x: f32, max: f32) -> i32 {
    (x.clamp(-1.0, 1.0) * max).round() as i32
}

pub fn write_wav_24(path: &Path, s: &Stereo) -> Result<(), String> {
    let spec = hound::WavSpec { channels: 2, sample_rate: SR, bits_per_sample: 24, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    for i in 0..s.len() {
        w.write_sample(quantise(s.l[i], 8_388_607.0)).map_err(|e| e.to_string())?;
        w.write_sample(quantise(s.r[i], 8_388_607.0)).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())
}

pub fn write_wav_16_mono(path: &Path, samples: &[f32], rate: u32) -> Result<(), String> {
    let spec = hound::WavSpec { channels: 1, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    for &x in samples {
        w.write_sample(quantise(x, 32_767.0) as i16).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())
}

pub const MAX_PEAK_BINS: usize = 2000;

/// Peak envelope for drawing a waveform: `bins` values in 0..=1, the loudest sample (any channel) in each slice.
pub fn peaks_of(channels: &[&[f32]], bins: usize) -> Vec<f32> {
    let bins = bins.clamp(8, MAX_PEAK_BINS);
    let len = channels.iter().map(|c| c.len()).max().unwrap_or(0);
    if len == 0 {
        return vec![0.0; bins];
    }
    (0..bins)
        .map(|b| {
            let (s, e) = (b * len / bins, ((b + 1) * len / bins).max(b * len / bins + 1).min(len));
            channels.iter().flat_map(|c| c[s.min(c.len())..e.min(c.len())].iter()).fold(0.0f32, |m, x| m.max(x.abs())).min(1.0)
        })
        .collect()
}

pub fn peaks(path: &Path, bins: usize) -> Result<Vec<f32>, String> {
    let d = read_wav(path)?;
    let refs: Vec<&[f32]> = d.channels.iter().map(|c| c.as_slice()).collect();
    Ok(peaks_of(&refs, bins))
}

/// Planted-defect checks run on every render (TEST_PLAN.md section 3).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QualityReport {
    pub clipped_samples: usize,
    pub longest_silence_ms: u64,
}

pub fn quality(samples: &[f32], rate: u32) -> QualityReport {
    let clipped = samples.iter().filter(|x| x.abs() >= 0.9999).count();
    let thresh = 10f32.powf(-60.0 / 20.0);
    let (mut run, mut longest) = (0usize, 0usize);
    for &x in samples {
        if x.abs() < thresh {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    QualityReport { clipped_samples: clipped, longest_silence_ms: longest as u64 * 1000 / rate.max(1) as u64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn sine(freq: f32, amp: f32, secs: f32, rate: u32) -> Vec<f32> {
        (0..(secs * rate as f32) as usize)
            .map(|n| amp * (2.0 * std::f32::consts::PI * freq * n as f32 / rate as f32).sin())
            .collect()
    }

    #[test]
    fn resample_keeps_duration_and_pitch() {
        let x = sine(440.0, 0.5, 1.0, 22_050);
        let y = resample(&x, 22_050, SR);
        assert!((y.len() as i64 - SR as i64).abs() <= 1);
        let peak = y.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((peak - 0.5).abs() < 0.02);
    }

    #[test]
    fn wav_round_trip_24_bit() {
        let dir = std::env::temp_dir().join(format!("ss-wav-{}", crate::util::new_id("t")));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.wav");
        let s = Stereo::from_mono(&sine(1000.0, 0.25, 0.1, SR));
        write_wav_24(&p, &s).unwrap();
        let d = read_wav(&p).unwrap();
        assert_eq!(d.rate, SR);
        assert_eq!(d.channels.len(), 2);
        assert_eq!(d.channels[0].len(), s.len());
        assert!((d.channels[0][50] - s.l[50]).abs() < 1e-5);
        assert_eq!(probe_duration_ms(&p).unwrap(), 100);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn quality_flags_planted_defects() {
        let mut x = sine(300.0, 0.3, 1.0, 1000);
        x.extend(vec![0.0; 4000]);
        x[10] = 1.0;
        let q = quality(&x, 1000);
        assert_eq!(q.clipped_samples, 1);
        assert!(q.longest_silence_ms >= 3999);
    }

    #[test]
    fn peaks_follow_the_shape_of_the_sound_and_stay_in_range() {
        let mut x = vec![0.0f32; 1000];
        for v in x.iter_mut().skip(500) { *v = -0.8; }
        let p = peaks_of(&[&x], 10);
        assert_eq!(p.len(), 10);
        assert!(p[..5].iter().all(|v| *v == 0.0));
        assert!(p[5..].iter().all(|v| (*v - 0.8).abs() < 1e-6), "negative samples count by size");
        assert_eq!(peaks_of(&[&[]], 50).len(), 50);
        assert_eq!(peaks_of(&[&x], 1_000_000).len(), MAX_PEAK_BINS);
        assert_eq!(peaks_of(&[&[5.0, -5.0]], 8).iter().cloned().fold(0.0, f32::max), 1.0);
    }

    #[test]
    fn rejects_garbage_files() {
        let p = std::env::temp_dir().join(format!("ss-bad-{}.wav", crate::util::new_id("t")));
        std::fs::write(&p, b"not a wav").unwrap();
        assert!(read_wav(&p).is_err());
        std::fs::remove_file(p).ok();
    }
}
