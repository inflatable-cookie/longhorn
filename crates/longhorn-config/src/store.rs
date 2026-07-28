mod document;
mod load;
pub(super) mod mutation;
mod publication;
mod types;

pub use types::{
    Durability, DurabilityRequirement, LoadDiagnostic, LoadDiagnosticCode, LoadOutcome,
    LoadedConfig, LoadedOrigin, MutationError, MutationOptions, MutationReceipt, MutationRefusal,
    PublicationFailure, PublicationStage, RecoveryKind, RecoveryState, SourceDocument, StoreError,
    UnavailableState,
};

use crate::{
    ConfigDomain, CoordinationAuthority, DomainIssue, DomainLocation, RegistrationError,
    StorageRoots, coordination::Coordinator, registry::DomainRegistry,
};

use self::load::{load_file, validated_default};

/// Registered configuration domain store.
#[derive(Debug)]
pub struct ConfigStore {
    pub(super) roots: StorageRoots,
    registry: DomainRegistry,
    pub(super) coordinator: Coordinator,
}

impl ConfigStore {
    /// Constructs an empty store over injected roots and coordination authority.
    #[must_use]
    pub fn new(roots: StorageRoots, coordination: CoordinationAuthority) -> Self {
        Self {
            roots,
            registry: DomainRegistry::default(),
            coordinator: Coordinator::new(coordination),
        }
    }

    /// Registers a typed domain descriptor.
    pub fn register<D: ConfigDomain>(&mut self, domain: &D) -> Result<(), RegistrationError> {
        self.registry.register(&self.roots, domain.descriptor())
    }

    /// Returns the registered domain's typed location.
    pub fn location<D: ConfigDomain>(&self, domain: &D) -> Result<DomainLocation, StoreError> {
        self.ensure_registered(domain)?;
        Ok(self.roots.resolve(domain.descriptor()))
    }

    /// Loads, validates, and when required migrates a registered domain.
    pub fn load<D: ConfigDomain>(&self, domain: &D) -> Result<LoadOutcome<D::Value>, StoreError> {
        self.ensure_registered(domain)?;
        let location = self.roots.resolve(domain.descriptor());

        Ok(match location {
            DomainLocation::DefaultsOnly => validated_default(domain, None, false),
            DomainLocation::SecureStoreRequired | DomainLocation::RootRequired { .. } => {
                LoadOutcome::Unavailable(UnavailableState { location })
            }
            DomainLocation::File(file) => load_file(domain, &file),
        })
    }

    /// Applies and atomically publishes a patch against the latest valid value.
    pub fn mutate<D, F>(
        &self,
        domain: &D,
        options: MutationOptions,
        patch: F,
    ) -> Result<MutationReceipt, MutationError>
    where
        D: ConfigDomain,
        F: FnOnce(&mut D::Value) -> Result<(), DomainIssue>,
    {
        mutation::mutate(self, domain, options, patch)
    }

    pub(super) fn ensure_registered<D: ConfigDomain>(&self, domain: &D) -> Result<(), StoreError> {
        let descriptor = domain.descriptor();
        let Some(registered) = self.registry.descriptor(descriptor.id()) else {
            return Err(StoreError::NotRegistered {
                id: descriptor.id().clone(),
            });
        };
        if registered != descriptor {
            return Err(StoreError::DescriptorChanged {
                id: descriptor.id().clone(),
            });
        }
        Ok(())
    }
}
