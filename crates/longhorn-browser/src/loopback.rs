//! The loopback listener that receives an RFC 8252 redirect.
//!
//! The other half of this crate's capability: [`launch`](crate::launch) sends
//! the operator to the system browser, and this is where the authorization
//! server sends them back. Host-agnostic for the same reason the launch is —
//! neither backend supplies it, so Longhorn implements it once.
//!
//! # What this does and does not check
//!
//! The listener extracts a [`Callback`] and nothing more. State validation is
//! `AccountFlow::accept_callback`'s, in constant time, and duplicating it here
//! would be a second implementation of the check that matters most. The
//! consequence is a deliberate trade: a hostile local process that races a
//! bogus callback onto the port makes the sign-in **fail closed** — the flow
//! consumes the bad callback, rejects its state, and the operator retries.
//! Nothing is exchangeable: state binds the callback to the flow, and PKCE
//! binds the code to the verifier this process holds.
//!
//! # Containment
//!
//! - Binds `127.0.0.1` only, never a routable interface.
//! - Reads a bounded request head from one connection at a time.
//! - Answers non-callback paths (a favicon probe, a scanner) with 404 and
//!   keeps waiting; only the callback path resolves the wait.
//! - The response page is static. Nothing from the request is echoed into it,
//!   so the page cannot be a reflection vector.

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};

use longhorn_licence::{Callback, CallbackOutcome};

/// The most request head a callback redirect can need.
///
/// An authorization redirect is a short GET; a request that exceeds this is
/// not one, whatever it is.
const MAXIMUM_HEAD_BYTES: usize = 8 * 1024;

/// The page the operator sees after the redirect lands.
///
/// Static on purpose — see the module note on reflection.
const RESPONSE_PAGE: &str = "<!doctype html><html><head><meta charset=\"utf-8\">\
<title>Sign-in complete</title></head><body style=\"font-family:sans-serif;\
margin:4rem auto;max-width:28rem\"><h1>Sign-in complete</h1>\
<p>You can close this tab and return to the application.</p></body></html>";

/// A one-flow loopback listener.
pub struct LoopbackRedirect {
    listener: TcpListener,
    port: u16,
}

/// Why no callback was produced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopbackError {
    /// The port could not be bound or the socket failed mid-exchange.
    Io {
        /// What the platform reported.
        detail: String,
    },
    /// Nothing arrived on the callback path before the deadline.
    TimedOut,
    /// A request reached the callback path without a usable callback in it.
    ///
    /// Fail closed rather than keep waiting: something is speaking to the
    /// port wrongly, and treating that as noise would leave the operator
    /// staring at a browser that says done and an application that says
    /// nothing.
    MalformedCallback,
}

impl std::fmt::Display for LoopbackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { detail } => write!(formatter, "loopback listener failed: {detail}"),
            Self::TimedOut => formatter.write_str("no authorization callback arrived in time"),
            Self::MalformedCallback => {
                formatter.write_str("the authorization callback could not be read")
            }
        }
    }
}

impl std::error::Error for LoopbackError {}

impl LoopbackRedirect {
    /// Binds an ephemeral loopback port.
    ///
    /// Ephemeral rather than fixed: RFC 8252 permits a variable loopback
    /// port precisely so native applications do not fight over one, and the
    /// flow's redirect URI embeds whatever was bound.
    pub fn bind() -> Result<Self, LoopbackError> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(io_error)?;
        let port = listener.local_addr().map_err(io_error)?.port();
        listener.set_nonblocking(true).map_err(io_error)?;
        Ok(Self { listener, port })
    }

    /// The port the flow's redirect URI must name.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Waits for the redirect and returns its callback.
    ///
    /// Blocks the calling thread up to `timeout`; a host runs it on a worker
    /// while the operator is in the browser. Consumes the listener — one
    /// flow, one callback, and the port closes either way.
    pub fn receive(self, timeout: Duration) -> Result<Callback, LoopbackError> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if let Some(callback) = answer(stream)? {
                        return Ok(callback);
                    }
                    // A non-callback path: answered 404, keep waiting.
                }
                Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err(LoopbackError::TimedOut);
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(io_error(error)),
            }
        }
    }
}

fn io_error(error: std::io::Error) -> LoopbackError {
    LoopbackError::Io {
        detail: error.to_string(),
    }
}

/// Answers one connection; `Some` when it carried the callback.
fn answer(mut stream: TcpStream) -> Result<Option<Callback>, LoopbackError> {
    // The listener is non-blocking so the accept loop can watch its deadline,
    // and on macOS the accepted socket inherits that -- the packaged update
    // proof paid a build cycle to learn it. Reads and writes here must block.
    stream.set_nonblocking(false).map_err(io_error)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(io_error)?;

    let mut head = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let count = stream.read(&mut buffer).map_err(io_error)?;
        if count == 0 {
            break;
        }
        head.extend_from_slice(&buffer[..count]);
        if head.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if head.len() > MAXIMUM_HEAD_BYTES {
            return Err(LoopbackError::MalformedCallback);
        }
    }

    let text = String::from_utf8_lossy(&head);
    let mut parts = text.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();

    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    };

    if method != "GET" || path != "/callback" {
        respond(&mut stream, "404 Not Found", "")?;
        return Ok(None);
    }

    let callback = parse_callback(query);
    // The browser gets its page whether or not the query parsed: the
    // operator's tab is not the place to debug a malformed redirect.
    respond(&mut stream, "200 OK", RESPONSE_PAGE)?;
    callback.map(Some).ok_or(LoopbackError::MalformedCallback)
}

