use std::{collections::BTreeMap, time::Duration};

use super::super::{StorageTransitionError, StorageTransitionRequest, TransitionDecision};

pub(crate) fn acquire_adapter_guards<'request>(
    request: &'request StorageTransitionRequest<'request>,
    timeout: Duration,
) -> Result<Vec<Box<dyn super::super::StorageTransitionGuard + 'request>>, StorageTransitionError> {
    let mut authorities = BTreeMap::new();
    for descriptor in request.source_store.registered_descriptors() {
        if let TransitionDecision::Custom(source, target) = request.catalog.decision(descriptor) {
            authorities
                .entry(source.transition_authority().to_owned())
                .or_insert((source, descriptor));
            authorities
                .entry(target.transition_authority().to_owned())
                .or_insert((target, descriptor));
        }
    }
    authorities
        .into_values()
        .map(|(adapter, descriptor)| {
            adapter
                .acquire_transition_guard(descriptor, timeout)
                .map_err(|error| StorageTransitionError::Adapter {
                    domain: descriptor.id().clone(),
                    detail: error.to_string(),
                })
        })
        .collect()
}

pub(crate) fn acquire_store_guards<'request>(
    request: &'request StorageTransitionRequest<'request>,
    timeout: Duration,
) -> Result<
    (
        crate::coordination::CoordinationGuard<'request>,
        Option<crate::coordination::CoordinationGuard<'request>>,
    ),
    StorageTransitionError,
> {
    let source_root = request.source_store.coordinator.authority_root();
    let target_root = request.target_store.coordinator.authority_root();
    if source_root == target_root {
        return Ok((
            request
                .source_store
                .coordinator
                .acquire(timeout)
                .map_err(StorageTransitionError::Coordination)?,
            None,
        ));
    }
    if source_root < target_root {
        Ok((
            request
                .source_store
                .coordinator
                .acquire(timeout)
                .map_err(StorageTransitionError::Coordination)?,
            Some(
                request
                    .target_store
                    .coordinator
                    .acquire(timeout)
                    .map_err(StorageTransitionError::Coordination)?,
            ),
        ))
    } else {
        Ok((
            request
                .target_store
                .coordinator
                .acquire(timeout)
                .map_err(StorageTransitionError::Coordination)?,
            Some(
                request
                    .source_store
                    .coordinator
                    .acquire(timeout)
                    .map_err(StorageTransitionError::Coordination)?,
            ),
        ))
    }
}
