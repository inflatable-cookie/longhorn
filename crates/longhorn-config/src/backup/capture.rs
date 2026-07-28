mod assembly;
mod error;
mod source;

use longhorn_core::DomainId;

use crate::{
    BackupCaptureOptions, BackupCatalog, BackupConsistencyGroup, BackupExclusion,
    BackupExclusionReason, BackupManifestDomain, BackupMetadata, BackupPayloadManifest,
    BackupPayloadPath, BackupScope, BackupSnapshot, BackupSnapshotPayload, BackupSourceState,
    DomainDescriptor, DomainLocation, coordination::CoordinationGuard, store::ConfigStore,
};

use assembly::{CaptureAssembly, DeferredAdapterCapture};
pub use error::BackupCaptureError;
use source::map_source_error;
pub(crate) use source::{CapturedSource, SourceCaptureError, capture_typed_source};

use super::{BackupAdapterCaptureMode, BackupAdapterCaptureRequest, CatalogDecision};

pub(crate) fn capture(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    scope: &BackupScope,
    metadata: BackupMetadata,
    options: BackupCaptureOptions,
) -> Result<BackupSnapshot, BackupCaptureError> {
    validate_catalog(store, catalog)?;
    let descriptors = selected_descriptors(store, scope)?;
    let mut assembly = CaptureAssembly::new(descriptors.len(), options.limits.max_total_bytes());
    let guard = store
        .coordinator
        .acquire(options.lock_timeout)
        .map_err(BackupCaptureError::Coordination)?;
    capture_coordinated(
        store,
        catalog,
        &descriptors,
        options,
        &guard,
        &mut assembly,
        true,
    )?;
    drop(guard);
    capture_external(options, &mut assembly)?;
    assembly.finish(metadata)
}

pub(crate) fn capture_guarded(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    scope: &BackupScope,
    metadata: BackupMetadata,
    options: BackupCaptureOptions,
    guard: &CoordinationGuard<'_>,
) -> Result<BackupSnapshot, BackupCaptureError> {
    validate_catalog(store, catalog)?;
    let descriptors = selected_descriptors(store, scope)?;
    let mut assembly = CaptureAssembly::new(descriptors.len(), options.limits.max_total_bytes());
    capture_coordinated(
        store,
        catalog,
        &descriptors,
        options,
        guard,
        &mut assembly,
        false,
    )?;
    assembly.finish(metadata)
}

