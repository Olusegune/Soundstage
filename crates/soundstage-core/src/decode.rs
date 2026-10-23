//! Decode MP3, FLAC, Ogg Vorbis, AAC/M4A and WAV into float samples (pure Rust, no external tools).
//! Providers return many formats; the rest of Soundstage works with 48 kHz stereo WAV.

use crate::audio::{read_wav, Decoded, MAX_IMPORT_SECONDS};
use std::io::Cursor;
use std::path::Path;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub fn decode_bytes(bytes: &[u8], ext_hint: Option<&str>) -> Result<Decoded, String> {
    if bytes.len() < 16 {
        return Err("The audio was empty or too short to read.".into());
    }
    let mss = MediaSourceStream::new(Box::new(Cursor::new(bytes.to_vec())), Default::default());
    let mut hint = Hint::new();
    if let Some(e) = ext_hint.filter(|e| e.len() <= 5 && e.bytes().all(|b| b.is_ascii_alphanumeric())) {
        hint.with_extension(e);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|_| "That audio format is not one Soundstage can read.".to_string())?;
    let mut format = probed.format;
    let track = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL).ok_or("The file has no audio in it.")?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).map_err(|_| "That audio codec is not supported.".to_string())?;

    let mut channels: Vec<Vec<f32>> = Vec::new();
    let mut rate = 0u32;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(Error::ResetRequired) => break,
            Err(_) => break,
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buf) => {
                let spec = *buf.spec();
                rate = spec.rate;
                let n = spec.channels.count();
                if channels.is_empty() {
                    channels = vec![Vec::new(); n.clamp(1, 8)];
                }
                let mut sb = symphonia::core::audio::SampleBuffer::<f32>::new(buf.capacity() as u64, spec);
                sb.copy_interleaved_ref(buf);
                let data = sb.samples();
                for (i, s) in data.iter().enumerate() {
                    let c = i % n;
                    if c < channels.len() {
                        channels[c].push(*s);
                    }
                }
                if channels[0].len() as u64 > MAX_IMPORT_SECONDS * rate.max(1) as u64 {
                    return Err("The audio is longer than 30 minutes, which Soundstage does not load.".into());
                }
            }
            Err(Error::DecodeError(_)) => continue, // one bad packet is not the end of the file
            Err(_) => break,
        }
    }
    if channels.is_empty() || channels[0].is_empty() || rate == 0 {
        return Err("No audio could be read from the file.".into());
    }
    Ok(Decoded { channels, rate })
}

/// Read an audio file of any supported format. WAV keeps its fast, exact path.
pub fn decode_file(path: &Path) -> Result<Decoded, String> {
    let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase());
    if ext.as_deref() == Some("wav") {
        if let Ok(d) = read_wav(path) {
            return Ok(d);
        }
    }
    let meta = std::fs::metadata(path).map_err(|e| format!("Could not open that file: {e}"))?;
    if meta.len() > 500 * 1024 * 1024 {
        return Err("That file is larger than 500 MB.".into());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("Could not read that file: {e}"))?;
    decode_bytes(&bytes, ext.as_deref())
}

/// Any supported audio, as the 48 kHz stereo the mix uses.
pub fn to_stereo_48k(bytes: &[u8], ext_hint: Option<&str>) -> Result<crate::audio::Stereo, String> {
    Ok(crate::audio::to_mix_format(&decode_bytes(bytes, ext_hint)?))
}

pub const IMPORT_EXTENSIONS: &[&str] = &["wav", "mp3", "flac", "ogg", "m4a", "aac", "mp4"];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{write_wav_16_mono, SR};
    use crate::output::{Codec, Ffmpeg};
    use crate::util::new_id;

    fn tone() -> Vec<f32> {
        (0..SR as usize).map(|n| 0.4 * (2.0 * std::f32::consts::PI * 440.0 * n as f32 / SR as f32).sin()).collect()
    }

    #[test]
    fn wav_bytes_decode() {
        let d = std::env::temp_dir().join(new_id("ss-dec"));
        std::fs::create_dir_all(&d).unwrap();
        write_wav_16_mono(&d.join("a.wav"), &tone(), 22_050).unwrap();
        let got = decode_bytes(&std::fs::read(d.join("a.wav")).unwrap(), Some("wav")).unwrap();
        assert_eq!((got.rate, got.channels.len()), (22_050, 1));
        assert_eq!(got.channels[0].len(), SR as usize);
        assert!((got.channels[0][100] - tone()[100]).abs() < 0.01);
        assert!(decode_file(&d.join("a.wav")).is_ok());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn garbage_empty_and_html_are_refused_politely() {
        for bad in [&b""[..], b"not audio at all, just some text bytes here", b"<html><body>Rate limited</body></html>"] {
            assert!(decode_bytes(bad, Some("mp3")).is_err());
        }
        assert!(decode_bytes(&[0u8; 4096], None).is_err());
    }

    #[test]
    fn real_mp3_flac_ogg_and_aac_decode_to_the_right_length_and_level() {
        let Some(ff) = Ffmpeg::detect(None) else { eprintln!("FFmpeg not found; skipping"); return };
        let d = std::env::temp_dir().join(new_id("ss-dec2"));
        std::fs::create_dir_all(&d).unwrap();
        let wav = d.join("src.wav");
        write_wav_16_mono(&wav, &tone(), SR).unwrap();
        let mut tried = 0;
        for (codec, ext) in [(Codec::Mp3 { kbps: 192 }, "mp3"), (Codec::Flac, "flac"), (Codec::Vorbis { quality: 5 }, "ogg"), (Codec::Aac { kbps: 192 }, "m4a")] {
            if !ff.supports(&codec) { continue; }
            let out = d.join(format!("o.{ext}"));
            ff.encode(&wav, &out, &codec, "t").unwrap();
            let got = decode_file(&out).unwrap_or_else(|e| panic!("{ext}: {e}"));
            let secs = got.channels[0].len() as f32 / got.rate as f32;
            assert!((secs - 1.0).abs() < 0.15, "{ext} length {secs}");
            let peak = got.channels[0].iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!((peak - 0.4).abs() < 0.08, "{ext} peak {peak}");
            tried += 1;
        }
        assert!(tried >= 2);
        std::fs::remove_dir_all(d).ok();
    }
}
