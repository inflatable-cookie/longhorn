//! Persistence encode and load failures.

use std::{error::Error, fmt};

use longhorn_core::{
    CompatibilityStore, FutureSchemaRefusal, FutureSchemaRefused, HistoryEntryId, HistoryId,
};

use crate::{HistoryLimitsError, HistoryStateError};

use super::{HistoryPayloadCodecFamily, HistoryPayloadCodecVersion};

/// Failed structural encoding.
#[derive(Debug)]
pub enum HistoryEncodeError<E> {
    /// Consumer payload encoding failed.
    Payload {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Consumer codec failure.
        error: E,
    },
    /// Encoded bytes disagreed with the admitted exact payload weight.
    PayloadWeightMismatch {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Weight retained by the authority.
        recorded: u64,
        /// Exact codec byte length.
        actual: u64,
    },
    /// Structural JSON encoding failed.
    Structural(serde_json::Error),
    /// One platform-size conversion overflowed.
    SizeOverflow,
    /// Encoded structural bytes exceeded the explicit caller bound.
    EnvelopeTooLarge {
        /// Configured maximum.
        maximum: u64,
        /// Encoded bytes.
        actual: u64,
    },
}

impl<E: fmt::Display> fmt::Display for HistoryEncodeError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Payload { entry_id, error } => {
                write!(formatter, "cannot encode history entry {entry_id}: {error}")
            }
            Self::PayloadWeightMismatch {
                entry_id,
                recorded,
                actual,
            } => write!(
                formatter,
                "history entry {entry_id} recorded weight {recorded}; codec produced {actual} bytes"
            ),
            Self::Structural(error) => write!(formatter, "cannot encode history envelope: {error}"),
            Self::SizeOverflow => formatter.write_str("history encoded size overflowed"),
            Self::EnvelopeTooLarge { maximum, actual } => write!(
                formatter,
                "history envelope is {actual} bytes; configured maximum is {maximum}"
            ),
        }
    }
}

impl<E> Error for HistoryEncodeError<E> where E: Error + 'static {}

