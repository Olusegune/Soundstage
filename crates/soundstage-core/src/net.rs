//! Networking rules shared by every adapter. This is the security boundary for outbound traffic.
//!
//! - A server the user configures may be `https` anywhere, but plain `http` only on this computer or a private
//!   network address. Script text is never sent over unencrypted internet connections.
//! - A file a provider tells us to download (a generated song, say) is fetched by `download`, which is https-only,
//!   refuses private, loopback and link-local addresses at connect time (so DNS tricks cannot reach the local
//!   network), follows at most three redirects that each pass the same checks, sends no credentials, and caps size.
//! - Provider error bodies are reduced to a short machine code. They are never shown or logged whole.

use crate::providers::AdapterError;
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseUrl {
    pub https: bool,
    pub host: String,
    pub port: Option<u16>,
    /// No trailing slash. May be empty.
    pub path: String,
}

impl BaseUrl {
    pub fn as_string(&self) -> String {
        let host = if self.host.contains(':') { format!("[{}]", self.host) } else { self.host.clone() };
        format!("{}://{}{}{}", if self.https { "https" } else { "http" }, host, self.port.map(|p| format!(":{p}")).unwrap_or_default(), self.path)
    }
    /// True when the server is on this computer, so nothing leaves it.
    pub fn is_local(&self) -> bool {
        is_loopback_host(&self.host)
    }
}

pub fn is_loopback_host(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    if h == "localhost" || h.ends_with(".localhost") {
        return true;
    }
    h.parse::<IpAddr>().map(|ip| ip.is_loopback()).unwrap_or(false)
}

fn v4_non_public(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.is_unspecified() || ip.is_broadcast() || ip.is_multicast()
        || (o[0] == 100 && (64..=127).contains(&o[1])) // carrier-grade NAT
        || (o[0] == 192 && o[1] == 0 && o[2] == 0)
        || (o[0] == 192 && o[1] == 0 && o[2] == 2) || (o[0] == 198 && o[1] == 51 && o[2] == 100) || (o[0] == 203 && o[1] == 0 && o[2] == 113)
        || o[0] == 0 || o[0] >= 240
}

fn v6_non_public(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return v4_non_public(v4);
    }
    let s = ip.segments();
    ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() || (s[0] & 0xfe00) == 0xfc00 /* unique local */ || (s[0] & 0xffc0) == 0xfe80 /* link local */
}

/// Not a public internet address (loopback, private, link-local, reserved, multicast, carrier-grade NAT).
pub fn is_non_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => v4_non_public(v),
        IpAddr::V6(v) => v6_non_public(v),
    }
}

fn is_private_literal(host: &str) -> bool {
    host.parse::<IpAddr>().map(|ip| match ip { IpAddr::V4(v) => v.is_private() || v.is_link_local(), IpAddr::V6(v) => (v.segments()[0] & 0xfe00) == 0xfc00 }).unwrap_or(false)
}

fn valid_host(h: &str) -> bool {
    if h.is_empty() || h.len() > 253 {
        return false;
    }
    if h.parse::<IpAddr>().is_ok() {
        return true;
    }
    h.split('.').all(|l| !l.is_empty() && l.len() <= 63 && !l.starts_with('-') && !l.ends_with('-') && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
}

/// Parse and vet a server address typed by the user.
pub fn parse_base_url(input: &str) -> Result<BaseUrl, String> {
    let s = input.trim();
    if s.is_empty() || s.len() > 300 || s.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("Enter a web address such as http://127.0.0.1:8880/v1".into());
    }
    let (https, rest) = if let Some(r) = s.strip_prefix("https://") { (true, r) } else if let Some(r) = s.strip_prefix("http://") { (false, r) } else {
        return Err("The address must start with http:// or https://".into());
    };
    if rest.contains('?') || rest.contains('#') {
        return Err("Leave out anything after a ? or #.".into());
    }
    let (authority, path) = match rest.find('/') { Some(i) => (&rest[..i], &rest[i..]), None => (rest, "") };
    if authority.contains('@') {
        return Err("Addresses with a username or password in them are not allowed. Put keys in the key field.".into());
    }
    let (host, port) = if let Some(r) = authority.strip_prefix('[') {
        let end = r.find(']').ok_or("That address is not valid.")?;
        let after = &r[end + 1..];
        (r[..end].to_string(), if after.is_empty() { None } else { Some(after.strip_prefix(':').ok_or("That address is not valid.")?.parse::<u16>().map_err(|_| "The port is not valid.")?) })
    } else if let Some((h, p)) = authority.rsplit_once(':') {
        (h.to_string(), Some(p.parse::<u16>().map_err(|_| "The port is not valid.")?))
    } else {
        (authority.to_string(), None)
    };
    if !valid_host(&host) {
        return Err("That address is not valid.".into());
    }
    if path.contains("..") || path.contains("//") {
        return Err("That address is not valid.".into());
    }
    let b = BaseUrl { https, host: host.to_ascii_lowercase(), port, path: path.trim_end_matches('/').to_string() };
    if !b.https && !(is_loopback_host(&b.host) || is_private_literal(&b.host)) {
        return Err("Servers on the internet must use https://. Plain http:// is only allowed on this computer or a private network.".into());
    }
    Ok(b)
}