fn capture_coordinated<'catalog>(
    store: &ConfigStore,
    catalog: &'catalog BackupCatalog<'catalog>,
    descriptors: &[DomainDescriptor],
    options: BackupCaptureOptions,
    _guard: &CoordinationGuard<'_>,
    assembly: &mut CaptureAssembly<'catalog>,
    allow_external: bool,
) -> Result<(), BackupCaptureError> {
    for descriptor in descriptors {
        let decision = catalog.decision(descriptor).or_else(|| {
            BackupExclusionReason::default_for(descriptor.storage_class())
                .map(CatalogDecision::Exclude)
        });
        let Some(decision) = decision else {
            return Err(BackupCaptureError::MissingPolicy {
                domain: descriptor.id().clone(),
            });
        };

        match decision {
            CatalogDecision::DescriptorChanged => {
                return Err(BackupCaptureError::DescriptorChanged {
                    domain: descriptor.id().clone(),
                });
            }
            CatalogDecision::Exclude(reason) => {
                assembly
                    .exclusions
                    .push(BackupExclusion::new(descriptor, &reason));
            }
            CatalogDecision::Custom(adapter) => match adapter.capabilities().capture() {
                BackupAdapterCaptureMode::CoordinatedBounded => {
                    assembly.add_group(BackupConsistencyGroup::ordinary())?;
                    let captured = adapter
                        .capture(BackupAdapterCaptureRequest::new(descriptor, options.limits))
                        .map_err(|error| BackupCaptureError::AdapterFailed {
                            domain: descriptor.id().clone(),
                            adapter: adapter.id().as_str().into(),
                            error,
                        })?;
                    assembly.add_adapter_capture(
                        descriptor,
                        adapter,
                        BackupConsistencyGroup::ordinary(),
                        captured,
                        options,
                    )?;
                }
                BackupAdapterCaptureMode::ExternalSnapshot(group) if allow_external => {
                    assembly.deferred.push(DeferredAdapterCapture {
                        descriptor: descriptor.clone(),
                        adapter,
                        group: BackupConsistencyGroup::external(group),
                    });
                }
                BackupAdapterCaptureMode::ExternalSnapshot(_) => {
                    return Err(BackupCaptureError::ExternalAdapterRequiresUnlockedCapture {
                        domain: descriptor.id().clone(),
                        adapter: adapter.id().as_str().into(),
                    });
                }
                BackupAdapterCaptureMode::Excluded(reason) => {
                    assembly
                        .exclusions
                        .push(BackupExclusion::new(descriptor, reason));
                }
            },
            CatalogDecision::Include(domain) => {
                assembly.add_group(BackupConsistencyGroup::ordinary())?;
                let location = store.roots.resolve(descriptor);
                let DomainLocation::File(file) = location else {
                    return Err(BackupCaptureError::Unavailable {
                        domain: descriptor.id().clone(),
                        location,
                    });
                };
                let source = domain
                    .capture_source(&file, options.limits.max_domain_bytes())
                    .map_err(|error| map_source_error(descriptor.id(), error))?;

                match source {
                    CapturedSource::Absent => {
                        assembly.absent_domains += 1;
                        assembly
                            .manifest_domains
                            .push(BackupManifestDomain::absent(descriptor));
                    }
                    CapturedSource::Present {
                        source_schema_version,
                        bytes,
                    } => {
                        assembly.total_payload_bytes = checked_total(
                            assembly.total_payload_bytes,
                            bytes.len(),
                            options.limits.max_total_bytes(),
                        )
                        .map_err(|kind| {
                            total_limit_error(
                                descriptor.id(),
                                options.limits.max_total_bytes(),
                                kind,
                            )
                        })?;
                        let path = BackupPayloadPath::ordinary(descriptor.id());
                        let evidence = BackupPayloadManifest::new(path.clone(), &bytes);
                        assembly
                            .manifest_domains
                            .push(BackupManifestDomain::with_source(
                                descriptor,
                                BackupSourceState::Present,
                                source_schema_version,
                                None,
                                evidence,
                            ));
                        assembly.payloads.push(BackupSnapshotPayload::new(
                            descriptor.id().clone(),
                            path,
                            bytes,
                        ));
                    }
                    CapturedSource::SourcePreserved {
                        source_schema_version,
                        issue,
                        bytes,
                    } => {
                        assembly.total_payload_bytes = checked_total(
                            assembly.total_payload_bytes,
                            bytes.len(),
                            options.limits.max_total_bytes(),
                        )
                        .map_err(|kind| {
                            total_limit_error(
                                descriptor.id(),
                                options.limits.max_total_bytes(),
                                kind,
                            )
                        })?;
                        assembly.source_preserved_domains += 1;
                        let path = BackupPayloadPath::ordinary(descriptor.id());
                        let evidence = BackupPayloadManifest::new(path.clone(), &bytes);
                        assembly
                            .manifest_domains
                            .push(BackupManifestDomain::with_source(
                                descriptor,
                                BackupSourceState::SourcePreserved,
                                source_schema_version,
                                Some(issue),
                                evidence,
                            ));
                        assembly.payloads.push(BackupSnapshotPayload::new(
                            descriptor.id().clone(),
                            path,
                            bytes,
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn capture_external(
    options: BackupCaptureOptions,
    assembly: &mut CaptureAssembly<'_>,
) -> Result<(), BackupCaptureError> {
    for deferred in std::mem::take(&mut assembly.deferred) {
        let captured = deferred
            .adapter
            .capture(BackupAdapterCaptureRequest::new(
                &deferred.descriptor,
                options.limits,
            ))
            .map_err(|error| BackupCaptureError::AdapterFailed {
                domain: deferred.descriptor.id().clone(),
                adapter: deferred.adapter.id().as_str().into(),
                error,
            })?;
        assembly.add_group(deferred.group.clone())?;
        assembly.add_adapter_capture(
            &deferred.descriptor,
            deferred.adapter,
            deferred.group,
            captured,
            options,
        )?;
    }
    Ok(())
}

fn validate_catalog(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
) -> Result<(), BackupCaptureError> {
    for descriptor in catalog.descriptors() {
        match store.registered_descriptor(descriptor.id()) {
            None => {
                return Err(BackupCaptureError::CatalogDomainNotRegistered {
                    domain: descriptor.id().clone(),
                });
            }
            Some(registered) if registered != descriptor => {
                return Err(BackupCaptureError::DescriptorChanged {
                    domain: descriptor.id().clone(),
                });
            }
            Some(_) => {}
        }
    }
    Ok(())
}

fn selected_descriptors(
    store: &ConfigStore,
    scope: &BackupScope,
) -> Result<Vec<DomainDescriptor>, BackupCaptureError> {
    match scope {
        BackupScope::AllRegistered => Ok(store.registered_descriptors().cloned().collect()),
        BackupScope::Selected(domains) => domains
            .iter()
            .map(|id| {
                store.registered_descriptor(id).cloned().ok_or_else(|| {
                    BackupCaptureError::ScopeDomainNotRegistered { domain: id.clone() }
                })
            })
            .collect(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TotalLimitFailure {
    Overflow,
    Exceeded { observed: usize },
}

fn checked_total(
    current: usize,
    additional: usize,
    limit: usize,
) -> Result<usize, TotalLimitFailure> {
    let observed = current
        .checked_add(additional)
        .ok_or(TotalLimitFailure::Overflow)?;
    if observed > limit {
        Err(TotalLimitFailure::Exceeded { observed })
    } else {
        Ok(observed)
    }
}

fn total_limit_error(
    domain: &DomainId,
    limit: usize,
    failure: TotalLimitFailure,
) -> BackupCaptureError {
    match failure {
        TotalLimitFailure::Overflow => BackupCaptureError::TotalSizeOverflow {
            domain: domain.clone(),
        },
        TotalLimitFailure::Exceeded { observed } => BackupCaptureError::TotalTooLarge {
            domain: domain.clone(),
            limit,
            observed,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_total_rejects_overflow_and_limit_crossing() {
        assert_eq!(
            checked_total(usize::MAX, 1, usize::MAX),
            Err(TotalLimitFailure::Overflow)
        );
        assert_eq!(
            checked_total(4, 2, 5),
            Err(TotalLimitFailure::Exceeded { observed: 6 })
        );
        assert_eq!(checked_total(4, 1, 5), Ok(5));
    }
}
