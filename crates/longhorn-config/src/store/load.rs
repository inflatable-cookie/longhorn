use std::{io, path::PathBuf};

use cap_std::{ambient_authority, fs::Dir};
use longhorn_core::SchemaVersion;
use serde_json::Value;

use crate::{ConfigDomain, DomainIssue, MigrationStep, ResolvedFile};

use super::{
    LoadDiagnostic, LoadDiagnosticCode, LoadOutcome, LoadedConfig, LoadedOrigin, RecoveryKind,
    RecoveryState, SourceDocument, SourcePosition, document::SerializedDocument,
};

pub(crate) fn load_file<D: ConfigDomain>(domain: &D, file: &ResolvedFile) -> LoadOutcome<D::Value> {
    #[cfg(feature = "toml")]
    if domain.descriptor().is_human_toml() {
        return crate::toml_domain::load_toml_file(domain, file);
    }

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
                position: None,
            });
        }
    };

    load_source(domain, source)
}

pub(crate) fn read_source(file: &ResolvedFile) -> io::Result<SourceDocument> {
    let directory = Dir::open_ambient_dir(file.root(), ambient_authority())?;
    let bytes = directory.read(file.relative_path().as_path())?;

    Ok(SourceDocument {
        path: file.full_path().to_path_buf(),
        bytes,
    })
}

pub(crate) fn validated_default<D: ConfigDomain>(
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
            None,
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

pub(crate) fn load_source<D: ConfigDomain>(
    domain: &D,
    source: SourceDocument,
) -> LoadOutcome<D::Value> {
    let document: SerializedDocument = match serde_json::from_slice(&source.bytes) {
        Ok(document) => document,
        Err(error) => {
            return LoadOutcome::Recovery(RecoveryState {
                kind: RecoveryKind::CorruptDocument,
                path: Some(source.path.clone()),
                source: Some(source),
                detail: error.to_string(),
                position: None,
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
            position: None,
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
            position: None,
        });
    }

    migrate_and_decode(domain, source, document)
}

fn migrate_and_decode<D: ConfigDomain>(
    domain: &D,
    source: SourceDocument,
    document: SerializedDocument,
) -> LoadOutcome<D::Value> {
    migrate_from(
        domain,
        source,
        document.schema_version,
        document.value,
        None,
    )
}

/// Validates, migrates in memory, and decodes a raw value declared at
/// `version`. `content_position` pins domain issues on the file's own content
/// to the document start for human TOML loads; JSON loads pass `None`.
pub(crate) fn migrate_from<D: ConfigDomain>(
    domain: &D,
    source: SourceDocument,
    version: SchemaVersion,
    mut value: Value,
    content_position: Option<SourcePosition>,
) -> LoadOutcome<D::Value> {
    let descriptor = domain.descriptor();
    let mut version = version;
    let source_version = version;

    if let Err(issue) = domain.validate_raw(version, &value) {
        return LoadOutcome::Recovery(recovery_from_issue(
            RecoveryKind::InvalidValue,
            Some(source.path.clone()),
            Some(source),
            issue,
            content_position,
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
                    position: None,
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
                    position: None,
                });
            }
            Err(issue) => {
                return LoadOutcome::Recovery(recovery_from_issue(
                    RecoveryKind::MigrationFailed,
                    Some(source.path.clone()),
                    Some(source),
                    issue,
                    content_position,
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
                content_position,
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
                content_position,
            ));
        }
    };
    if let Err(issue) = domain.validate(&decoded) {
        return LoadOutcome::Recovery(recovery_from_issue(
            RecoveryKind::InvalidValue,
            Some(source.path.clone()),
            Some(source),
            issue,
            content_position,
        ));
    }

    let origin = if source_version == version {
        LoadedOrigin::File
    } else {
        LoadedOrigin::MigratedInMemory {
            from: source_version,
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
        position: None,
    }
}

fn recovery_from_issue(
    kind: RecoveryKind,
    path: Option<PathBuf>,
    source: Option<SourceDocument>,
    issue: DomainIssue,
    position: Option<SourcePosition>,
) -> RecoveryState {
    RecoveryState {
        kind,
        path,
        source,
        detail: format!("{}: {}", issue.code, issue.message),
        position,
    }
}
