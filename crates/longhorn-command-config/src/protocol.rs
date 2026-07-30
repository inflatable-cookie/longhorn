use longhorn_command::{
    CommandDiscoveryRecord, CommandEffectiveBinding, CommandKeymapConflict, CommandKeymapOverride,
    CommandRegistryDigest, CommandRegistryGeneration,
};
use longhorn_config::{Durability, LoadDiagnostic, LoadedOrigin, RecoveryKind};
use longhorn_core::{CommandBindingId, CommandKeymapPresetId, CommandRequestId, SchemaVersion};
use serde::{Deserialize, Serialize};

use crate::{CommandKeymapPatchDigest, CommandKeymapRevision, CommandKeymapState};

/// Current checked command/keymap host protocol version.
pub const COMMAND_KEYMAP_PROTOCOL_VERSION: u32 = 1;

/// Exact command/keymap protocol line.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct CommandKeymapProtocolVersion(u32);

impl CommandKeymapProtocolVersion {
    /// Current exact protocol.
    pub const CURRENT: Self = Self(COMMAND_KEYMAP_PROTOCOL_VERSION);

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Immutable preset metadata exposed to clients.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapPresetRecord {
    /// Stable preset identity.
    pub id: CommandKeymapPresetId,
    /// Immutable content version.
    pub version: SchemaVersion,
}

/// Sealed command catalogue and available preset metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandCatalogueSnapshot {
    /// Exact protocol line.
    pub protocol_version: CommandKeymapProtocolVersion,
    /// Sealed registry generation.
    pub registry_generation: CommandRegistryGeneration,
    /// Sealed canonical registry digest.
    pub registry_digest: CommandRegistryDigest,
    /// Stable command discovery records.
    pub commands: Vec<CommandDiscoveryRecord>,
    /// Available immutable presets.
    pub presets: Vec<CommandKeymapPresetRecord>,
}

/// Non-durable catalogue invalidation hint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandCatalogueChangedEvent {
    /// Exact protocol line.
    pub protocol_version: CommandKeymapProtocolVersion,
    /// Fresh sealed registry generation.
    pub registry_generation: CommandRegistryGeneration,
}

/// One typed patch over active preset selection and sparse directives.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapPatch {
    /// Optional active preset replacement.
    pub active_preset_id: Option<CommandKeymapPresetId>,
    /// Whether all current sparse directives are removed first.
    pub clear_overrides: bool,
    /// Stable directive identities to remove.
    pub remove_binding_ids: Vec<CommandBindingId>,
    /// Sparse directives to insert or replace by stable binding identity.
    pub upsert_overrides: Vec<CommandKeymapOverride>,
}

impl CommandKeymapPatch {
    /// Computes the order-invariant canonical patch digest.
    pub fn digest(&self) -> Result<CommandKeymapPatchDigest, serde_json::Error> {
        let canonical = self.canonical();
        serde_json::to_vec(&canonical).map(|bytes| CommandKeymapPatchDigest::from_bytes(&bytes))
    }

    pub(crate) fn canonical(&self) -> Self {
        let mut canonical = self.clone();
        canonical.remove_binding_ids.sort();
        canonical.upsert_overrides.sort_by_key(override_binding_id);
        canonical
    }
}

/// Exact evidence binding one preview to one commit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapCommitEvidence {
    /// Sealed registry generation used by preview.
    pub registry_generation: CommandRegistryGeneration,
    /// Current keymap revision used by preview.
    pub keymap_revision: CommandKeymapRevision,
    /// Active preset used by preview.
    pub active_preset_id: CommandKeymapPresetId,
    /// Active immutable preset version used by preview.
    pub active_preset_version: SchemaVersion,
    /// Canonical proposed patch digest.
    pub patch_digest: CommandKeymapPatchDigest,
}

/// Preview request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapPreview {
    /// Expected sealed registry generation.
    pub registry_generation: CommandRegistryGeneration,
    /// Expected current keymap revision.
    pub keymap_revision: CommandKeymapRevision,
    /// Expected active preset.
    pub active_preset_id: CommandKeymapPresetId,
    /// Expected active preset content version.
    pub active_preset_version: SchemaVersion,
    /// Proposed typed patch.
    pub patch: CommandKeymapPatch,
}

/// Commit request carrying exact preview evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapCommit {
    /// Correlation identity.
    pub request_id: CommandRequestId,
    /// Exact accepted preview evidence.
    pub evidence: CommandKeymapCommitEvidence,
    /// Proposed typed patch.
    pub patch: CommandKeymapPatch,
}

