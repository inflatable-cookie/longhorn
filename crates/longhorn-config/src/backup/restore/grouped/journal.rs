use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use cap_std::{ambient_authority, fs::Dir};
use longhorn_core::DomainId;
use serde::{Deserialize, Serialize};

use crate::atomic_file::{sync_directory, write_private_file};
use crate::journal_file::{self, JournalVersioned};
use crate::{
    BackupAdapterId, BackupAdapterPayload, BackupAdapterRelativePath, BackupAdapterStateEvidence,
    BackupLimits, DomainDescriptor, Sha256Digest,
};

use super::super::RestoreOperationState;

const STATE_DIRECTORY: &str = ".longhorn/grouped-adapter-restore";
const PAYLOAD_DIRECTORY: &str = "payloads";
const JOURNAL_FILE: &str = "journal.json";
const JOURNAL_VERSION: u32 = 2;
const JOURNAL_SUBJECT: &str = "grouped restore journal";

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
        JOURNAL_SUBJECT,
    )?;
    if let Some(journal) = &journal {
        // Version 2 records exact lengths, so recovery can honor the admitted
        // aggregate and reject impossible totals without the execution options.
        let limits = BackupLimits::HARD_MAXIMUM;
        let mut remaining = limits.max_total_bytes() as u64;
        for (domain_index, entry) in journal.entries.iter().enumerate() {
            validate_payload_names(domain_index, "target", &entry.target_payloads)?;
            validate_payload_names(domain_index, "rollback", &entry.rollback_payloads)?;
            validate_evidence_payload_shape(&entry.target_evidence, &entry.target_payloads)?;
            validate_evidence_payload_shape(&entry.rollback_evidence, &entry.rollback_payloads)?;
            let domain_bytes = entry
                .target_payloads
                .iter()
                .chain(&entry.rollback_payloads)
                .try_fold(0u64, |total, record| total.checked_add(record.byte_length))
                .ok_or_else(|| {
                    io::Error::other("grouped restore journal payload length overflow")
                })?;
            if domain_bytes > limits.max_domain_bytes() as u64 {
                return Err(io::Error::other(
                    "grouped restore journal exceeds domain payload byte limit",
                ));
            }
            remaining = remaining.checked_sub(domain_bytes).ok_or_else(|| {
                io::Error::other("grouped restore journal exceeds total payload byte limit")
            })?;
        }
    }
    Ok(journal)
}

fn validate_payload_names(
    domain_index: usize,
    kind: &str,
    records: &[GroupedPayloadRecord],
) -> io::Result<()> {
    for (payload_index, record) in records.iter().enumerate() {
        if record.file_name != payload_file_name(domain_index, kind, payload_index) {
            return Err(io::Error::other(
                "grouped restore payload name does not match its journal position",
            ));
        }
    }
    Ok(())
}

fn payload_file_name(domain_index: usize, kind: &str, payload_index: usize) -> String {
    format!("{domain_index:04}-{kind}-{payload_index:04}.payload")
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
    let entries = prepared
        .iter()
        .enumerate()
        .map(|(domain_index, domain)| GroupedJournalEntry {
            domain: domain.domain.clone(),
            adapter: domain.adapter.as_str().to_owned(),
            descriptor_digest: domain.descriptor_digest.clone(),
            target_evidence: domain.target_evidence.clone(),
            rollback_evidence: domain.rollback_evidence.clone(),
            target_payloads: payload_records(domain_index, "target", &domain.target_payloads),
            rollback_payloads: payload_records(domain_index, "rollback", &domain.rollback_payloads),
        })
        .collect();
    let mut journal = GroupedRestoreJournal {
        version: JOURNAL_VERSION,
        operation_id,
        archive_sha256,
        confirmation_digest,
        phase: GroupedJournalPhase::RecoveryRequired,
        entries,
    };
    // Only the phase changes after preparation, and recovery-required has
    // the longest serialized name. Checking it reserves room for every phase.
    journal_file::encode(&journal, JOURNAL_SUBJECT)?;
    journal.phase = GroupedJournalPhase::Prepared;

    clear_state(authority_root)?;
    let payload_root = payload_directory(authority_root);
    crate::dir_sync::create_dir_all_synced_to_root(authority_root, &payload_root)?;
    for (domain, entry) in prepared.iter().zip(&journal.entries) {
        persist_payload_set(
            &payload_root,
            &domain.target_payloads,
            &entry.target_payloads,
        )?;
        persist_payload_set(
            &payload_root,
            &domain.rollback_payloads,
            &entry.rollback_payloads,
        )?;
    }
    crate::dir_sync::sync_ancestors_to_root(authority_root, &payload_root)?;
    publish(authority_root, &journal)?;
    Ok(journal)
}

