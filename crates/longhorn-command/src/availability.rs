use std::{error::Error, fmt};

use longhorn_core::{CommandAvailabilityReasonId, CommandEvidenceCode, CommandId};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{CommandContextRevision, CommandRegistryGeneration};

/// Maximum UTF-8 bytes in optional command availability or outcome detail.
pub const MAXIMUM_COMMAND_DIAGNOSTIC_BYTES: usize = 4_096;
const HARD_MAXIMUM_AVAILABILITY_RECORDS: usize = 65_536;

/// Bounded nonempty consumer-owned diagnostic text.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "string"))]
pub struct CommandDiagnostic(String);

impl CommandDiagnostic {
    /// Validates and constructs bounded diagnostic text.
    pub fn new(value: impl Into<String>) -> Result<Self, CommandDiagnosticError> {
        let value = value.into();
        if value.is_empty() {
            return Err(CommandDiagnosticError::Empty);
        }
        if value.len() > MAXIMUM_COMMAND_DIAGNOSTIC_BYTES {
            return Err(CommandDiagnosticError::TooLong {
                maximum: MAXIMUM_COMMAND_DIAGNOSTIC_BYTES,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Returns the diagnostic text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommandDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for CommandDiagnostic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CommandDiagnostic {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Invalid command diagnostic text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandDiagnosticError {
    /// Text is empty.
    Empty,
    /// Text exceeds the fixed defensive bound.
    TooLong {
        /// Maximum accepted bytes.
        maximum: usize,
        /// Supplied bytes.
        actual: usize,
    },
}

impl fmt::Display for CommandDiagnosticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("command diagnostic cannot be empty"),
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "command diagnostic contains {actual} bytes; maximum is {maximum}"
            ),
        }
    }
}

impl Error for CommandDiagnosticError {}

/// Stable reason category for current command availability.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "kind", content = "code")]
pub enum CommandAvailabilityReasonCode {
    /// The current hot path does not admit the command.
    ContextNotAllowed,
    /// A required command capability is absent.
    MissingCapability,
    /// Consumer-owned current availability reason.
    Consumer(CommandAvailabilityReasonId),
}

/// Stable coded current availability reason with optional bounded detail.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandAvailabilityReason {
    code: CommandAvailabilityReasonCode,
    detail: Option<CommandDiagnostic>,
}

impl CommandAvailabilityReason {
    /// Constructs one coded reason.
    #[must_use]
    pub const fn new(
        code: CommandAvailabilityReasonCode,
        detail: Option<CommandDiagnostic>,
    ) -> Self {
        Self { code, detail }
    }

    /// Returns the stable reason category.
    #[must_use]
    pub const fn code(&self) -> &CommandAvailabilityReasonCode {
        &self.code
    }

    /// Returns optional consumer-owned detail.
    #[must_use]
    pub const fn detail(&self) -> Option<&CommandDiagnostic> {
        self.detail.as_ref()
    }
}

/// Current command availability state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum CommandAvailabilityState {
    /// Current facts admit the command.
    Available,
    /// Current facts reject the command.
    Unavailable,
    /// Current policy conceals the command.
    Hidden,
    /// The current composition cannot support the command.
    Unsupported,
}

/// Checked current availability for one command.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandAvailability {
    state: CommandAvailabilityState,
    reason: Option<CommandAvailabilityReason>,
}

impl CommandAvailability {
    /// Constructs available posture with no contradictory reason.
    #[must_use]
    pub const fn available() -> Self {
        Self {
            state: CommandAvailabilityState::Available,
            reason: None,
        }
    }

    /// Constructs unavailable posture.
    #[must_use]
    pub const fn unavailable(reason: CommandAvailabilityReason) -> Self {
        Self {
            state: CommandAvailabilityState::Unavailable,
            reason: Some(reason),
        }
    }

    /// Constructs hidden posture.
    #[must_use]
    pub const fn hidden(reason: CommandAvailabilityReason) -> Self {
        Self {
            state: CommandAvailabilityState::Hidden,
            reason: Some(reason),
        }
    }

    /// Constructs unsupported posture.
    #[must_use]
    pub const fn unsupported(reason: CommandAvailabilityReason) -> Self {
        Self {
            state: CommandAvailabilityState::Unsupported,
            reason: Some(reason),
        }
    }

    /// Returns current availability state.
    #[must_use]
    pub const fn state(&self) -> CommandAvailabilityState {
        self.state
    }

    /// Returns the required reason for any non-available state.
    #[must_use]
    pub const fn reason(&self) -> Option<&CommandAvailabilityReason> {
        self.reason.as_ref()
    }

