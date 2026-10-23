//! A tiny local HTTP server for adapter tests, so nothing here ever touches the network.
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct Req {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}
pub type Handler = Box<dyn Fn(&Req, usize) -> (u16, Vec<u8>) + Send + Sync>;

pub struct Fake {
    pub base: String,
    pub hits: Arc<Mutex<Vec<Req>>>,
}

pub fn start(handler: Handler) -> Fake {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let hits: Arc<Mutex<Vec<Req>>> = Arc::default();
    let h2 = hits.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut s) = stream else { continue };
            let mut data = Vec::new();
            let mut chunk = [0u8; 4096];
            let (head_end, len) = loop {
                let n = s.read(&mut chunk).unwrap_or(0);
                if n == 0 { break (0, 0) }
                data.extend_from_slice(&chunk[..n]);
                if let Some(p) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&data[..p]).to_lowercase();
                    let len = head.lines().find_map(|l| l.strip_prefix("content-length:")).and_then(|v| v.trim().parse().ok()).unwrap_or(0usize);
                    break (p + 4, len);
                }
            };
            while head_end > 0 && data.len() < head_end + len {
                let n = s.read(&mut chunk).unwrap_or(0);
                if n == 0 { break }
                data.extend_from_slice(&chunk[..n]);
            }
            let head = String::from_utf8_lossy(&data[..head_end.max(1) - 1]).to_string();
            let mut lines = head.lines();
            let first = lines.next().unwrap_or("").to_string();
            let mut parts = first.split_whitespace();
            let req = Req {
                method: parts.next().unwrap_or("").into(),
                path: parts.next().unwrap_or("").into(),
                headers: lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_string())).collect(),
                body: String::from_utf8_lossy(&data[head_end.min(data.len())..]).to_string(),
            };
            let idx = { let mut g = h2.lock().unwrap(); g.push(req.clone()); g.len() - 1 };
            let (code, body) = handler(&req, idx);
            let _ = write!(s, "HTTP/1.1 {code} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = s.write_all(&body);
        }
    });
    Fake { base, hits }
}

