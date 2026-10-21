use std::path::{Path, PathBuf};

use longhorn_storage_layout::{AccessMode, RootKind, StorageRoots};

use crate::{DomainDescriptor, DomainFilePath, StorageClass};

/// Resolves a domain to a typed storage location.
///
/// `StorageRoots` lives in `longhorn-storage-layout`, which holds no domain
/// vocabulary. This extension trait carries the domain-class to root
/// authority mapping without making that crate depend on config types.
pub trait StorageRootsResolve {
    /// Resolves a domain to a typed storage location.
    #[must_use]
    fn resolve(&self, descriptor: &DomainDescriptor) -> DomainLocation;
}

impl StorageRootsResolve for StorageRoots {
    fn resolve(&self, descriptor: &DomainDescriptor) -> DomainLocation {
        let class = descriptor.storage_class();

        match class {
            StorageClass::Defaults => DomainLocation::DefaultsOnly,
            StorageClass::Secret => DomainLocation::SecureStoreRequired,
            StorageClass::Policy => self.resolve_optional(
                RootKind::Policy,
                self.policy(),
                descriptor,
                AccessMode::ReadOnly,
            ),
            StorageClass::WorkspaceLocal => self.resolve_optional(
                RootKind::Workspace,
                self.workspace(),
                descriptor,
                AccessMode::ReadWrite,
            ),
            StorageClass::ProjectShared => self.resolve_optional(
                RootKind::Project,
                self.project(),
                descriptor,
                AccessMode::ReadWrite,
            ),
            StorageClass::UserConfig => self.resolve_required(
                RootKind::Config,
                self.config(),
                descriptor,
                AccessMode::ReadWrite,
            ),
            StorageClass::MachineState => self.resolve_required(
                RootKind::State,
                self.state(),
                descriptor,
                AccessMode::ReadWrite,
            ),
            StorageClass::Cache => self.resolve_required(
                RootKind::Cache,
                self.cache(),
                descriptor,
                AccessMode::ReadWrite,
            ),
            StorageClass::Runtime => self.resolve_required(
                RootKind::Runtime,
                self.runtime(),
                descriptor,
                AccessMode::ReadWrite,
            ),
            StorageClass::Log => {
                self.resolve_required(RootKind::Log, self.log(), descriptor, AccessMode::ReadWrite)
            }
        }
    }
}

trait ResolveRootsPrivate {
    fn resolve_optional(
        &self,
        kind: RootKind,
        root: Option<&Path>,
        descriptor: &DomainDescriptor,
        access: AccessMode,
    ) -> DomainLocation;

    fn resolve_required(
        &self,
        kind: RootKind,
        root: &Path,
        descriptor: &DomainDescriptor,
        access: AccessMode,
    ) -> DomainLocation;
}

impl ResolveRootsPrivate for StorageRoots {
    fn resolve_optional(
        &self,
        kind: RootKind,
        root: Option<&Path>,
        descriptor: &DomainDescriptor,
        access: AccessMode,
    ) -> DomainLocation {
        root.map_or_else(
            || DomainLocation::RootRequired {
                root: kind,
                relative_path: descriptor
                    .file_path()
                    .cloned()
                    .expect("validated descriptor for a file-backed class has a file path"),
            },
            |root| resolved_file(kind, root, descriptor, access),
        )
    }

    fn resolve_required(
        &self,
        kind: RootKind,
        root: &Path,
        descriptor: &DomainDescriptor,
        access: AccessMode,
    ) -> DomainLocation {
        resolved_file(kind, root, descriptor, access)
    }
}

fn resolved_file(
    kind: RootKind,
    root: &Path,
    descriptor: &DomainDescriptor,
    access: AccessMode,
) -> DomainLocation {
    let relative_path = descriptor
        .file_path()
        .cloned()
        .expect("validated descriptor for a file-backed class has a file path");
    let full_path = root.join(relative_path.as_path());

    DomainLocation::File(ResolvedFile {
        root_kind: kind,
        root: root.to_path_buf(),
        relative_path,
        full_path,
        access,
    })
}

/// A domain's resolved storage authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainLocation {
    /// Capability-confined ordinary file.
    File(ResolvedFile),
    /// Compiled defaults with no file.
    DefaultsOnly,
    /// Secure-store adapter required.
    SecureStoreRequired,
    /// A required explicit root was not injected.
    RootRequired {
        /// Missing root.
        root: RootKind,
        /// Domain path relative to that root.
        relative_path: DomainFilePath,
    },
}

/// Capability-confined ordinary file location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedFile {
    root_kind: RootKind,
    root: PathBuf,
    relative_path: DomainFilePath,
    full_path: PathBuf,
    access: AccessMode,
}

impl ResolvedFile {
    /// Returns the root class.
    #[must_use]
    pub const fn root_kind(&self) -> RootKind {
        self.root_kind
    }

    /// Returns the ambient root used to open a capability directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the validated path below the root.
    #[must_use]
    pub fn relative_path(&self) -> &DomainFilePath {
        &self.relative_path
    }

    /// Returns the display and diagnostic path.
    #[must_use]
    pub fn full_path(&self) -> &Path {
        &self.full_path
    }

    /// Returns the class access policy.
    #[must_use]
    pub const fn access(&self) -> AccessMode {
        self.access
    }
}

#[cfg(test)]
mod tests;
