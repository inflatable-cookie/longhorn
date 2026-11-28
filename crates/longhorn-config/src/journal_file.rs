//! Bounded journal document plumbing shared by restore flows.
//!
//! Encoding and loading share the same byte limit. Load also checks the
//! version and returns `None` for a missing journal. Publish uses the shared
//! unique-temporary publication model from [`crate::atomic_file`]. The
//! transition journal has a separate document model and error mapping.

use std::{io, path::Path};

use serde::{Serialize, de::DeserializeOwned};

use crate::{
    atomic_file::UniqueTemporary,
    bounded_file::{self, BoundedReadError},
};

pub(crate) const MAX_JOURNAL_BYTES: usize = 4 * 1024 * 1024;

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
    let bytes = match bounded_file::read_path(path, MAX_JOURNAL_BYTES) {
        Ok(bytes) => bytes,
        Err(BoundedReadError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(BoundedReadError::TooLarge { .. }) => {
            return Err(io::Error::other(format!("{subject} exceeds byte limit")));
        }
        Err(error) => return Err(error.into()),
    };
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

/// Encodes a journal with the same byte limit enforced by [`load`].
pub(crate) fn encode<J: Serialize>(journal: &J, subject: &str) -> io::Result<Vec<u8>> {
    let mut bytes = JournalBytes(Vec::new());
    // Compact JSON leaves room for phase changes in older pretty-printed journals.
    serde_json::to_writer(&mut bytes, journal)
        .map_err(|error| io::Error::other(format!("cannot encode {subject}: {error}")))?;
    Ok(bytes.0)
}

struct JournalBytes(Vec<u8>);

impl io::Write for JournalBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_JOURNAL_BYTES - self.0.len() {
            return Err(io::Error::other("exceeds byte limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Serializes and atomically publishes a journal document below `directory`,
/// ensuring every ancestor from `directory` up to `sync_root` is durable.
pub(crate) fn publish<J: Serialize>(
    sync_root: &Path,
    directory: &Path,
    file_name: &str,
    journal: &J,
    subject: &str,
) -> io::Result<()> {
    let bytes = encode(journal, subject)?;
    crate::dir_sync::create_dir_all_synced_to_root(sync_root, directory)?;
    let mut temporary = UniqueTemporary::create(directory, file_name)?;
    temporary
        .write(&bytes)
        .map_err(|error| io::Error::other(format!("cannot publish {subject}: {error}")))?;
    temporary
        .commit(&directory.join(file_name))
        .map_err(|error| io::Error::other(format!("cannot publish {subject}: {error}")))?;
    crate::dir_sync::sync_ancestors_to_root(sync_root, directory)
        .map_err(|error| io::Error::other(format!("cannot sync ancestors for {subject}: {error}")))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[derive(Debug, PartialEq, serde::Deserialize, Serialize)]
    struct TestJournal {
        version: u32,
        payload: String,
    }

    impl JournalVersioned for TestJournal {
        fn version(&self) -> u32 {
            self.version
        }
    }

    #[test]
    fn publication_and_load_share_the_inclusive_limit() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        let file = "journal.json";
        let path = root.join(file);
        let subject = "test journal";
        let mut document = TestJournal {
            version: 1,
            payload: String::new(),
        };
        let padding = MAX_JOURNAL_BYTES - encode(&document, subject).unwrap().len();
        document.payload = "x".repeat(padding);
        publish(root, root, file, &document, subject).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().len(), MAX_JOURNAL_BYTES as u64);

        document.payload.push('x');
        let error = publish(root, root, file, &document, subject).unwrap_err();
        assert!(error.to_string().contains("exceeds byte limit"), "{error}");

        fs::write(
            root.join("oversized.json"),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
        let error = load::<TestJournal>(&root.join("oversized.json"), 1, subject).unwrap_err();
        assert!(error.to_string().contains("exceeds byte limit"), "{error}");

        document.payload.pop();
        assert_eq!(load(&path, 1, subject).unwrap(), Some(document));
    }
}
