use std::{collections::BTreeSet, error::Error, fmt, time::Duration};

use longhorn_core::DomainId;
use serde::{Deserialize, Deserializer, Serialize, de};

pub(super) const MAX_METADATA_BYTES: usize = 512;
const DEFAULT_MAX_DOMAIN_BYTES: usize = 4 * 1024 * 1024;
const DEFAULT_MAX_TOTAL_BYTES: usize = 32 * 1024 * 1024;
const HARD_MAX_DOMAIN_BYTES: usize = 64 * 1024 * 1024;
const HARD_MAX_TOTAL_BYTES: usize = 256 * 1024 * 1024;

/// Purpose of one backup snapshot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupKind {
    /// Consumer-scheduled operational recovery point.
    Operational,
    /// User-requested portable export.
    UserExport,
    /// Safety point before a destructive schema migration.
    PreMigration,
    /// Safety point before a restore.
    PreRestore,
}

/// Application identity recorded in a backup manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupApplication {
    #[serde(deserialize_with = "deserialize_metadata")]
    id: String,
    #[serde(deserialize_with = "deserialize_metadata")]
    version: String,
}

impl BackupApplication {
    /// Constructs validated application metadata.
    pub fn new(
        id: impl Into<String>,
        version: impl Into<String>,
    ) -> Result<Self, BackupMetadataError> {
        Ok(Self {
            id: validated_metadata("application id", id.into())?,
            version: validated_metadata("application version", version.into())?,
        })
    }

    /// Returns the stable application id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the application version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// Longhorn producer identity recorded in a backup manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupProducer {
    #[serde(deserialize_with = "deserialize_metadata")]
    name: String,
    #[serde(deserialize_with = "deserialize_metadata")]
    version: String,
}

impl BackupProducer {
    /// Constructs validated producer metadata.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Result<Self, BackupMetadataError> {
        Ok(Self {
            name: validated_metadata("producer name", name.into())?,
            version: validated_metadata("producer version", version.into())?,
        })
    }

    /// Returns the producer name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the producer version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// Caller-supplied identity and time metadata for one snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupMetadata {
    pub(super) archive_id: String,
    pub(super) kind: BackupKind,
    pub(super) created_at: String,
    pub(super) application: BackupApplication,
    pub(super) producer: BackupProducer,
}

impl BackupMetadata {
    /// Constructs bounded backup metadata.
    pub fn new(
        archive_id: impl Into<String>,
        kind: BackupKind,
        created_at: impl Into<String>,
        application: BackupApplication,
        producer: BackupProducer,
    ) -> Result<Self, BackupMetadataError> {
        Ok(Self {
            archive_id: validated_metadata("archive id", archive_id.into())?,
            kind,
            created_at: validated_utc_timestamp(created_at.into())?,
            application,
            producer,
        })
    }

    /// Returns the caller-supplied archive id.
    #[must_use]
    pub fn archive_id(&self) -> &str {
        &self.archive_id
    }

    /// Returns the safety or operational purpose.
    #[must_use]
    pub const fn kind(&self) -> BackupKind {
        self.kind
    }

    /// Returns the strict UTC creation time.
    #[must_use]
    pub fn created_at(&self) -> &str {
        &self.created_at
    }

    /// Returns application identity.
    #[must_use]
    pub fn application(&self) -> &BackupApplication {
        &self.application
    }

    /// Returns producer identity.
    #[must_use]
    pub fn producer(&self) -> &BackupProducer {
        &self.producer
    }
}

/// Invalid bounded backup metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupMetadataError {
    field: &'static str,
    detail: String,
}

impl fmt::Display for BackupMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid backup metadata {}: {}",
            self.field, self.detail
        )
    }
}

impl Error for BackupMetadataError {}

/// Explicit set of registered domains considered by one capture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupScope {
    /// Consider every domain registered in the store.
    AllRegistered,
    /// Consider exactly the named registered domains.
    Selected(BTreeSet<DomainId>),
}

impl BackupScope {
    /// Constructs a non-empty selected-domain scope.
    pub fn selected(domains: impl IntoIterator<Item = DomainId>) -> Result<Self, BackupScopeError> {
        let domains = domains.into_iter().collect::<BTreeSet<_>>();
        if domains.is_empty() {
            Err(BackupScopeError::Empty)
        } else {
            Ok(Self::Selected(domains))
        }
    }
}

/// Invalid explicit backup scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupScopeError {
    /// A selected-domain scope contained no domains.
    Empty,
}

impl fmt::Display for BackupScopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selected backup scope cannot be empty")
    }
}

impl Error for BackupScopeError {}

/// Bounded ordinary snapshot payload limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackupLimits {
    max_domain_bytes: usize,
    max_total_bytes: usize,
}

impl BackupLimits {
    /// Constructs limits within the hard in-memory safety ceiling.
    pub fn new(max_domain_bytes: usize, max_total_bytes: usize) -> Result<Self, BackupLimitsError> {
        if max_domain_bytes == 0 || max_total_bytes == 0 {
            return Err(BackupLimitsError::Zero);
        }
        if max_domain_bytes > max_total_bytes {
            return Err(BackupLimitsError::DomainExceedsTotal);
        }
        if max_domain_bytes > HARD_MAX_DOMAIN_BYTES || max_total_bytes > HARD_MAX_TOTAL_BYTES {
            return Err(BackupLimitsError::HardCeiling {
                max_domain_bytes: HARD_MAX_DOMAIN_BYTES,
                max_total_bytes: HARD_MAX_TOTAL_BYTES,
            });
        }
        Ok(Self {
            max_domain_bytes,
            max_total_bytes,
        })
    }