/// Reset request returning to compiled default state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapReset {
    /// Correlation identity.
    pub request_id: CommandRequestId,
    /// Expected sealed registry generation.
    pub registry_generation: CommandRegistryGeneration,
    /// Expected current keymap revision.
    pub keymap_revision: CommandKeymapRevision,
    /// Expected active preset.
    pub active_preset_id: CommandKeymapPresetId,
    /// Expected active preset version.
    pub active_preset_version: SchemaVersion,
}

/// Source posture of one loaded authoritative snapshot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum CommandKeymapLoadOrigin {
    /// Compiled default because no file exists.
    Default,
    /// Current published file.
    File,
    /// Older source migrated in memory and preserved.
    Migrated {
        /// Original schema.
        from: SchemaVersion,
        /// Current schema.
        to: SchemaVersion,
    },
}

impl From<LoadedOrigin> for CommandKeymapLoadOrigin {
    fn from(value: LoadedOrigin) -> Self {
        match value {
            LoadedOrigin::Default => Self::Default,
            LoadedOrigin::File => Self::File,
            LoadedOrigin::MigratedInMemory { from, to } => Self::Migrated { from, to },
        }
    }
}

/// Stable non-fatal source diagnostic.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapDiagnostic {
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable detail.
    pub detail: String,
}

impl From<&LoadDiagnostic> for CommandKeymapDiagnostic {
    fn from(value: &LoadDiagnostic) -> Self {
        Self {
            code: format!("{:?}", value.code).to_ascii_lowercase(),
            detail: value.message.clone(),
        }
    }
}

/// Authoritative effective keymap snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapSnapshot {
    /// Exact protocol line.
    pub protocol_version: CommandKeymapProtocolVersion,
    /// Sealed registry generation.
    pub registry_generation: CommandRegistryGeneration,
    /// Sealed registry digest.
    pub registry_digest: CommandRegistryDigest,
    /// Persisted active preset and sparse override state.
    pub state: CommandKeymapState,
    /// Active immutable preset content version.
    pub active_preset_version: SchemaVersion,
    /// Effective stable bindings.
    pub bindings: Vec<CommandEffectiveBinding>,
    /// Unresolved conflicts. Published snapshots always contain none.
    pub conflicts: Vec<CommandKeymapConflict>,
    /// Source and migration posture.
    pub origin: CommandKeymapLoadOrigin,
    /// Non-fatal source diagnostics.
    pub diagnostics: Vec<CommandKeymapDiagnostic>,
}

/// Non-durable keymap invalidation hint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapChangedEvent {
    /// Exact protocol line.
    pub protocol_version: CommandKeymapProtocolVersion,
    /// Registry generation against which the keymap compiled.
    pub registry_generation: CommandRegistryGeneration,
    /// Fresh authoritative keymap revision.
    pub keymap_revision: CommandKeymapRevision,
}

/// Stable preview or commit rejection category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum CommandKeymapRejectionCode {
    /// Patch contains duplicate or contradictory operations.
    InvalidPatch,
    /// Proposed state fails command/keymap validation.
    InvalidKeymap,
    /// Proposed state contains an unresolved conflict.
    Conflict,
    /// Monotonic revision cannot advance.
    RevisionOverflow,
}

/// Typed rejected proposed state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapRejection {
    /// Stable rejection category.
    pub code: CommandKeymapRejectionCode,
    /// Human-readable detail.
    pub detail: String,
}

/// Preview result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum CommandKeymapPreviewResult {
    /// Patch is valid against exact base evidence.
    Accepted {
        /// Evidence required by commit.
        evidence: CommandKeymapCommitEvidence,
        /// Proposed effective snapshot.
        snapshot: CommandKeymapSnapshot,
    },
    /// Base evidence changed.
    Stale {
        /// Fresh authoritative state.
        snapshot: CommandKeymapSnapshot,
    },
    /// Patch is invalid or conflicting.
    Rejected {
        /// Rejection.
        rejection: CommandKeymapRejection,
        /// Fresh authoritative state.
        snapshot: CommandKeymapSnapshot,
        /// Proposed conflicts when available.
        conflicts: Vec<CommandKeymapConflict>,
    },
}

