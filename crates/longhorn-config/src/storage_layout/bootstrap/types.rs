use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

use crate::{Sha256Digest, StorageProfile};

/// Fixed native files used to select and recover a storage profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageBootstrapPaths {
    directory: PathBuf,
    locator: PathBuf,
    journal: PathBuf,
    transitions: PathBuf,
}

impl StorageBootstrapPaths {
    pub(crate) fn new(directory: PathBuf) -> Self {
        Self {
            locator: directory.join("storage-profile.json"),
            journal: directory.join("storage-transition.json"),
            transitions: directory.join("storage-transitions"),
            directory,
        }
    }

    /// Returns the fixed native bootstrap directory.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Returns the selected-profile locator.
    #[must_use]
    pub fn locator(&self) -> &Path {
        &self.locator
    }

    /// Returns the durable transition journal.
    #[must_use]
    pub fn journal(&self) -> &Path {
        &self.journal
    }

    /// Returns the private transition staging parent.
    #[must_use]
    pub fn transitions(&self) -> &Path {
        &self.transitions
    }
}

/// Valid profile selection independent of its resolved platform roots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageProfileSelection {
    profile: StorageProfile,
    explicit_root: Option<PathBuf>,
}

impl StorageProfileSelection {
    /// Selects the compiled native default.
    #[must_use]
    pub const fn platform_native() -> Self {
        Self {
            profile: StorageProfile::PlatformNativeV1,
            explicit_root: None,
        }
    }

    /// Selects the native unified root profile.
    #[must_use]
    pub const fn unified() -> Self {
        Self {
            profile: StorageProfile::UnifiedAppRootV1,
            explicit_root: None,
        }
    }

    /// Selects the shared durable product root profile.
    #[must_use]
    pub const fn shared_product() -> Self {
        Self {
            profile: StorageProfile::SharedProductRootV1,
            explicit_root: None,
        }
    }

    /// Selects an explicit absolute portable root.
    pub fn portable(root: impl Into<PathBuf>) -> Result<Self, StorageProfileSelectionError> {
        let root = root.into();
        if !root.is_absolute() {
            return Err(StorageProfileSelectionError::RootNotAbsolute { root });
        }
        Ok(Self {
            profile: StorageProfile::PortableV1,
            explicit_root: Some(root),
        })
    }

    pub(crate) fn from_locator(
        profile_id: &str,
        explicit_root: Option<PathBuf>,
    ) -> Result<Self, StorageProfileSelectionError> {
        let profile = StorageProfile::from_id(profile_id).map_err(|_| {
            StorageProfileSelectionError::UnknownProfile {
                id: profile_id.to_owned(),
            }
        })?;
        match (profile, explicit_root) {
            (StorageProfile::PortableV1, Some(root)) => Self::portable(root),
            (StorageProfile::PortableV1, None) => {
                Err(StorageProfileSelectionError::PortableRootRequired)
            }
            (_, Some(root)) => Err(StorageProfileSelectionError::RootForbidden { profile, root }),
            (StorageProfile::PlatformNativeV1, None) => Ok(Self::platform_native()),
            (StorageProfile::UnifiedAppRootV1, None) => Ok(Self::unified()),
            (StorageProfile::SharedProductRootV1, None) => Ok(Self::shared_product()),
        }
    }

    /// Returns the immutable profile contract.
    #[must_use]
    pub const fn profile(&self) -> StorageProfile {
        self.profile
    }

    /// Returns the profile's explicit root, when required.
    #[must_use]
    pub fn explicit_root(&self) -> Option<&Path> {
        self.explicit_root.as_deref()
    }
}

/// Invalid explicit profile selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageProfileSelectionError {
    /// Locator named an unsupported profile.
    UnknownProfile {
        /// Unsupported id.
        id: String,
    },
    /// Portable profile omitted its root.
    PortableRootRequired,
    /// Supplied root was relative.
    RootNotAbsolute {
        /// Rejected root.
        root: PathBuf,
    },
    /// A non-portable profile carried a root.
    RootForbidden {
        /// Selected profile.
        profile: StorageProfile,
        /// Rejected root.
        root: PathBuf,
    },
}

impl fmt::Display for StorageProfileSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProfile { id } => write!(formatter, "unknown storage profile {id:?}"),
            Self::PortableRootRequired => {
                formatter.write_str("portable-v1 locator requires an explicit root")
            }
            Self::RootNotAbsolute { root } => {
                write!(
                    formatter,
                    "profile root must be absolute: {}",
                    root.display()
                )
            }
            Self::RootForbidden { profile, .. } => {
                write!(
                    formatter,
                    "{} locator forbids an explicit root",
                    profile.id()
                )
            }
        }
    }
}

impl Error for StorageProfileSelectionError {}

/// Authority that selected the active profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageBootstrapOrigin {
    /// No locator exists; compiled native default applies.
    MissingDefault,
    /// A valid fixed native locator selected the profile.
    Locator,
    /// Explicit host policy bypassed locator I/O.
    HostBypass,
}

/// Valid selected bootstrap state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageBootstrapSelection {
    selection: StorageProfileSelection,
    origin: StorageBootstrapOrigin,
    transition_id: Option<String>,
    last_committed_layout_digest: Option<Sha256Digest>,
    paths: Option<StorageBootstrapPaths>,
    locator: Option<StorageProfileLocator>,
}

