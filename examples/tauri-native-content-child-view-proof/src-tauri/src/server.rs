//! Controlled loopback HTTP content for the packaged proof.

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
        let origin = format!(
            "http://{}",
            listener.local_addr().map_err(|error| error.to_string())?
        );
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

    pub(crate) fn page_url(&self, session: &str) -> String {
        format!("{}/proof?session={session}", self.origin)
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
    log.record("http_request", json!({"path": path}))?;
    match path {
        "/proof" => write_response(stream, "200 OK", "text/html; charset=utf-8", proof_page()),
        "/event" => {
            log.record(
                "content_event",
                json!({
                    "name": query_value(query, "name").unwrap_or_else(|| "missing".into()),
                    "session": query_value(query, "session").unwrap_or_else(|| "missing".into()),
                    "counter": query_value(query, "counter").unwrap_or_else(|| "missing".into()),
                }),
            )?;
            write_response(stream, "200 OK", "text/plain; charset=utf-8", "ok")
        }
        "/download" => write_response(stream, "200 OK", "application/octet-stream", "denied"),
        "/popup" => write_response(stream, "200 OK", "text/html", "<title>denied</title>"),
        _ => write_response(stream, "404 Not Found", "text/plain", "not found"),
    }
}

fn proof_page() -> &'static str {
    r#"<!doctype html><meta charset="utf-8"><title>Controlled child</title>
<h1>Controlled child content</h1><script>
const p=new URLSearchParams(location.search);const session=p.get('session')||'missing';let counter=0;
const emit=name=>{counter++;const q=new URLSearchParams({name,session,counter:String(counter)});fetch('/event?'+q,{cache:'no-store'});};
emit('loaded');setInterval(()=>emit('heartbeat'),100);
const popup=document.createElement('a');popup.href='/popup';popup.target='_blank';document.body.append(popup);popup.click();
const download=document.createElement('a');download.href='/download';download.download='denied.bin';document.body.append(download);download.click();
</script>"#
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> Result<(), String> {
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; script-src 'unsafe-inline'; connect-src 'self'\r\nConnection: close\r\n\r\n",
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
                if let (Some(high), Some(low)) =
                    (bytes.next().and_then(hex), bytes.next().and_then(hex))
                {
                    output.push(high * 16 + low);
                }
            }
            b'+' => output.push(b' '),
            other => output.push(other),
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}

const fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
