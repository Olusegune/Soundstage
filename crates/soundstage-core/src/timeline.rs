//! A timeline for video editors, in OpenTimelineIO (OTIO) JSON.
//!
//! The master and stems are for listening and for simple hand-off. An editor who wants to move a line, trim a breath or
//! swap a take needs each clip on its own, in the right place. This builds that: one OTIO file that points at the
//! individual takes and sound files, with silences as gaps.
//!
//! Times are in milliseconds (a rate of 1000), which is exact for everything Soundstage places. Levels and fades have no
//! standard place in OTIO, so they are kept in each clip's `soundstage` metadata for tools and people that want them.

use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub name: String,
    /// Path of the media file, relative to the timeline file.
    pub media: String,
    pub start_ms: u64,
    pub dur_ms: u64,
    pub media_dur_ms: u64,
    pub gain_db: f32,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackSpec {
    pub name: String,
    pub clips: Vec<Clip>,
}

/// Put clips on as few tracks as possible, with no two clips overlapping on one track. Clips are placed in start order,
/// each on the first track where it fits, so layers (music under ambience) get their own tracks.
pub fn assign_lanes(base: &str, mut clips: Vec<Clip>) -> Vec<TrackSpec> {
    clips.sort_by_key(|c| (c.start_ms, c.dur_ms));
    let mut lanes: Vec<(u64, Vec<Clip>)> = Vec::new();
    for c in clips {
        match lanes.iter_mut().find(|(end, _)| *end <= c.start_ms) {
            Some((end, list)) => {
                *end = c.start_ms + c.dur_ms;
                list.push(c);
            }
            None => lanes.push((c.start_ms + c.dur_ms, vec![c])),
        }
    }
    lanes
        .into_iter()
        .enumerate()
        .map(|(i, (_, clips))| TrackSpec { name: if i == 0 { base.to_string() } else { format!("{base} {}", i + 1) }, clips })
        .collect()
}

fn time(ms: u64) -> Value {
    json!({ "OTIO_SCHEMA": "RationalTime.1", "rate": 1000.0, "value": ms as f64 })
}

fn range(start_ms: u64, dur_ms: u64) -> Value {
    json!({ "OTIO_SCHEMA": "TimeRange.1", "start_time": time(start_ms), "duration": time(dur_ms) })
}

fn gap(dur_ms: u64) -> Value {
    json!({ "OTIO_SCHEMA": "Gap.1", "name": "", "source_range": range(0, dur_ms), "effects": [], "markers": [], "enabled": true, "metadata": {} })
}

fn clip(c: &Clip) -> Value {
    json!({
        "OTIO_SCHEMA": "Clip.2",
        "name": c.name,
        "source_range": range(0, c.dur_ms.min(c.media_dur_ms.max(1))),
        "media_reference": {
            "OTIO_SCHEMA": "ExternalReference.1",
            "name": c.name,
            "target_url": c.media,
            "available_range": range(0, c.media_dur_ms),
            "metadata": {},
        },
        "effects": [], "markers": [], "enabled": true,
        "metadata": { "soundstage": { "gain_db": c.gain_db, "fade_in_ms": c.fade_in_ms, "fade_out_ms": c.fade_out_ms } },
    })
}

fn track(t: &TrackSpec) -> Value {
    let mut children = Vec::new();
    let mut cursor = 0u64;
    for c in &t.clips {
        if c.start_ms > cursor {
            children.push(gap(c.start_ms - cursor));
        }
        children.push(clip(c));
        cursor = c.start_ms.max(cursor) + c.dur_ms;
    }
    json!({ "OTIO_SCHEMA": "Track.1", "name": t.name, "kind": "Audio", "children": children, "source_range": null, "effects": [], "markers": [], "enabled": true, "metadata": {} })
}

/// The whole timeline. Empty tracks are left out.
pub fn to_otio(name: &str, app_version: &str, tracks: &[TrackSpec]) -> Value {
    json!({
        "OTIO_SCHEMA": "Timeline.1",
        "name": name,
        "global_start_time": null,
        "metadata": { "soundstage": { "version": app_version, "note": "Dialogue is AI-generated. Times are in milliseconds." } },
        "tracks": {
            "OTIO_SCHEMA": "Stack.1", "name": "tracks", "source_range": null, "effects": [], "markers": [], "enabled": true, "metadata": {},
            "children": tracks.iter().filter(|t| !t.clips.is_empty()).map(track).collect::<Vec<_>>(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(name: &str, start: u64, dur: u64) -> Clip {
        Clip { name: name.into(), media: format!("media/{name}.wav"), start_ms: start, dur_ms: dur, media_dur_ms: dur, gain_db: -6.0, fade_in_ms: 10, fade_out_ms: 20 }
    }

    #[test]
    fn overlapping_clips_get_their_own_tracks_and_touching_clips_share_one() {
        let lanes = assign_lanes("Music", vec![c("a", 0, 1000), c("b", 1000, 500), c("under", 200, 2000), c("late", 5000, 100)]);
        assert_eq!(lanes.len(), 2);
        assert_eq!(lanes[0].name, "Music");
        assert_eq!(lanes[0].clips.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), vec!["a", "b", "late"]);
        assert_eq!((lanes[1].name.as_str(), lanes[1].clips[0].name.as_str()), ("Music 2", "under"));
    }

    #[test]
    fn silences_become_gaps_and_times_are_exact() {
        let v = to_otio("Test", "0.2.0", &[TrackSpec { name: "Dialogue".into(), clips: vec![c("one", 500, 1000), c("two", 2000, 250)] }, TrackSpec { name: "Empty".into(), clips: vec![] }]);
        assert_eq!(v["OTIO_SCHEMA"], "Timeline.1");
        let tracks = v["tracks"]["children"].as_array().unwrap();
        assert_eq!(tracks.len(), 1, "empty tracks are left out");
        let kids = tracks[0]["children"].as_array().unwrap();
        let kinds: Vec<&str> = kids.iter().map(|k| k["OTIO_SCHEMA"].as_str().unwrap()).collect();
        assert_eq!(kinds, vec!["Gap.1", "Clip.2", "Gap.1", "Clip.2"]);
        assert_eq!(kids[0]["source_range"]["duration"]["value"], 500.0);
        assert_eq!(kids[2]["source_range"]["duration"]["value"], 500.0, "the gap between the end of one clip and the start of the next");
        assert_eq!(kids[1]["media_reference"]["target_url"], "media/one.wav");
        assert_eq!(kids[1]["metadata"]["soundstage"]["gain_db"], -6.0);
        assert_eq!(tracks[0]["kind"], "Audio");
    }
}