    /// Returns the maximum bytes accepted from one domain source.
    #[must_use]
    pub const fn max_domain_bytes(self) -> usize {
        self.max_domain_bytes
    }

    /// Returns the maximum bytes retained by the complete snapshot.
    #[must_use]
    pub const fn max_total_bytes(self) -> usize {
        self.max_total_bytes
    }
}

impl Default for BackupLimits {
    fn default() -> Self {
        Self {
            max_domain_bytes: DEFAULT_MAX_DOMAIN_BYTES,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
        }
    }
}

/// Invalid backup snapshot limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupLimitsError {
    /// A byte limit was zero.
    Zero,
    /// The per-domain limit exceeded the total limit.
    DomainExceedsTotal,
    /// A limit exceeded the library's hard in-memory ceiling.
    HardCeiling {
        /// Maximum supported per-domain bytes.
        max_domain_bytes: usize,
        /// Maximum supported total bytes.
        max_total_bytes: usize,
    },
}

impl fmt::Display for BackupLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("backup byte limits must be non-zero"),
            Self::DomainExceedsTotal => {
                formatter.write_str("backup domain limit cannot exceed total limit")
            }
            Self::HardCeiling {
                max_domain_bytes,
                max_total_bytes,
            } => write!(
                formatter,
                "backup limits exceed hard ceilings of {max_domain_bytes} per domain and {max_total_bytes} total"
            ),
        }
    }
}

impl Error for BackupLimitsError {}

/// Coordination and memory policy for one capture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackupCaptureOptions {
    /// Maximum time spent acquiring the store coordinator.
    pub lock_timeout: Duration,
    /// Bounded in-memory payload limits.
    pub limits: BackupLimits,
}

impl BackupCaptureOptions {
    /// Constructs explicit capture options.
    #[must_use]
    pub const fn new(lock_timeout: Duration, limits: BackupLimits) -> Self {
        Self {
            lock_timeout,
            limits,
        }
    }
}

fn validated_metadata(field: &'static str, value: String) -> Result<String, BackupMetadataError> {
    if value.is_empty() {
        return Err(BackupMetadataError {
            field,
            detail: "cannot be empty".into(),
        });
    }
    if value.len() > MAX_METADATA_BYTES {
        return Err(BackupMetadataError {
            field,
            detail: format!("cannot exceed {MAX_METADATA_BYTES} bytes"),
        });
    }
    if value.chars().any(char::is_control) {
        return Err(BackupMetadataError {
            field,
            detail: "cannot contain control characters".into(),
        });
    }
    Ok(value)
}

pub(super) fn deserialize_metadata<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    validated_metadata("manifest text", String::deserialize(deserializer)?)
        .map_err(de::Error::custom)
}

pub(super) fn deserialize_utc_timestamp<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    parse_utc_timestamp(&value)
        .map(|_| value)
        .map_err(de::Error::custom)
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct UtcTimestamp {
    pub(crate) seconds: i64,
    pub(crate) nanoseconds: u32,
}

pub(crate) fn parse_utc_timestamp(value: &str) -> Result<UtcTimestamp, BackupMetadataError> {
    let bytes = value.as_bytes();
    let valid_shape = matches!(bytes.len(), 20 | 22..=30)
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes.get(10) == Some(&b'T')
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':')
        && bytes.last() == Some(&b'Z')
        && (bytes.len() == 20 || bytes.get(19) == Some(&b'.'));
    if !valid_shape {
        return Err(timestamp_error(
            "must use UTC RFC 3339 form YYYY-MM-DDTHH:MM:SS[.fraction]Z",
        ));
    }

    let year = parse_digits(bytes, 0, 4)?;
    let month = parse_digits(bytes, 5, 2)?;
    let day = parse_digits(bytes, 8, 2)?;
    let hour = parse_digits(bytes, 11, 2)?;
    let minute = parse_digits(bytes, 14, 2)?;
    let second = parse_digits(bytes, 17, 2)?;
    if year == 0
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(timestamp_error("contains an out-of-range calendar value"));
    }

    let nanoseconds = if bytes.len() == 20 {
        0
    } else {
        let fraction = &bytes[20..bytes.len() - 1];
        if fraction.is_empty() || fraction.len() > 9 || !fraction.iter().all(u8::is_ascii_digit) {
            return Err(timestamp_error(
                "fractional seconds must contain one to nine digits",
            ));
        }
        let value = fraction.iter().fold(0_u32, |accumulator, digit| {
            accumulator * 10 + u32::from(digit - b'0')
        });
        value * 10_u32.pow(u32::try_from(9 - fraction.len()).unwrap())
    };

    let days = days_from_civil(i64::from(year), i64::from(month), i64::from(day));
    Ok(UtcTimestamp {
        seconds: days * 86_400
            + i64::from(hour) * 3_600
            + i64::from(minute) * 60
            + i64::from(second),
        nanoseconds,
    })
}

fn validated_utc_timestamp(value: String) -> Result<String, BackupMetadataError> {
    parse_utc_timestamp(&value)?;
    Ok(value)
}

fn parse_digits(bytes: &[u8], start: usize, length: usize) -> Result<u32, BackupMetadataError> {
    let digits = bytes
        .get(start..start + length)
        .ok_or_else(|| timestamp_error("is truncated"))?;
    if !digits.iter().all(u8::is_ascii_digit) {
        return Err(timestamp_error("contains a non-digit calendar value"));
    }
    Ok(digits.iter().fold(0_u32, |accumulator, digit| {
        accumulator * 10 + u32::from(digit - b'0')
    }))
}

const fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let adjusted_year = year - if month <= 2 { 1 } else { 0 };
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn timestamp_error(detail: impl Into<String>) -> BackupMetadataError {
    BackupMetadataError {
        field: "creation time",
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests;
