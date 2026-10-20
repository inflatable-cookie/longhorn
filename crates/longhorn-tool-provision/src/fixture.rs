//! A loopback fixture server for release tests (feature `fixture-server`).
//!
//! It serves recorded routes at the production URL shapes, on
//! `127.0.0.1` with an OS-assigned port. It is test support, not a mock of
//! GitHub: the production [`HttpReleases`](crate::HttpReleases) runs against
//! it through [`ReleaseEndpoints::loopback_for_tests`](crate::ReleaseEndpoints::loopback_for_tests).
//! It answers one request at a time, records every request path and its
//! headers, and stops when dropped.

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{ReleaseEndpoints, ReleaseError};

/// One recorded request: its path and its headers, names lowercased.
type Recorded = Vec<(String, Vec<(String, String)>)>;

/// One served route: a request path, a status, extra headers and a body.
/// `Content-Length` is written from the body; do not add it here.
#[derive(Clone, Debug)]
pub struct FixtureRoute {
    path: String,
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl FixtureRoute {
    /// A route with no extra headers.
    pub fn new(path: impl Into<String>, status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            path: path.into(),
            status,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    /// Adds one response header. Use it for `Location` redirects and for the
    /// rate-limit headers.
    #[must_use]
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    fn to_wire(&self) -> Vec<u8> {
        let mut head = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
            self.status,
            reason(self.status),
            self.body.len()
        );
        for (name, value) in &self.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str("\r\n");
        let mut wire = head.into_bytes();
        wire.extend_from_slice(&self.body);
        wire
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        302 => "Found",
        403 => "Forbidden",
        404 => "Not Found",
        429 => "Too Many Requests",
        _ => "Status",
    }
}

/// A running fixture server. Dropping it stops the server.
pub struct FixtureServer {
    port: u16,
    recorded: Arc<Mutex<Recorded>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl FixtureServer {
    /// Starts a server on a fresh loopback port with these routes.
    ///
    /// # Errors
    ///
    /// Returns the bind error when the loopback listener cannot start.
    pub fn spawn(routes: Vec<FixtureRoute>) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let recorded = Arc::clone(&recorded);
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                for stream in listener.incoming() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(stream) = stream else {
                        break;
                    };
                    serve(stream, &routes, &recorded);
                }
            })
        };
        Ok(Self {
            port,
            recorded,
            stop,
            worker: Some(worker),
        })
    }

    /// The server's base URL, `http://127.0.0.1:{port}`.
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Endpoints for a release source that uses this server for both the
    /// listing and the downloads.
    ///
    /// # Errors
    ///
    /// Returns a usage fault if the base URL is not loopback (it always is).
    pub fn endpoints(&self) -> Result<ReleaseEndpoints, ReleaseError> {
        let base = self.base_url();
        ReleaseEndpoints::loopback_for_tests(&base, &base)
    }

    /// The path of the stable latest-release listing for a repository.
    pub fn latest_path(owner: &str, repository: &str) -> String {
        format!("/repos/{owner}/{repository}/releases/latest")
    }

    /// The path of one exact release listing, by tag.
    pub fn tag_path(owner: &str, repository: &str, tag: &str) -> String {
        format!("/repos/{owner}/{repository}/releases/tags/{tag}")
    }

    /// The path of one release asset, in the production download shape.
    pub fn asset_path(owner: &str, repository: &str, tag: &str, asset: &str) -> String {
        format!("/{owner}/{repository}/releases/download/{tag}/{asset}")
    }

    /// Every request path the server answered, in request order.
    pub fn request_paths(&self) -> Vec<String> {
        self.lock_recorded()
            .iter()
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// The `Authorization` header of the first request for `path`, if it sent
    /// one. `None` also covers a request for that path that never happened.
    pub fn authorization_for(&self, path: &str) -> Option<String> {
        self.lock_recorded()
            .iter()
            .find(|(recorded, _)| recorded == path)
            .and_then(|(_, headers)| {
                headers
                    .iter()
                    .find(|(name, _)| name == "authorization")
                    .map(|(_, value)| value.clone())
            })
    }

    fn lock_recorded(&self) -> std::sync::MutexGuard<'_, Recorded> {
        self.recorded.lock().expect("fixture request record mutex")
    }
}

impl Drop for FixtureServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop so it sees the stop flag.
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Answers one connection: reads the request head, records it, writes the
/// matching route or a 404, and closes.
fn serve(mut stream: TcpStream, routes: &[FixtureRoute], recorded: &Mutex<Recorded>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut request = Vec::new();
    let mut chunk = [0u8; 4096];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => request.extend_from_slice(&chunk[..read]),
        }
    }
    let text = String::from_utf8_lossy(&request);
    let mut lines = text.split("\r\n");
    let path = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default()
        .to_owned();
    let headers: Vec<(String, String)> = lines
        .map_while(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    recorded
        .lock()
        .expect("fixture request record mutex")
        .push((path.clone(), headers));
    let wire = match routes.iter().find(|route| route.path == path) {
        Some(route) => route.to_wire(),
        None => FixtureRoute::new(path, 404, Vec::new()).to_wire(),
    };
    let _ = stream.write_all(&wire);
    let _ = stream.flush();
}
