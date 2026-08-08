//! Payload codec identity and structural migration seams.

use std::{convert::Infallible, error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::Value;

/// Stable structural format family for public linear history envelopes.
pub const HISTORY_FORMAT_FAMILY: &str = "longhorn.linear-history";
/// Current structural envelope version.
pub const CURRENT_HISTORY_STRUCTURAL_VERSION: u32 = 1;
/// Defensive hard ceiling for one encoded structural envelope.
pub const MAXIMUM_HISTORY_PERSISTED_BYTES: u64 = 1 << 41;
/// Defensive hard ceiling for one payload codec family identity.
pub const MAXIMUM_HISTORY_PAYLOAD_CODEC_FAMILY_BYTES: usize = 128;

/// Registered, bounded consumer payload codec family.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct HistoryPayloadCodecFamily(String);

impl HistoryPayloadCodecFamily {
    /// Validates and constructs one stable codec family.
    pub fn new(value: impl Into<String>) -> Result<Self, HistoryPayloadCodecFamilyError> {
        let value = value.into();
        if value.is_empty() {
            return Err(HistoryPayloadCodecFamilyError::Empty);
        }
        if value.len() > MAXIMUM_HISTORY_PAYLOAD_CODEC_FAMILY_BYTES {
            return Err(HistoryPayloadCodecFamilyError::TooLong {
                maximum: MAXIMUM_HISTORY_PAYLOAD_CODEC_FAMILY_BYTES,
                actual: value.len(),
            });
        }
        if let Some((index, _)) = value.char_indices().find(|(_, character)| {
            !(character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '_' | ':' | '-'))
        }) {
            return Err(HistoryPayloadCodecFamilyError::InvalidCharacter { index });
        }
        Ok(Self(value))
    }

    /// Returns the serialized codec family.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HistoryPayloadCodecFamily {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for HistoryPayloadCodecFamily {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Invalid payload codec family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryPayloadCodecFamilyError {
    /// The family was empty.
    Empty,
    /// The family exceeded the defensive byte ceiling.
    TooLong {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied bytes.
        actual: usize,
    },
    /// One character fell outside the stable lowercase ASCII grammar.
    InvalidCharacter {
        /// Invalid byte index.
        index: usize,
    },
}

impl fmt::Display for HistoryPayloadCodecFamilyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("history payload codec family cannot be empty"),
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "history payload codec family is {actual} bytes; maximum is {maximum}"
            ),
            Self::InvalidCharacter { index } => write!(
                formatter,
                "history payload codec family has an invalid character at byte {index}"
            ),
        }
    }
}

impl Error for HistoryPayloadCodecFamilyError {}

/// Independent version of one consumer payload codec family.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct HistoryPayloadCodecVersion(u32);

impl HistoryPayloadCodecVersion {
    /// Constructs one codec version.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    pub(super) fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// One exact step produced by a registered payload migration hook.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryPayloadMigrationStep {
    version: HistoryPayloadCodecVersion,
    bytes: Vec<u8>,
}

impl HistoryPayloadMigrationStep {
    /// Constructs one migration step.
    #[must_use]
    pub const fn new(version: HistoryPayloadCodecVersion, bytes: Vec<u8>) -> Self {
        Self { version, bytes }
    }

    /// Returns the version produced by this step.
    #[must_use]
    pub const fn version(&self) -> HistoryPayloadCodecVersion {
        self.version
    }

    /// Returns migrated payload bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the step into its produced version and migrated bytes.
    #[must_use]
    pub fn into_parts(self) -> (HistoryPayloadCodecVersion, Vec<u8>) {
        (self.version, self.bytes)
    }
}

/// Current registered payload codec authority exposed to migration hooks.
#[derive(Clone, Copy, Debug)]
pub struct HistoryPayloadMigrationTarget<'target> {
    pub(super) family: &'target HistoryPayloadCodecFamily,
    pub(super) version: HistoryPayloadCodecVersion,
}

impl<'target> HistoryPayloadMigrationTarget<'target> {
    /// Constructs a migration target for a compatible optional history layer.
    #[must_use]
    pub const fn new(
        family: &'target HistoryPayloadCodecFamily,
        version: HistoryPayloadCodecVersion,
    ) -> Self {
        Self { family, version }
    }

    /// Returns the registered codec family.
    #[must_use]
    pub const fn family(self) -> &'target HistoryPayloadCodecFamily {
        self.family
    }

    /// Returns the registered current codec version.
    #[must_use]
    pub const fn version(self) -> HistoryPayloadCodecVersion {
        self.version
    }
}

