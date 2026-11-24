use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Sha256Digest, journal_file};
use longhorn_storage_layout::StorageBootstrapPaths;

use super::{
    StorageFileEvidence, StorageTransitionAction, StorageTransitionError, StorageTransitionPlan, io,
};

const JOURNAL_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum JournalPhase {
    Prepared,
    Publishing,
    LocatorCommitted,
}

#[derive(Clone, Debug)]
pub(super) struct TransitionJournal {
    pub(super) transition_id: String,
    pub(super) phase: JournalPhase,
    pub(super) source_layout_digest: Sha256Digest,
    pub(super) target_layout_digest: Sha256Digest,
    pub(super) confirmation_digest: Sha256Digest,
    pub(super) domains: Vec<JournalDomain>,
}

#[derive(Clone, Debug)]
pub(super) struct JournalDomain {
    pub(super) domain: String,
    pub(super) source_path: Option<PathBuf>,
    pub(super) target_path: Option<PathBuf>,
    pub(super) source: Option<StorageFileEvidence>,
    pub(super) target_expected: Option<StorageFileEvidence>,
    pub(super) custom: bool,
    pub(super) target_was_absent: bool,
}

impl TransitionJournal {
    pub(super) fn from_plan(
        transition_id: String,
        plan: &StorageTransitionPlan,
    ) -> Result<Self, StorageTransitionError> {
        let domains = plan
            .preview
            .domains
            .iter()
            .filter_map(|entry| {
                let custom = matches!(entry.action, StorageTransitionAction::CustomAdapter { .. });
                let target_was_absent =
                    matches!(entry.action, StorageTransitionAction::CopyOrdinary);
                (!matches!(entry.action, StorageTransitionAction::Excluded(_))).then(|| {
                    JournalDomain {
                        domain: entry.domain.as_str().into(),
                        source_path: entry.source_path.clone(),
                        target_path: entry.target_path.clone(),
                        source: entry.source_evidence.clone(),
                        target_expected: match entry.action {
                            StorageTransitionAction::CopyOrdinary
                            | StorageTransitionAction::CustomAdapter { .. } => {
                                entry.source_evidence.clone()
                            }
                            _ => entry.target_evidence.clone(),
                        },
                        custom,
                        target_was_absent,
                    }
                })
            })
            .collect();
        let mut journal = Self {
            transition_id,
            phase: JournalPhase::LocatorCommitted,
            source_layout_digest: plan.preview.source_layout_digest.clone(),
            target_layout_digest: plan.preview.target_layout_digest.clone(),
            confirmation_digest: plan.preview.confirmation_digest.clone(),
            domains,
        };
        // Check the longest phase and exact path serialization before staging.
        encode(&journal)?;
        journal.phase = JournalPhase::Prepared;
        Ok(journal)
    }
}

pub(super) fn write(
    paths: &StorageBootstrapPaths,
    journal: &TransitionJournal,
) -> Result<(), StorageTransitionError> {
    io::atomic_write(paths.journal(), &encode(journal)?)
}

fn encode(journal: &TransitionJournal) -> Result<Vec<u8>, StorageTransitionError> {
    let document = JournalDocument {
        schema_version: JOURNAL_SCHEMA_VERSION,
        transition_id: journal.transition_id.clone(),
        phase: journal.phase,
        source_layout_sha256: journal.source_layout_digest.as_str().into(),
        target_layout_sha256: journal.target_layout_digest.as_str().into(),
        confirmation_sha256: journal.confirmation_digest.as_str().into(),
        domains: journal
            .domains
            .iter()
            .map(JournalDomainDocument::from_domain)
            .collect(),
    };
    journal_file::encode(&document, "storage transition journal")
        .map_err(|error| StorageTransitionError::Journal(error.to_string()))
}

pub(super) fn read(
    paths: &StorageBootstrapPaths,
) -> Result<Option<TransitionJournal>, StorageTransitionError> {
    let relative = Path::new(paths.journal().file_name().expect("fixed journal name"));
    let Some(bytes) = io::read(paths.directory(), relative, journal_file::MAX_JOURNAL_BYTES)
        .map_err(|error| StorageTransitionError::Journal(error.to_string()))?
    else {
        return Ok(None);
    };
    let document: JournalDocument = serde_json::from_slice(&bytes)
        .map_err(|error| StorageTransitionError::Journal(error.to_string()))?;
    if document.schema_version != JOURNAL_SCHEMA_VERSION {
        return Err(StorageTransitionError::Journal(
            "unsupported transition journal schema".into(),
        ));
    }
    Ok(Some(TransitionJournal {
        transition_id: document.transition_id,
        phase: document.phase,
        source_layout_digest: parse_digest(document.source_layout_sha256)?,
        target_layout_digest: parse_digest(document.target_layout_sha256)?,
        confirmation_digest: parse_digest(document.confirmation_sha256)?,
        domains: document
            .domains
            .into_iter()
            .map(JournalDomainDocument::into_domain)
            .collect::<Result<_, _>>()?,
    }))
}

pub(super) fn cleanup(
    paths: &StorageBootstrapPaths,
    transition_id: &str,
) -> Result<(), StorageTransitionError> {
    io::remove_file(paths.journal())?;
    io::remove_tree(&paths.transitions().join(transition_id))
}

