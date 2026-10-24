//! Delivery formats: WAV written natively at 16, 24 or 32-bit float and any sample rate, and FLAC, MP3, AAC (M4A),
//! Ogg Vorbis, Opus and MP4 (audio over a waveform picture) through an FFmpeg the user has installed. Also SRT and VTT
//! subtitles built from the real timeline. FFmpeg is run as a separate program with an argument list, never through a
//! shell, and only the files Soundstage itself names are passed to it.

use crate::audio::{resample, Stereo, SR};
use crate::layout::Layout;
use crate::project::Project;
use crate::script::Parsed;
use crate::tags::{spoken_text, tokenize};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Aspect {
    Wide,
    Tall,
    Square,
}

impl Aspect {
    pub fn size(self) -> (u32, u32) {
        match self {
            Aspect::Wide => (1920, 1080),
            Aspect::Tall => (1080, 1920),
            Aspect::Square => (1080, 1080),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Codec {
    Wav { bits: u8 },
    Flac,
    Mp3 { kbps: u32 },
    Aac { kbps: u32 },
    Vorbis { quality: u8 },
    Opus { kbps: u32 },
    /// Audio over an animated waveform, for sharing where only video is accepted.
    Mp4 { aspect: Aspect, kbps: u32 },
}

impl Codec {
    pub fn ext(&self) -> &'static str {
        match self {
            Codec::Wav { .. } => "wav",
            Codec::Flac => "flac",
            Codec::Mp3 { .. } => "mp3",
            Codec::Aac { .. } => "m4a",
            Codec::Vorbis { .. } => "ogg",
            Codec::Opus { .. } => "opus",
            Codec::Mp4 { .. } => "mp4",
        }
    }
    pub fn label(&self) -> String {
        match self {
            Codec::Wav { bits: 32 } => "WAV 32-bit float".into(),
            Codec::Wav { bits } => format!("WAV {bits}-bit"),
            Codec::Flac => "FLAC (lossless)".into(),
            Codec::Mp3 { kbps } => format!("MP3 {kbps} kbps"),
            Codec::Aac { kbps } => format!("AAC (M4A) {kbps} kbps"),
            Codec::Vorbis { quality } => format!("Ogg Vorbis quality {quality}"),
            Codec::Opus { kbps } => format!("Opus {kbps} kbps"),
            Codec::Mp4 { aspect, .. } => format!("MP4 video, {}", match aspect { Aspect::Wide => "16:9", Aspect::Tall => "9:16", Aspect::Square => "1:1" }),
        }
    }
    pub fn needs_ffmpeg(&self) -> bool {
        !matches!(self, Codec::Wav { .. })
    }
    /// FFmpeg encoders that must exist for this format.
    pub fn encoders(&self) -> &'static [&'static str] {
        match self {
            Codec::Wav { .. } => &[],
            Codec::Flac => &["flac"],
            Codec::Mp3 { .. } => &["libmp3lame"],
            Codec::Aac { .. } => &["aac"],
            Codec::Vorbis { .. } => &["libvorbis"],
            Codec::Opus { .. } => &["libopus"],
            // The video encoder is chosen at run time (see `Ffmpeg::video_encoder`), so only the audio one is fixed here.
            Codec::Mp4 { .. } => &["aac"],
        }
    }
    pub fn lossless(&self) -> bool {
        matches!(self, Codec::Wav { .. } | Codec::Flac)
    }
    pub fn validate(&self) -> Result<(), String> {
        let ok = match self {
            Codec::Wav { bits } => matches!(bits, 16 | 24 | 32),
            Codec::Mp3 { kbps } | Codec::Aac { kbps } | Codec::Mp4 { kbps, .. } => (64..=320).contains(kbps),
            Codec::Opus { kbps } => (32..=256).contains(kbps),
            Codec::Vorbis { quality } => *quality <= 10,
            Codec::Flac => true,
        };
        if ok { Ok(()) } else { Err(format!("{} has a setting outside its allowed range.", self.label())) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubtitleFormat {
    Srt,
    Vtt,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ExportOptions {
    pub master: Codec,
    /// `None` writes no stems.
    pub stems: Option<Codec>,
    pub sample_rate: u32,
    pub subtitles: Vec<SubtitleFormat>,
    pub speaker_labels: bool,
    /// A specific ffmpeg.exe chosen by the user. When set, only that file is tried.
    pub ffmpeg_path: Option<String>,
    /// Also write an OpenTimelineIO file and every clip as its own file, for video editors.
    pub timeline: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions { master: Codec::Wav { bits: 24 }, stems: Some(Codec::Wav { bits: 24 }), sample_rate: SR, subtitles: vec![], speaker_labels: false, ffmpeg_path: None, timeline: false }
    }
}

impl ExportOptions {
    pub fn validate(&self) -> Result<(), String> {
        self.master.validate()?;
        if let Some(s) = &self.stems {
            s.validate()?;
            if matches!(s, Codec::Mp4 { .. }) {
                return Err("Stems cannot be written as MP4 video. Choose an audio format for stems.".into());
            }
        }
        if !matches!(self.sample_rate, 44_100 | 48_000 | 96_000) {
            return Err("Sample rate must be 44.1, 48 or 96 kHz.".into());
        }
        Ok(())
    }
}

/* ------------------------------------------------------------------ WAV */

fn xorshift(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

/// Write a stereo WAV at `rate` and `bits` (16, 24 or 32-bit float). 16-bit output is dithered (TPDF) so quiet
/// passages do not turn gritty. The dither is deterministic, so the same mix gives the same file.
pub fn write_wav(path: &Path, s: &Stereo, rate: u32, bits: u8) -> Result<(), String> {
    let s48;
    let s = if rate == SR { s } else { s48 = Stereo { l: resample(&s.l, SR, rate), r: resample(&s.r, SR, rate) }; &s48 };
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: rate,
        bits_per_sample: bits as u16,
        sample_format: if bits == 32 { hound::SampleFormat::Float } else { hound::SampleFormat::Int },
    };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    let mut seed = 0x9e37_79b9u32;
    let mut put = |x: f32| -> Result<(), String> {
        match bits {
            32 => w.write_sample(x).map_err(|e| e.to_string()),
            24 => w.write_sample((x.clamp(-1.0, 1.0) * 8_388_607.0).round() as i32).map_err(|e| e.to_string()),
            _ => {
                let tpdf = ((xorshift(&mut seed) as f32 / u32::MAX as f32) - (xorshift(&mut seed) as f32 / u32::MAX as f32)) / 32_768.0;
                w.write_sample(((x + tpdf).clamp(-1.0, 1.0) * 32_767.0).round() as i16).map_err(|e| e.to_string())
            }
        }
    };
    for i in 0..s.len() {
        put(s.l[i])?;
        put(s.r[i])?;
    }
    w.finalize().map_err(|e| e.to_string())
}

/* ------------------------------------------------------------------ FFmpeg */

#[derive(Clone, Debug, Serialize)]
pub struct Ffmpeg {
    pub path: PathBuf,
    pub version: String,
    pub encoders: Vec<String>,
}

fn run_capture(path: &Path, args: &[String], timeout: Duration) -> Result<(String, String, bool), String> {
    let mut cmd = Command::new(path);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = cmd.spawn().map_err(|e| format!("could not start {}: {e}", path.display()))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                return Err("the encoder took too long and was stopped".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(30)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let mut out = String::new();
    let mut err = String::new();
    // Read what is left. Output is small (-version, -encoders) or short error text.
    if let Some(mut o) = child.stdout.take() { let _ = o.by_ref().take(1 << 20).read_to_string(&mut out); }
    if let Some(mut e) = child.stderr.take() { let _ = e.by_ref().take(1 << 20).read_to_string(&mut err); }
    let ok = child.wait().map(|s| s.success()).unwrap_or(false);
    Ok((out, err, ok))
}

fn looks_like_ffmpeg(p: &Path) -> bool {
    p.is_file() && p.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case("ffmpeg"))
}

impl Ffmpeg {
    fn probe(path: &Path) -> Option<Ffmpeg> {
        if !looks_like_ffmpeg(path) {
            return None;
        }
        let (out, _, ok) = run_capture(path, &["-hide_banner".into(), "-version".into()], Duration::from_secs(10)).ok()?;
        let first = out.lines().next()?.to_string();
        if !ok || !first.to_lowercase().starts_with("ffmpeg version") {
            return None;
        }
        let (enc, _, _) = run_capture(path, &["-hide_banner".into(), "-encoders".into()], Duration::from_secs(10)).ok()?;
        let encoders = enc
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let flags = it.next()?;
                (flags.len() == 6 && (flags.starts_with('A') || flags.starts_with('V'))).then(|| it.next().map(String::from)).flatten()
            })
            .collect();
        Some(Ffmpeg { path: path.to_path_buf(), version: first.chars().take(120).collect(), encoders })
    }

    /// `ffmpeg.exe` installed next to the app (in `resources\\`), if any.
    pub fn bundled() -> Option<PathBuf> {
        let exe = std::env::current_exe().ok()?;
        let dir = exe.parent()?;
        [dir.join("resources").join("ffmpeg.exe"), dir.join("ffmpeg.exe")].into_iter().find(|p| p.is_file())
    }

    /// Find FFmpeg. If the user chose a file, only that file is tried, so a wrong choice fails loudly
    /// instead of quietly using another copy.
    pub fn detect(configured: Option<&str>) -> Option<Ffmpeg> {
        if let Some(c) = configured.filter(|c| !c.trim().is_empty()) {
            return Ffmpeg::probe(Path::new(c.trim()));
        }
        // The copy shipped with Soundstage comes first, so exports behave the same on every computer.
        if let Some(found) = Ffmpeg::bundled().and_then(|p| Ffmpeg::probe(&p)) {
            return Some(found);
        }
        let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local).join("Microsoft").join("WinGet").join("Links"));
        }
        dirs.extend(["C:\\ffmpeg\\bin", "C:\\Program Files\\ffmpeg\\bin"].map(PathBuf::from));
        dirs.iter().map(|d| d.join("ffmpeg.exe")).chain(dirs.iter().map(|d| d.join("ffmpeg"))).find_map(|p| Ffmpeg::probe(&p))
    }

    pub fn has(&self, encoder: &str) -> bool {
        self.encoders.iter().any(|e| e == encoder)
    }

    pub fn supports(&self, codec: &Codec) -> bool {
        codec.encoders().iter().all(|e| self.has(e)) && (!matches!(codec, Codec::Mp4 { .. }) || self.video_encoder().is_some())
    }

    /// The best H.264-or-better-compatible video encoder this build has. x264 comes with GPL builds, OpenH264 with the
    /// bundled LGPL build, and MPEG-4 part 2 is the last resort that every build has.
    pub fn video_encoder(&self) -> Option<&'static str> {
        ["libx264", "libopenh264", "mpeg4"].into_iter().find(|e| self.has(e))
    }

    pub fn encode(&self, input_wav: &Path, out: &Path, codec: &Codec, title: &str) -> Result<(), String> {
        let args = args_for_video(codec, input_wav, out, title, self.video_encoder().unwrap_or("libx264"));
        let (_, err, ok) = run_capture(&self.path, &args, Duration::from_secs(900))?;
        if !ok || !out.is_file() {
            let line = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("the encoder failed");
            return Err(format!("{} could not be written: {}", codec.label(), line.chars().take(200).collect::<String>()));
        }
        Ok(())
    }
}