/// Registered consumer codec for one typed payload family.
pub trait HistoryPayloadCodec<P> {
    /// Codec or payload migration failure.
    type Error;

    /// Returns the stable registered codec family.
    fn family(&self) -> &HistoryPayloadCodecFamily;

    /// Returns the current codec version.
    fn version(&self) -> HistoryPayloadCodecVersion;

    /// Encodes one current typed payload.
    fn encode(&self, payload: &P) -> Result<Vec<u8>, Self::Error>;

    /// Decodes one payload at the current version.
    fn decode(&self, bytes: &[u8]) -> Result<P, Self::Error>;

    /// Migrates one older payload version or returns `None` when unsupported.
    fn migrate_one(
        &self,
        _from: HistoryPayloadCodecVersion,
        _bytes: Vec<u8>,
        _target: HistoryPayloadMigrationTarget<'_>,
    ) -> Result<Option<HistoryPayloadMigrationStep>, Self::Error> {
        Ok(None)
    }
}

/// One exact step produced by a structural envelope migration hook.
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryStructuralMigrationStep {
    version: u32,
    document: Value,
}

impl HistoryStructuralMigrationStep {
    /// Constructs one structural migration step.
    #[must_use]
    pub const fn new(version: u32, document: Value) -> Self {
        Self { version, document }
    }

    /// Returns the structural version produced by this step.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Returns the migrated raw structural document.
    #[must_use]
    pub const fn document(&self) -> &Value {
        &self.document
    }

    pub(super) fn into_parts(self) -> (u32, Value) {
        (self.version, self.document)
    }
}

/// Current Longhorn structural authority exposed to migration hooks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryStructuralMigrationTarget {
    pub(super) version: u32,
}

impl HistoryStructuralMigrationTarget {
    /// Returns the stable structural format family.
    #[must_use]
    pub const fn family(self) -> &'static str {
        HISTORY_FORMAT_FAMILY
    }

    /// Returns the current structural version.
    #[must_use]
    pub const fn version(self) -> u32 {
        self.version
    }
}

/// Registered one-step migration for older structural envelopes.
pub trait HistoryStructuralMigration {
    /// Structural migration failure.
    type Error;

    /// Migrates one older structural document or returns `None`.
    fn migrate_one(
        &self,
        from: u32,
        document: Value,
        target: HistoryStructuralMigrationTarget,
    ) -> Result<Option<HistoryStructuralMigrationStep>, Self::Error>;
}

/// Explicit registration with no older structural migration.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoHistoryStructuralMigration;

impl HistoryStructuralMigration for NoHistoryStructuralMigration {
    type Error = Infallible;

    fn migrate_one(
        &self,
        _from: u32,
        _document: Value,
        _target: HistoryStructuralMigrationTarget,
    ) -> Result<Option<HistoryStructuralMigrationStep>, Self::Error> {
        Ok(None)
    }
}

/// Caller-selected bound for untrusted structural bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryPersistenceLimits {
    maximum_envelope_bytes: u64,
}

impl HistoryPersistenceLimits {
    /// Validates one explicit load and encode bound.
    pub const fn new(maximum_envelope_bytes: u64) -> Result<Self, HistoryPersistenceLimitsError> {
        if maximum_envelope_bytes == 0 {
            return Err(HistoryPersistenceLimitsError::Zero);
        }
        if maximum_envelope_bytes > MAXIMUM_HISTORY_PERSISTED_BYTES {
            return Err(HistoryPersistenceLimitsError::TooLarge {
                maximum: MAXIMUM_HISTORY_PERSISTED_BYTES,
                actual: maximum_envelope_bytes,
            });
        }
        Ok(Self {
            maximum_envelope_bytes,
        })
    }

    /// Returns the maximum accepted or produced envelope size.
    #[must_use]
    pub const fn maximum_envelope_bytes(self) -> u64 {
        self.maximum_envelope_bytes
    }
}

/// Invalid structural persistence bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryPersistenceLimitsError {
    /// The bound was zero.
    Zero,
    /// The bound exceeded the defensive ceiling.
    TooLarge {
        /// Defensive ceiling.
        maximum: u64,
        /// Supplied bound.
        actual: u64,
    },
}

impl fmt::Display for HistoryPersistenceLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("history persistence bound must be nonzero"),
            Self::TooLarge { maximum, actual } => write!(
                formatter,
                "history persistence bound is {actual}; hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for HistoryPersistenceLimitsError {}
