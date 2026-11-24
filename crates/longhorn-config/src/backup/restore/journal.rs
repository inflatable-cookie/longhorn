use std::{
    fs,
    io::{self},
    path::{Path, PathBuf},
};

use longhorn_core::DomainId;
use serde::{Deserialize, Serialize};

use crate::atomic_file::{sync_directory, write_private_file};
use crate::journal_file::{self, JournalVersioned};
use crate::{Sha256Digest, StorageClass};

use super::{RestoreAction, RestoreCurrentEvidence, RestoreOperationState, types::StagedDomain};

const STATE_DIRECTORY: &str = ".longhorn/restore";
const ROLLBACK_DIRECTORY: &str = "rollback";
const JOURNAL_FILE: &str = "journal.json";
const JOURNAL_VERSION: u32 = 1;
const JOURNAL_SUBJECT: &str = "restore journal";

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

impl JournalVersioned for RestoreJournal {
    fn version(&self) -> u32 {
        self.version
    }
}

impl RestoreJournal {
    fn prepare(seed: JournalSeed, entries: Vec<JournalEntry>) -> io::Result<Self> {
        let mut journal = Self {
            version: JOURNAL_VERSION,
            operation_id: seed.operation_id,
            plan_digest: seed.plan_digest,
            safety_path: seed.safety_path,
            safety_sha256: seed.safety_sha256,
            phase: JournalPhase::RecoveryRequired,
            entries,
        };
        // Only the phase changes after preparation, and recovery-required has
        // the longest serialized name.
        journal_file::encode(&journal, JOURNAL_SUBJECT)?;
        journal.phase = JournalPhase::Prepared;
        Ok(journal)
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
    journal_file::load(
        &journal_path(authority_root),
        JOURNAL_VERSION,
        JOURNAL_SUBJECT,
    )
}

pub(super) fn publish(authority_root: &Path, journal: &RestoreJournal) -> io::Result<()> {
    journal_file::publish(
        authority_root,
        &state_directory(authority_root),
        JOURNAL_FILE,
        journal,
        JOURNAL_SUBJECT,
    )
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
    crate::dir_sync::create_dir_all_synced_to_root(authority_root, &rollback_path)?;
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
    crate::dir_sync::sync_ancestors_to_root(authority_root, &rollback_path)?;
    RestoreJournal::prepare(seed, entries)
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
    let JournalEvidence::Present { byte_length, .. } = entry.old else {
        return Err(io::Error::other(
            "absent rollback evidence has a rollback payload",
        ));
    };
    let bound = usize::try_from(byte_length)
        .ok()
        .filter(|length| *length <= crate::store::load::max_live_domain_bytes())
        .ok_or_else(|| io::Error::other("rollback payload exceeds live domain byte limit"))?;
    let relative = Path::new(STATE_DIRECTORY)
        .join(ROLLBACK_DIRECTORY)
        .join(file_name);
    let bytes =
        crate::store::load::read_bounded(authority_root, &relative, bound).map_err(|error| {
            if error.kind() == io::ErrorKind::InvalidData {
                io::Error::other(format!(
                    "rollback payload length or digest does not match journal: {error}"
                ))
            } else {
                error
            }
        })?;
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