fn clean_meta(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).take(120).collect()
}

/// The exact FFmpeg arguments for a format. Pure, so it can be tested without running FFmpeg.
pub fn args_for(codec: &Codec, input: &Path, out: &Path, title: &str) -> Vec<String> {
    args_for_video(codec, input, out, title, "libx264")
}

/// Video encoder settings differ: x264 takes a quality factor, OpenH264 and MPEG-4 take a rate or quality number.
fn video_args(encoder: &str) -> Vec<String> {
    let s = |x: &str| x.to_string();
    match encoder {
        "libx264" => vec![s("-c:v"), s("libx264"), s("-preset"), s("medium"), s("-crf"), s("20")],
        "libopenh264" => vec![s("-c:v"), s("libopenh264"), s("-b:v"), s("2500k"), s("-profile:v"), s("high")],
        _ => vec![s("-c:v"), s("mpeg4"), s("-q:v"), s("3")],
    }
}

pub fn args_for_video(codec: &Codec, input: &Path, out: &Path, title: &str, vcodec: &str) -> Vec<String> {
    let s = |x: &str| x.to_string();
    let mut a = vec![s("-hide_banner"), s("-loglevel"), s("error"), s("-y"), s("-i"), input.to_string_lossy().to_string()];
    match codec {
        Codec::Wav { .. } => {}
        Codec::Flac => a.extend([s("-c:a"), s("flac"), s("-compression_level"), s("8")]),
        Codec::Mp3 { kbps } => a.extend([s("-c:a"), s("libmp3lame"), s("-b:a"), format!("{kbps}k")]),
        Codec::Aac { kbps } => a.extend([s("-c:a"), s("aac"), s("-b:a"), format!("{kbps}k"), s("-movflags"), s("+faststart")]),
        Codec::Vorbis { quality } => a.extend([s("-c:a"), s("libvorbis"), s("-q:a"), quality.to_string()]),
        Codec::Opus { kbps } => a.extend([s("-c:a"), s("libopus"), s("-b:a"), format!("{kbps}k")]),
        Codec::Mp4 { aspect, kbps } => {
            let (w, h) = aspect.size();
            a.extend([
                s("-filter_complex"),
                format!("[0:a]showwaves=s={w}x{h}:mode=cline:colors=0xE9B85C:rate=30,format=yuv420p[v]"),
                s("-map"), s("[v]"), s("-map"), s("0:a"),
            ]);
            a.extend(video_args(vcodec));
            a.extend([
                s("-c:a"), s("aac"), s("-b:a"), format!("{kbps}k"),
                s("-shortest"), s("-movflags"), s("+faststart"),
            ]);
        }
    }
    a.extend([s("-metadata"), format!("title={}", clean_meta(title)), s("-metadata"), s("comment=AI-generated dialogue. Made with Soundstage.")]);
    a.push(out.to_string_lossy().to_string());
    a
}

