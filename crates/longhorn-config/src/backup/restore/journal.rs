use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use longhorn_core::DomainId;
use serde::{Deserialize, Serialize};

use crate::{Sha256Digest, StorageClass};

use super::{RestoreAction, RestoreCurrentEvidence, RestoreOperationState, types::StagedDomain};

const STATE_DIRECTORY: &str = ".longhorn/restore";
const ROLLBACK_DIRECTORY: &str = "rollback";
const JOURNAL_FILE: &str = "journal.json";
const JOURNAL_TEMPORARY: &str = ".journal.json.tmp";
const JOURNAL_VERSION: u32 = 1;
const MAX_JOURNAL_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RestoreJournal {
    version: u32,
    pub(super) operation_id: String,
    pub(super) plan_digest: Sha256Digest,
    pub(super) safety_path: PathBuf,
    pub(super) safety_sha256: Sha256Digest,
    pub(super) phase: JournalPhase,
    pub(super) entries: Vec<JournalEntry>,
}

impl RestoreJournal {
    pub(super) fn new(
        operation_id: String,
        plan_digest: Sha256Digest,
        safety_path: PathBuf,
        safety_sha256: Sha256Digest,
        entries: Vec<JournalEntry>,
    ) -> Self {
        Self {
            version: JOURNAL_VERSION,
            operation_id,
            plan_digest,
            safety_path,
            safety_sha256,
            phase: JournalPhase::Prepared,
            entries,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum JournalPhase {
    Prepared,
    Applying,
    Verifying,
    RollingBack,
    RecoveryRequired,
    Succeeded,
    RolledBack,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct JournalEntry {
    pub(super) domain: DomainId,
    pub(super) storage_class: StorageClass,
    pub(super) relative_path: String,
    pub(super) action: JournalAction,
    pub(super) old: JournalEvidence,
    pub(super) target: JournalEvidence,
    pub(super) rollback_file: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum JournalAction {
    Create,
    Replace,
    Delete,
    Migrate,
    Unchanged,
}

impl From<RestoreAction> for JournalAction {
    fn from(action: RestoreAction) -> Self {
        match action {
            RestoreAction::Create => Self::Create,
            RestoreAction::Replace => Self::Replace,
            RestoreAction::Delete => Self::Delete,
            RestoreAction::Migrate => Self::Migrate,
            RestoreAction::Unchanged => Self::Unchanged,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub(super) enum JournalEvidence {
    Absent,
    Present {
        byte_length: u64,
        sha256: Sha256Digest,
    },
}

impl JournalEvidence {
    pub(super) fn from_current(evidence: &RestoreCurrentEvidence) -> Self {
        match evidence {
            RestoreCurrentEvidence::Absent => Self::Absent,
            RestoreCurrentEvidence::Present {
                byte_length,
                sha256,
            } => Self::Present {
                byte_length: *byte_length,
                sha256: sha256.clone(),
            },
        }
    }

    pub(super) fn from_target(bytes: Option<&[u8]>) -> Self {
        bytes.map_or(Self::Absent, |bytes| Self::Present {
            byte_length: bytes.len() as u64,
            sha256: Sha256Digest::from_bytes(bytes),
        })
    }

    pub(super) fn matches(&self, bytes: Option<&[u8]>) -> bool {
        match (self, bytes) {
            (Self::Absent, None) => true,
            (
                Self::Present {
                    byte_length,
                    sha256,
                },
                Some(bytes),
            ) => *byte_length == bytes.len() as u64 && *sha256 == Sha256Digest::from_bytes(bytes),
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct RollbackEntry {
    pub(super) domain: DomainId,
    pub(super) bytes: Option<Vec<u8>>,
}

pub(super) struct JournalDescriptor {
    pub(super) domain: DomainId,
    pub(super) storage_class: StorageClass,
    pub(super) relative_path: String,
}

pub(super) struct JournalSeed {
    pub(super) operation_id: String,
    pub(super) plan_digest: Sha256Digest,
    pub(super) safety_path: PathBuf,
    pub(super) safety_sha256: Sha256Digest,
}

pub(super) fn operation_state(authority_root: &Path) -> RestoreOperationState {
    match load(authority_root) {
        Ok(None) => RestoreOperationState::Inactive,
        Ok(Some(journal)) => match journal.phase {
            JournalPhase::RecoveryRequired => RestoreOperationState::RecoveryRequired,
            JournalPhase::Prepared
            | JournalPhase::Applying
            | JournalPhase::Verifying
            | JournalPhase::RollingBack
            | JournalPhase::Succeeded
            | JournalPhase::RolledBack => RestoreOperationState::Active,
        },
        Err(_) => RestoreOperationState::RecoveryRequired,
    }
}

pub(super) fn load(authority_root: &Path) -> io::Result<Option<RestoreJournal>> {
    let path = journal_path(authority_root);
    let mut input = match fs::File::open(&path) {
        Ok(input) => input,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let observed = input.metadata()?.len();
    if observed > MAX_JOURNAL_BYTES as u64 {
        return Err(io::Error::other("restore journal exceeds byte limit"));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(observed).unwrap_or(0));
    Read::by_ref(&mut input)
        .take(MAX_JOURNAL_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_JOURNAL_BYTES {
        return Err(io::Error::other("restore journal exceeds byte limit"));
    }
    let journal: RestoreJournal = serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::other(format!("invalid restore journal: {error}")))?;
    if journal.version != JOURNAL_VERSION {
        return Err(io::Error::other(format!(
            "unsupported restore journal version {}",
            journal.version
        )));
    }
    Ok(Some(journal))
}

pub(super) fn publish(authority_root: &Path, journal: &RestoreJournal) -> io::Result<()> {
    let directory_path = state_directory(authority_root);
    fs::create_dir_all(&directory_path)?;
    let temporary_path = directory_path.join(JOURNAL_TEMPORARY);
    match fs::remove_file(&temporary_path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let bytes = serde_json::to_vec_pretty(journal)
        .map_err(|error| io::Error::other(format!("cannot encode restore journal: {error}")))?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    set_private_mode(&mut options);
    let mut output = options.open(&temporary_path)?;
    output.write_all(&bytes)?;
    output.sync_all()?;
    drop(output);
    fs::rename(&temporary_path, journal_path(authority_root))?;
    sync_directory(&directory_path)
}

pub(super) fn persist_rollback(
    authority_root: &Path,
    staging: &[StagedDomain],
    rollback: &[RollbackEntry],
    descriptors: &[JournalDescriptor],
    seed: JournalSeed,
) -> io::Result<RestoreJournal> {
    clear_rollback(authority_root)?;
    let rollback_path = rollback_directory(authority_root);
    fs::create_dir_all(&rollback_path)?;
    let mut entries = Vec::with_capacity(staging.len());
    for staged in staging {
        let old = rollback
            .iter()
            .find(|entry| entry.domain == staged.domain)
            .ok_or_else(|| io::Error::other("rollback set omits staged domain"))?;
        let (storage_class, relative_path) = descriptors
            .iter()
            .find(|descriptor| descriptor.domain == staged.domain)
            .map(|descriptor| (descriptor.storage_class, descriptor.relative_path.clone()))
            .ok_or_else(|| io::Error::other("descriptor set omits staged domain"))?;
        let rollback_file = if let Some(bytes) = old.bytes.as_deref() {
            let file_name = format!("{}.rollback", staged.domain.as_str());
            write_private_file(&rollback_path.join(&file_name), bytes)?;
            Some(file_name)
        } else {
            None
        };
        entries.push(JournalEntry {
            domain: staged.domain.clone(),
            storage_class,
            relative_path,
            action: staged.action.into(),
            old: JournalEvidence::from_current(&staged.current),
            target: JournalEvidence::from_target(staged.bytes.as_deref()),
            rollback_file,
        });
    }
    sync_directory(&rollback_path)?;
    Ok(RestoreJournal::new(
        seed.operation_id,
        seed.plan_digest,
        seed.safety_path,
        seed.safety_sha256,
        entries,
    ))
}

pub(super) fn read_rollback(
    authority_root: &Path,
    entry: &JournalEntry,
) -> io::Result<Option<Vec<u8>>> {
    let Some(file_name) = entry.rollback_file.as_deref() else {
        if matches!(entry.old, JournalEvidence::Absent) {
            return Ok(None);
        }
        return Err(io::Error::other(
            "present rollback evidence has no rollback payload",
        ));
    };
    if file_name != format!("{}.rollback", entry.domain.as_str()) {
        return Err(io::Error::other(
            "rollback payload name does not match domain",
        ));
    }
    let bytes = fs::read(rollback_directory(authority_root).join(file_name))?;
    if !entry.old.matches(Some(&bytes)) {
        return Err(io::Error::other(
            "rollback payload length or digest does not match journal",
        ));
    }
    Ok(Some(bytes))
}

pub(super) fn cleanup(authority_root: &Path) -> io::Result<()> {
    clear_rollback(authority_root)?;
    let state = state_directory(authority_root);
    match fs::remove_file(journal_path(authority_root)) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    match fs::remove_file(state.join(JOURNAL_TEMPORARY)) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    sync_directory(&state)
}

pub(super) fn journal_path(authority_root: &Path) -> PathBuf {
    state_directory(authority_root).join(JOURNAL_FILE)
}

fn clear_rollback(authority_root: &Path) -> io::Result<()> {
    let path = rollback_directory(authority_root);
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn state_directory(authority_root: &Path) -> PathBuf {
    authority_root.join(STATE_DIRECTORY)
}

fn rollback_directory(authority_root: &Path) -> PathBuf {
    state_directory(authority_root).join(ROLLBACK_DIRECTORY)
}

fn write_private_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    set_private_mode(&mut options);
    let mut output = options.open(path)?;
    output.write_all(bytes)?;
    output.sync_all()
}

fn sync_directory(path: &Path) -> io::Result<()> {
    fs::File::open(path)?.sync_all()
}

#[cfg(unix)]
fn set_private_mode(options: &mut fs::OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;

    options.mode(0o600);
}

#[cfg(not(unix))]
fn set_private_mode(_options: &mut fs::OpenOptions) {}
