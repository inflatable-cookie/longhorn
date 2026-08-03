use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use longhorn_core::DomainId;
use serde::{Deserialize, Serialize};

use crate::{
    BackupAdapterId, BackupAdapterPayload, BackupAdapterRelativePath, DomainDescriptor,
    Sha256Digest,
};

use super::super::RestoreOperationState;

const STATE_DIRECTORY: &str = ".longhorn/grouped-adapter-restore";
const PAYLOAD_DIRECTORY: &str = "payloads";
const JOURNAL_FILE: &str = "journal.json";
const JOURNAL_TEMPORARY: &str = ".journal.json.tmp";
const JOURNAL_VERSION: u32 = 1;
const MAX_JOURNAL_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GroupedRestoreJournal {
    version: u32,
    pub(super) operation_id: String,
    pub(super) archive_sha256: Sha256Digest,
    pub(super) confirmation_digest: Sha256Digest,
    pub(super) phase: GroupedJournalPhase,
    pub(super) entries: Vec<GroupedJournalEntry>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum GroupedJournalPhase {
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
pub(super) struct GroupedJournalEntry {
    pub(super) domain: DomainId,
    pub(super) adapter: String,
    pub(super) descriptor_digest: Sha256Digest,
    pub(super) target_evidence: Sha256Digest,
    pub(super) current_evidence: Option<Sha256Digest>,
    pub(super) target_payloads: Vec<GroupedPayloadRecord>,
    pub(super) rollback_payloads: Vec<GroupedPayloadRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GroupedPayloadRecord {
    relative_path: String,
    file_name: String,
    byte_length: u64,
    sha256: Sha256Digest,
}

pub(super) struct PreparedGroupedDomain {
    pub(super) domain: DomainId,
    pub(super) adapter: BackupAdapterId,
    pub(super) descriptor_digest: Sha256Digest,
    pub(super) target_evidence: Sha256Digest,
    pub(super) current_evidence: Option<Sha256Digest>,
    pub(super) target_payloads: Vec<BackupAdapterPayload>,
    pub(super) rollback_payloads: Vec<BackupAdapterPayload>,
}

pub(super) fn descriptor_digest(descriptor: &DomainDescriptor) -> Sha256Digest {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "domain": descriptor.id().as_str(),
        "schemaVersion": descriptor.schema_version().get(),
        "storageClass": descriptor.storage_class(),
        "filePath": descriptor.file_path().map(|path| path.as_str()),
    }))
    .expect("domain descriptor confirmation form is serializable");
    Sha256Digest::from_bytes(&bytes)
}

pub(super) fn operation_state(authority_root: &Path) -> RestoreOperationState {
    match load(authority_root) {
        Ok(None) => RestoreOperationState::Inactive,
        Ok(Some(journal)) => match journal.phase {
            GroupedJournalPhase::RecoveryRequired => RestoreOperationState::RecoveryRequired,
            GroupedJournalPhase::Prepared
            | GroupedJournalPhase::Applying
            | GroupedJournalPhase::Verifying
            | GroupedJournalPhase::RollingBack
            | GroupedJournalPhase::Succeeded
            | GroupedJournalPhase::RolledBack => RestoreOperationState::Active,
        },
        Err(_) => RestoreOperationState::RecoveryRequired,
    }
}

pub(super) fn exists(authority_root: &Path) -> bool {
    journal_path(authority_root).exists()
}

