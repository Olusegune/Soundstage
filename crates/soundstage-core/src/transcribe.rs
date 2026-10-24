//! Speech to text. A transcript is words with times; subtitles are built from those words, so they line up with the
//! audio the same way the app's own subtitles line up with a mix.
//!
//! The audio that is sent is a 16 kHz mono WAV made on this computer from whatever the user chose, trimmed to a fixed
//! length, so the upload is small and predictable.

use crate::audio::{resample, Stereo, SR};
use crate::output::{to_srt, to_vtt, SubCue};
use serde::Serialize;
use std::io::Cursor;

/// Longest recording that is sent. Longer ones are refused, not silently cut, so a transcript is never half of a file.
pub const MAX_SECONDS: f32 = 25.0 * 60.0;
const RATE: u32 = 16_000;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Word {
    pub text: String,
    pub start: f32,
    pub end: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct Transcript {
    pub text: String,
    pub language: String,
    pub words: Vec<Word>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TranscriptOut {
    pub text: String,
    pub language: String,
    pub srt: String,
    pub vtt: String,
    pub seconds: f32,
    pub has_times: bool,
    /// Subtitle cues with times, for translation and dubbing.
    pub cues: Vec<SubCue>,
}

/// 16 kHz mono 16-bit WAV, or a plain reason the recording cannot be used.
pub fn prepare_audio(audio: &Stereo) -> Result<(Vec<u8>, f32), String> {
    let seconds = audio.len() as f32 / SR as f32;
    if seconds < 0.5 {
        return Err("That recording is too short to transcribe.".into());
    }
    if seconds > MAX_SECONDS {
        return Err(format!("That recording is {:.0} minutes long. Soundstage transcribes up to {:.0} minutes at a time; split it first.", seconds / 60.0, MAX_SECONDS / 60.0));
    }
    let mono: Vec<f32> = (0..audio.len()).map(|i| (audio.l[i] + audio.r[i]) * 0.5).collect();
    let low = resample(&mono, SR, RATE);
    let spec = hound::WavSpec { channels: 1, sample_rate: RATE, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut cur = Cursor::new(Vec::new());
    {
        let mut w = hound::WavWriter::new(&mut cur, spec).map_err(|e| e.to_string())?;
        for s in low {
            w.write_sample((s.clamp(-1.0, 1.0) * 32_767.0).round() as i16).map_err(|e| e.to_string())?;
        }
        w.finalize().map_err(|e| e.to_string())?;
    }
    Ok((cur.into_inner(), seconds))
}

/// Longest recording sent for cleaning.
pub const MAX_CLEAN_SECONDS: f32 = 10.0 * 60.0;

/// 44.1 kHz mono 16-bit WAV for audio cleaning, or a plain reason it cannot be used.
pub fn prepare_isolation(audio: &Stereo) -> Result<Vec<u8>, String> {
    let seconds = audio.len() as f32 / SR as f32;
    if seconds < 0.5 {
        return Err("That recording is too short to clean.".into());
    }
    if seconds > MAX_CLEAN_SECONDS {
        return Err(format!("That recording is {:.0} minutes long. Soundstage cleans up to {:.0} minutes at a time; split it first.", seconds / 60.0, MAX_CLEAN_SECONDS / 60.0));
    }
    let mono: Vec<f32> = (0..audio.len()).map(|i| (audio.l[i] + audio.r[i]) * 0.5).collect();
    let spec = hound::WavSpec { channels: 1, sample_rate: 44_100, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut cur = Cursor::new(Vec::new());
    {
        let mut w = hound::WavWriter::new(&mut cur, spec).map_err(|e| e.to_string())?;
        for s in resample(&mono, SR, 44_100) {
            w.write_sample((s.clamp(-1.0, 1.0) * 32_767.0).round() as i16).map_err(|e| e.to_string())?;
        }
        w.finalize().map_err(|e| e.to_string())?;
    }
    Ok(cur.into_inner())
}

fn clean_word(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect::<String>().trim().chars().take(80).collect()
}

fn lang(v: &serde_json::Value) -> String {
    let l = v.as_str().unwrap_or("");
    if (2..=8).contains(&l.len()) && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') { l.to_string() } else { String::new() }
}

/// ElevenLabs: `{ text, language_code, words: [{ text, type, start, end }] }`. Only items of type `word` are kept.
pub fn parse_elevenlabs(v: &serde_json::Value) -> Result<Transcript, String> {
    let text = v["text"].as_str().ok_or("The transcript was not in the expected format.")?.trim().to_string();
    let words = v["words"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|w| w["type"].as_str().unwrap_or("word") == "word")
                .filter_map(|w| Some(Word { text: clean_word(w["text"].as_str()?), start: w["start"].as_f64()? as f32, end: w["end"].as_f64()? as f32 }))
                .filter(|w| !w.text.is_empty() && w.end >= w.start && w.start >= 0.0)
                .collect()
        })
        .unwrap_or_default();
    Ok(Transcript { text, language: lang(&v["language_code"]), words })
}

/// OpenAI-style `verbose_json`: `{ text, language, words: [{ word, start, end }], segments: [{ text, start, end }] }`.
/// When there are no word times, segments are used as coarse pieces.
pub fn parse_openai(v: &serde_json::Value) -> Result<Transcript, String> {
    let text = v["text"].as_str().ok_or("The transcript was not in the expected format.")?.trim().to_string();
    let pick = |arr: &serde_json::Value, key: &str| -> Vec<Word> {
        arr.as_array()
            .map(|a| a.iter().filter_map(|w| Some(Word { text: clean_word(w[key].as_str()?), start: w["start"].as_f64()? as f32, end: w["end"].as_f64()? as f32 })).filter(|w| !w.text.is_empty() && w.end >= w.start && w.start >= 0.0).collect())
            .unwrap_or_default()
    };
    let mut words = pick(&v["words"], "word");
    if words.is_empty() {
        words = pick(&v["segments"], "text");
    }
    Ok(Transcript { text, language: lang(&v["language"]), words })
}

/// Group words into readable subtitle cues: short lines, split at long gaps and at the end of sentences.
pub fn cues(words: &[Word]) -> Vec<SubCue> {
    let mut out = Vec::new();
    let mut cur: Vec<&Word> = Vec::new();
    let flush = |cur: &mut Vec<&Word>, out: &mut Vec<SubCue>| {
        if let (Some(first), Some(last)) = (cur.first(), cur.last()) {
            let text = cur.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
            out.push(SubCue { start_ms: (first.start * 1000.0) as u64, end_ms: ((last.end * 1000.0) as u64).max((first.start * 1000.0) as u64 + 300), text });
        }
        cur.clear();
    };
    for w in words {
        if let Some(prev) = cur.last() {
            let len: usize = cur.iter().map(|x| x.text.chars().count() + 1).sum();
            let gap = w.start - prev.end;
            let sentence_end = prev.text.ends_with(['.', '!', '?']);
            let too_long = w.end - cur[0].start > 6.0;
            if gap > 0.8 || len > 42 || (sentence_end && len >= 12) || too_long {
                flush(&mut cur, &mut out);
            }
        }
        cur.push(w);
    }
    flush(&mut cur, &mut out);
    out
}

pub fn finish(t: Transcript, seconds: f32) -> TranscriptOut {
    let c = cues(&t.words);
    TranscriptOut { has_times: !c.is_empty(), srt: to_srt(&c), vtt: to_vtt(&c), cues: c, text: t.text, language: t.language, seconds }
}

/// Subtitle text from cues the user may have edited (a translation, for example). Bad times and control characters are
/// refused or removed, so a damaged cue can never produce a broken subtitle file.
pub fn subtitles(cues: &[SubCue], kind: &str) -> Result<String, String> {
    if cues.is_empty() || cues.len() > 5000 {
        return Err("There are no subtitle lines to save.".into());
    }
    let mut clean = Vec::with_capacity(cues.len());
    let mut last_start = 0;
    for c in cues {
        let text: String = c.text.chars().filter(|ch| !ch.is_control() || *ch == '\n').take(500).collect();
        if c.end_ms < c.start_ms || c.end_ms > 24 * 3600 * 1000 || c.start_ms < last_start {
            return Err("The subtitle times are not in order.".into());
        }
        last_start = c.start_ms;
        clean.push(SubCue { start_ms: c.start_ms, end_ms: c.end_ms, text: text.trim().to_string() });
    }
    match kind {
        "srt" => Ok(to_srt(&clean)),
        "vtt" => Ok(to_vtt(&clean)),
        _ => Err("Choose SRT or WebVTT.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(secs: f32) -> Stereo {
        let n = (secs * SR as f32) as usize;
        let s: Vec<f32> = (0..n).map(|i| 0.3 * (i as f32 * 0.05).sin()).collect();
        Stereo { l: s.clone(), r: s }
    }

    #[test]
    fn audio_is_prepared_small_mono_and_long_files_are_refused() {
        let (wav, secs) = prepare_audio(&tone(3.0)).unwrap();
        let r = hound::WavReader::new(Cursor::new(&wav)).unwrap();
        assert_eq!((r.spec().channels, r.spec().sample_rate), (1, 16_000));
        assert!((secs - 3.0).abs() < 0.01);
        assert!(prepare_audio(&tone(0.1)).is_err());
        assert!(prepare_audio(&tone(26.0 * 60.0)).unwrap_err().contains("up to 25 minutes"));
    }

    #[test]
    fn elevenlabs_transcripts_keep_words_only() {
        let v = serde_json::json!({ "text": " Hello there. ", "language_code": "eng", "words": [
            {"text":"Hello","type":"word","start":0.0,"end":0.4},{"text":" ","type":"spacing","start":0.4,"end":0.5},
            {"text":"(laughter)","type":"audio_event","start":0.5,"end":0.9},{"text":"there.","type":"word","start":0.9,"end":1.3},
            {"text":"bad","type":"word","start":2.0,"end":1.0}] });
        let t = parse_elevenlabs(&v).unwrap();
        assert_eq!((t.text.as_str(), t.language.as_str()), ("Hello there.", "eng"));
        assert_eq!(t.words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(), vec!["Hello", "there."], "spacing, audio events and impossible times are dropped");
        assert!(parse_elevenlabs(&serde_json::json!({"nope":1})).is_err());
    }

    #[test]
    fn openai_transcripts_fall_back_to_segments() {
        let w = parse_openai(&serde_json::json!({"text":"a b","language":"en","words":[{"word":"a","start":0,"end":0.2},{"word":"b","start":0.3,"end":0.5}]})).unwrap();
        assert_eq!(w.words.len(), 2);
        let s = parse_openai(&serde_json::json!({"text":"a b","segments":[{"text":" a b ","start":0,"end":1.0}]})).unwrap();
        assert_eq!(s.words[0].text, "a b");
        let none = parse_openai(&serde_json::json!({"text":"just text"})).unwrap();
        assert!(none.words.is_empty() && !finish(none, 1.0).has_times);
    }

    #[test]
    fn subtitles_split_at_gaps_sentences_and_length() {
        let mk = |t: &str, a: f32, b: f32| Word { text: t.into(), start: a, end: b };
        let words = vec![mk("Good", 0.0, 0.3), mk("morning.", 0.3, 0.8), mk("How", 0.9, 1.1), mk("are", 1.1, 1.3), mk("you", 1.3, 1.5), mk("later", 4.0, 4.4)];
        let c = cues(&words);
        assert_eq!(c.iter().map(|x| x.text.as_str()).collect::<Vec<_>>(), vec!["Good morning.", "How are you", "later"]);
        assert!(c.windows(2).all(|p| p[0].end_ms <= p[1].start_ms + 100));
        let out = finish(Transcript { text: "t".into(), language: "en".into(), words }, 5.0);
        assert!(out.srt.starts_with("1\n00:00:00,000 --> 00:00:00,800\nGood morning."));
        assert!(out.vtt.starts_with("WEBVTT"));
        assert_eq!(out.cues.len(), 3);
        let edited: Vec<SubCue> = out.cues.iter().map(|c| SubCue { text: format!("{}\u{7}!", c.text), ..c.clone() }).collect();
        assert!(subtitles(&edited, "srt").unwrap().contains("Good morning.!"), "control characters are removed");
        let mut out_of_order = out.cues.clone();
        out_of_order.swap(0, 2);
        assert!(subtitles(&out_of_order, "srt").is_err() && subtitles(&[], "srt").is_err() && subtitles(&out.cues, "doc").is_err());
    }
}
