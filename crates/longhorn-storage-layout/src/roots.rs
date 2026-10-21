use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

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

    /// Returns the optional administrator policy root.
    #[must_use]
    pub fn policy(&self) -> Option<&Path> {
        self.policy.as_deref()
    }

    /// Returns the workspace-keyed personal state root.
    #[must_use]
    pub fn workspace(&self) -> Option<&Path> {
        self.workspace.as_deref()
    }

    /// Returns the explicit project-shared root.
    #[must_use]
    pub fn project(&self) -> Option<&Path> {
        self.project.as_deref()
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
}

fn validate_root(kind: RootKind, path: PathBuf) -> Result<PathBuf, StorageRootError> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(StorageRootError { kind, path })
    }
}

/// Returns the stable identifier used by layout receipts and projections.
#[must_use]
pub const fn root_kind_id(kind: RootKind) -> &'static str {
    match kind {
        RootKind::Config => "config",
        RootKind::Data => "data",
        RootKind::State => "state",
        RootKind::Cache => "cache",
        RootKind::Runtime => "runtime",
        RootKind::Log => "log",
        RootKind::Backup => "backup",
        RootKind::Policy => "policy",
        RootKind::Workspace => "workspace",
        RootKind::Project => "project",
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
mod tests {
    use super::*;

    #[test]
    fn roots_must_be_absolute() {
        let result = StorageRoots::new(
            "config", "/data", "/state", "/cache", "/tmp", "/logs", "/backups",
        );

        assert_eq!(
            result,
            Err(StorageRootError {
                kind: RootKind::Config,
                path: PathBuf::from("config")
            })
        );
    }

    #[test]
    fn optional_roots_round_trip_through_getters() {
        let roots = StorageRoots::new(
            "/config", "/data", "/state", "/cache", "/tmp", "/logs", "/backups",
        )
        .unwrap()
        .with_policy("/policy")
        .unwrap()
        .with_workspace("/workspace")
        .unwrap()
        .with_project("/project")
        .unwrap();

        assert_eq!(roots.policy(), Some(Path::new("/policy")));
        assert_eq!(roots.workspace(), Some(Path::new("/workspace")));
        assert_eq!(roots.project(), Some(Path::new("/project")));
    }
}
