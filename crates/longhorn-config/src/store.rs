mod load;
mod types;

pub use types::{
    LoadDiagnostic, LoadDiagnosticCode, LoadOutcome, LoadedConfig, LoadedOrigin, RecoveryKind,
    RecoveryState, SourceDocument, StoreError, UnavailableState,
};

use crate::{
    ConfigDomain, DomainLocation, RegistrationError, StorageRoots, registry::DomainRegistry,
};

use self::load::{load_file, validated_default};

/// Registered, read-only configuration domain store.
#[derive(Debug)]
pub struct ConfigStore {
    roots: StorageRoots,
    registry: DomainRegistry,
}

impl ConfigStore {
    /// Constructs an empty store over injected roots.
    #[must_use]
    pub fn new(roots: StorageRoots) -> Self {
        Self {
            roots,
            registry: DomainRegistry::default(),
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

    fn ensure_registered<D: ConfigDomain>(&self, domain: &D) -> Result<(), StoreError> {
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
