use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

use crate::{DomainDescriptor, DomainFilePath, StorageClass};

/// Injected root required by a storage class.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RootKind {
    /// Platform application configuration root.
    Config,
    /// Platform application data root.
    Data,
    /// Platform application state root.
    State,
    /// Platform application cache root.
    Cache,
    /// Platform temporary root.
    Runtime,
    /// Platform application log root.
    Log,
    /// Operational backup root.
    Backup,
    /// Optional administrator policy root.
    Policy,
    /// Workspace-keyed personal state root.
    Workspace,
    /// Explicit project-shared root.
    Project,
}

/// Filesystem access policy for a resolved location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessMode {
    /// Domain files may only be read.
    ReadOnly,
    /// Coordinated domain mutation may write this location.
    ReadWrite,
}

/// Validated roots injected by a host adapter or test.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageRoots {
    config: PathBuf,
    data: PathBuf,
    state: PathBuf,
    cache: PathBuf,
    runtime: PathBuf,
    log: PathBuf,
    backup: PathBuf,
    policy: Option<PathBuf>,
    workspace: Option<PathBuf>,
    project: Option<PathBuf>,
}

impl StorageRoots {
    /// Constructs the required platform root set.
    pub fn new(
        config: impl Into<PathBuf>,
        data: impl Into<PathBuf>,
        state: impl Into<PathBuf>,
        cache: impl Into<PathBuf>,
        runtime: impl Into<PathBuf>,
        log: impl Into<PathBuf>,
        backup: impl Into<PathBuf>,
    ) -> Result<Self, StorageRootError> {
        let config = validate_root(RootKind::Config, config.into())?;
        let data = validate_root(RootKind::Data, data.into())?;
        let state = validate_root(RootKind::State, state.into())?;
        let cache = validate_root(RootKind::Cache, cache.into())?;
        let runtime = validate_root(RootKind::Runtime, runtime.into())?;
        let log = validate_root(RootKind::Log, log.into())?;
        let backup = validate_root(RootKind::Backup, backup.into())?;

        Ok(Self {
            config,
            data,
            state,
            cache,
            runtime,
            log,
            backup,
            policy: None,
            workspace: None,
            project: None,
        })
    }

    /// Returns the application configuration root.
    #[must_use]
    pub fn config(&self) -> &Path {
        &self.config
    }

    /// Returns the durable application data root.
    #[must_use]
    pub fn data(&self) -> &Path {
        &self.data
    }

    /// Returns the machine-local state root.
    #[must_use]
    pub fn state(&self) -> &Path {
        &self.state
    }

    /// Returns the rebuildable cache root.
    #[must_use]
    pub fn cache(&self) -> &Path {
        &self.cache
    }

    /// Returns the runtime root.
    #[must_use]
    pub fn runtime(&self) -> &Path {
        &self.runtime
    }

    /// Returns the log root.
    #[must_use]
    pub fn log(&self) -> &Path {
        &self.log
    }

    /// Returns the operational backup root.
    #[must_use]
    pub fn backup(&self) -> &Path {
        &self.backup
    }

    /// Adds an optional administrator policy root.
    pub fn with_policy(mut self, root: impl Into<PathBuf>) -> Result<Self, StorageRootError> {
        self.policy = Some(validate_root(RootKind::Policy, root.into())?);
        Ok(self)
    }

    /// Adds a workspace-keyed personal state root.
    pub fn with_workspace(mut self, root: impl Into<PathBuf>) -> Result<Self, StorageRootError> {
        self.workspace = Some(validate_root(RootKind::Workspace, root.into())?);
        Ok(self)
    }

    /// Adds an explicit project-shared root.
    pub fn with_project(mut self, root: impl Into<PathBuf>) -> Result<Self, StorageRootError> {
        self.project = Some(validate_root(RootKind::Project, root.into())?);
        Ok(self)
    }

    /// Resolves a domain to a typed storage location.
    #[must_use]
    pub fn resolve(&self, descriptor: &DomainDescriptor) -> DomainLocation {
        let class = descriptor.storage_class();

        match class {
            StorageClass::Defaults => DomainLocation::DefaultsOnly,
            StorageClass::Secret => DomainLocation::SecureStoreRequired,
            StorageClass::Policy => self.resolve_optional(
                RootKind::Policy,
                self.policy.as_deref(),
                descriptor,
                AccessMode::ReadOnly,
            ),
            StorageClass::WorkspaceLocal => self.resolve_optional(
                RootKind::Workspace,
                self.workspace.as_deref(),
                descriptor,
                AccessMode::ReadWrite,
            ),
            StorageClass::ProjectShared => self.resolve_optional(
                RootKind::Project,
                self.project.as_deref(),
                descriptor,
                AccessMode::ReadWrite,
            ),
            StorageClass::UserConfig => self.resolve_required(
                RootKind::Config,
                &self.config,
                descriptor,
                AccessMode::ReadWrite,
            ),
            StorageClass::MachineState => self.resolve_required(
                RootKind::State,
                &self.state,
                descriptor,
                AccessMode::ReadWrite,
            ),
            StorageClass::Cache => self.resolve_required(
                RootKind::Cache,
                &self.cache,
                descriptor,
                AccessMode::ReadWrite,
            ),
            StorageClass::Runtime => self.resolve_required(
                RootKind::Runtime,
                &self.runtime,
                descriptor,
                AccessMode::ReadWrite,
            ),
            StorageClass::Log => {
                self.resolve_required(RootKind::Log, &self.log, descriptor, AccessMode::ReadWrite)
            }
        }
    }

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

fn validate_root(kind: RootKind, path: PathBuf) -> Result<PathBuf, StorageRootError> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(StorageRootError { kind, path })
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

/// An injected storage root was not absolute.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageRootError {
    /// Invalid root class.
    pub kind: RootKind,
    /// Rejected path.
    pub path: PathBuf,
}

impl fmt::Display for StorageRootError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} root must be absolute: {}",
            self.kind,
            self.path.display()
        )
    }
}

impl Error for StorageRootError {}

#[cfg(test)]
mod tests;