fn parse_digest(value: String) -> Result<Sha256Digest, StorageTransitionError> {
    Sha256Digest::new(value).map_err(|error| StorageTransitionError::Journal(error.to_string()))
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct JournalDocument {
    schema_version: u32,
    transition_id: String,
    phase: JournalPhase,
    source_layout_sha256: String,
    target_layout_sha256: String,
    confirmation_sha256: String,
    domains: Vec<JournalDomainDocument>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct JournalDomainDocument {
    domain: String,
    source_path: Option<PathBuf>,
    target_path: Option<PathBuf>,
    source: Option<EvidenceDocument>,
    target_expected: Option<EvidenceDocument>,
    custom: bool,
    #[serde(default)]
    target_was_absent: bool,
}

impl JournalDomainDocument {
    fn from_domain(domain: &JournalDomain) -> Self {
        Self {
            domain: domain.domain.clone(),
            source_path: domain.source_path.clone(),
            target_path: domain.target_path.clone(),
            source: domain.source.as_ref().map(EvidenceDocument::from_evidence),
            target_expected: domain
                .target_expected
                .as_ref()
                .map(EvidenceDocument::from_evidence),
            custom: domain.custom,
            target_was_absent: domain.target_was_absent,
        }
    }

    fn into_domain(self) -> Result<JournalDomain, StorageTransitionError> {
        Ok(JournalDomain {
            domain: self.domain,
            source_path: self.source_path,
            target_path: self.target_path,
            source: self
                .source
                .map(EvidenceDocument::into_evidence)
                .transpose()?,
            target_expected: self
                .target_expected
                .map(EvidenceDocument::into_evidence)
                .transpose()?,
            custom: self.custom,
            target_was_absent: self.target_was_absent,
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EvidenceDocument {
    state: EvidenceState,
    byte_length: Option<usize>,
    sha256: Option<String>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum EvidenceState {
    Absent,
    Present,
    Semantic,
}

impl EvidenceDocument {
    fn from_evidence(evidence: &StorageFileEvidence) -> Self {
        match evidence {
            StorageFileEvidence::Absent => Self {
                state: EvidenceState::Absent,
                byte_length: None,
                sha256: None,
            },
            StorageFileEvidence::Present {
                byte_length,
                sha256,
            } => Self {
                state: EvidenceState::Present,
                byte_length: Some(*byte_length),
                sha256: Some(sha256.as_str().into()),
            },
            StorageFileEvidence::Semantic { sha256 } => Self {
                state: EvidenceState::Semantic,
                byte_length: None,
                sha256: Some(sha256.as_str().into()),
            },
        }
    }

    fn into_evidence(self) -> Result<StorageFileEvidence, StorageTransitionError> {
        match (self.state, self.byte_length, self.sha256) {
            (EvidenceState::Absent, None, None) => Ok(StorageFileEvidence::Absent),
            (EvidenceState::Present, Some(byte_length), Some(sha256)) => {
                Ok(StorageFileEvidence::Present {
                    byte_length,
                    sha256: parse_digest(sha256)?,
                })
            }
            (EvidenceState::Semantic, None, Some(sha256)) => Ok(StorageFileEvidence::Semantic {
                sha256: parse_digest(sha256)?,
            }),
            _ => Err(StorageTransitionError::Journal(
                "invalid transition evidence shape".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use longhorn_storage_layout::{
        PlatformDirectoryFacts, StorageIdentity, TargetPlatform, resolve_storage_bootstrap_paths,
    };

    fn journal() -> TransitionJournal {
        let digest = Sha256Digest::from_bytes(b"layout");
        TransitionJournal {
            transition_id: "interrupted".into(),
            phase: JournalPhase::Prepared,
            source_layout_digest: digest.clone(),
            target_layout_digest: digest.clone(),
            confirmation_digest: digest,
            domains: Vec::new(),
        }
    }

    #[test]
    fn journal_read_and_write_enforce_the_same_byte_bound() {
        let temp = tempfile::tempdir().unwrap();
        let identity = StorageIdentity::new("com.example.transfer").unwrap();
        let facts = PlatformDirectoryFacts::complete(
            TargetPlatform::Linux,
            temp.path().join("config"),
            temp.path().join("data"),
            temp.path().join("state"),
            temp.path().join("cache"),
            temp.path().join("log"),
            temp.path().join("runtime"),
        );
        let paths = resolve_storage_bootstrap_paths(&identity, &facts).unwrap();
        let mut journal = journal();
        journal.transition_id = "x".repeat(journal_file::MAX_JOURNAL_BYTES);
        assert!(
            matches!(write(&paths, &journal), Err(StorageTransitionError::Journal(detail))
            if detail.contains("exceeds byte limit"))
        );
        assert!(!paths.journal().exists());

        std::fs::create_dir_all(paths.directory()).unwrap();
        std::fs::File::create(paths.journal())
            .unwrap()
            .set_len(journal_file::MAX_JOURNAL_BYTES as u64 + 1)
            .unwrap();
        assert!(
            matches!(read(&paths), Err(StorageTransitionError::Journal(detail))
            if detail.contains("bound exceeded"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn journal_refuses_paths_it_cannot_round_trip() {
        use std::os::unix::ffi::OsStringExt;

        let mut journal = journal();
        journal.domains.push(JournalDomain {
            domain: "example.path".into(),
            source_path: Some(std::ffi::OsString::from_vec(vec![b'/', 0xff]).into()),
            target_path: None,
            source: Some(StorageFileEvidence::Absent),
            target_expected: Some(StorageFileEvidence::Absent),
            custom: false,
            target_was_absent: false,
        });
        assert!(matches!(
            encode(&journal),
            Err(StorageTransitionError::Journal(_))
        ));
    }
}
