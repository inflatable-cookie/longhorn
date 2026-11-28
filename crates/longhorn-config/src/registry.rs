use std::{collections::BTreeMap, error::Error, fmt, path::PathBuf};

use longhorn_core::DomainId;

use crate::StorageRootsResolve;
use crate::{DomainDescriptor, DomainLocation};
use longhorn_storage_layout::{RootKind, StorageRoots};

#[derive(Debug)]
enum LocationKey {
    File(PathBuf),
    Deferred(RootKind, PathBuf),
}

/// Descriptor registry for one configuration store.
#[derive(Debug, Default)]
pub(crate) struct DomainRegistry {
    descriptors: BTreeMap<DomainId, DomainDescriptor>,
    locations: Vec<(LocationKey, DomainId)>,
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
        if let Some(key) = location_key.as_ref()
            && let Some((_, existing)) = self
                .locations
                .iter()
                .find(|(other, _)| key.conflicts(other))
        {
            return Err(RegistrationError::DuplicateLocation {
                existing: existing.clone(),
                incoming: descriptor.id().clone(),
            });
        }

        if let Some(key) = location_key {
            self.locations.push((key, descriptor.id().clone()));
        }
        self.descriptors
            .insert(descriptor.id().clone(), descriptor.clone());

        Ok(())
    }

    pub(crate) fn descriptor(&self, id: &DomainId) -> Option<&DomainDescriptor> {
        self.descriptors.get(id)
    }

    pub(crate) fn descriptors(&self) -> impl Iterator<Item = &DomainDescriptor> {
        self.descriptors.values()
    }
}

impl LocationKey {
    fn conflicts(&self, other: &Self) -> bool {
        let (left, right) = match (self, other) {
            (Self::File(left), Self::File(right)) => (left, right),
            (Self::Deferred(left_root, left), Self::Deferred(right_root, right))
                if left_root == right_root =>
            {
                (left, right)
            }
            _ => return false,
        };
        left.starts_with(right) || right.starts_with(left)
    }
}

fn folded_path(path: &std::path::Path) -> PathBuf {
    let mut path = path.as_os_str().to_owned();
    path.make_ascii_lowercase();
    path.into()
}

fn location_key(location: DomainLocation) -> Option<LocationKey> {
    match location {
        DomainLocation::File(file) => Some(LocationKey::File(folded_path(file.full_path()))),
        DomainLocation::RootRequired {
            root,
            relative_path,
        } => Some(LocationKey::Deferred(
            root,
            folded_path(relative_path.as_path()),
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
    /// Two domains have conflicting storage locations.
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
                "domains {existing} and {incoming} resolve to conflicting locations"
            ),
        }
    }
}

impl Error for RegistrationError {}
