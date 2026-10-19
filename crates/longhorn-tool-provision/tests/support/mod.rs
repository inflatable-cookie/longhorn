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
