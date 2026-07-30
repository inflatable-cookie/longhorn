use longhorn_core::{
    SettingsApplyUnitId, SettingsEntryId, SettingsPageId, SettingsRequestId, SettingsScopeId,
};
use serde::{Deserialize, Serialize};

use crate::{SettingsOpaqueValue, SettingsRegistryGeneration};

use super::{
    authority::{
        SettingsActivationRequirement, SettingsAuthorityExpectation, SettingsScopeSnapshot,
    },
    identity::SettingsProtocolVersion,
};

/// Loads one scope under a sealed registry generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsLoadCommand {
    /// Exact protocol version.
    pub protocol_version: SettingsProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: SettingsRequestId,
    /// Registry generation expected by the client.
    pub registry_generation: SettingsRegistryGeneration,
    /// Scope to project.
    pub scope_id: SettingsScopeId,
    /// Previously checked authority for conditional reload, if available.
    pub known_authority: Option<SettingsAuthorityExpectation>,
}

/// Applies one consumer-owned intent through one failure-atomic unit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsApplyCommand {
    /// Exact protocol version.
    pub protocol_version: SettingsProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: SettingsRequestId,
    /// Page that issued the mutation.
    pub page_id: SettingsPageId,
    /// Failure-atomic unit selected by the page.
    pub apply_unit_id: SettingsApplyUnitId,
    /// Scope to mutate.
    pub scope_id: SettingsScopeId,
    /// Generation, revision, and token checked by the client.
    pub authority: SettingsAuthorityExpectation,
    /// Versioned consumer-owned mutation intent.
    pub intent: SettingsOpaqueValue,
}

/// Removes selected user overrides through one failure-atomic unit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsResetCommand {
    /// Exact protocol version.
    pub protocol_version: SettingsProtocolVersion,
    /// Idempotency and correlation identity.
    pub request_id: SettingsRequestId,
    /// Page that issued the reset.
    pub page_id: SettingsPageId,
    /// Failure-atomic unit selected by the page.
    pub apply_unit_id: SettingsApplyUnitId,
    /// Scope containing the user overrides.
    pub scope_id: SettingsScopeId,
    /// Generation, revision, and token checked by the client.
    pub authority: SettingsAuthorityExpectation,
    /// Consumer-owned fields whose user overrides should be removed.
    pub entry_ids: Vec<SettingsEntryId>,
}

/// Result of one accepted authoritative mutation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum SettingsMutationOutcome {
    /// Authoritative state changed.
    Changed,
    /// The accepted mutation was already reflected in authoritative state.
    Unchanged,
}

/// Evidence that mutation publication met its authority's durability contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum SettingsDurabilityEvidence {
    /// The authority has no durable publication contract.
    NotApplicable,
    /// The authority confirmed its durable publication contract.
    Confirmed {
        /// Optional authority-specific evidence under a consumer codec.
        evidence: Option<SettingsOpaqueValue>,
    },
}

/// Exact successful settings mutation receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsMutationReceipt {
    /// Request accepted by authority.
    pub request_id: SettingsRequestId,
    /// Page that issued the mutation.
    pub page_id: SettingsPageId,
    /// Failure-atomic unit that executed the mutation.
    pub apply_unit_id: SettingsApplyUnitId,
    /// Mutated scope.
    pub scope_id: SettingsScopeId,
    /// Authority before mutation.
    pub previous_authority: SettingsAuthorityExpectation,
    /// Authority after mutation.
    pub committed_authority: SettingsAuthorityExpectation,
    /// Whether authoritative state changed.
    pub outcome: SettingsMutationOutcome,
    /// Authority-specific durability evidence.
    pub durability: SettingsDurabilityEvidence,
    /// Runtime activation kept distinct from persistence success.
    pub activation_requirements: Vec<SettingsActivationRequirement>,
}

/// Stale client authority returned without silent overwrite or merge.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsConflict {
    /// Authority supplied by the client.
    pub expected: SettingsAuthorityExpectation,
    /// Current checked authority.
    pub actual: SettingsAuthorityExpectation,
}

/// Stable category for a rejected settings command.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum SettingsRejectionCode {
    /// Consumer intent failed authoritative validation.
    InvalidIntent,
    /// Managed policy prevents this mutation.
    PolicyBlocked,
    /// The target is read-only.
    ReadOnly,
    /// The target is hidden.
    Hidden,
    /// The target is unsupported.
    Unsupported,
    /// The scope requires recovery.
    RecoveryRequired,
    /// Host authorization rejected the request.
    Unauthorized,
    /// Registry generation or declaration no longer matches.
    RegistryChanged,
}

/// Typed authoritative command rejection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsRejection {
    /// Stable rejection category.
    pub code: SettingsRejectionCode,
    /// Optional consumer-owned structured diagnostic.
    pub diagnostic: Option<SettingsOpaqueValue>,
}

/// Outcome of an apply or reset command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum SettingsMutationResult {
    /// Mutation succeeded and returned fresh authority.
    Applied {
        /// Fresh authoritative snapshot.
        snapshot: SettingsScopeSnapshot,
        /// Exact successful receipt.
        receipt: SettingsMutationReceipt,
    },
    /// Mutation was stale and did not publish.
    Conflict {
        /// Expected and actual authority.
        conflict: SettingsConflict,
        /// Fresh authoritative snapshot.
        snapshot: SettingsScopeSnapshot,
    },
    /// Mutation was rejected without publication.
    Rejected {
        /// Typed rejection.
        rejection: SettingsRejection,
        /// Fresh snapshot when authority could safely provide one.
        snapshot: Option<SettingsScopeSnapshot>,
    },
}

/// Outcome of a settings scope load.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum SettingsLoadOutcome {
    /// Scope loaded with checked authority.
    Loaded {
        /// Fresh authoritative snapshot.
        snapshot: SettingsScopeSnapshot,
    },
    /// Scope load was rejected.
    Rejected {
        /// Typed rejection.
        rejection: SettingsRejection,
    },
}
