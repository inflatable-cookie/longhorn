use std::{io, path::PathBuf};

use cap_std::{ambient_authority, fs::Dir};
use longhorn_core::SchemaVersion;

use crate::{ConfigDomain, DomainIssue, MigrationStep, ResolvedFile};

use super::{
    LoadDiagnostic, LoadDiagnosticCode, LoadOutcome, LoadedConfig, LoadedOrigin, RecoveryKind,
    RecoveryState, SourceDocument, document::SerializedDocument,
};

pub(super) fn load_file<D: ConfigDomain>(domain: &D, file: &ResolvedFile) -> LoadOutcome<D::Value> {
    let source = match read_source(file) {
        Ok(source) => source,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return validated_default(domain, Some(file.full_path().to_path_buf()), true);
        }
        Err(error) => {
            return LoadOutcome::Recovery(RecoveryState {
                kind: RecoveryKind::ReadFailed,
                path: Some(file.full_path().to_path_buf()),
                source: None,
                detail: error.to_string(),
            });
        }
    };

    load_source(domain, source)
}

fn read_source(file: &ResolvedFile) -> io::Result<SourceDocument> {
    let directory = Dir::open_ambient_dir(file.root(), ambient_authority())?;
    let bytes = directory.read(file.relative_path().as_path())?;

    Ok(SourceDocument {
        path: file.full_path().to_path_buf(),
        bytes,
    })
}

pub(super) fn validated_default<D: ConfigDomain>(
    domain: &D,
    path: Option<PathBuf>,
    missing: bool,
) -> LoadOutcome<D::Value> {
    let value = domain.default_value();
    if let Err(issue) = domain.validate(&value) {
        return LoadOutcome::Recovery(recovery_from_issue(
            RecoveryKind::InvalidDefault,
            path,
            None,
            issue,
        ));
    }

    let diagnostics = if missing {
        vec![LoadDiagnostic {
            code: LoadDiagnosticCode::Missing,
            message: "domain file is missing; using the compiled default".to_owned(),
        }]
    } else {
        Vec::new()
    };

    LoadOutcome::Ready(LoadedConfig {
        value,
        schema_version: domain.descriptor().schema_version(),
        origin: LoadedOrigin::Default,
        diagnostics,
        source: None,
    })
}

fn load_source<D: ConfigDomain>(domain: &D, source: SourceDocument) -> LoadOutcome<D::Value> {
    let document: SerializedDocument = match serde_json::from_slice(&source.bytes) {
        Ok(document) => document,
        Err(error) => {
            return LoadOutcome::Recovery(RecoveryState {
                kind: RecoveryKind::CorruptDocument,
                path: Some(source.path.clone()),
                source: Some(source),
                detail: error.to_string(),
            });
        }
    };

    let descriptor = domain.descriptor();
    if document.domain != *descriptor.id() {
        return LoadOutcome::Recovery(RecoveryState {
            kind: RecoveryKind::DomainMismatch,
            path: Some(source.path.clone()),
            source: Some(source),
            detail: format!(
                "expected domain {}, found {}",
                descriptor.id(),
                document.domain
            ),
        });
    }

    if document.schema_version > descriptor.schema_version() {
        return LoadOutcome::Recovery(RecoveryState {
            kind: RecoveryKind::FutureSchema,
            path: Some(source.path.clone()),
            source: Some(source),
            detail: format!(
                "schema {} is newer than supported schema {}",
                document.schema_version,
                descriptor.schema_version()
            ),
        });
    }

    migrate_and_decode(domain, source, document)
}

fn migrate_and_decode<D: ConfigDomain>(
    domain: &D,
    source: SourceDocument,
    document: SerializedDocument,
) -> LoadOutcome<D::Value> {
    let descriptor = domain.descriptor();
    let original_version = document.schema_version;
    let mut version = original_version;
    let mut value = document.value;

    if let Err(issue) = domain.validate_raw(version, &value) {
        return LoadOutcome::Recovery(recovery_from_issue(
            RecoveryKind::InvalidValue,
            Some(source.path.clone()),
            Some(source),
            issue,
        ));
    }

    while version < descriptor.schema_version() {
        let expected = match version.checked_next() {
            Some(expected) => expected,
            None => {
                return LoadOutcome::Recovery(RecoveryState {
                    kind: RecoveryKind::InvalidMigrationStep,
                    path: Some(source.path.clone()),
                    source: Some(source),
                    detail: format!("schema {version} cannot advance"),
                });
            }
        };

        let step = match domain.migrate_one(version, value) {
            Ok(Some(step)) => step,
            Ok(None) => {
                return LoadOutcome::Recovery(RecoveryState {
                    kind: RecoveryKind::MissingMigration,
                    path: Some(source.path.clone()),
                    source: Some(source),
                    detail: format!("missing migration from schema {version}"),
                });
            }
            Err(issue) => {
                return LoadOutcome::Recovery(recovery_from_issue(
                    RecoveryKind::MigrationFailed,
                    Some(source.path.clone()),
                    Some(source),
                    issue,
                ));
            }
        };

        if step.schema_version != expected || step.schema_version > descriptor.schema_version() {
            return LoadOutcome::Recovery(invalid_migration_step(source, version, expected, step));
        }

        if let Err(issue) = domain.validate_raw(step.schema_version, &step.value) {
            return LoadOutcome::Recovery(recovery_from_issue(
                RecoveryKind::InvalidValue,
                Some(source.path.clone()),
                Some(source),
                issue,
            ));
        }

        version = step.schema_version;
        value = step.value;
    }

    let decoded = match domain.decode(value) {
        Ok(decoded) => decoded,
        Err(issue) => {
            return LoadOutcome::Recovery(recovery_from_issue(
                RecoveryKind::DecodeFailed,
                Some(source.path.clone()),
                Some(source),
                issue,
            ));
        }
    };
    if let Err(issue) = domain.validate(&decoded) {
        return LoadOutcome::Recovery(recovery_from_issue(
            RecoveryKind::InvalidValue,
            Some(source.path.clone()),
            Some(source),
            issue,
        ));
    }

    let origin = if original_version == version {
        LoadedOrigin::File
    } else {
        LoadedOrigin::MigratedInMemory {
            from: original_version,
            to: version,
        }
    };

    LoadOutcome::Ready(LoadedConfig {
        value: decoded,
        schema_version: version,
        origin,
        diagnostics: Vec::new(),
        source: Some(source),
    })
}

fn invalid_migration_step(
    source: SourceDocument,
    from: SchemaVersion,
    expected: SchemaVersion,
    step: MigrationStep,
) -> RecoveryState {
    RecoveryState {
        kind: RecoveryKind::InvalidMigrationStep,
        path: Some(source.path.clone()),
        source: Some(source),
        detail: format!(
            "migration from schema {from} returned schema {}; expected {expected}",
            step.schema_version
        ),
    }
}

fn recovery_from_issue(
    kind: RecoveryKind,
    path: Option<PathBuf>,
    source: Option<SourceDocument>,
    issue: DomainIssue,
) -> RecoveryState {
    RecoveryState {
        kind,
        path,
        source,
        detail: format!("{}: {}", issue.code, issue.message),
    }
}