fn payload_records(
    domain_index: usize,
    kind: &str,
    payloads: &[BackupAdapterPayload],
) -> Vec<GroupedPayloadRecord> {
    payloads
        .iter()
        .enumerate()
        .map(|(payload_index, payload)| GroupedPayloadRecord {
            relative_path: payload.relative_path().as_str().to_owned(),
            file_name: payload_file_name(domain_index, kind, payload_index),
            byte_length: payload.bytes().len() as u64,
            sha256: Sha256Digest::from_bytes(payload.bytes()),
        })
        .collect()
}

fn persist_payload_set(
    root: &Path,
    payloads: &[BackupAdapterPayload],
    records: &[GroupedPayloadRecord],
) -> io::Result<()> {
    for (payload, record) in payloads.iter().zip(records) {
        write_private_file(&root.join(&record.file_name), payload.bytes())?;
    }
    Ok(())
}

pub(super) fn read_payloads(
    authority_root: &Path,
    records: &[GroupedPayloadRecord],
) -> io::Result<Vec<BackupAdapterPayload>> {
    if records.is_empty() {
        return Ok(Vec::new());
    }
    let root = Dir::open_ambient_dir(authority_root, ambient_authority())?;
    let directory = root.open_dir(Path::new(STATE_DIRECTORY).join(PAYLOAD_DIRECTORY))?;
    records
        .iter()
        .map(|record| {
            let mut input = directory.open(&record.file_name)?;
            let expected = record.byte_length;
            let observed = input.metadata()?.len();
            if observed != expected {
                return Err(io::Error::other(format!(
                    "grouped restore payload length changed: expected {expected}, observed {observed}"
                )));
            }
            let length = usize::try_from(expected)
                .map_err(|_| io::Error::other("grouped restore payload length is not addressable"))?;
            let mut bytes = vec![0; length];
            input.read_exact(&mut bytes)?;
            if input.read(&mut [0])? != 0 || Sha256Digest::from_bytes(&bytes) != record.sha256 {
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
    journal_file::publish(
        authority_root,
        &state_directory(authority_root),
        JOURNAL_FILE,
        journal,
        JOURNAL_SUBJECT,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preparation_reserves_space_for_later_phases() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        let max_journal_bytes = 4 * 1024 * 1024;
        let mut journal = GroupedRestoreJournal {
            version: JOURNAL_VERSION,
            operation_id: String::new(),
            archive_sha256: Sha256Digest::from_bytes(b"archive"),
            confirmation_digest: Sha256Digest::from_bytes(b"confirmation"),
            phase: GroupedJournalPhase::Prepared,
            entries: Vec::new(),
        };
        let prepared = serde_json::to_vec(&journal).unwrap();
        journal.operation_id = "x".repeat(max_journal_bytes - prepared.len());
        assert_eq!(
            journal_file::encode(&journal, JOURNAL_SUBJECT)
                .unwrap()
                .len(),
            max_journal_bytes
        );
        let error = persist_prepared(
            root,
            journal.operation_id.clone(),
            journal.archive_sha256.clone(),
            journal.confirmation_digest.clone(),
            &[],
        )
        .unwrap_err();
        assert!(error.to_string().contains("exceeds byte limit"), "{error}");
        assert!(!state_directory(root).exists());

        // An older writer could publish Prepared right at the limit.
        journal.operation_id.clear();
        let pretty_base = serde_json::to_vec_pretty(&journal).unwrap().len();
        journal.operation_id = "x".repeat(max_journal_bytes - pretty_base);
        fs::create_dir_all(state_directory(root)).unwrap();
        fs::write(
            journal_path(root),
            serde_json::to_vec_pretty(&journal).unwrap(),
        )
        .unwrap();
        let mut loaded = load(root).unwrap().unwrap();
        assert_eq!(loaded.phase, GroupedJournalPhase::Prepared);
        loaded.phase = GroupedJournalPhase::RecoveryRequired;
        publish(root, &loaded).unwrap();
        assert_eq!(load(root).unwrap(), Some(loaded));
    }
}
