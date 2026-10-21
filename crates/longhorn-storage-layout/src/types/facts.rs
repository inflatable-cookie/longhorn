use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Desktop platform whose directory policy is being resolved.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TargetPlatform {
    /// Apple macOS.
    MacOs,
    /// Microsoft Windows.
    Windows,
    /// Linux desktop.
    Linux,
}

impl TargetPlatform {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::MacOs => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
        }
    }
}

/// Host-supplied platform base directory.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PlatformDirectoryFact {
    /// User configuration base.
    Config,
    /// Durable application data base.
    Data,
    /// Durable per-user product-data base shared by cooperating processes.
    SharedData,
    /// Persistent machine-local state base.
    State,
    /// Rebuildable cache base.
    Cache,
    /// Application log base.
    Log,
    /// Session or temporary runtime base.
    Runtime,
}

impl PlatformDirectoryFact {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Data => "data",
            Self::SharedData => "shared-data",
            Self::State => "state",
            Self::Cache => "cache",
            Self::Log => "log",
            Self::Runtime => "runtime",
        }
    }
}

/// Injected platform base directories with no ambient lookup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformDirectoryFacts {
    platform: TargetPlatform,
    paths: BTreeMap<PlatformDirectoryFact, PathBuf>,
}

impl PlatformDirectoryFacts {
    /// Starts an empty fact set for one platform.
    #[must_use]
    pub fn new(platform: TargetPlatform) -> Self {
        Self {
            platform,
            paths: BTreeMap::new(),
        }
    }

    /// Adds or replaces one host-supplied base directory.
    #[must_use]
    pub fn with(mut self, fact: PlatformDirectoryFact, path: impl Into<PathBuf>) -> Self {
        self.paths.insert(fact, path.into());
        self
    }

    /// Constructs a complete ordinary fact set.
    #[must_use]
    pub fn complete(
        platform: TargetPlatform,
        config: impl Into<PathBuf>,
        data: impl Into<PathBuf>,
        state: impl Into<PathBuf>,
        cache: impl Into<PathBuf>,
        log: impl Into<PathBuf>,
        runtime: impl Into<PathBuf>,
    ) -> Self {
        Self::new(platform)
            .with(PlatformDirectoryFact::Config, config)
            .with(PlatformDirectoryFact::Data, data)
            .with(PlatformDirectoryFact::State, state)
            .with(PlatformDirectoryFact::Cache, cache)
            .with(PlatformDirectoryFact::Log, log)
            .with(PlatformDirectoryFact::Runtime, runtime)
    }

    /// Returns the target platform.
    #[must_use]
    pub const fn platform(&self) -> TargetPlatform {
        self.platform
    }

    /// Returns one supplied path without resolving a profile.
    #[must_use]
    pub fn get(&self, fact: PlatformDirectoryFact) -> Option<&Path> {
        self.paths.get(&fact).map(PathBuf::as_path)
    }
}
