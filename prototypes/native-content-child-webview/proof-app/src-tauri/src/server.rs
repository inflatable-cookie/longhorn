//! Controlled remote HTTP fixture used by the packaged child-webview proof.

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use serde_json::json;

use crate::evidence::EvidenceLog;

pub(crate) struct ProofServer {
    origin: String,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ProofServer {
    pub(crate) fn start(log: Arc<EvidenceLog>) -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
        listener
            .set_nonblocking(true)
            .map_err(|error| error.to_string())?;
        let address = listener.local_addr().map_err(|error| error.to_string())?;
        let origin = format!("http://{address}");
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread = thread::spawn(move || {
            while !thread_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        if let Err(error) = handle_request(&mut stream, &log) {
                            let _ = log.record("fixture_error", json!({"detail": error}));
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => {
                        let _ = log.record("fixture_error", json!({"detail": error.to_string()}));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            origin,
            stop,
            thread: Some(thread),
        })
    }

    pub(crate) fn page_url(&self, generation: u64, session: &str) -> String {
        format!(
            "{}/proof?generation={generation}&session={session}",
            self.origin
        )
    }

    pub(crate) fn origin(&self) -> &str {
        &self.origin
    }
}

impl Drop for ProofServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn handle_request(stream: &mut TcpStream, log: &EvidenceLog) -> Result<(), String> {
    stream
        .set_nonblocking(false)
        .map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_millis(250)))
        .map_err(|error| error.to_string())?;
    let mut buffer = [0_u8; 8192];
    let read = match stream.read(&mut buffer) {
        Ok(0) => return Ok(()),
        Ok(read) => read,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            return Ok(());
        }
        Err(error) => return Err(error.to_string()),
    };
    let request = String::from_utf8_lossy(&buffer[..read]);
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| "fixture request has no target".to_string())?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    match path {
        "/proof" => write_response(stream, "200 OK", "text/html; charset=utf-8", proof_page()),
        "/event" => {
            let name = query_value(query, "name").unwrap_or_else(|| "missing".to_string());
            let session = query_value(query, "session").unwrap_or_else(|| "missing".to_string());
            let generation =
                query_value(query, "generation").unwrap_or_else(|| "missing".to_string());
            let counter = query_value(query, "counter").unwrap_or_else(|| "missing".to_string());
            log.record(
                "content_event",
                json!({
                    "name": name,
                    "session": session,
                    "generation": generation,
                    "counter": counter,
                }),
            )?;
            write_response(stream, "200 OK", "text/plain; charset=utf-8", "ok")
        }
        "/download" => write_download(stream),
        "/popup-target" => write_response(
            stream,
            "200 OK",
            "text/html; charset=utf-8",
            "<!doctype html><title>popup should have been denied</title>",
        ),
        _ => write_response(
            stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            "not found",
        ),
    }
}

fn proof_page() -> &'static str {
    r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Longhorn controlled child content</title>
</head>
<body>
  <main><h1>Controlled child content</h1><p id="state">active</p></main>
  <script>
    const parameters = new URLSearchParams(window.location.search);
    const session = parameters.get('session') || 'missing';
    const generation = parameters.get('generation') || 'missing';
    let counter = 0;
    const emit = (name) => {
      counter += 1;
      const query = new URLSearchParams({ name, session, generation, counter: String(counter) });
      return fetch(`/event?${query.toString()}`, { cache: 'no-store' });
    };
    window.__longhornProofProbe = (name) => emit(name);
    window.__longhornSecurityProbe = () => {
      const popup = document.createElement('a');
      popup.href = '/popup-target';
      popup.target = '_blank';
      popup.rel = 'opener';
      document.body.appendChild(popup);
      popup.click();
      popup.remove();

      const download = document.createElement('a');
      download.href = '/download';
      download.download = 'forbidden.txt';
      document.body.appendChild(download);
      download.click();
      download.remove();

      window.setTimeout(() => window.location.assign('https://example.invalid/blocked'), 100);
      return emit('security-probe');
    };
    void emit('loaded');
  </script>
</body>
</html>"#
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> Result<(), String> {
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; script-src 'unsafe-inline'; connect-src 'self'; style-src 'none'; img-src 'none'; object-src 'none'; frame-src 'none'\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(headers.as_bytes())
        .and_then(|()| stream.write_all(body.as_bytes()))
        .map_err(|error| error.to_string())
}

fn write_download(stream: &mut TcpStream) -> Result<(), String> {
    let body = "download policy must deny this payload";
    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Disposition: attachment; filename=forbidden.txt\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(headers.as_bytes())
        .and_then(|()| stream.write_all(body.as_bytes()))
        .map_err(|error| error.to_string())
}

fn query_value(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|part| {
        let (candidate, value) = part.split_once('=')?;
        (candidate == key).then(|| percent_decode(value))
    })
}

fn percent_decode(value: &str) -> String {
    let mut output = Vec::with_capacity(value.len());
    let mut bytes = value.as_bytes().iter().copied();
    while let Some(byte) = bytes.next() {
        match byte {
            b'%' => {
                let high = bytes.next().and_then(hex_value);
                let low = bytes.next().and_then(hex_value);
                if let (Some(high), Some(low)) = (high, low) {
                    output.push(high * 16 + low);
                }
            }
            b'+' => output.push(b' '),
            other => output.push(other),
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
