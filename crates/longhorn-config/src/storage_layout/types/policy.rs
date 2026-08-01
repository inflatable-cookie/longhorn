use std::{error::Error, fmt};

use crate::RootKind;

use super::PlatformDirectoryFact;

/// Versioned built-in layout policy.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StorageProfile {
    /// Platform lifecycle roots with canonical or explicit app leaf.
    #[default]
    PlatformNativeV1,
    /// One native durable app root with typed children.
    UnifiedAppRootV1,
    /// One shared durable product root with typed children.
    SharedProductRootV1,
    /// One caller-supplied absolute root with typed children.
    PortableV1,
}

impl StorageProfile {
    /// Returns the immutable profile id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::PlatformNativeV1 => "platform-native-v1",
            Self::UnifiedAppRootV1 => "unified-app-root-v1",
            Self::SharedProductRootV1 => "shared-product-root-v1",
            Self::PortableV1 => "portable-v1",
        }
    }

    /// Parses a built-in profile id.
    pub fn from_id(value: &str) -> Result<Self, StorageProfileIdError> {
        match value {
            "platform-native-v1" => Ok(Self::PlatformNativeV1),
            "unified-app-root-v1" => Ok(Self::UnifiedAppRootV1),
            "shared-product-root-v1" => Ok(Self::SharedProductRootV1),
            "portable-v1" => Ok(Self::PortableV1),
            _ => Err(StorageProfileIdError(value.to_owned())),
        }
    }
}

/// Unknown storage profile id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageProfileIdError(String);

impl StorageProfileIdError {
    /// Returns the unknown id.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StorageProfileIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown storage profile {:?}", self.0)
    }
}

impl Error for StorageProfileIdError {}

/// Origin of the effective app-specific directory leaf.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageLeafProvenance {
    /// Canonical application id supplied the leaf.
    CanonicalApplicationId,
    /// Explicit stable storage name supplied the leaf.
    StableStorageName,
}

impl StorageLeafProvenance {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::CanonicalApplicationId => "canonical-application-id",
            Self::StableStorageName => "stable-storage-name",
        }
    }
}

/// Origin of one resolved root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageRootProvenance {
    /// Platform-native profile and base fact.
    PlatformProfile(PlatformDirectoryFact),
    /// Unified profile below the data base.
    UnifiedProfile(PlatformDirectoryFact),
    /// Shared-product profile below the shared durable data base.
    SharedProductProfile(PlatformDirectoryFact),
    /// Portable profile below one explicit root.
    PortableProfile,
    /// Root derived from another resolved root.
    DerivedFrom(RootKind),
    /// Exact caller-supplied root override.
    ExplicitOverride,
}

impl StorageRootProvenance {
    pub(crate) fn id(self) -> String {
        match self {
            Self::PlatformProfile(fact) => format!("platform:{}", fact.id()),
            Self::UnifiedProfile(fact) => format!("unified:{}", fact.id()),
            Self::SharedProductProfile(fact) => format!("shared-product:{}", fact.id()),
            Self::PortableProfile => "portable".to_owned(),
            Self::DerivedFrom(root) => format!("derived:{}", root_kind_id(root)),
            Self::ExplicitOverride => "override".to_owned(),
        }
    }
}

pub(crate) const fn root_kind_id(kind: RootKind) -> &'static str {
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

/// Consequence of selecting a non-native lifecycle layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageLayoutWarning {
    /// Unified cache is outside the platform cache classification.
    UnifiedCacheLifecycle,
    /// Unified runtime storage is durable rather than session-scoped.
    UnifiedRuntimeLifecycle,
    /// Unified backups share the general durable root.
    UnifiedBackupClassification,
    /// Shared-product cache is outside the platform cache classification.
    SharedProductCacheLifecycle,
    /// Shared-product logs are outside the platform log classification.
    SharedProductLogLifecycle,
    /// Shared-product runtime storage is durable rather than session-scoped.
    SharedProductRuntimeLifecycle,
    /// Shared-product backups share the general durable root.
    SharedProductBackupClassification,
    /// Portable layout does not inherit native lifecycle handling.
    PortableLifecycle,
}

impl StorageLayoutWarning {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::UnifiedCacheLifecycle => "unified-cache-lifecycle",
            Self::UnifiedRuntimeLifecycle => "unified-runtime-lifecycle",
            Self::UnifiedBackupClassification => "unified-backup-classification",
            Self::SharedProductCacheLifecycle => "shared-product-cache-lifecycle",
            Self::SharedProductLogLifecycle => "shared-product-log-lifecycle",
            Self::SharedProductRuntimeLifecycle => "shared-product-runtime-lifecycle",
            Self::SharedProductBackupClassification => "shared-product-backup-classification",
            Self::PortableLifecycle => "portable-lifecycle",
        }
    }
}
