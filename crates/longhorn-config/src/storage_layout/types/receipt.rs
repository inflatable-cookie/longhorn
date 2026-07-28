use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::{RootKind, Sha256Digest, StorageIdentity, StorageRoots};

use super::{
    StorageLayoutWarning, StorageLeafProvenance, StorageProfile, StorageRootProvenance,
    TargetPlatform,
};

/// One resolved root and its authority provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedStorageRoot {
    kind: RootKind,
    path: PathBuf,
    provenance: StorageRootProvenance,
}

impl ResolvedStorageRoot {
    pub(crate) fn new(kind: RootKind, path: PathBuf, provenance: StorageRootProvenance) -> Self {
        Self {
            kind,
            path,
            provenance,
        }
    }

    /// Returns the root purpose.
    #[must_use]
    pub const fn kind(&self) -> RootKind {
        self.kind
    }

    /// Returns the resolved absolute path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns how this root was selected.
    #[must_use]
    pub const fn provenance(&self) -> StorageRootProvenance {
        self.provenance
    }
}

/// Owned projection for diagnostics and settings presentation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageLayoutDiagnostic {
    profile_id: String,
    platform: TargetPlatform,
    canonical_application_id: String,
    effective_leaf: String,
    leaf_provenance: StorageLeafProvenance,
    roots: Vec<ResolvedStorageRoot>,
    warnings: Vec<StorageLayoutWarning>,
    digest: Sha256Digest,
}

impl StorageLayoutDiagnostic {
    /// Returns the selected profile id.
    #[must_use]
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    /// Returns the target platform.
    #[must_use]
    pub const fn platform(&self) -> TargetPlatform {
        self.platform
    }

    /// Returns canonical machine identity.
    #[must_use]
    pub fn canonical_application_id(&self) -> &str {
        &self.canonical_application_id
    }

    /// Returns the effective path leaf.
    #[must_use]
    pub fn effective_leaf(&self) -> &str {
        &self.effective_leaf
    }

    /// Returns the leaf source.
    #[must_use]
    pub const fn leaf_provenance(&self) -> StorageLeafProvenance {
        self.leaf_provenance
    }

    /// Returns every resolved root in stable root-kind order.
    #[must_use]
    pub fn roots(&self) -> &[ResolvedStorageRoot] {
        &self.roots
    }

    /// Returns visible profile consequences.
    #[must_use]
    pub fn warnings(&self) -> &[StorageLayoutWarning] {
        &self.warnings
    }

    /// Returns the deterministic layout digest.
    #[must_use]
    pub fn digest(&self) -> &Sha256Digest {
        &self.digest
    }
}

/// Fully resolved layout and roots ready for `ConfigStore`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedStorageLayout {
    pub(crate) profile: StorageProfile,
    pub(crate) platform: TargetPlatform,
    pub(crate) identity: StorageIdentity,
    pub(crate) effective_leaf: String,
    pub(crate) leaf_provenance: StorageLeafProvenance,
    pub(crate) roots: BTreeMap<RootKind, ResolvedStorageRoot>,
    pub(crate) storage_roots: StorageRoots,
    pub(crate) warnings: Vec<StorageLayoutWarning>,
    pub(crate) digest: Sha256Digest,
}

impl ResolvedStorageLayout {
    /// Returns the selected profile.
    #[must_use]
    pub const fn profile(&self) -> StorageProfile {
        self.profile
    }

    /// Returns the target platform.
    #[must_use]
    pub const fn platform(&self) -> TargetPlatform {
        self.platform
    }

    /// Returns immutable app identity.
    #[must_use]
    pub fn identity(&self) -> &StorageIdentity {
        &self.identity
    }

    /// Returns the effective path leaf.
    #[must_use]
    pub fn effective_leaf(&self) -> &str {
        &self.effective_leaf
    }

    /// Returns the effective leaf source.
    #[must_use]
    pub const fn leaf_provenance(&self) -> StorageLeafProvenance {
        self.leaf_provenance
    }

    /// Returns one resolved root, including optional authorities.
    #[must_use]
    pub fn root(&self, kind: RootKind) -> Option<&ResolvedStorageRoot> {
        self.roots.get(&kind)
    }

    /// Returns roots ready for the configuration store.
    #[must_use]
    pub fn storage_roots(&self) -> &StorageRoots {
        &self.storage_roots
    }

    /// Returns visible profile consequences.
    #[must_use]
    pub fn warnings(&self) -> &[StorageLayoutWarning] {
        &self.warnings
    }

    /// Returns the deterministic layout digest.
    #[must_use]
    pub fn digest(&self) -> &Sha256Digest {
        &self.digest
    }

    /// Returns the conventional durable database directory.
    #[must_use]
    pub fn durable_database_dir(&self) -> PathBuf {
        self.storage_roots.data().join("databases")
    }

    /// Returns the conventional machine-state database directory.
    #[must_use]
    pub fn state_database_dir(&self) -> PathBuf {
        self.storage_roots.state().join("databases")
    }

    /// Returns the conventional rebuildable database directory.
    #[must_use]
    pub fn cache_database_dir(&self) -> PathBuf {
        self.storage_roots.cache().join("databases")
    }

    /// Projects the complete receipt into an owned diagnostic shape.
    #[must_use]
    pub fn diagnostic(&self) -> StorageLayoutDiagnostic {
        StorageLayoutDiagnostic {
            profile_id: self.profile.id().to_owned(),
            platform: self.platform,
            canonical_application_id: self.identity.canonical_application_id().to_owned(),
            effective_leaf: self.effective_leaf.clone(),
            leaf_provenance: self.leaf_provenance,
            roots: self.roots.values().cloned().collect(),
            warnings: self.warnings.clone(),
            digest: self.digest.clone(),
        }
    }
}