/// Failed checked load. No authority is returned or replaced.
#[derive(Debug)]
pub enum HistoryLoadError<CE, PE, ME> {
    /// Source bytes exceeded the explicit caller bound.
    EnvelopeTooLarge {
        /// Configured maximum.
        maximum: u64,
        /// Supplied bytes.
        actual: u64,
    },
    /// Source was not valid JSON.
    InvalidJson(serde_json::Error),
    /// The minimum structural header was absent or malformed.
    InvalidHeader(HistoryStructuralHeaderError),
    /// The structural format family is not Longhorn linear history.
    ForeignFormatFamily {
        /// Supplied family.
        actual: String,
    },
    /// The structural version is newer than this implementation.
    FutureStructuralVersion {
        /// Supplied version.
        actual: u32,
        /// Maximum supported version.
        maximum: u32,
    },
    /// No structural step was registered for an older version.
    MissingStructuralMigration {
        /// Unsupported source version.
        from: u32,
    },
    /// A structural hook skipped or mis-stamped one version.
    InvalidStructuralMigration {
        /// Source version.
        from: u32,
        /// Version produced by the hook.
        produced: u32,
    },
    /// A registered structural migration failed.
    StructuralMigration(ME),
    /// Current-version source did not match the strict envelope.
    InvalidEnvelope(serde_json::Error),
    /// The source belongs to a different history authority.
    ForeignHistory {
        /// Expected authority.
        expected: HistoryId,
        /// Supplied authority.
        actual: HistoryId,
    },
    /// The source payload family differs from the registered codec.
    ForeignPayloadCodecFamily {
        /// Registered family.
        expected: HistoryPayloadCodecFamily,
        /// Supplied family.
        actual: HistoryPayloadCodecFamily,
    },
    /// The payload codec version is newer than the registered codec.
    FuturePayloadCodecVersion {
        /// Supplied version.
        actual: HistoryPayloadCodecVersion,
        /// Maximum supported version.
        maximum: HistoryPayloadCodecVersion,
    },
    /// No payload step was registered for one older entry.
    MissingPayloadMigration {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Unsupported source version.
        from: HistoryPayloadCodecVersion,
    },
    /// A payload hook skipped or mis-stamped one version.
    InvalidPayloadMigration {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Source version.
        from: HistoryPayloadCodecVersion,
        /// Version produced by the hook.
        produced: HistoryPayloadCodecVersion,
    },
    /// A registered payload migration failed.
    PayloadMigration {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Consumer codec failure.
        error: CE,
    },
    /// Current payload decoding failed.
    Payload {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Consumer codec failure.
        error: CE,
    },
    /// Decoded payload policy validation failed.
    Policy {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Consumer policy failure.
        error: PE,
    },
    /// Encoded payload length disagreed with source evidence.
    PayloadWeightMismatch {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Source-declared weight.
        recorded: u64,
        /// Exact source byte length.
        actual: u64,
    },
    /// Current codec bytes and consumer policy weight disagreed.
    PolicyWeightMismatch {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Exact codec byte length.
        codec: u64,
        /// Consumer policy measurement.
        policy: u64,
    },
    /// A decoded retained payload is no longer admissible.
    NoOpPayload(HistoryEntryId),
    /// One current payload exceeded the retained weight limit.
    PayloadTooHeavy {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Configured maximum.
        maximum: u64,
        /// Actual current bytes.
        actual: u64,
    },
    /// Total current payload bytes exceeded the retained weight limit.
    RetainedWeightTooLarge {
        /// Configured maximum.
        maximum: u64,
        /// Actual current bytes.
        actual: u64,
    },
    /// Persisted limits were invalid.
    Limits(HistoryLimitsError),
    /// Current position exceeded platform size.
    PositionOverflow,
    /// Current position exceeded retained entries.
    InvalidCurrentPosition {
        /// Retained entries.
        entries: usize,
        /// Supplied position.
        actual: usize,
    },
    /// The next insertion sequence was zero.
    InvalidNextSequence,
    /// Retained-baseline sequence was zero.
    InvalidBaselineSequence,
    /// One retained entry sequence was zero.
    InvalidEntrySequence {
        /// Affected entry.
        entry_id: HistoryEntryId,
    },
    /// One retained label was invalid.
    InvalidLabel {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Label validation failure.
        error: crate::HistoryLabelError,
    },
    /// One platform-size or retained-weight conversion overflowed.
    SizeOverflow,
    /// Fully decoded structural state failed current invariants.
    State(HistoryStateError),
}