/// Reads a callback out of the redirect's query string.
///
/// `state` plus either `code` or `error` is a callback; anything else is not.
/// `error_description` is preferred over the bare `error` code for the denied
/// reason, because it is the human sentence the server chose to send.
fn parse_callback(query: &str) -> Option<Callback> {
    let mut state = None;
    let mut code = None;
    let mut error = None;
    let mut description = None;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value);
        match key {
            "state" => state = Some(value),
            "code" => code = Some(value),
            "error" => error = Some(value),
            "error_description" => description = Some(value),
            _ => {}
        }
    }
    let state = state?;
    if let Some(code) = code {
        if code.is_empty() {
            return None;
        }
        return Some(Callback {
            state,
            outcome: CallbackOutcome::Code(code),
        });
    }
    error.map(|error| Callback {
        state,
        outcome: CallbackOutcome::Denied {
            reason: description.unwrap_or(error),
        },
    })
}

/// Decodes `%xx` and `+`, which is all a query component needs.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'+' => {
                output.push(b' ');
                at += 1;
            }
            b'%' if at + 2 < bytes.len() => {
                let decoded = u8::from_str_radix(
                    std::str::from_utf8(&bytes[at + 1..at + 3]).unwrap_or_default(),
                    16,
                );
                match decoded {
                    Ok(byte) => {
                        output.push(byte);
                        at += 3;
                    }
                    Err(_) => {
                        output.push(b'%');
                        at += 1;
                    }
                }
            }
            other => {
                output.push(other);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}

fn respond(stream: &mut TcpStream, status: &str, body: &str) -> Result<(), LoopbackError> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).map_err(io_error)?;
    stream.flush().map_err(io_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sends one request to a live listener and returns the response body.
    fn exchange(port: u16, target: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .write_all(format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").as_bytes())
            .expect("send");
        let mut response = String::new();
        drop(stream.read_to_string(&mut response));
        response
    }

    fn receive_in_background(
        listener: LoopbackRedirect,
    ) -> std::thread::JoinHandle<Result<Callback, LoopbackError>> {
        std::thread::spawn(move || listener.receive(Duration::from_secs(5)))
    }

    /// The whole exchange over a real socket: browser-shaped GET in,
    /// callback out, page back.
    #[test]
    fn a_code_callback_round_trips() {
        let listener = LoopbackRedirect::bind().expect("bind");
        let port = listener.port();
        let handle = receive_in_background(listener);

        let response = exchange(port, "/callback?state=sixteen-byte-state&code=authcode");

        assert!(response.contains("200 OK"));
        assert!(response.contains("close this tab"));
        assert_eq!(
            handle.join().expect("join").expect("callback"),
            Callback {
                state: "sixteen-byte-state".to_owned(),
                outcome: CallbackOutcome::Code("authcode".to_owned()),
            }
        );
    }

    /// A denial carries the server's human sentence, percent-decoded.
    #[test]
    fn a_denied_callback_carries_the_description() {
        let listener = LoopbackRedirect::bind().expect("bind");
        let port = listener.port();
        let handle = receive_in_background(listener);

        exchange(
            port,
            "/callback?state=sixteen-byte-state&error=access_denied&error_description=The+user+declined%2E",
        );

        assert_eq!(
            handle.join().expect("join").expect("callback"),
            Callback {
                state: "sixteen-byte-state".to_owned(),
                outcome: CallbackOutcome::Denied {
                    reason: "The user declined.".to_owned(),
                },
            }
        );
    }

    /// A favicon probe must not resolve the wait; the callback after it must.
    #[test]
    fn a_probe_is_answered_and_ignored() {
        let listener = LoopbackRedirect::bind().expect("bind");
        let port = listener.port();
        let handle = receive_in_background(listener);

        let probe = exchange(port, "/favicon.ico");
        assert!(probe.contains("404"));

        exchange(port, "/callback?state=sixteen-byte-state&code=after-probe");
        assert_eq!(
            handle.join().expect("join").expect("callback").outcome,
            CallbackOutcome::Code("after-probe".to_owned())
        );
    }

    /// The page must not reflect anything from the request. A state carrying
    /// markup arrives in the callback and never in the browser tab.
    #[test]
    fn nothing_from_the_request_reaches_the_response() {
        let listener = LoopbackRedirect::bind().expect("bind");
        let port = listener.port();
        let handle = receive_in_background(listener);

        let response = exchange(
            port,
            "/callback?state=%3Cscript%3Ealert(1)%3C%2Fscript%3E&code=x",
        );

        assert!(!response.contains("script"));
        assert_eq!(
            handle.join().expect("join").expect("callback").state,
            "<script>alert(1)</script>"
        );
    }

    /// A callback path with no usable callback fails closed.
    #[test]
    fn a_malformed_callback_fails_rather_than_waits() {
        let listener = LoopbackRedirect::bind().expect("bind");
        let port = listener.port();
        let handle = receive_in_background(listener);

        exchange(port, "/callback?unrelated=1");

        assert_eq!(
            handle.join().expect("join"),
            Err(LoopbackError::MalformedCallback)
        );
    }

    #[test]
    fn nothing_arriving_times_out() {
        let listener = LoopbackRedirect::bind().expect("bind");
        assert_eq!(
            listener.receive(Duration::from_millis(50)),
            Err(LoopbackError::TimedOut)
        );
    }
}
