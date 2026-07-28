use std::io::{self, Read};

use cap_std::{ambient_authority, fs::Dir};
use longhorn_core::{DomainId, SchemaVersion};

use crate::{
    BackupCaptureError, BackupSourceIssue, ConfigDomain, LoadOutcome, LoadedOrigin, RecoveryKind,
    ResolvedFile, SourceDocument, store::document::SerializedDocument, store::load::load_source,
};

pub(crate) fn capture_typed_source<D: ConfigDomain>(
    domain: &D,
    file: &ResolvedFile,
    max_bytes: usize,
) -> Result<CapturedSource, SourceCaptureError> {
    let source = match read_source_bounded(file, max_bytes) {
        Ok(source) => source,
        Err(BoundedReadError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(CapturedSource::Absent);
        }
        Err(error) => return Err(error.into()),
    };
    let source_schema_version = serde_json::from_slice::<SerializedDocument>(&source.bytes)
        .ok()
        .map(|document| document.schema_version);

    match load_source(domain, source.clone()) {
        LoadOutcome::Ready(loaded) => {
            debug_assert!(matches!(
                loaded.origin,
                LoadedOrigin::File | LoadedOrigin::MigratedInMemory { .. }
            ));
            Ok(CapturedSource::Present {
                source_schema_version,
                bytes: source.bytes,
            })
        }
        LoadOutcome::Recovery(recovery) => {
            let Some(source) = recovery.source else {
                return Err(SourceCaptureError::UnclassifiedRecovery {
                    kind: recovery.kind,
                    detail: recovery.detail,
                });
            };
            let Some(issue) = BackupSourceIssue::from_recovery(recovery.kind) else {
                return Err(SourceCaptureError::UnclassifiedRecovery {
                    kind: recovery.kind,
                    detail: recovery.detail,
                });
            };
            Ok(CapturedSource::SourcePreserved {
                source_schema_version,
                issue,
                bytes: source.bytes,
            })
        }
        LoadOutcome::Unavailable(unavailable) => Err(SourceCaptureError::UnexpectedUnavailable {
            detail: format!("{unavailable:?}"),
        }),
    }
}

fn read_source_bounded(
    file: &ResolvedFile,
    max_bytes: usize,
) -> Result<SourceDocument, BoundedReadError> {
    let directory =
        Dir::open_ambient_dir(file.root(), ambient_authority()).map_err(BoundedReadError::Io)?;
    let input = directory
        .open(file.relative_path().as_path())
        .map_err(BoundedReadError::Io)?;
    let metadata = input.metadata().map_err(BoundedReadError::Io)?;
    let observed = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
    if observed > max_bytes {
        return Err(BoundedReadError::TooLarge {
            limit: max_bytes,
            observed,
        });
    }

    let read_limit = u64::try_from(max_bytes)
        .ok()
        .and_then(|limit| limit.checked_add(1))
        .expect("backup hard ceiling always fits u64");
    let mut bytes = Vec::with_capacity(observed);
    input
        .take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(BoundedReadError::Io)?;
    if bytes.len() > max_bytes {
        return Err(BoundedReadError::TooLarge {
            limit: max_bytes,
            observed: bytes.len(),
        });
    }

    Ok(SourceDocument {
        path: file.full_path().to_path_buf(),
        bytes,
    })
}

pub(super) fn map_source_error(domain: &DomainId, error: SourceCaptureError) -> BackupCaptureError {
    match error {
        SourceCaptureError::Read(error) => match error {
            BoundedReadError::TooLarge { limit, observed } => BackupCaptureError::DomainTooLarge {
                domain: domain.clone(),
                limit,
                observed,
            },
            BoundedReadError::Io(error) => BackupCaptureError::Unreadable {
                domain: domain.clone(),
                detail: error.to_string(),
            },
        },
        SourceCaptureError::UnclassifiedRecovery { kind, detail } => {
            BackupCaptureError::UnclassifiedRecovery {
                domain: domain.clone(),
                kind,
                detail,
            }
        }
        SourceCaptureError::UnexpectedUnavailable { detail } => BackupCaptureError::Unreadable {
            domain: domain.clone(),
            detail,
        },
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CapturedSource {
    Absent,
    Present {
        source_schema_version: Option<SchemaVersion>,
        bytes: Vec<u8>,
    },
    SourcePreserved {
        source_schema_version: Option<SchemaVersion>,
        issue: BackupSourceIssue,
        bytes: Vec<u8>,
    },
}

pub(crate) enum SourceCaptureError {
    Read(BoundedReadError),
    UnclassifiedRecovery { kind: RecoveryKind, detail: String },
    UnexpectedUnavailable { detail: String },
}

impl From<BoundedReadError> for SourceCaptureError {
    fn from(error: BoundedReadError) -> Self {
        Self::Read(error)
    }
}

pub(crate) enum BoundedReadError {
    Io(io::Error),
    TooLarge { limit: usize, observed: usize },
}
