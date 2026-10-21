use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::{RootKind, StorageIdentity};

use super::{PlatformDirectoryFacts, StorageProfile};

/// Explicit whole-root replacements keyed by purpose.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StorageLayoutOverrides {
    roots: BTreeMap<RootKind, PathBuf>,
}

impl StorageLayoutOverrides {
    /// Starts with no overrides.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces one exact root.
    #[must_use]
    pub fn with(mut self, kind: RootKind, path: impl Into<PathBuf>) -> Self {
        self.roots.insert(kind, path.into());
        self
    }

    pub(crate) fn get(&self, kind: RootKind) -> Option<&Path> {
        self.roots.get(&kind).map(PathBuf::as_path)
    }
}

/// Complete pure input to storage layout resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageLayoutRequest {
    pub(crate) identity: StorageIdentity,
    pub(crate) facts: PlatformDirectoryFacts,
    pub(crate) profile: StorageProfile,
    pub(crate) portable_root: Option<PathBuf>,
    pub(crate) overrides: StorageLayoutOverrides,
}

impl StorageLayoutRequest {
    /// Constructs a platform-native request.
    #[must_use]
    pub fn new(identity: StorageIdentity, facts: PlatformDirectoryFacts) -> Self {
        Self {
            identity,
            facts,
            profile: StorageProfile::default(),
            portable_root: None,
            overrides: StorageLayoutOverrides::default(),
        }
    }

    /// Selects one immutable built-in profile.
    #[must_use]
    pub fn with_profile(mut self, profile: StorageProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Supplies the one required root for `portable-v1`.
    #[must_use]
    pub fn with_portable_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.portable_root = Some(root.into());
        self
    }

    /// Supplies exact per-purpose deployment or test overrides.
    #[must_use]
    pub fn with_overrides(mut self, overrides: StorageLayoutOverrides) -> Self {
        self.overrides = overrides;
        self
    }
}
