use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Sha256Digest, StorageBootstrapPaths};

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
}

impl TransitionJournal {
    pub(super) fn from_plan(transition_id: String, plan: &StorageTransitionPlan) -> Self {
        let domains = plan
            .preview
            .domains
            .iter()
            .filter_map(|entry| {
                let custom = matches!(entry.action, StorageTransitionAction::CustomAdapter { .. });
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
                    }
                })
            })
            .collect();
        Self {
            transition_id,
            phase: JournalPhase::Prepared,
            source_layout_digest: plan.preview.source_layout_digest.clone(),
            target_layout_digest: plan.preview.target_layout_digest.clone(),
            confirmation_digest: plan.preview.confirmation_digest.clone(),
            domains,
        }
    }
}

pub(super) fn write(
    paths: &StorageBootstrapPaths,
    journal: &TransitionJournal,
) -> Result<(), StorageTransitionError> {
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
    let bytes = serde_json::to_vec(&document)
        .map_err(|error| StorageTransitionError::Journal(error.to_string()))?;
    io::atomic_write(paths.journal(), &bytes)
}

pub(super) fn read(
    paths: &StorageBootstrapPaths,
) -> Result<Option<TransitionJournal>, StorageTransitionError> {
    let bytes = match fs::read(paths.journal()) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(StorageTransitionError::Journal(error.to_string())),
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
    source_path: Option<String>,
    target_path: Option<String>,
    source: Option<EvidenceDocument>,
    target_expected: Option<EvidenceDocument>,
    custom: bool,
}

impl JournalDomainDocument {
    fn from_domain(domain: &JournalDomain) -> Self {
        Self {
            domain: domain.domain.clone(),
            source_path: domain
                .source_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            target_path: domain
                .target_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            source: domain.source.as_ref().map(EvidenceDocument::from_evidence),
            target_expected: domain
                .target_expected
                .as_ref()
                .map(EvidenceDocument::from_evidence),
            custom: domain.custom,
        }
    }

    fn into_domain(self) -> Result<JournalDomain, StorageTransitionError> {
        Ok(JournalDomain {
            domain: self.domain,
            source_path: self.source_path.map(PathBuf::from),
            target_path: self.target_path.map(PathBuf::from),
            source: self
                .source
                .map(EvidenceDocument::into_evidence)
                .transpose()?,
            target_expected: self
                .target_expected
                .map(EvidenceDocument::into_evidence)
                .transpose()?,
            custom: self.custom,
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
