use std::{
    io::{self, Read},
    path::Path,
};

use cap_std::{ambient_authority, fs::Dir};

use crate::{DurabilityRequirement, ResolvedFile, store::publication::publish};

use super::journal::JournalEvidence;

pub(super) fn read_exact_state(
    file: &ResolvedFile,
    expected: &JournalEvidence,
) -> io::Result<Option<Vec<u8>>> {
    let root = Dir::open_ambient_dir(file.root(), ambient_authority())?;
    let mut input = match root.open(file.relative_path().as_path()) {
        Ok(input) => input,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return if expected.matches(None) {
                Ok(None)
            } else {
                Err(io::Error::other("registered source is unexpectedly absent"))
            };
        }
        Err(error) => return Err(error),
    };
    let observed = input.metadata()?.len();
    let expected_length = match expected {
        JournalEvidence::Absent => {
            return Err(io::Error::other(
                "registered source is unexpectedly present",
            ));
        }
        JournalEvidence::Present { byte_length, .. } => *byte_length,
    };
    if observed != expected_length {
        return Err(io::Error::other(format!(
            "registered source length changed: expected {expected_length}, observed {observed}"
        )));
    }
    let capacity = usize::try_from(observed)
        .map_err(|_| io::Error::other("registered source exceeds addressable memory"))?;
    let mut bytes = Vec::with_capacity(capacity);
    input
        .by_ref()
        .take(observed.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if !expected.matches(Some(&bytes)) {
        return Err(io::Error::other(
            "registered source digest changed after confirmation",
        ));
    }
    Ok(Some(bytes))
}

pub(super) fn publish_state(file: &ResolvedFile, bytes: Option<&[u8]>) -> io::Result<()> {
    match bytes {
        Some(bytes) => publish(file, bytes, DurabilityRequirement::Durable)
            .map(|_| ())
            .map_err(|error| {
                io::Error::other(format!(
                    "publication {:?} failed at {}: {}",
                    error.stage,
                    error.path.display(),
                    error.detail
                ))
            }),
        None => delete_state(file),
    }
}

pub(super) fn verify_state(file: &ResolvedFile, expected: &JournalEvidence) -> io::Result<()> {
    read_exact_state(file, expected).map(|_| ())
}

fn delete_state(file: &ResolvedFile) -> io::Result<()> {
    let root = Dir::open_ambient_dir(file.root(), ambient_authority())?;
    let relative = file.relative_path().as_path();
    let parent_path = relative.parent().unwrap_or_else(|| Path::new(""));
    let parent = if parent_path.as_os_str().is_empty() {
        root.try_clone()?
    } else {
        root.open_dir(parent_path)?
    };
    let file_name = relative
        .file_name()
        .ok_or_else(|| io::Error::other("registered target has no file name"))?;
    match parent.remove_file(file_name) {
        Ok(()) => parent.into_std_file().sync_all(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
