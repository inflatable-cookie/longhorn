use std::{error::Error, fmt, path::Path};

use longhorn_core::{DomainId, SchemaVersion};
use serde_json::Value;

/// Persistence authority and default root for a configuration domain.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StorageClass {
    /// Compiled product defaults with no writable file.
    Defaults,
    /// Optional administrator or deployment policy.
    Policy,
    /// Portable user intent and preferences.
    UserConfig,
    /// Machine-specific state such as displays and devices.
    MachineState,
    /// Personal state tied to an injected workspace root.
    WorkspaceLocal,
    /// State tied to an explicitly injected project root.
    ProjectShared,
    /// Credentials delegated to a secure-store adapter.
    Secret,
    /// Re-creatable performance data.
    Cache,
    /// Ephemeral runtime data.
    Runtime,
    /// Application logs.
    Log,
}

impl StorageClass {
    const fn requires_file_path(self) -> bool {
        !matches!(self, Self::Defaults | Self::Secret)
    }
}

/// Portable relative path for one JSON domain file.
///
/// Paths use `/` separators and ASCII alphanumeric, `.`, `_`, or `-`
/// characters in each segment. They must end in `.json`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DomainFilePath(String);

impl DomainFilePath {
    /// Validates and constructs a domain file path.
    pub fn new(value: impl Into<String>) -> Result<Self, DomainFilePathError> {
        let value = value.into();

        if value.is_empty() {
            return Err(DomainFilePathError::Empty);
        }
        if value.starts_with('/') || value.starts_with('\\') {
            return Err(DomainFilePathError::Absolute);
        }
        if value.contains('\\') {
            return Err(DomainFilePathError::InvalidSeparator);
        }

        for (index, segment) in value.split('/').enumerate() {
            if segment.is_empty() || matches!(segment, "." | "..") {
                return Err(DomainFilePathError::InvalidSegment { index });
            }
            if !segment.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
            }) {
                return Err(DomainFilePathError::InvalidCharacter { index });
            }
        }

        if !value.ends_with(".json") {
            return Err(DomainFilePathError::NotJson);
        }

        Ok(Self(value))
    }

    /// Returns the portable serialized path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the path for capability-scoped filesystem access.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl fmt::Display for DomainFilePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Domain file path validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainFilePathError {
    /// The path was empty.
    Empty,
    /// The path was absolute.
    Absolute,
    /// The path used a non-portable separator.
    InvalidSeparator,
    /// A segment was empty, `.` or `..`.
    InvalidSegment {
        /// Zero-based segment index.
        index: usize,
    },
    /// A segment contained a non-portable character.
    InvalidCharacter {
        /// Zero-based segment index.
        index: usize,
    },
    /// The path did not end in `.json`.
    NotJson,
}

impl fmt::Display for DomainFilePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("domain file path cannot be empty"),
            Self::Absolute => formatter.write_str("domain file path must be relative"),
            Self::InvalidSeparator => {
                formatter.write_str("domain file path must use portable '/' separators")
            }
            Self::InvalidSegment { index } => {
                write!(formatter, "domain file path segment {index} is invalid")
            }
            Self::InvalidCharacter { index } => {
                write!(
                    formatter,
                    "domain file path segment {index} contains an invalid character"
                )
            }
            Self::NotJson => formatter.write_str("domain file path must end in .json"),
        }
    }
}

impl Error for DomainFilePathError {}

/// Stable registration metadata for one domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainDescriptor {
    id: DomainId,
    schema_version: SchemaVersion,
    storage_class: StorageClass,
    file_path: Option<DomainFilePath>,
}

impl DomainDescriptor {
    /// Constructs a descriptor and enforces its storage-class path policy.
    pub fn new(
        id: DomainId,
        schema_version: SchemaVersion,
        storage_class: StorageClass,
        file_path: Option<DomainFilePath>,
    ) -> Result<Self, DomainDescriptorError> {
        match (storage_class.requires_file_path(), file_path.is_some()) {
            (true, false) => {
                return Err(DomainDescriptorError::FilePathRequired { storage_class });
            }
            (false, true) => {
                return Err(DomainDescriptorError::FilePathForbidden { storage_class });
            }
            _ => {}
        }

        Ok(Self {
            id,
            schema_version,
            storage_class,
            file_path,
        })
    }