impl StorageBootstrapSelection {
    pub(crate) fn new(
        selection: StorageProfileSelection,
        origin: StorageBootstrapOrigin,
        transition_id: Option<String>,
        last_committed_layout_digest: Option<Sha256Digest>,
        paths: Option<StorageBootstrapPaths>,
    ) -> Self {
        Self {
            selection,
            origin,
            transition_id,
            last_committed_layout_digest,
            paths,
            locator: None,
        }
    }

    pub(crate) fn with_locator(mut self, locator: StorageProfileLocator) -> Self {
        self.locator = Some(locator);
        self
    }

    /// Returns the selected profile and explicit root.
    #[must_use]
    pub fn selection(&self) -> &StorageProfileSelection {
        &self.selection
    }

    /// Returns the selection authority.
    #[must_use]
    pub const fn origin(&self) -> StorageBootstrapOrigin {
        self.origin
    }

    /// Returns the last committed transition id.
    #[must_use]
    pub fn transition_id(&self) -> Option<&str> {
        self.transition_id.as_deref()
    }

    /// Returns the last committed layout evidence.
    #[must_use]
    pub fn last_committed_layout_digest(&self) -> Option<&Sha256Digest> {
        self.last_committed_layout_digest.as_ref()
    }

    /// Returns fixed bootstrap paths, absent for a host bypass.
    #[must_use]
    pub fn paths(&self) -> Option<&StorageBootstrapPaths> {
        self.paths.as_ref()
    }

    /// Returns the validated locator document when locator-backed.
    #[must_use]
    pub fn locator(&self) -> Option<&StorageProfileLocator> {
        self.locator.as_ref()
    }
}

/// Valid public projection of a persisted locator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageProfileLocator {
    canonical_application_id: String,
    selection: StorageProfileSelection,
    transition_id: Option<String>,
    last_committed_layout_digest: Option<Sha256Digest>,
}

impl StorageProfileLocator {
    pub(crate) fn new(
        canonical_application_id: String,
        selection: StorageProfileSelection,
        transition_id: Option<String>,
        last_committed_layout_digest: Option<Sha256Digest>,
    ) -> Self {
        Self {
            canonical_application_id,
            selection,
            transition_id,
            last_committed_layout_digest,
        }
    }

    /// Returns canonical machine identity.
    #[must_use]
    pub fn canonical_application_id(&self) -> &str {
        &self.canonical_application_id
    }

    /// Returns the selected profile.
    #[must_use]
    pub fn selection(&self) -> &StorageProfileSelection {
        &self.selection
    }

    /// Returns the committed transition id.
    #[must_use]
    pub fn transition_id(&self) -> Option<&str> {
        self.transition_id.as_deref()
    }

    /// Returns the committed target-layout evidence.
    #[must_use]
    pub fn last_committed_layout_digest(&self) -> Option<&Sha256Digest> {
        self.last_committed_layout_digest.as_ref()
    }
}

/// Locator bootstrap state. Recovery never carries an implicit fallback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageBootstrapState {
    /// Profile selection is authoritative.
    Selected(StorageBootstrapSelection),
    /// Locator exists or was expected but cannot select authority safely.
    Recovery(StorageBootstrapRecovery),
}

/// Stable locator recovery category.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageBootstrapRecoveryKind {
    /// Locator could not be read.
    Unreadable,
    /// Locator was not strict valid JSON.
    InvalidDocument,
    /// Locator schema is unsupported.
    UnsupportedSchema {
        /// Observed schema.
        observed: u32,
    },
    /// Locator belongs to another canonical app.
    CanonicalApplicationMismatch,
    /// Locator names an unsupported profile.
    UnknownProfile,
    /// Profile root is missing, forbidden, or relative.
    InvalidExplicitRoot,
    /// Last committed layout digest is malformed.
    InvalidLayoutDigest,
}

/// Typed fail-closed bootstrap recovery state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageBootstrapRecovery {
    paths: StorageBootstrapPaths,
    kind: StorageBootstrapRecoveryKind,
    detail: String,
}

impl StorageBootstrapRecovery {
    pub(crate) fn new(
        paths: StorageBootstrapPaths,
        kind: StorageBootstrapRecoveryKind,
        detail: String,
    ) -> Self {
        Self {
            paths,
            kind,
            detail,
        }
    }

    /// Returns fixed paths without selecting a layout.
    #[must_use]
    pub fn paths(&self) -> &StorageBootstrapPaths {
        &self.paths
    }

    /// Returns the stable recovery category.
    #[must_use]
    pub fn kind(&self) -> &StorageBootstrapRecoveryKind {
        &self.kind
    }

    /// Returns bounded diagnostic detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// Fixed bootstrap path cannot be derived from platform facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageBootstrapPathError {
    /// Host omitted its native configuration base.
    MissingConfigFact,
    /// Host supplied a relative native configuration base.
    InvalidConfigFact {
        /// Rejected path.
        path: PathBuf,
    },
}

impl fmt::Display for StorageBootstrapPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingConfigFact => {
                formatter.write_str("fixed storage bootstrap requires the native config fact")
            }
            Self::InvalidConfigFact { path } => {
                write!(
                    formatter,
                    "native config fact must be absolute: {}",
                    path.display()
                )
            }
        }
    }
}

impl Error for StorageBootstrapPathError {}