/* ------------------------------------------------------------------ errors and JSON */

/// Short machine code from an error body, limited to a safe alphabet. The body itself is never kept.
pub fn error_code(r: ureq::Response) -> String {
    let mut buf = Vec::new();
    let _ = r.into_reader().take(16 * 1024).read_to_end(&mut buf);
    let v: serde_json::Value = serde_json::from_slice(&buf).unwrap_or_default();
    let d = &v["detail"];
    let raw = d["code"].as_str().or_else(|| d["status"].as_str()).or_else(|| v["error"]["code"].as_str()).or_else(|| v["code"].as_str()).unwrap_or("");
    raw.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(60).collect()
}

/// Map an HTTP failure into the common error set.
pub fn error_from_status(code: u16, r: ureq::Response) -> AdapterError {
    let c = error_code(r);
    if c.contains("quota") || c.contains("insufficient") || code == 402 {
        return AdapterError::Quota;
    }
    match code {
        401 | 403 => AdapterError::Auth,
        404 => AdapterError::UnknownVoice(if c.is_empty() { "not found".into() } else { c }),
        422 | 400 => AdapterError::ProviderError("the text or settings were rejected".into()),
        429 => AdapterError::RateLimit,
        _ => AdapterError::ProviderError(format!("HTTP {code} {c}").trim().to_string()),
    }
}

pub fn read_limited(r: ureq::Response, max: usize) -> Result<Vec<u8>, AdapterError> {
    let mut buf = Vec::new();
    r.into_reader().take(max as u64 + 1).read_to_end(&mut buf).map_err(|_| AdapterError::Network)?;
    if buf.len() > max {
        return Err(AdapterError::ProviderError("the response was larger than expected".into()));
    }
    Ok(buf)
}

pub fn read_json(r: ureq::Response, max: usize) -> Result<serde_json::Value, AdapterError> {
    serde_json::from_slice(&read_limited(r, max)?).map_err(|_| AdapterError::ProviderError("the response was not understood".into()))
}

/// Standard base64, ignoring whitespace. `None` for anything that is not valid base64.
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits, mut pad) = (0u32, 0u32, 0usize);
    for c in s.bytes().filter(|b| !b.is_ascii_whitespace()) {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => {
                pad += 1;
                continue;
            }
            _ => return None,
        };
        if pad > 0 {
            return None;
        }
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    (pad <= 2).then_some(out)
}

pub fn url_encode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/* ------------------------------------------------------------------ downloading provider-supplied files */

struct PublicOnly;

impl ureq::Resolver for PublicOnly {
    fn resolve(&self, netloc: &str) -> std::io::Result<Vec<SocketAddr>> {
        let all: Vec<SocketAddr> = netloc.to_socket_addrs()?.collect();
        let ok: Vec<SocketAddr> = all.into_iter().filter(|a| !is_non_public(a.ip())).collect();
        if ok.is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "that address is not on the public internet"));
        }
        Ok(ok)
    }
}

/// Vet a URL a provider handed back. Returns the host.
pub fn check_download_url(url: &str, allowed_suffixes: &[&str]) -> Result<String, String> {
    let b = parse_base_url(url).map_err(|_| "The provider returned an address Soundstage will not fetch.".to_string())?;
    if !b.https {
        return Err("The provider returned a non-https download address, which Soundstage will not fetch.".into());
    }
    if b.host.parse::<IpAddr>().is_ok() || is_loopback_host(&b.host) || b.host.ends_with(".local") || b.host.ends_with(".internal") || !b.host.contains('.') {
        return Err("The provider returned a download address that is not a public website name.".into());
    }
    if !allowed_suffixes.is_empty() && !allowed_suffixes.iter().any(|s| b.host == s.trim_start_matches('.') || b.host.ends_with(s)) {
        return Err(format!("The download address ({}) is not on the provider's known file hosts.", b.host));
    }
    Ok(b.host)
}

