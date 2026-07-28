mod adapter;

use std::{collections::BTreeMap, path::Path};

use longhorn_core::SchemaVersion;

use crate::{
    AccessMode, BackupAdapterPayloadRef, BackupApplication, BackupArchiveInspection, BackupCatalog,
    BackupManifestDomain, BackupProducer, BackupSourceState, ConfigStore, DomainDescriptor,
    DomainLocation, RootKind, backup::CatalogDecision,
};

use super::{
    staging::PrepareSourceError,
    types::{
        PreparedAdapterTarget, PreparedTarget, RestoreDomainCompatibility, RestoreDomainInspection,
        RestoreExclusionInspection, RestoreIdentityInspection, RestoreInspection,
        RestoreInspectionReceipt,
    },
};

pub(crate) fn inspect(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    archive: &BackupArchiveInspection,
    application: &BackupApplication,
    producer: &BackupProducer,
) -> RestoreInspection {
    let manifest = archive.manifest();
    let identity = RestoreIdentityInspection::inspect(manifest, application, producer);
    let mut domains = Vec::with_capacity(manifest.domains().len());
    let mut prepared = BTreeMap::new();
    let mut custom_prepared = BTreeMap::new();

    for source in manifest.domains() {
        let (report, target, custom_target) = inspect_domain(store, catalog, archive, source);
        if let Some(target) = target {
            prepared.insert(source.domain().clone(), target);
        }
        if let Some(target) = custom_target {
            custom_prepared.insert(source.domain().clone(), target);
        }
        domains.push(report);
    }

    let exclusions = manifest
        .exclusions()
        .iter()
        .cloned()
        .map(|exclusion| {
            let registered = store.registered_descriptor(exclusion.domain()).is_some();
            RestoreExclusionInspection::new(exclusion, registered)
        })
        .collect::<Vec<_>>();

    let restorable = domains
        .iter()
        .filter(|domain| domain.compatibility().is_restorable())
        .count();
    let migrations = domains
        .iter()
        .filter(|domain| {
            matches!(
                domain.compatibility(),
                RestoreDomainCompatibility::MigrationRequired { .. }
            )
        })
        .count();
    let adapter_restorable = custom_prepared.len();
    let receipt = RestoreInspectionReceipt {
        manifest_domains: domains.len(),
        exclusions: exclusions.len(),
        restorable,
        migrations,
        adapter_restorable,
        blocked: domains.len() - restorable - adapter_restorable,
    };
    RestoreInspection {
        manifest: manifest.clone(),
        archive_sha256: archive.archive_sha256().clone(),
        identity,
        domains,
        exclusions,
        prepared,
        custom_prepared,
        receipt,
    }
}

fn inspect_domain(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    archive: &BackupArchiveInspection,
    source: &BackupManifestDomain,
) -> (
    RestoreDomainInspection,
    Option<PreparedTarget>,
    Option<PreparedAdapterTarget>,
) {
    let id = source.domain().clone();
    let Some(descriptor) = store.registered_descriptor(&id) else {
        return (
            domain_report(source, None, RestoreDomainCompatibility::UnknownDomain),
            None,
            None,
        );
    };
    let target_schema = Some(descriptor.schema_version());
    if descriptor.storage_class() != source.storage_class() {
        return (
            domain_report(
                source,
                target_schema,
                RestoreDomainCompatibility::DescriptorMismatch,
            ),
            None,
            None,
        );
    }

    let Some(decision) = catalog.decision(descriptor) else {
        return (
            domain_report(
                source,
                target_schema,
                RestoreDomainCompatibility::DomainCodeUnavailable,
            ),
            None,
            None,
        );
    };
    match decision {
        CatalogDecision::Include(domain) => {
            inspect_ordinary_domain(store, archive, source, descriptor, target_schema, domain)
        }
        CatalogDecision::Exclude(reason) => (
            domain_report(
                source,
                target_schema,
                RestoreDomainCompatibility::PolicyExcluded {
                    reason: reason.as_str().to_owned(),
                },
            ),
            None,
            None,
        ),
        CatalogDecision::Custom(adapter) => {
            adapter::inspect_custom_domain(archive, source, descriptor, target_schema, adapter)
        }
        CatalogDecision::DescriptorChanged => (
            domain_report(
                source,
                target_schema,
                RestoreDomainCompatibility::DescriptorMismatch,
            ),
            None,
            None,
        ),
    }
}

