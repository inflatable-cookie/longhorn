//! Bounded journal document plumbing shared by restore flows.
//!
//! Load is bounded (a journal larger than `MAX_JOURNAL_BYTES` is rejected
//! before it is read into memory), version-checked, and returns `None` for a
//! missing journal. Publish uses the shared unique-temporary publication
//! model from [`crate::atomic_file`]. The transition journal is not served
//! here: its document model and error mapping differ enough that sharing
//! would add indirection without removing duplication.

use std::{
    fs,
    io::{self, Read},
    path::Path,
};

use serde::{Serialize, de::DeserializeOwned};

use crate::atomic_file::UniqueTemporary;

const MAX_JOURNAL_BYTES: usize = 4 * 1024 * 1024;

/// A journal document carrying a version field.
pub(crate) trait JournalVersioned {
    /// The document's version.
    fn version(&self) -> u32;
}

/// Reads and bounds a journal document, verifying its version.
///
/// `subject` names the journal in error messages, for example
/// `"restore journal"`.
pub(crate) fn load<J>(path: &Path, expected_version: u32, subject: &str) -> io::Result<Option<J>>
where
    J: DeserializeOwned + JournalVersioned,
{
    let mut input = match fs::File::open(path) {
        Ok(input) => input,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let observed = input.metadata()?.len();
    if observed > MAX_JOURNAL_BYTES as u64 {
        return Err(io::Error::other(format!("{subject} exceeds byte limit")));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(observed).unwrap_or(0));
    Read::by_ref(&mut input)
        .take(MAX_JOURNAL_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_JOURNAL_BYTES {
        return Err(io::Error::other(format!("{subject} exceeds byte limit")));
    }
    let journal: J = serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::other(format!("invalid {subject}: {error}")))?;
    if journal.version() != expected_version {
        return Err(io::Error::other(format!(
            "unsupported {subject} version {}",
            journal.version()
        )));
    }
    Ok(Some(journal))
}

/// Serializes and atomically publishes a journal document below `directory`.
pub(crate) fn publish<J: Serialize>(
    directory: &Path,
    file_name: &str,
    journal: &J,
    subject: &str,
) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    let bytes = serde_json::to_vec_pretty(journal)
        .map_err(|error| io::Error::other(format!("cannot encode {subject}: {error}")))?;
    let mut temporary = UniqueTemporary::create(directory, file_name)?;
    temporary
        .write(&bytes)
        .map_err(|error| io::Error::other(format!("cannot publish {subject}: {error}")))?;
    temporary
        .commit(&directory.join(file_name))
        .map_err(|error| io::Error::other(format!("cannot publish {subject}: {error}")))
}