    /// Returns the stable domain id.
    #[must_use]
    pub fn id(&self) -> &DomainId {
        &self.id
    }

    /// Returns the current schema version.
    #[must_use]
    pub const fn schema_version(&self) -> SchemaVersion {
        self.schema_version
    }

    /// Returns the domain storage class.
    #[must_use]
    pub const fn storage_class(&self) -> StorageClass {
        self.storage_class
    }

    /// Returns the registered relative file path, when file-backed.
    #[must_use]
    pub fn file_path(&self) -> Option<&DomainFilePath> {
        self.file_path.as_ref()
    }
}

/// Invalid descriptor shape for its storage class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainDescriptorError {
    /// A file-backed class did not declare a path.
    FilePathRequired {
        /// Class requiring the path.
        storage_class: StorageClass,
    },
    /// A non-file class declared an ordinary path.
    FilePathForbidden {
        /// Class forbidding the path.
        storage_class: StorageClass,
    },
}

impl fmt::Display for DomainDescriptorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FilePathRequired { storage_class } => {
                write!(formatter, "{storage_class:?} requires a domain file path")
            }
            Self::FilePathForbidden { storage_class } => {
                write!(
                    formatter,
                    "{storage_class:?} cannot use an ordinary file path"
                )
            }
        }
    }
}

impl Error for DomainDescriptorError {}

/// Stable consumer-supplied validation or migration failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainIssue {
    /// Stable machine-readable issue code.
    pub code: String,
    /// Human-readable diagnostic.
    pub message: String,
}

impl DomainIssue {
    /// Constructs a domain issue.
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// One ordered schema migration result.
#[derive(Clone, Debug, PartialEq)]
pub struct MigrationStep {
    /// Target schema version.
    pub schema_version: SchemaVersion,
    /// Raw target document value.
    pub value: Value,
}

/// Consumer-owned schema and codec behavior for one typed domain.
pub trait ConfigDomain {
    /// Current decoded value.
    type Value;

    /// Returns stable domain registration metadata.
    fn descriptor(&self) -> &DomainDescriptor;

    /// Returns the compiled product default.
    fn default_value(&self) -> Self::Value;

    /// Decodes a current-version raw JSON value.
    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue>;

    /// Encodes a current value as raw JSON.
    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue>;

    /// Validates a decoded current value.
    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue>;

    /// Validates raw data at a known schema version.
    fn validate_raw(&self, schema_version: SchemaVersion, value: &Value)
    -> Result<(), DomainIssue>;

    /// Migrates one version or returns `None` when the step is missing.
    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_confined_json_path() {
        let path = DomainFilePath::new("workspace/window-state.json").unwrap();

        assert_eq!(path.as_str(), "workspace/window-state.json");
    }

    #[test]
    fn rejects_traversal_and_non_portable_paths() {
        assert_eq!(
            DomainFilePath::new("../settings.json"),
            Err(DomainFilePathError::InvalidSegment { index: 0 })
        );
        assert_eq!(
            DomainFilePath::new(r"workspace\\settings.json"),
            Err(DomainFilePathError::InvalidSeparator)
        );
        assert_eq!(
            DomainFilePath::new("/settings.json"),
            Err(DomainFilePathError::Absolute)
        );
    }

    #[test]
    fn descriptor_enforces_file_policy() {
        let id = DomainId::new("example.settings").unwrap();
        let version = SchemaVersion::new(1).unwrap();

        assert_eq!(
            DomainDescriptor::new(id.clone(), version, StorageClass::UserConfig, None),
            Err(DomainDescriptorError::FilePathRequired {
                storage_class: StorageClass::UserConfig
            })
        );
        assert_eq!(
            DomainDescriptor::new(
                id,
                version,
                StorageClass::Secret,
                Some(DomainFilePath::new("secret.json").unwrap())
            ),
            Err(DomainDescriptorError::FilePathForbidden {
                storage_class: StorageClass::Secret
            })
        );
    }
}
