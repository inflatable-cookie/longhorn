//! A loopback HTTP fixture server for the GitHub sign-in tests (feature
//! `fixture-server`).
//!
//! It answers recorded routes on `127.0.0.1` with an OS-assigned port, in
//! `http` so no certificate is needed, and records every request's method,
//! path, headers and body. It is test support, not a mock of GitHub's product
//! surface: the production clients run against it through the loopback
//! constructors, and only a loopback address is ever accepted there.
//!
//! A route may answer the same response to every request, or a script of
//! responses in order (the last one repeats). That is what proves a poll
//! sequence without a live host.

use std::{
    collections::VecDeque,
    io::{Read as _, Write as _},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

/// One recorded request. Headers are lowercased by name; the body is the form
/// body or the empty string.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureRequest {
    /// The request method, uppercase.
    pub method: String,
    /// The request path, without a query string.
    pub path: String,
    /// Every request header, names lowercased.
    pub headers: Vec<(String, String)>,
    /// The request body as text.
    pub body: String,
}

impl FixtureRequest {
    /// The value of one header, if the request sent it.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header == name)
            .map(|(_, value)| value.as_str())
    }
}

/// One response a route answers with.
#[derive(Clone, Debug)]
struct FixtureResponse {
    status: u16,
    body: Vec<u8>,
}

/// One served route: a path and the responses it answers with, in order.
pub struct FixtureRoute {
    path: String,
    responses: VecDeque<FixtureResponse>,
}

impl FixtureRoute {
    /// A route that answers every request with the same status and body.
    pub fn new(path: impl Into<String>, status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            path: path.into(),
            responses: VecDeque::from([FixtureResponse {
                status,
                body: body.into(),
            }]),
        }
    }

    /// A route that answers the first request with the first pair, the second
    /// with the second, and every later request with the last pair. An empty
    /// script is a route that is not served: every request for it is a 404.
    pub fn script(path: impl Into<String>, responses: Vec<(u16, String)>) -> Self {
        let responses = responses
            .into_iter()
            .map(|(status, body)| FixtureResponse {
                status,
                body: body.into_bytes(),
            })
            .collect();
        Self {
            path: path.into(),
            responses,
        }
    }

    fn next_response(&mut self) -> Option<FixtureResponse> {
        if self.responses.len() > 1 {
            self.responses.pop_front()
        } else {
            self.responses.front().cloned()
        }
    }
}

fn to_wire(response: &FixtureResponse) -> Vec<u8> {
    let mut wire = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
        response.status,
        reason(response.status),
        response.body.len()
    )
    .into_bytes();
    wire.extend_from_slice(&response.body);
    wire
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        429 => "Too Many Requests",
        _ => "Status",
    }
}

/// A running fixture server. Dropping it stops the server.
pub struct FixtureServer {
    port: u16,
    recorded: Arc<Mutex<Vec<FixtureRequest>>>,
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
        let routes = Arc::new(Mutex::new(routes));
        let worker = {
            let routes = Arc::clone(&routes);
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
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// The absolute URL of one path on this server.
    #[must_use]
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url())
    }

    /// Every request the server answered, in request order.
    #[must_use]
    pub fn requests(&self) -> Vec<FixtureRequest> {
        self.recorded
            .lock()
            .expect("the fixture request record is not poisoned")
            .clone()
    }

    /// How many requests this path received.
    #[must_use]
    pub fn request_count(&self, path: &str) -> usize {
        self.requests()
            .iter()
            .filter(|request| request.path == path)
            .count()
    }

    /// The body of every request this path received, in order.
    #[must_use]
    pub fn bodies_for(&self, path: &str) -> Vec<String> {
        self.requests()
            .into_iter()
            .filter(|request| request.path == path)
            .map(|request| request.body)
            .collect()
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

/// Answers one connection: reads the request, records it, writes the matching
/// route's next response or a 404, and closes.
fn serve(
    mut stream: TcpStream,
    routes: &Mutex<Vec<FixtureRoute>>,
    recorded: &Mutex<Vec<FixtureRequest>>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut request = Vec::new();
    let mut chunk = [0u8; 4096];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => request.extend_from_slice(&chunk[..read]),
        }
    }
    let head_end = request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
        .unwrap_or(request.len());
    let head = String::from_utf8_lossy(&request[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next().unwrap_or_default().split_whitespace();
    let method = request_line.next().unwrap_or_default().to_owned();
    let path = request_line
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default()
        .to_owned();
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    let content_length = headers
        .iter()
        .find(|(name, _)| name == "content-length")
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);
    while request.len() - head_end < content_length {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => request.extend_from_slice(&chunk[..read]),
        }
    }
    let body_end = (head_end + content_length).min(request.len());
    let body = String::from_utf8_lossy(&request[head_end..body_end]).into_owned();
    recorded
        .lock()
        .expect("the fixture request record is not poisoned")
        .push(FixtureRequest {
            method,
            path: path.clone(),
            headers,
            body,
        });
    let wire = {
        let mut routes = routes
            .lock()
            .expect("the fixture route list is not poisoned");
        let response = routes
            .iter_mut()
            .find(|route| route.path == path)
            .and_then(FixtureRoute::next_response)
            .unwrap_or(FixtureResponse {
                status: 404,
                body: Vec::new(),
            });
        to_wire(&response)
    };
    let _ = stream.write_all(&wire);
    let _ = stream.flush();
}
