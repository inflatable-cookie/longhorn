//! Archive and file builders shared by the contract 027 tests.
//!
//! Every name is neutral: the fixtures stand in for a vendor's release, and
//! no product, vendor or pinned digest appears here.

#![allow(dead_code)]

use std::{io::Cursor, path::Path};

use flate2::{Compression, write::GzEncoder};
use sha2::{Digest, Sha256};

/// A tar member, with its name written verbatim (a name that the tar
/// builder would refuse, such as `..`, is written raw).
pub enum TarEntry<'a> {
    File {
        name: &'a str,
        bytes: Vec<u8>,
        mode: u32,
    },
    Dir {
        name: &'a str,
    },
    Symlink {
        name: &'a str,
        target: &'a str,
    },
    HardLink {
        name: &'a str,
        target: &'a str,
    },
    Fifo {
        name: &'a str,
    },
}

/// A zip member.
pub enum ZipEntry<'a> {
    File {
        name: &'a str,
        bytes: Vec<u8>,
    },
    Dir {
        name: &'a str,
    },
    Symlink {
        name: &'a str,
        target: &'a str,
    },
    /// A member whose full Unix mode, file type included, is written as given.
    Mode {
        name: &'a str,
        bytes: Vec<u8>,
        mode: u32,
    },
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn raw_name(header: &mut tar::Header, name: &str) {
    let field = &mut header.as_old_mut().name;
    field.fill(0);
    field[..name.len()].copy_from_slice(name.as_bytes());
}

pub fn tar_gz(entries: &[TarEntry<'_>]) -> Vec<u8> {
    let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    for entry in entries {
        let mut header = tar::Header::new_ustar();
        header.set_mode(0o644);
        match entry {
            TarEntry::File { name, bytes, mode } => {
                raw_name(&mut header, name);
                header.set_entry_type(tar::EntryType::Regular);
                header.set_size(bytes.len() as u64);
                header.set_mode(*mode);
                header.set_cksum();
                builder
                    .append(&header, bytes.as_slice())
                    .expect("file member");
            }
            TarEntry::Dir { name } => {
                raw_name(&mut header, name);
                header.set_entry_type(tar::EntryType::Directory);
                header.set_size(0);
                header.set_mode(0o755);
                header.set_cksum();
                builder.append(&header, &[][..]).expect("dir member");
            }
            TarEntry::Symlink { name, target } | TarEntry::HardLink { name, target } => {
                raw_name(&mut header, name);
                header.set_entry_type(match entry {
                    TarEntry::Symlink { .. } => tar::EntryType::Symlink,
                    _ => tar::EntryType::Link,
                });
                header.set_link_name_literal(target).expect("link target");
                header.set_size(0);
                header.set_cksum();
                builder.append(&header, &[][..]).expect("link member");
            }
            TarEntry::Fifo { name } => {
                raw_name(&mut header, name);
                header.set_entry_type(tar::EntryType::Fifo);
                header.set_size(0);
                header.set_cksum();
                builder.append(&header, &[][..]).expect("fifo member");
            }
        }
    }
    builder
        .into_inner()
        .expect("tar builds")
        .finish()
        .expect("gzip finishes")
}

pub fn zip_of(entries: &[ZipEntry<'_>]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
    for entry in entries {
        match entry {
            ZipEntry::File { name, bytes } => {
                zip.start_file(*name, options).expect("zip member starts");
                std::io::Write::write_all(&mut zip, bytes).expect("zip member writes");
            }
            ZipEntry::Dir { name } => {
                zip.add_directory(*name, options).expect("zip dir");
            }
            ZipEntry::Mode { name, bytes, mode } => {
                let options =
                    zip::write::SimpleFileOptions::default().external_attributes(mode << 16);
                zip.start_file(*name, options)
                    .expect("zip mode member starts");
                std::io::Write::write_all(&mut zip, bytes).expect("zip mode member writes");
            }
            ZipEntry::Symlink { name, target } => {
                zip.add_symlink(*name, *target, options)
                    .expect("zip symlink");
            }
        }
    }
    zip.finish().expect("zip finishes").into_inner()
}

/// Writes `bytes` to a new file under `dir` and returns its path.
pub fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("fixture writes");
    path
}

/// A release listing as GitHub answers it, for a fixture route.
pub fn release_listing(tag: &str, draft: bool, prerelease: bool, assets: &[&str]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "tag_name": tag,
        "draft": draft,
        "prerelease": prerelease,
        "assets": assets.iter().map(|name| serde_json::json!({ "name": name })).collect::<Vec<_>>(),
    }))
    .expect("a fixture listing serialises")
}

/// A loopback server for one connection, for the responses the fixture server
/// cannot express: an undeclared length, a truncated body, or silence.
pub struct RawServer {
    port: u16,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl RawServer {
    /// Answers the first connection with `wire` and closes it. With `silent`,
    /// reads the request and never answers, so the client's timeout fires.
    pub fn answering(wire: Vec<u8>, silent: bool) -> Self {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("loopback listener binds");
        let port = listener.local_addr().expect("local addr").port();
        let worker = std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut request = Vec::new();
            let mut chunk = [0u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => request.extend_from_slice(&chunk[..read]),
                }
            }
            if silent {
                std::thread::sleep(std::time::Duration::from_millis(1500));
                return;
            }
            let _ = stream.write_all(&wire);
        });
        Self {
            port,
            worker: Some(worker),
        }
    }

    /// The base URL, `http://127.0.0.1:{port}`.
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for RawServer {
    fn drop(&mut self) {
        // Wake the worker if the client never connected, then join it so the
        // loopback port does not outlive the test.
        let _ = std::net::TcpStream::connect(("127.0.0.1", self.port));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// A response with no `Content-Length`, sent with chunked transfer encoding.
/// The client learns no length in advance.
pub fn chunked_wire(body: &[u8]) -> Vec<u8> {
    let mut wire =
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
    wire.extend_from_slice(format!("{:x}\r\n", body.len()).as_bytes());
    wire.extend_from_slice(body);
    wire.extend_from_slice(b"\r\n0\r\n\r\n");
    wire
}

/// A response that declares `declared` bytes and sends only `sent`, then
/// closes: the body is truncated.
pub fn truncated_wire(declared: usize, sent: &[u8]) -> Vec<u8> {
    let mut wire =
        format!("HTTP/1.1 200 OK\r\nContent-Length: {declared}\r\nConnection: close\r\n\r\n")
            .into_bytes();
    wire.extend_from_slice(sent);
    wire
}
