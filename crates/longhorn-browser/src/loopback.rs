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
//!   keeps waiting; only the callback path resolves the wait. A connection
//!   that hangs up, resets, or says nothing at all is noise on the same
//!   terms — otherwise opening a socket would be enough to cancel a sign-in.
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

/// How long one connection may say nothing before it is treated as a probe.
///
/// A browser following a redirect sends its GET immediately, so this only
/// ever expires on traffic that was never the callback. It also bounds how
/// long a silent peer occupies the accept loop.
const PROBE_READ_TIMEOUT: Duration = Duration::from_secs(5);

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
    ///
    /// Probe traffic — a local scanner connecting and hanging up, a request
    /// to any other path — is answered and ignored, not fatal. Only the
    /// callback path fails closed.
    pub fn receive(self, timeout: Duration) -> Result<Callback, LoopbackError> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => match answer(stream, deadline)? {
                    Answer::Callback(callback) => return Ok(callback),
                    // A non-callback path: answered 404, keep waiting.
                    Answer::Probe => {}
                },
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

/// What one connection turned out to be.
enum Answer {
    /// It carried the callback.
    Callback(Callback),
    /// Probe traffic: answered 404 where possible, then closed. A dead
    /// connection, a silent one, or a long garbage head is noise, and killing
    /// the sign-in flow over it would let any local process cancel logins —
    /// by hanging up abruptly or by saying nothing at all, neither of which
    /// needs more than an open socket.
    Probe,
}

/// Answers one connection, returning what it carried.
///
/// The per-read timeout bounds a slow peer; `deadline` bounds the whole
/// connection, so a byte at a time cannot hold the flow open past it.
fn answer(mut stream: TcpStream, deadline: Instant) -> Result<Answer, LoopbackError> {
    // The listener is non-blocking so the accept loop can watch its deadline,
    // and on macOS the accepted socket inherits that -- the packaged update
    // proof paid a build cycle to learn it. Reads and writes here must block.
    stream.set_nonblocking(false).map_err(io_error)?;
    stream
        .set_read_timeout(Some(PROBE_READ_TIMEOUT))
        .map_err(io_error)?;

    let mut head = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(LoopbackError::TimedOut);
        }
        // Each read is bounded by the flow's own deadline, so a peer
        // dribbling a byte at a time cannot hold the connection past it.
        stream
            .set_read_timeout(Some(remaining.min(PROBE_READ_TIMEOUT)))
            .map_err(io_error)?;
        match stream.read(&mut buffer) {
            Ok(0) => return Ok(Answer::Probe),
            Ok(count) => head.extend_from_slice(&buffer[..count]),
            // Any read failure short of the flow deadline is a probe. A clean
            // hang-up arrives as `Ok(0)`, but an abrupt one arrives as
            // `ECONNRESET`, and a peer that connects and then says nothing
            // arrives as the per-read timeout -- and treating either as fatal
            // would let any local process cancel a sign-in by opening a socket
            // and walking away. Only the flow's own deadline ends the wait.
            //
            // The cost is that one silent connection occupies the accept loop
            // for up to `PROBE_READ_TIMEOUT`; the flow outlives that, and a
            // real callback is accepted as soon as it clears.
            Err(_) => {
                if Instant::now() >= deadline {
                    return Err(LoopbackError::TimedOut);
                }
                return Ok(Answer::Probe);
            }
        }
        if head.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if head.len() > MAXIMUM_HEAD_BYTES {
            // An oversized head that was never a callback is a probe. One
            // addressed at the callback fails closed.
            if head.starts_with(b"GET /callback") {
                return Err(LoopbackError::MalformedCallback);
            }
            return Ok(Answer::Probe);
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
        // The 404 is courtesy; a probe that hung up already must not turn
        // the write failure into a flow failure.
        drop(respond(&mut stream, "404 Not Found", ""));
        return Ok(Answer::Probe);
    }

    let callback = parse_callback(query);
    // The browser gets its page whether or not the query parsed: the
    // operator's tab is not the place to debug a malformed redirect.
    respond(&mut stream, "200 OK", RESPONSE_PAGE)?;
    callback
        .map(Answer::Callback)
        .ok_or(LoopbackError::MalformedCallback)
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

    /// A scanner that connects and hangs up mid-request must not kill the
    /// wait. Dropping the stream sends a FIN, which the read sees as `Ok(0)`.
    #[test]
    fn a_probe_disconnecting_mid_request_does_not_end_the_wait() {
        let listener = LoopbackRedirect::bind().expect("bind");
        let port = listener.port();
        let handle = receive_in_background(listener);

        let mut probe = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        probe.write_all(b"GET /fav").expect("write");
        drop(probe);

        exchange(port, "/callback?state=sixteen-byte-state&code=after-hangup");
        assert_eq!(
            handle.join().expect("join").expect("callback").outcome,
            CallbackOutcome::Code("after-hangup".to_owned())
        );
    }

    /// A connection that says nothing at all must not kill the wait either.
    ///
    /// This is the branch a hang-up cannot reach: the peer neither sends nor
    /// closes, so the read fails with the per-read timeout rather than
    /// `Ok(0)`. Treating that as fatal would let any local process cancel a
    /// sign-in by opening a socket and walking away. Costs
    /// `PROBE_READ_TIMEOUT` in wall-clock, which is why there is one of it.
    #[test]
    fn a_silent_connection_does_not_end_the_wait() {
        let listener = LoopbackRedirect::bind().expect("bind");
        let port = listener.port();
        // Comfortably past the per-read timeout, so the stall expires inside
        // a flow that is still live.
        let handle = std::thread::spawn(move || listener.receive(Duration::from_secs(60)));

        let silent = TcpStream::connect(("127.0.0.1", port)).expect("connect");

        exchange(
            port,
            "/callback?state=sixteen-byte-state&code=after-silence",
        );
        assert_eq!(
            handle.join().expect("join").expect("callback").outcome,
            CallbackOutcome::Code("after-silence".to_owned())
        );
        drop(silent);
    }

    /// A peer dribbling a byte at a time must not outlive the flow deadline.
    #[test]
    fn a_trickling_connection_cannot_outlive_the_deadline() {
        let listener = LoopbackRedirect::bind().expect("bind");
        let port = listener.port();
        let handle = std::thread::spawn(move || listener.receive(Duration::from_millis(400)));

        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        let started = Instant::now();
        while started.elapsed() < Duration::from_millis(900) {
            if stream.write_all(b"x").is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(matches!(
            handle.join().expect("join"),
            Err(LoopbackError::TimedOut)
        ));
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