/// Config recovery category projected without source bytes or paths.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum CommandKeymapRecoveryCode {
    /// Source could not be read.
    ReadFailed,
    /// Source JSON or envelope is corrupt.
    Corrupt,
    /// Source belongs to another domain.
    DomainMismatch,
    /// Source uses a future schema.
    FutureSchema,
    /// Source or migration failed validation.
    InvalidValue,
    /// Required migration is unavailable or invalid.
    MigrationFailed,
    /// Required storage or recovery authority is unavailable.
    AuthorityUnavailable,
}

impl From<RecoveryKind> for CommandKeymapRecoveryCode {
    fn from(value: RecoveryKind) -> Self {
        match value {
            RecoveryKind::ReadFailed => Self::ReadFailed,
            RecoveryKind::CorruptDocument => Self::Corrupt,
            RecoveryKind::DomainMismatch => Self::DomainMismatch,
            RecoveryKind::FutureSchema => Self::FutureSchema,
            RecoveryKind::MissingMigration
            | RecoveryKind::InvalidMigrationStep
            | RecoveryKind::MigrationFailed => Self::MigrationFailed,
            RecoveryKind::InvalidDefault
            | RecoveryKind::InvalidValue
            | RecoveryKind::DecodeFailed => Self::InvalidValue,
        }
    }
}

/// Recovery state preserving whether exact source bytes remain available.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapRecovery {
    /// Stable recovery category.
    pub code: CommandKeymapRecoveryCode,
    /// Human-readable detail.
    pub detail: String,
    /// Whether exact readable source bytes were preserved by config.
    pub source_preserved: bool,
}

/// Authoritative load outcome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum CommandKeymapLoadOutcome {
    /// Valid current state.
    Loaded {
        /// Authoritative effective snapshot.
        snapshot: CommandKeymapSnapshot,
    },
    /// Invalid source preserved for explicit recovery.
    Recovery {
        /// Recovery evidence.
        recovery: CommandKeymapRecovery,
    },
    /// Storage authority is unavailable.
    Unavailable {
        /// Diagnostic safe for the client.
        detail: String,
    },
}

/// Whether a successful commit changed published bytes.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum CommandKeymapMutationOutcome {
    /// Authoritative state changed and published.
    Changed,
    /// Proposed patch was already effective.
    Unchanged,
}

/// Publication durability projected without filesystem paths.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum CommandKeymapDurability {
    /// No bytes changed.
    NotApplicable,
    /// File contents were atomically replaced and synced.
    FileSynced,
    /// File and containing directory were synced.
    FileAndDirectorySynced,
}

impl From<Durability> for CommandKeymapDurability {
    fn from(value: Durability) -> Self {
        match value {
            Durability::FileSynced => Self::FileSynced,
            Durability::FileAndDirectorySynced => Self::FileAndDirectorySynced,
        }
    }
}

/// Successful request-correlated publication receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapMutationReceipt {
    /// Request correlation.
    pub request_id: CommandRequestId,
    /// Previous authoritative keymap revision.
    pub previous_revision: CommandKeymapRevision,
    /// Committed authoritative keymap revision.
    pub committed_revision: CommandKeymapRevision,
    /// Changed or unchanged posture.
    pub outcome: CommandKeymapMutationOutcome,
    /// Exact achieved durability.
    pub durability: CommandKeymapDurability,
    /// Canonical committed patch digest when commit used a patch.
    pub patch_digest: Option<CommandKeymapPatchDigest>,
}

/// Commit or reset result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum CommandKeymapMutationResult {
    /// Published or already-effective state.
    Applied {
        /// Authoritative state.
        snapshot: CommandKeymapSnapshot,
        /// Exact mutation receipt.
        receipt: CommandKeymapMutationReceipt,
    },
    /// Base evidence changed.
    Stale {
        /// Fresh authoritative state.
        snapshot: CommandKeymapSnapshot,
    },
    /// Proposed state was rejected.
    Rejected {
        /// Stable rejection.
        rejection: CommandKeymapRejection,
        /// Fresh authoritative state.
        snapshot: CommandKeymapSnapshot,
        /// Proposed conflicts when available.
        conflicts: Vec<CommandKeymapConflict>,
    },
}

pub(crate) fn override_binding_id(value: &CommandKeymapOverride) -> CommandBindingId {
    match value {
        CommandKeymapOverride::Disable { binding_id }
        | CommandKeymapOverride::Replace { binding_id, .. } => binding_id.clone(),
        CommandKeymapOverride::Add { binding } => binding.id.clone(),
    }
}