    /// Returns whether execution may proceed.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self.state, CommandAvailabilityState::Available)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CommandAvailabilityWire {
    state: CommandAvailabilityState,
    reason: Option<CommandAvailabilityReason>,
}

impl<'de> Deserialize<'de> for CommandAvailability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CommandAvailabilityWire::deserialize(deserializer)?;
        match (wire.state, wire.reason) {
            (CommandAvailabilityState::Available, None) => Ok(Self::available()),
            (CommandAvailabilityState::Unavailable, Some(reason)) => Ok(Self::unavailable(reason)),
            (CommandAvailabilityState::Hidden, Some(reason)) => Ok(Self::hidden(reason)),
            (CommandAvailabilityState::Unsupported, Some(reason)) => Ok(Self::unsupported(reason)),
            (CommandAvailabilityState::Available, Some(_)) => {
                Err(de::Error::custom("available command cannot carry a reason"))
            }
            (_, None) => Err(de::Error::custom(
                "non-available command must carry a reason",
            )),
        }
    }
}

/// One command entry in a current availability snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandAvailabilityRecord {
    command_id: CommandId,
    availability: CommandAvailability,
}

impl CommandAvailabilityRecord {
    pub(crate) const fn new(command_id: CommandId, availability: CommandAvailability) -> Self {
        Self {
            command_id,
            availability,
        }
    }

    /// Returns the semantic command identity.
    #[must_use]
    pub const fn command_id(&self) -> &CommandId {
        &self.command_id
    }

    /// Returns current checked availability.
    #[must_use]
    pub const fn availability(&self) -> &CommandAvailability {
        &self.availability
    }
}

/// Complete current availability projection for one registry and context revision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandAvailabilitySnapshot {
    registry_generation: CommandRegistryGeneration,
    context_revision: CommandContextRevision,
    records: Vec<CommandAvailabilityRecord>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CommandAvailabilitySnapshotWire {
    registry_generation: CommandRegistryGeneration,
    context_revision: CommandContextRevision,
    records: Vec<CommandAvailabilityRecord>,
}

impl<'de> Deserialize<'de> for CommandAvailabilitySnapshot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CommandAvailabilitySnapshotWire::deserialize(deserializer)?;
        if wire.records.len() > HARD_MAXIMUM_AVAILABILITY_RECORDS {
            return Err(de::Error::custom(
                "command availability snapshot exceeds the hard record limit",
            ));
        }
        if wire
            .records
            .windows(2)
            .any(|pair| pair[0].command_id >= pair[1].command_id)
        {
            return Err(de::Error::custom(
                "command availability records must be unique and in command-id order",
            ));
        }
        Ok(Self::new(
            wire.registry_generation,
            wire.context_revision,
            wire.records,
        ))
    }
}

impl CommandAvailabilitySnapshot {
    pub(crate) const fn new(
        registry_generation: CommandRegistryGeneration,
        context_revision: CommandContextRevision,
        records: Vec<CommandAvailabilityRecord>,
    ) -> Self {
        Self {
            registry_generation,
            context_revision,
            records,
        }
    }

    /// Returns the sealed registry generation.
    #[must_use]
    pub const fn registry_generation(&self) -> CommandRegistryGeneration {
        self.registry_generation
    }

    /// Returns the consumer context revision used for projection.
    #[must_use]
    pub const fn context_revision(&self) -> CommandContextRevision {
        self.context_revision
    }

    /// Returns records in stable command-id order.
    pub fn records(&self) -> impl ExactSizeIterator<Item = &CommandAvailabilityRecord> {
        self.records.iter()
    }

    /// Returns current availability for one command.
    #[must_use]
    pub fn command(&self, command_id: &CommandId) -> Option<&CommandAvailability> {
        self.records
            .binary_search_by(|record| record.command_id.cmp(command_id))
            .ok()
            .map(|index| &self.records[index].availability)
    }
}

/// Bounded consumer-owned evidence attached to an execution outcome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandEvidence {
    code: CommandEvidenceCode,
    detail: Option<CommandDiagnostic>,
}

impl CommandEvidence {
    /// Constructs opaque bounded evidence without interpreting product meaning.
    #[must_use]
    pub const fn new(code: CommandEvidenceCode, detail: Option<CommandDiagnostic>) -> Self {
        Self { code, detail }
    }

    /// Returns the stable consumer-owned code.
    #[must_use]
    pub const fn code(&self) -> &CommandEvidenceCode {
        &self.code
    }

    /// Returns optional bounded consumer detail.
    #[must_use]
    pub const fn detail(&self) -> Option<&CommandDiagnostic> {
        self.detail.as_ref()
    }
}