pub(super) fn load(authority_root: &Path) -> io::Result<Option<GroupedRestoreJournal>> {
    let path = journal_path(authority_root);
    let mut input = match fs::File::open(&path) {
        Ok(input) => input,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let observed = input.metadata()?.len();
    if observed > MAX_JOURNAL_BYTES as u64 {
        return Err(io::Error::other(
            "grouped restore journal exceeds byte limit",
        ));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(observed).unwrap_or(0));
    Read::by_ref(&mut input)
        .take(MAX_JOURNAL_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_JOURNAL_BYTES {
        return Err(io::Error::other(
            "grouped restore journal exceeds byte limit",
        ));
    }
    let journal: GroupedRestoreJournal = serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::other(format!("invalid grouped restore journal: {error}")))?;
    if journal.version != JOURNAL_VERSION {
        return Err(io::Error::other(format!(
            "unsupported grouped restore journal version {}",
            journal.version
        )));
    }
    Ok(Some(journal))
}

pub(super) fn persist_prepared(
    authority_root: &Path,
    operation_id: String,
    archive_sha256: Sha256Digest,
    confirmation_digest: Sha256Digest,
    prepared: &[PreparedGroupedDomain],
) -> io::Result<GroupedRestoreJournal> {
    clear_state(authority_root)?;
    let payload_root = payload_directory(authority_root);
    fs::create_dir_all(&payload_root)?;
    let mut entries = Vec::with_capacity(prepared.len());
    for (domain_index, domain) in prepared.iter().enumerate() {
        entries.push(GroupedJournalEntry {
            domain: domain.domain.clone(),
            adapter: domain.adapter.as_str().to_owned(),
            descriptor_digest: domain.descriptor_digest.clone(),
            target_evidence: domain.target_evidence.clone(),
            current_evidence: domain.current_evidence.clone(),
            target_payloads: persist_payload_set(
                &payload_root,
                domain_index,
                "target",
                &domain.target_payloads,
            )?,
            rollback_payloads: persist_payload_set(
                &payload_root,
                domain_index,
                "rollback",
                &domain.rollback_payloads,
            )?,
        });
    }
    sync_directory(&payload_root)?;
    let journal = GroupedRestoreJournal {
        version: JOURNAL_VERSION,
        operation_id,
        archive_sha256,
        confirmation_digest,
        phase: GroupedJournalPhase::Prepared,
        entries,
    };
    publish(authority_root, &journal)?;
    Ok(journal)
}

fn persist_payload_set(
    root: &Path,
    domain_index: usize,
    kind: &str,
    payloads: &[BackupAdapterPayload],
) -> io::Result<Vec<GroupedPayloadRecord>> {
    payloads
        .iter()
        .enumerate()
        .map(|(payload_index, payload)| {
            let file_name = format!("{domain_index:04}-{kind}-{payload_index:04}.payload");
            write_private_file(&root.join(&file_name), payload.bytes())?;
            Ok(GroupedPayloadRecord {
                relative_path: payload.relative_path().as_str().to_owned(),
                file_name,
                byte_length: payload.bytes().len() as u64,
                sha256: Sha256Digest::from_bytes(payload.bytes()),
            })
        })
        .collect()
}

pub(super) fn read_payloads(
    authority_root: &Path,
    records: &[GroupedPayloadRecord],
) -> io::Result<Vec<BackupAdapterPayload>> {
    records
        .iter()
        .map(|record| {
            let bytes = fs::read(payload_directory(authority_root).join(&record.file_name))?;
            if bytes.len() as u64 != record.byte_length
                || Sha256Digest::from_bytes(&bytes) != record.sha256
            {
                return Err(io::Error::other(
                    "grouped restore payload does not match journal evidence",
                ));
            }
            let relative = BackupAdapterRelativePath::new(record.relative_path.clone())
                .map_err(|error| io::Error::other(error.to_string()))?;
            Ok(BackupAdapterPayload::new(relative, bytes))
        })
        .collect()
}

pub(super) fn publish(authority_root: &Path, journal: &GroupedRestoreJournal) -> io::Result<()> {
    let directory = state_directory(authority_root);
    fs::create_dir_all(&directory)?;
    sync_directory(authority_root)?;
    let temporary = directory.join(JOURNAL_TEMPORARY);
    match fs::remove_file(&temporary) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let bytes = serde_json::to_vec_pretty(journal)
        .map_err(|error| io::Error::other(format!("cannot encode grouped journal: {error}")))?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    set_private_mode(&mut options);
    let mut output = options.open(&temporary)?;
    output.write_all(&bytes)?;
    output.sync_all()?;
    drop(output);
    fs::rename(&temporary, journal_path(authority_root))?;
    sync_directory(&directory)
}

pub(super) fn cleanup(authority_root: &Path) -> io::Result<()> {
    clear_state(authority_root)
}

pub(super) fn journal_path(authority_root: &Path) -> PathBuf {
    state_directory(authority_root).join(JOURNAL_FILE)
}

fn clear_state(authority_root: &Path) -> io::Result<()> {
    let path = state_directory(authority_root);
    match fs::remove_dir_all(path) {
        Ok(()) => sync_directory(authority_root),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn state_directory(authority_root: &Path) -> PathBuf {
    authority_root.join(STATE_DIRECTORY)
}

fn payload_directory(authority_root: &Path) -> PathBuf {
    state_directory(authority_root).join(PAYLOAD_DIRECTORY)
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