impl<CE: fmt::Display, PE: fmt::Display, ME: fmt::Display> fmt::Display
    for HistoryLoadError<CE, PE, ME>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EnvelopeTooLarge { maximum, actual } => write!(
                formatter,
                "history envelope is {actual} bytes; configured maximum is {maximum}"
            ),
            Self::InvalidJson(error) => write!(formatter, "invalid history JSON: {error}"),
            Self::InvalidHeader(error) => write!(formatter, "invalid history header: {error}"),
            Self::ForeignFormatFamily { actual } => {
                write!(formatter, "unsupported history format family {actual}")
            }
            Self::FutureStructuralVersion { actual, maximum } => write!(
                formatter,
                "history structural version {actual} is newer than supported version {maximum}"
            ),
            Self::MissingStructuralMigration { from } => {
                write!(
                    formatter,
                    "no history structural migration from version {from}"
                )
            }
            Self::InvalidStructuralMigration { from, produced } => write!(
                formatter,
                "history structural migration from {from} produced invalid version {produced}"
            ),
            Self::StructuralMigration(error) => {
                write!(formatter, "history structural migration failed: {error}")
            }
            Self::InvalidEnvelope(error) => write!(formatter, "invalid history envelope: {error}"),
            Self::ForeignHistory { expected, actual } => write!(
                formatter,
                "history envelope belongs to {actual}; expected {expected}"
            ),
            Self::ForeignPayloadCodecFamily { expected, actual } => write!(
                formatter,
                "history payload codec family is {actual}; expected {expected}"
            ),
            Self::FuturePayloadCodecVersion { actual, maximum } => write!(
                formatter,
                "history payload codec version {} is newer than supported version {}",
                actual.get(),
                maximum.get()
            ),
            Self::MissingPayloadMigration { entry_id, from } => write!(
                formatter,
                "no payload migration for entry {entry_id} from version {}",
                from.get()
            ),
            Self::InvalidPayloadMigration {
                entry_id,
                from,
                produced,
            } => write!(
                formatter,
                "payload migration for entry {entry_id} from {} produced invalid version {}",
                from.get(),
                produced.get()
            ),
            Self::PayloadMigration { entry_id, error } => {
                write!(
                    formatter,
                    "payload migration for entry {entry_id} failed: {error}"
                )
            }
            Self::Payload { entry_id, error } => {
                write!(
                    formatter,
                    "history payload {entry_id} failed to decode: {error}"
                )
            }
            Self::Policy { entry_id, error } => {
                write!(
                    formatter,
                    "history payload {entry_id} failed policy validation: {error}"
                )
            }
            Self::PayloadWeightMismatch {
                entry_id,
                recorded,
                actual,
            } => write!(
                formatter,
                "history payload {entry_id} records weight {recorded}; source has {actual} bytes"
            ),
            Self::PolicyWeightMismatch {
                entry_id,
                codec,
                policy,
            } => write!(
                formatter,
                "history payload {entry_id} has {codec} codec bytes; policy measured {policy}"
            ),
            Self::NoOpPayload(entry_id) => {
                write!(formatter, "history payload {entry_id} is now a no-op")
            }
            Self::PayloadTooHeavy {
                entry_id,
                maximum,
                actual,
            } => write!(
                formatter,
                "history payload {entry_id} weighs {actual}; maximum is {maximum}"
            ),
            Self::RetainedWeightTooLarge { maximum, actual } => write!(
                formatter,
                "history retained payload weight is {actual}; maximum is {maximum}"
            ),
            Self::Limits(error) => write!(formatter, "invalid persisted history limits: {error}"),
            Self::PositionOverflow => {
                formatter.write_str("persisted history current position overflowed")
            }
            Self::InvalidCurrentPosition { entries, actual } => write!(
                formatter,
                "persisted current position {actual} exceeds {entries} entries"
            ),
            Self::InvalidNextSequence => {
                formatter.write_str("persisted history next sequence is zero")
            }
            Self::InvalidBaselineSequence => {
                formatter.write_str("persisted history baseline sequence is zero")
            }
            Self::InvalidEntrySequence { entry_id } => {
                write!(
                    formatter,
                    "persisted history entry {entry_id} has sequence zero"
                )
            }
            Self::InvalidLabel { entry_id, error } => {
                write!(
                    formatter,
                    "persisted history entry {entry_id} label is invalid: {error}"
                )
            }
            Self::SizeOverflow => formatter.write_str("persisted history size overflowed"),
            Self::State(error) => write!(formatter, "persisted history state is invalid: {error}"),
        }
    }
}

impl<CE, PE, ME> Error for HistoryLoadError<CE, PE, ME>
where
    CE: Error + 'static,
    PE: Error + 'static,
    ME: Error + 'static,
{
}

impl<CE, PE, ME> FutureSchemaRefused for HistoryLoadError<CE, PE, ME> {
    /// Linear history versions its structural envelope and its payload codec
    /// independently, and either can be ahead on a channel rejoin.
    fn future_schema_refusal(&self) -> Option<FutureSchemaRefusal> {
        match self {
            Self::FutureStructuralVersion { actual, maximum } => Some(
                FutureSchemaRefusal::versioned(CompatibilityStore::History, *actual, *maximum),
            ),
            Self::FuturePayloadCodecVersion { actual, maximum } => {
                Some(FutureSchemaRefusal::versioned(
                    CompatibilityStore::History,
                    actual.get(),
                    maximum.get(),
                ))
            }
            _ => None,
        }
    }
}

/// Invalid minimal structural header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryStructuralHeaderError {
    /// The document root was not an object.
    NotObject,
    /// The format family was absent or not a string.
    InvalidFamily,
    /// The structural version was absent or not an unsigned 32-bit integer.
    InvalidVersion,
}

impl fmt::Display for HistoryStructuralHeaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotObject => formatter.write_str("history document root must be an object"),
            Self::InvalidFamily => formatter.write_str("history formatFamily must be a string"),
            Self::InvalidVersion => formatter.write_str("history structuralVersion must be a u32"),
        }
    }
}

impl Error for HistoryStructuralHeaderError {}
