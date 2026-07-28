use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use longhorn_core::DomainId;

use crate::{
    BackupAdapter, BackupAdapterError, BackupExclusionReason, ConfigDomain, DomainDescriptor,
    Sha256Digest,
};

/// Opaque guard held across adapter snapshot, restore, and locator commit.
pub trait StorageTransitionGuard {}

impl<T> StorageTransitionGuard for T {}

/// Card-010 adapter with a quiesced transition authority.
pub trait StorageTransitionAdapter: BackupAdapter {
    /// Returns a stable key used for deterministic authority ordering.
    fn transition_authority(&self) -> &str;

    /// Quiesces the adapter authority until the returned guard is dropped.
    fn acquire_transition_guard(
        &self,
        descriptor: &DomainDescriptor,
        timeout: Duration,
    ) -> Result<Box<dyn StorageTransitionGuard + '_>, BackupAdapterError>;

    /// Returns exact adapter-owned files hidden from ordinary unknown-file inventory.
    fn owned_paths(&self, descriptor: &DomainDescriptor) -> Vec<PathBuf>;

    /// Inspects current state without mutation.
    fn current_evidence(
        &self,
        descriptor: &DomainDescriptor,
    ) -> Result<Option<Sha256Digest>, BackupAdapterError>;
}

/// Explicit per-domain policy for one source-to-target layout transition.
pub struct StorageTransitionCatalog<'adapters> {
    entries: BTreeMap<DomainId, Entry<'adapters>>,
}

impl<'adapters> StorageTransitionCatalog<'adapters> {
    /// Starts an empty transition policy.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Includes one ordinary registered file domain.
    pub fn include<D: ConfigDomain>(&mut self, domain: &D) -> Result<(), DomainId> {
        self.insert(domain.descriptor(), EntryPolicy::Include)
    }

    /// Excludes one domain with a stable visible reason.
    pub fn exclude<D: ConfigDomain>(
        &mut self,
        domain: &D,
        reason: BackupExclusionReason,
    ) -> Result<(), DomainId> {
        self.insert(domain.descriptor(), EntryPolicy::Exclude(reason))
    }

    /// Assigns schema-opaque source and target adapters.
    pub fn custom<D: ConfigDomain>(
        &mut self,
        domain: &D,
        source: &'adapters dyn StorageTransitionAdapter,
        target: &'adapters dyn StorageTransitionAdapter,
    ) -> Result<(), DomainId> {
        self.insert(domain.descriptor(), EntryPolicy::Custom { source, target })
    }

    fn insert(
        &mut self,
        descriptor: &DomainDescriptor,
        policy: EntryPolicy<'adapters>,
    ) -> Result<(), DomainId> {
        let id = descriptor.id().clone();
        if self.entries.contains_key(&id) {
            return Err(id);
        }
        self.entries.insert(
            id,
            Entry {
                descriptor: descriptor.clone(),
                policy,
            },
        );
        Ok(())
    }

    pub(crate) fn decision(&self, descriptor: &DomainDescriptor) -> TransitionDecision<'_> {
        let Some(entry) = self.entries.get(descriptor.id()) else {
            return TransitionDecision::Missing;
        };
        if entry.descriptor != *descriptor {
            return TransitionDecision::DescriptorChanged;
        }
        match &entry.policy {
            EntryPolicy::Include => TransitionDecision::Include,
            EntryPolicy::Exclude(reason) => TransitionDecision::Exclude(reason),
            EntryPolicy::Custom { source, target } => TransitionDecision::Custom(*source, *target),
        }
    }
}

impl Default for StorageTransitionCatalog<'_> {
    fn default() -> Self {
        Self::new()
    }
}

struct Entry<'adapters> {
    descriptor: DomainDescriptor,
    policy: EntryPolicy<'adapters>,
}

enum EntryPolicy<'adapters> {
    Include,
    Exclude(BackupExclusionReason),
    Custom {
        source: &'adapters dyn StorageTransitionAdapter,
        target: &'adapters dyn StorageTransitionAdapter,
    },
}

pub(crate) enum TransitionDecision<'adapters> {
    Include,
    Exclude(&'adapters BackupExclusionReason),
    Custom(
        &'adapters dyn StorageTransitionAdapter,
        &'adapters dyn StorageTransitionAdapter,
    ),
    Missing,
    DescriptorChanged,
}