fn inspect_ordinary_domain(
    store: &ConfigStore,
    archive: &BackupArchiveInspection,
    source: &BackupManifestDomain,
    descriptor: &DomainDescriptor,
    target_schema: Option<SchemaVersion>,
    domain: &dyn super::super::catalog::ErasedBackupDomain,
) -> (
    RestoreDomainInspection,
    Option<PreparedTarget>,
    Option<PreparedAdapterTarget>,
) {
    if source.adapter() != "longhorn-json-v1" {
        return (
            domain_report(
                source,
                target_schema,
                RestoreDomainCompatibility::CustomAdapterUnavailable {
                    adapter: source.adapter().into(),
                },
            ),
            None,
            None,
        );
    }
    let location = store.roots.resolve(descriptor);
    if !ordinary_restore_target(&location) {
        return (
            domain_report(
                source,
                target_schema,
                RestoreDomainCompatibility::TargetUnavailable { location },
            ),
            None,
            None,
        );
    }
    match source.state() {
        BackupSourceState::Absent => (
            domain_report(source, target_schema, RestoreDomainCompatibility::Ready),
            Some(PreparedTarget {
                bytes: None,
                schema_version: None,
            }),
            None,
        ),
        BackupSourceState::SourcePreserved => {
            let issue = source
                .source_issue()
                .expect("verified source-preserved manifest declares an issue");
            (
                domain_report(
                    source,
                    target_schema,
                    RestoreDomainCompatibility::SourcePreserved { issue },
                ),
                None,
                None,
            )
        }
        BackupSourceState::Present => {
            let Some(payload) = payload_for(archive, source) else {
                return (
                    domain_report(
                        source,
                        target_schema,
                        RestoreDomainCompatibility::TargetPreparationFailed {
                            detail: "verified archive payload is unavailable".into(),
                        },
                    ),
                    None,
                    None,
                );
            };
            match domain.prepare_restore_source(
                payload,
                Path::new(
                    source
                        .payloads()
                        .first()
                        .expect("verified present source declares one payload")
                        .path()
                        .as_str(),
                ),
            ) {
                Ok(target) => {
                    let compatibility = if target.source_schema_version < target.schema_version {
                        RestoreDomainCompatibility::MigrationRequired {
                            from: target.source_schema_version,
                            to: target.schema_version,
                        }
                    } else {
                        RestoreDomainCompatibility::Ready
                    };
                    (
                        domain_report(source, target_schema, compatibility),
                        Some(PreparedTarget {
                            bytes: Some(target.bytes),
                            schema_version: Some(target.schema_version),
                        }),
                        None,
                    )
                }
                Err(PrepareSourceError::Source(issue)) => (
                    domain_report(
                        source,
                        target_schema,
                        RestoreDomainCompatibility::SourceRejected { issue },
                    ),
                    None,
                    None,
                ),
                Err(PrepareSourceError::Target(detail)) => (
                    domain_report(
                        source,
                        target_schema,
                        RestoreDomainCompatibility::TargetPreparationFailed { detail },
                    ),
                    None,
                    None,
                ),
            }
        }
    }
}

pub(super) fn domain_report(
    source: &BackupManifestDomain,
    target_schema: Option<SchemaVersion>,
    compatibility: RestoreDomainCompatibility,
) -> RestoreDomainInspection {
    RestoreDomainInspection::new(
        source.domain().clone(),
        source.state(),
        source.source_schema_version(),
        target_schema,
        compatibility,
    )
}

fn ordinary_restore_target(location: &DomainLocation) -> bool {
    matches!(
        location,
        DomainLocation::File(file)
            if file.access() == AccessMode::ReadWrite && file.root_kind() != RootKind::Project
    )
}

pub(super) fn payload_for<'archive>(
    archive: &'archive BackupArchiveInspection,
    source: &BackupManifestDomain,
) -> Option<&'archive [u8]> {
    let [payload] = source.payloads() else {
        return None;
    };
    let path = payload.path();
    archive
        .payloads()
        .iter()
        .find(|payload| payload.path() == path)
        .map(|payload| payload.bytes())
}

pub(super) fn payloads_for_adapter<'archive>(
    archive: &'archive BackupArchiveInspection,
    source: &BackupManifestDomain,
) -> Option<Vec<BackupAdapterPayloadRef<'archive>>> {
    source
        .payloads()
        .iter()
        .map(|manifest| {
            archive
                .payloads()
                .iter()
                .find(|payload| payload.path() == manifest.path())
                .map(|payload| BackupAdapterPayloadRef::new(payload.path(), payload.bytes()))
        })
        .collect()
}
