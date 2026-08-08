use std::{
    fs,
    io::{self},
    path::{Path, PathBuf},
};

use longhorn_core::DomainId;
use serde::{Deserialize, Serialize};

use crate::atomic_file::{sync_directory, write_private_file};
use crate::journal_file::{self, JournalVersioned};
use crate::{
    BackupAdapterId, BackupAdapterPayload, BackupAdapterRelativePath, BackupAdapterStateEvidence,
    DomainDescriptor, Sha256Digest,
};

use super::super::RestoreOperationState;

const STATE_DIRECTORY: &str = ".longhorn/grouped-adapter-restore";
const PAYLOAD_DIRECTORY: &str = "payloads";
const JOURNAL_FILE: &str = "journal.json";
const JOURNAL_VERSION: u32 = 2;

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

impl JournalVersioned for GroupedRestoreJournal {
    fn version(&self) -> u32 {
        self.version
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct GroupedJournalEntry {
    pub(super) domain: DomainId,
    pub(super) adapter: String,
    pub(super) descriptor_digest: Sha256Digest,
    pub(super) target_evidence: BackupAdapterStateEvidence,
    pub(super) rollback_evidence: BackupAdapterStateEvidence,
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
    pub(super) target_evidence: BackupAdapterStateEvidence,
    pub(super) rollback_evidence: BackupAdapterStateEvidence,
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
    let journal: Option<GroupedRestoreJournal> = journal_file::load(
        &journal_path(authority_root),
        JOURNAL_VERSION,
        "grouped restore journal",
    )?;
    if let Some(journal) = &journal {
        for entry in &journal.entries {
            validate_evidence_payload_shape(&entry.target_evidence, &entry.target_payloads)?;
            validate_evidence_payload_shape(&entry.rollback_evidence, &entry.rollback_payloads)?;
        }
    }
    Ok(journal)
}

fn validate_evidence_payload_shape(
    evidence: &BackupAdapterStateEvidence,
    payloads: &[GroupedPayloadRecord],
) -> io::Result<()> {
    let valid = match evidence {
        BackupAdapterStateEvidence::Absent => payloads.is_empty(),
        BackupAdapterStateEvidence::Present { .. } => !payloads.is_empty(),
    };
    if valid {
        Ok(())
    } else {
        Err(io::Error::other(
            "grouped restore journal evidence contradicts payload presence",
        ))
    }
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
            rollback_evidence: domain.rollback_evidence.clone(),
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
    sync_directory(authority_root)?;
    journal_file::publish(
        &state_directory(authority_root),
        JOURNAL_FILE,
        journal,
        "grouped restore journal",
    )
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
