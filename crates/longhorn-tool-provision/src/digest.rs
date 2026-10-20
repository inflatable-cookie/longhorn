//! Digest formats (contract 027): release checksum files, `.sha256`
//! sidecars, and SHA-256 of a file.

use std::{io::Read, path::Path};

use sha2::{Digest, Sha256};

use crate::failure::{Fault, ProvisionError, Refusal};

/// The SHA-256 of one file, lowercase hex.
///
/// # Errors
///
/// A file that cannot be opened or read is a [`Fault::Io`].
pub fn sha256_file(path: &Path) -> Result<String, ProvisionError> {
    let io_fault = |error: std::io::Error| {
        ProvisionError::Fault(Fault::Io {
            path: path.to_path_buf(),
            detail: error.to_string(),
        })
    };
    let mut file = std::fs::File::open(path).map_err(io_fault)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(io_fault)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// The digest one asset carries in a multi-asset release checksum file.
///
/// Each line is `<digest>  <name>` (two or more whitespace characters, or a
/// binary-marker `*` before the name, as Codex and gh publish). The digest may
/// be upper or lower case and is returned lowercase. Blank lines are skipped.
///
/// # Errors
///
/// [`Refusal::ChecksumNotCovered`] when no line names `asset`;
/// [`Refusal::MalformedChecksum`] when the matching line's digest is not
/// SHA-256 hex, or when more than one line names `asset`. Duplicates are
/// refused because two digests for one asset cannot both be trusted; the
/// donor took the first.
pub fn digest_from_checksums(checksums: &str, asset: &str) -> Result<String, ProvisionError> {
    let mut found: Option<String> = None;
    for line in checksums.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((digest, name)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let name = name.trim().trim_start_matches('*');
        if name != asset {
            continue;
        }
        let digest = digest.trim().to_ascii_lowercase();
        if !is_sha256_hex(&digest) || found.is_some() {
            return Err(ProvisionError::Refused(Refusal::MalformedChecksum {
                asset: asset.to_owned(),
            }));
        }
        found = Some(digest);
    }
    found.ok_or_else(|| {
        ProvisionError::Refused(Refusal::ChecksumNotCovered {
            asset: asset.to_owned(),
        })
    })
}

/// The digest in an artifact-specific `.sha256` sidecar.
///
/// The format is stricter than a checksum file: exactly one line,
/// `<64 lowercase hex>  <asset>` followed by a single newline. Uppercase hex,
/// a different asset name, or any extra line refuses.
///
/// # Errors
///
/// [`Refusal::MalformedSidecar`] for any other shape.
pub fn digest_from_sidecar(sidecar: &str, asset: &str) -> Result<String, ProvisionError> {
    let malformed = || {
        ProvisionError::Refused(Refusal::MalformedSidecar {
            asset: asset.to_owned(),
        })
    };
    let suffix = format!("  {asset}\n");
    let digest = sidecar.strip_suffix(&suffix).ok_or_else(malformed)?;
    if !is_sha256_hex(digest) {
        return Err(malformed());
    }
    Ok(digest.to_owned())
}

/// Whether `digest` is 64 lowercase hex characters.
pub(crate) fn is_sha256_hex(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