/// Write one audio file in the chosen format. Lossy and video formats go through a temporary 24-bit WAV.
pub fn write_audio(buf: &Stereo, out_without_ext: &Path, codec: &Codec, rate: u32, ff: Option<&Ffmpeg>, title: &str, tmp: &Path) -> Result<PathBuf, String> {
    let out = PathBuf::from(format!("{}.{}", out_without_ext.display(), codec.ext()));
    match codec {
        Codec::Wav { bits } => write_wav(&out, buf, rate, *bits)?,
        _ => {
            let ff = ff.ok_or("FFmpeg is needed for this format.")?;
            let t = tmp.join(format!("{}.wav", crate::util::new_id("enc")));
            write_wav(&t, buf, rate, 24)?;
            let r = ff.encode(&t, &out, codec, title);
            let _ = std::fs::remove_file(&t);
            r?;
        }
    }
    Ok(out)
}

/* ------------------------------------------------------------------ subtitles */

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubCue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

fn clean_cue_text(s: &str) -> String {
    s.chars().filter(|c| !c.is_control() || *c == '\n').collect::<String>().replace("-->", "->").trim().to_string()
}

fn wrap(text: &str, width: usize) -> String {
    let mut lines: Vec<String> = vec![];
    let mut cur = String::new();
    for w in text.split_whitespace() {
        if !cur.is_empty() && cur.chars().count() + 1 + w.chars().count() > width {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(w);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines.join("\n")
}

/// One cue per placed line, timed from the real timeline. Directions in [brackets] are never shown.
pub fn subtitle_cues(project: &Project, parsed: &Parsed, layout: &Layout, speaker_labels: bool) -> Vec<SubCue> {
    let mut cues: Vec<SubCue> = layout
        .lines
        .iter()
        .filter_map(|slot| {
            let line = parsed.lines.iter().find(|l| l.key == slot.key)?;
            let words = spoken_text(&tokenize(&line.text).0);
            if words.trim().is_empty() {
                return None;
            }
            let _ = project;
            let text = if speaker_labels { format!("{}: {}", line.speaker, words) } else { words };
            Some(SubCue { start_ms: slot.start_ms, end_ms: slot.end_ms.max(slot.start_ms + 400), text: wrap(&clean_cue_text(&text), 42) })
        })
        .collect();
    cues.sort_by_key(|c| c.start_ms);
    // Cues never overlap.
    for i in 1..cues.len() {
        let next_start = cues[i].start_ms;
        if cues[i - 1].end_ms > next_start {
            cues[i - 1].end_ms = next_start.max(cues[i - 1].start_ms + 1);
        }
    }
    cues
}

fn stamp(ms: u64, sep: char) -> String {
    format!("{:02}:{:02}:{:02}{sep}{:03}", ms / 3_600_000, (ms / 60_000) % 60, (ms / 1000) % 60, ms % 1000)
}

pub fn to_srt(cues: &[SubCue]) -> String {
    cues.iter().enumerate().map(|(i, c)| format!("{}\n{} --> {}\n{}\n", i + 1, stamp(c.start_ms, ','), stamp(c.end_ms, ','), c.text)).collect::<Vec<_>>().join("\n")
}

pub fn to_vtt(cues: &[SubCue]) -> String {
    let esc = |t: &str| t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let body = cues.iter().map(|c| format!("{} --> {}\n{}\n", stamp(c.start_ms, '.'), stamp(c.end_ms, '.'), esc(&c.text))).collect::<Vec<_>>().join("\n");
    format!("WEBVTT\n\n{body}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::read_wav;
    use crate::util::new_id;

    fn tone(secs: f32, amp: f32) -> Stereo {
        Stereo::from_mono(&(0..(secs * SR as f32) as usize).map(|n| amp * (2.0 * std::f32::consts::PI * 440.0 * n as f32 / SR as f32).sin()).collect::<Vec<_>>())
    }
    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(new_id("ss-out"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn wav_bit_depths_and_sample_rates_are_written_correctly() {
        let d = tmp();
        let t = tone(0.5, 0.5);
        for (bits, rate) in [(16u8, 44_100u32), (24, 48_000), (32, 96_000)] {
            let p = d.join(format!("a{bits}.wav"));
            write_wav(&p, &t, rate, bits).unwrap();
            let r = hound::WavReader::open(&p).unwrap();
            assert_eq!((r.spec().bits_per_sample, r.spec().sample_rate, r.spec().channels), (bits as u16, rate, 2));
            let dec = read_wav(&p).unwrap();
            let want = (0.5 * rate as f32) as usize;
            assert!((dec.channels[0].len() as i64 - want as i64).abs() <= 2, "{bits}-bit length");
            let peak = dec.channels[0].iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!((peak - 0.5).abs() < 0.02, "{bits}-bit peak {peak}");
        }
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn sixteen_bit_dither_is_deterministic_and_quiet() {
        let d = tmp();
        let t = tone(0.2, 0.0001);
        write_wav(&d.join("a.wav"), &t, 48_000, 16).unwrap();
        write_wav(&d.join("b.wav"), &t, 48_000, 16).unwrap();
        assert_eq!(std::fs::read(d.join("a.wav")).unwrap(), std::fs::read(d.join("b.wav")).unwrap());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn options_are_validated() {
        assert!(ExportOptions::default().validate().is_ok());
        let mut o = ExportOptions::default();
        o.master = Codec::Mp3 { kbps: 9999 };
        assert!(o.validate().is_err());
        o.master = Codec::Wav { bits: 12 };
        assert!(o.validate().is_err());
        o = ExportOptions::default();
        o.sample_rate = 22_050;
        assert!(o.validate().is_err());
        o = ExportOptions::default();
        o.stems = Some(Codec::Mp4 { aspect: Aspect::Wide, kbps: 192 });
        assert!(o.validate().is_err(), "stems cannot be video");
        // options round-trip through JSON, which is how the UI sends them
        let j = serde_json::json!({ "master": { "kind": "mp3", "kbps": 192 }, "stems": null, "sample_rate": 44100, "subtitles": ["srt", "vtt"] });
        let o: ExportOptions = serde_json::from_value(j).unwrap();
        assert_eq!((o.master.clone(), o.stems.clone(), o.subtitles.len()), (Codec::Mp3 { kbps: 192 }, None, 2));
    }

    #[test]
    fn ffmpeg_arguments_are_exact_and_end_with_the_output_path() {
        let a = args_for(&Codec::Mp3 { kbps: 256 }, Path::new("C:/t/in.wav"), Path::new("C:/o/out.mp3"), "My\nFilm");
        assert!(a.windows(2).any(|w| w == ["-c:a", "libmp3lame"]) && a.windows(2).any(|w| w == ["-b:a", "256k"]));
        assert_eq!(a.last().unwrap(), "C:/o/out.mp3");
        assert!(a.contains(&"title=MyFilm".to_string()), "control characters never reach metadata");
        let v = args_for(&Codec::Mp4 { aspect: Aspect::Tall, kbps: 192 }, Path::new("i.wav"), Path::new("o.mp4"), "t");
        assert!(v.iter().any(|x| x.contains("showwaves=s=1080x1920")) && v.contains(&"libx264".to_string()));
        assert!(a.iter().any(|x| x.starts_with("comment=AI-generated")), "provenance is embedded in the file");
    }

    #[test]
    fn a_chosen_ffmpeg_path_is_the_only_one_tried() {
        assert!(Ffmpeg::detect(Some("C:\\definitely\\not\\here\\ffmpeg.exe")).is_none());
        let d = tmp();
        std::fs::write(d.join("notffmpeg.exe"), b"x").unwrap();
        assert!(Ffmpeg::detect(Some(d.join("notffmpeg.exe").to_str().unwrap())).is_none(), "wrong file name is rejected");
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn real_ffmpeg_writes_every_available_format_and_the_files_decode() {
        let Some(ff) = Ffmpeg::detect(None) else {
            eprintln!("FFmpeg not found; skipping real encode test");
            return;
        };
        let d = tmp();
        let t = tone(1.0, 0.3);
        let codecs = [
            Codec::Flac, Codec::Mp3 { kbps: 192 }, Codec::Aac { kbps: 192 }, Codec::Vorbis { quality: 5 }, Codec::Opus { kbps: 128 },
            Codec::Mp4 { aspect: Aspect::Square, kbps: 192 },
        ];
        let mut tried = 0;
        for c in &codecs {
            if !ff.supports(c) {
                eprintln!("encoder missing for {}; skipping", c.label());
                continue;
            }
            let out = write_audio(&t, &d.join("o"), c, 48_000, Some(&ff), "Test", &d).unwrap();
            assert!(std::fs::metadata(&out).unwrap().len() > 1000, "{} is empty", c.label());
            // The result must be a valid file that FFmpeg itself can read back.
            let (_, err, ok) = run_capture(&ff.path, &["-v".into(), "error".into(), "-i".into(), out.to_string_lossy().to_string(), "-f".into(), "null".into(), "-".into()], Duration::from_secs(60)).unwrap();
            assert!(ok && err.trim().is_empty(), "{} does not decode cleanly: {err}", c.label());
            std::fs::remove_file(&out).ok();
            tried += 1;
        }
        assert!(tried >= 1);
        std::fs::remove_dir_all(d).ok();
    }

    /// The LGPL build that ships in the installer must write every format, with no GPL encoder (x264) to lean on.
    #[test]
    fn bundled_lgpl_ffmpeg_writes_every_format() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/src-tauri/resources/ffmpeg.exe");
        let Some(ff) = path.is_file().then(|| Ffmpeg::detect(Some(path.to_str().unwrap()))).flatten() else {
            eprintln!("bundled ffmpeg.exe not fetched (run tools/fetch-ffmpeg.mjs); skipping");
            return;
        };
        assert!(!ff.has("libx264"), "the bundled build must not rely on GPL x264");
        assert!(ff.video_encoder().is_some());
        let d = tmp();
        let t = tone(1.0, 0.3);
        for c in [Codec::Flac, Codec::Mp3 { kbps: 192 }, Codec::Aac { kbps: 192 }, Codec::Vorbis { quality: 5 }, Codec::Opus { kbps: 128 }, Codec::Mp4 { aspect: Aspect::Square, kbps: 192 }] {
            assert!(ff.supports(&c), "{} is not supported by the bundled build", c.label());
            let out = write_audio(&t, &d.join("o"), &c, 48_000, Some(&ff), "Test", &d).unwrap();
            let (_, err, ok) = run_capture(&ff.path, &["-v".into(), "error".into(), "-i".into(), out.to_string_lossy().to_string(), "-f".into(), "null".into(), "-".into()], Duration::from_secs(60)).unwrap();
            assert!(ok && err.trim().is_empty(), "{} does not decode cleanly: {err}", c.label());
            std::fs::remove_file(&out).ok();
        }
        std::fs::remove_dir_all(d).ok();
    }

    fn fixture() -> (Project, Parsed, Layout) {
        use crate::project::{Take, TakeStatus};
        let mut p = Project::new("S", false);
        p.script = "# One\nAMA: [sigh] Hello there, this is a rather long line of dialogue that must wrap onto two lines <ok> & fine\nKOLA: [laugh]\nKOLA: Bye --> now".into();
        let parsed = crate::script::parse(&p.script);
        for (k, id) in [("s0-l0", "t1"), ("s0-l1", "t2"), ("s0-l2", "t3")] {
            p.takes.push(Take { id: id.into(), line_key: k.into(), speaker: "X".into(), text: "x".into(), voice_id: "v".into(), adapter_id: "a".into(), provider_version: "1".into(), cache_key: "k".into(), file: "takes/x.wav".into(), duration_ms: 2000, sample_rate: 22050, direction_summary: String::new(), created: String::new(), status: TakeStatus::Pending, approver: None });
            p.selected.insert(k.into(), id.into());
        }
        let layout = crate::layout::compute(&p, &parsed);
        (p, parsed, layout)
    }

    #[test]
    fn subtitles_follow_the_timeline_and_hide_directions() {
        let (p, parsed, layout) = fixture();
        let cues = subtitle_cues(&p, &parsed, &layout, false);
        assert_eq!(cues.len(), 2, "a line that is only a [laugh] has no words to show");
        assert!(!cues[0].text.contains('[') && cues[0].text.contains('\n'), "wrapped, no tags: {:?}", cues[0].text);
        assert_eq!(cues[0].start_ms, 350);
        assert!(cues[0].end_ms <= cues[1].start_ms);
        assert!(!cues[1].text.contains("-->"), "cue text cannot break the format");
        let labelled = subtitle_cues(&p, &parsed, &layout, true);
        assert!(labelled[0].text.starts_with("AMA: "));
    }

    #[test]
    fn srt_and_vtt_are_well_formed() {
        let (p, parsed, layout) = fixture();
        let cues = subtitle_cues(&p, &parsed, &layout, false);
        let srt = to_srt(&cues);
        assert!(srt.starts_with("1\n00:00:00,350 --> 00:00:02,350\n"), "{srt}");
        assert!(srt.contains("\n2\n"));
        let vtt = to_vtt(&cues);
        assert!(vtt.starts_with("WEBVTT\n\n00:00:00.350 --> 00:00:02.350\n"));
        assert!(vtt.contains("&lt;ok&gt; &amp; fine"), "VTT text is escaped: {vtt}");
        assert_eq!(stamp(3_661_001, ','), "01:01:01,001");
    }
}
