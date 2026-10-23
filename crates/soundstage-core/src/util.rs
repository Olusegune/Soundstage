//! Dates, ids and hashing helpers with no external time dependency.

use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Days since 1970-01-01 for a civil date (proleptic Gregorian).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Parse `YYYY-MM-DD` into days since the epoch.
pub fn parse_date(s: &str) -> Option<i64> {
    let mut it = s.trim().split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    if it.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let days = days_from_civil(y, m, d);
    // Reject impossible dates such as 2026-02-31 by round-tripping.
    (civil_from_days(days) == (y, m, d)).then_some(days)
}

pub fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn today_days() -> i64 {
    (unix_now() / 86_400) as i64
}

pub fn date_string(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn today_string() -> String {
    date_string(today_days())
}

pub fn iso_now() -> String {
    let t = unix_now();
    let (days, rem) = ((t / 86_400) as i64, t % 86_400);
    format!("{}T{:02}:{:02}:{:02}Z", date_string(days), rem / 3600, (rem % 3600) / 60, rem % 60)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Short unique id such as `take-3fa91c02`. Unique per process and time; not a secret.
pub fn new_id(prefix: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let h = sha256_hex(format!("{prefix}{nanos}{n}{}", std::process::id()).as_bytes());
    format!("{prefix}-{}", &h[..8])
}

/// Lower-case slug limited to `[a-z0-9-]`, never empty.
pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() { "untitled".into() } else { out.chars().take(40).collect() }
}

/// A file-name-safe version of a user-supplied name (no separators, no reserved characters).
pub fn safe_file_stem(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').to_string();
    if cleaned.is_empty() { "untitled".into() } else { cleaned.chars().take(60).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_round_trip() {
        assert_eq!(parse_date("1970-01-01"), Some(0));
        assert_eq!(date_string(parse_date("2026-10-23").unwrap()), "2026-10-23");
        assert_eq!(parse_date("2026-02-31"), None);
        assert_eq!(parse_date("nope"), None);
        assert_eq!(parse_date("2026-10-23").unwrap() - parse_date("2026-07-24").unwrap(), 91);
    }

    #[test]
    fn ids_are_unique_and_slugs_safe() {
        assert_ne!(new_id("take"), new_id("take"));
        assert_eq!(slugify("  My Film: Ep.1 "), "my-film-ep-1");
        assert_eq!(slugify("***"), "untitled");
        let f = safe_file_stem("../../evil:name");
        assert!(!f.contains('/') && !f.contains('.') && !f.contains(':'));
        assert_eq!(safe_file_stem("   "), "untitled");
    }
}