pub fn download(url: &str, allowed_suffixes: &[&str], max_bytes: usize) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new().resolver(PublicOnly).redirects(0).timeout_connect(Duration::from_secs(10)).timeout_read(Duration::from_secs(120)).user_agent("Soundstage").build();
    let mut current = url.to_string();
    for _ in 0..4 {
        check_download_url(&current, allowed_suffixes)?;
        match agent.get(&current).call() {
            Ok(r) if r.status() == 200 => return read_limited(r, max_bytes).map_err(|e| e.to_string()),
            Ok(r) if matches!(r.status(), 301 | 302 | 303 | 307 | 308) => {
                current = r.header("location").ok_or("The download redirected without saying where.")?.to_string();
            }
            Ok(r) => return Err(format!("The download failed (HTTP {}).", r.status())),
            Err(ureq::Error::Status(code, _)) => return Err(format!("The download failed (HTTP {code}).")),
            Err(_) => return Err("The download could not be reached.".into()),
        }
    }
    Err("The download redirected too many times.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_urls_are_vetted() {
        let ok = parse_base_url("http://127.0.0.1:8880/v1/").unwrap();
        assert_eq!((ok.https, ok.host.as_str(), ok.port, ok.path.as_str()), (false, "127.0.0.1", Some(8880), "/v1"));
        assert!(ok.is_local());
        assert_eq!(ok.as_string(), "http://127.0.0.1:8880/v1");
        assert!(parse_base_url("http://localhost:11434").unwrap().is_local());
        assert!(parse_base_url("http://[::1]:8000/v1").unwrap().is_local());
        assert!(!parse_base_url("http://192.168.1.20:8000").unwrap().is_local(), "a LAN server is allowed but not 'local'");
        assert!(parse_base_url("https://openrouter.ai/api/v1").is_ok());
    }

    #[test]
    fn unsafe_addresses_are_refused_with_a_reason() {
        for bad in [
            "http://example.com/v1", "ftp://x.com", "127.0.0.1:8000", "https://user:pw@example.com", "https://example.com/?a=1", "https://example.com/#x",
            "https://exa mple.com", "https://-bad.com", "http://", "https://example.com:99999", "https://example.com/../x", "", "https://ex\u{0}ample.com",
        ] {
            assert!(parse_base_url(bad).is_err(), "{bad:?} should be refused");
        }
        assert!(parse_base_url("http://example.com").unwrap_err().contains("https"));
    }

    #[test]
    fn non_public_addresses_are_recognised() {
        for ip in ["127.0.0.1", "10.1.2.3", "172.16.0.1", "192.168.1.1", "169.254.169.254", "100.64.0.1", "0.0.0.0", "224.0.0.1", "::1", "fd00::1", "fe80::1", "::ffff:127.0.0.1", "::ffff:10.0.0.1", "240.0.0.1"] {
            assert!(is_non_public(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "1.1.1.1", "93.184.216.34", "2606:4700:4700::1111", "172.32.0.1"] {
            assert!(!is_non_public(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn the_resolver_refuses_local_names_so_dns_tricks_cannot_reach_the_network() {
        use ureq::Resolver;
        assert!(PublicOnly.resolve("127.0.0.1:443").is_err());
        assert!(PublicOnly.resolve("localhost:443").is_err());
        assert!(PublicOnly.resolve("[::1]:443").is_err());
        assert!(PublicOnly.resolve("192.168.0.5:443").is_err());
        assert!(PublicOnly.resolve("169.254.169.254:80").is_err(), "cloud metadata address");
    }

    #[test]
    fn download_urls_are_checked_before_any_request() {
        assert!(check_download_url("https://v3.fal.media/files/a.mp3", &[".fal.media", "fal.media"]).is_ok());
        assert!(check_download_url("https://fal.media/a.mp3", &[".fal.media", "fal.media"]).is_ok());
        for bad in ["http://v3.fal.media/a.mp3", "https://127.0.0.1/a.mp3", "https://localhost/a.mp3", "https://printer.local/a", "https://intranet/a", "https://evil.example.com/a.mp3", "https://fal.media.evil.com/a.mp3", "https://8.8.8.8/a"] {
            assert!(check_download_url(bad, &[".fal.media", "fal.media"]).is_err(), "{bad}");
        }
        assert!(check_download_url("https://cdn.example.com/a.mp3", &[]).is_ok(), "no allowlist means any public website name");
        assert!(check_download_url("https://169.254.169.254/latest", &[]).is_err());
    }

    #[test]
    fn base64_decodes_known_vectors_and_rejects_junk() {
        assert_eq!(base64_decode("").unwrap(), b"");
        assert_eq!(base64_decode("Zg==").unwrap(), b"f");
        assert_eq!(base64_decode("Zm9vYmFy").unwrap(), b"foobar");
        assert_eq!(base64_decode("Zm9v\nYmFy ").unwrap(), b"foobar");
        assert!(base64_decode("Zm9v!").is_none());
        assert!(base64_decode("Zg==Zg").is_none());
    }

    #[test]
    fn url_encoding() {
        assert_eq!(url_encode("a b/c"), "a%20b%2Fc");
    }
}

/// A multipart/form-data body with text fields and one file. Field values are ours (never raw user bytes in a header),
/// and the boundary is random, so nothing in the audio can end a part early.
pub fn multipart(fields: &[(&str, &str)], file: (&str, &str, &str, &[u8])) -> (String, Vec<u8>) {
    let boundary = format!("----soundstage{}", crate::util::new_id("b"));
    let mut b = Vec::new();
    for (k, v) in fields {
        let v: String = v.chars().filter(|c| !c.is_control()).collect();
        b.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{k}\"\r\n\r\n{v}\r\n").as_bytes());
    }
    let (field, filename, ctype, bytes) = file;
    b.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"{filename}\"\r\nContent-Type: {ctype}\r\n\r\n").as_bytes());
    b.extend_from_slice(bytes);
    b.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (boundary, b)
}
