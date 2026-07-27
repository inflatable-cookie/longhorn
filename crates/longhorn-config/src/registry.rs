use std::{collections::HashMap, error::Error, fmt, path::PathBuf};

use longhorn_core::DomainId;

use crate::{DomainDescriptor, DomainLocation, RootKind, StorageRoots};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum LocationKey {
    File(PathBuf),
    Deferred(RootKind, String),
}

/// Descriptor registry for one configuration store.
#[derive(Debug, Default)]
pub(crate) struct DomainRegistry {
    descriptors: HashMap<DomainId, DomainDescriptor>,
    locations: HashMap<LocationKey, DomainId>,
}

impl DomainRegistry {
    pub(crate) fn register(
        &mut self,
        roots: &StorageRoots,
        descriptor: &DomainDescriptor,
    ) -> Result<(), RegistrationError> {
        if self.descriptors.contains_key(descriptor.id()) {
            return Err(RegistrationError::DuplicateDomainId {
                id: descriptor.id().clone(),
            });
        }

        let location_key = location_key(roots.resolve(descriptor));
        if let Some(key) = location_key.as_ref() {
            if let Some(existing) = self.locations.get(key) {
                return Err(RegistrationError::DuplicateLocation {
                    existing: existing.clone(),
                    incoming: descriptor.id().clone(),
                });
            }
        }

        if let Some(key) = location_key {
            self.locations.insert(key, descriptor.id().clone());
        }
        self.descriptors
            .insert(descriptor.id().clone(), descriptor.clone());

        Ok(())
    }

    pub(crate) fn descriptor(&self, id: &DomainId) -> Option<&DomainDescriptor> {
        self.descriptors.get(id)
    }
}

fn location_key(location: DomainLocation) -> Option<LocationKey> {
    match location {
        DomainLocation::File(file) => Some(LocationKey::File(file.full_path().to_path_buf())),
        DomainLocation::RootRequired {
            root,
            relative_path,
        } => Some(LocationKey::Deferred(
            root,
            relative_path.as_str().to_owned(),
        )),
        DomainLocation::DefaultsOnly | DomainLocation::SecureStoreRequired => None,
    }
}

/// Domain registration failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistrationError {
    /// A domain id was already registered.
    DuplicateDomainId {
        /// Conflicting domain id.
        id: DomainId,
    },
    /// Two domains resolved to the same storage location.
    DuplicateLocation {
        /// Previously registered domain.
        existing: DomainId,
        /// Domain rejected from registration.
        incoming: DomainId,
    },
}

impl fmt::Display for RegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateDomainId { id } => {
                write!(formatter, "domain {id} is already registered")
            }
            Self::DuplicateLocation { existing, incoming } => write!(
                formatter,
                "domains {existing} and {incoming} resolve to the same location"
            ),
        }
    }
}

impl Error for RegistrationError {}
