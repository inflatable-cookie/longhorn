use std::{
    io::{self, Read},
    path::PathBuf,
};

use cap_std::{ambient_authority, fs::Dir};
use longhorn_core::SchemaVersion;
use serde_json::Value;

use crate::{BackupLimits, ConfigDomain, DomainIssue, MigrationStep, ResolvedFile};

use super::{
    LoadDiagnostic, LoadDiagnosticCode, LoadOutcome, LoadedConfig, LoadedOrigin, RecoveryKind,
    RecoveryState, SourceDocument, SourcePosition, document::SerializedDocument,
};

/// Result of reading and decoding one domain file, excluding store-level
/// unavailability states that file loading cannot produce.
pub(crate) enum FileLoadOutcome<T> {
    Ready(LoadedConfig<T>),
    Recovery(RecoveryState),
}

impl<T> FileLoadOutcome<T> {
    pub(crate) fn into_load_outcome(self) -> LoadOutcome<T> {
        match self {
            Self::Ready(loaded) => LoadOutcome::Ready(loaded),
            Self::Recovery(recovery) => LoadOutcome::Recovery(recovery),
        }
    }
}

pub(crate) fn load_file<D: ConfigDomain>(
    domain: &D,
    file: &ResolvedFile,
) -> FileLoadOutcome<D::Value> {
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
            return FileLoadOutcome::Recovery(RecoveryState {
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
    let max_bytes = BackupLimits::default().max_domain_bytes();
    let directory = Dir::open_ambient_dir(file.root(), ambient_authority())?;
    let input = directory.open(file.relative_path().as_path())?;
    let observed = usize::try_from(input.metadata()?.len()).unwrap_or(usize::MAX);
    if observed > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("domain file exceeds {max_bytes}-byte limit ({observed} bytes)"),
        ));
    }
    let read_limit = u64::try_from(max_bytes)
        .ok()
        .and_then(|limit| limit.checked_add(1))
        .expect("domain byte limit fits u64");
    let mut bytes = Vec::with_capacity(observed);
    input.take(read_limit).read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "domain file exceeds {max_bytes}-byte limit ({} bytes)",
                bytes.len()
            ),
        ));
    }

    Ok(SourceDocument {
        path: file.full_path().to_path_buf(),
        bytes,
    })
}

pub(crate) fn validated_default<D: ConfigDomain>(
    domain: &D,
    path: Option<PathBuf>,
    missing: bool,
) -> FileLoadOutcome<D::Value> {
    let value = domain.default_value();
    if let Err(issue) = domain.validate(&value) {
        return FileLoadOutcome::Recovery(recovery_from_issue(
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

    FileLoadOutcome::Ready(LoadedConfig {
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
) -> FileLoadOutcome<D::Value> {
    let document: SerializedDocument = match serde_json::from_slice(&source.bytes) {
        Ok(document) => document,
        Err(error) => {
            return FileLoadOutcome::Recovery(RecoveryState {
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
        return FileLoadOutcome::Recovery(RecoveryState {
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
        return FileLoadOutcome::Recovery(RecoveryState {
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
) -> FileLoadOutcome<D::Value> {
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
) -> FileLoadOutcome<D::Value> {
    let descriptor = domain.descriptor();
    let mut version = version;
    let source_version = version;

    if let Err(issue) = domain.validate_raw(version, &value) {
        return FileLoadOutcome::Recovery(recovery_from_issue(
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
                return FileLoadOutcome::Recovery(RecoveryState {
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
                return FileLoadOutcome::Recovery(RecoveryState {
                    kind: RecoveryKind::MissingMigration,
                    path: Some(source.path.clone()),
                    source: Some(source),
                    detail: format!("missing migration from schema {version}"),
                    position: None,
                });
            }
            Err(issue) => {
                return FileLoadOutcome::Recovery(recovery_from_issue(
                    RecoveryKind::MigrationFailed,
                    Some(source.path.clone()),
                    Some(source),
                    issue,
                    content_position,
                ));
            }
        };

        if step.schema_version != expected || step.schema_version > descriptor.schema_version() {
            return FileLoadOutcome::Recovery(invalid_migration_step(
                source, version, expected, step,
            ));
        }

        if let Err(issue) = domain.validate_raw(step.schema_version, &step.value) {
            return FileLoadOutcome::Recovery(recovery_from_issue(
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
            return FileLoadOutcome::Recovery(recovery_from_issue(
                RecoveryKind::DecodeFailed,
                Some(source.path.clone()),
                Some(source),
                issue,
                content_position,
            ));
        }
    };
    if let Err(issue) = domain.validate(&decoded) {
        return FileLoadOutcome::Recovery(recovery_from_issue(
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

    FileLoadOutcome::Ready(LoadedConfig {
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

#[cfg(test)]
mod tests {
    use std::fs;

    use longhorn_core::{DomainId, SchemaVersion};
    use longhorn_storage_layout::StorageRoots;
    use tempfile::TempDir;

    use crate::{
        DomainDescriptor, DomainFilePath, DomainLocation, StorageClass, StorageRootsResolve,
    };

    use super::*;

    #[test]
    fn read_source_rejects_domain_file_exceeding_byte_limit() {
        let temp = TempDir::new().unwrap();
        let config = temp.path().join("config");
        fs::create_dir_all(&config).unwrap();
        let roots = StorageRoots::new(
            &config,
            temp.path().join("data"),
            temp.path().join("state"),
            temp.path().join("cache"),
            temp.path().join("runtime"),
            temp.path().join("log"),
            temp.path().join("backups"),
        )
        .unwrap();
        let descriptor = DomainDescriptor::new(
            DomainId::new("example.oversized").unwrap(),
            SchemaVersion::new(1).unwrap(),
            StorageClass::UserConfig,
            Some(DomainFilePath::new("oversized.json").unwrap()),
        )
        .unwrap();
        let DomainLocation::File(file) = roots.resolve(&descriptor) else {
            panic!("expected file location");
        };
        let oversized = BackupLimits::default().max_domain_bytes() + 1;
        fs::write(file.full_path(), vec![b' '; oversized]).unwrap();

        let error = read_source(&file).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("exceeds"));
    }
}
