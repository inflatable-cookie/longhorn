use longhorn_core::{
    SettingsActivationTargetId, SettingsAuthorityToken, SettingsEntryId, SettingsPolicySourceId,
    SettingsScopeId,
};
use serde::{Deserialize, Serialize};

use crate::{SettingsOpaqueValue, SettingsRegistryGeneration};

use super::identity::{SettingsProtocolVersion, SettingsScopeRevision};

/// Why the effective value was selected.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum SettingsEffectiveSource {
    /// No user override or policy replaced the compiled default.
    CompiledDefault,
    /// A configured user override is effective.
    UserConfiguration,
    /// Managed policy supplies the effective value.
    ManagedPolicy,
}

/// Whether a projected field can be presented or mutated.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum SettingsEditability {
    /// The field is visible and accepts mutation.
    Editable,
    /// The field is visible but cannot be mutated.
    ReadOnly,
    /// The field must not be shown.
    Hidden,
    /// The current authority cannot support the field.
    Unsupported,
}

/// How managed policy affects a projected field.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum SettingsPolicyEffect {
    /// Policy supplies the effective value.
    Override,
    /// Policy constrains otherwise configurable values.
    Constraint,
}

/// Managed-policy provenance and optional consumer-owned constraints.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsPolicyProjection {
    /// Stable policy source.
    pub source_id: SettingsPolicySourceId,
    /// Policy effect on the field.
    pub effect: SettingsPolicyEffect,
    /// Consumer codec constraints, when the effect is `Constraint`.
    pub constraints: Option<SettingsOpaqueValue>,
}

/// Typed source diagnostic attached to a checked projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsSourceDiagnostic {
    /// Stable lowercase diagnostic code.
    pub code: String,
    /// Optional consumer-owned structured detail.
    pub detail: Option<SettingsOpaqueValue>,
}

/// Checked configured, effective, default, and policy state for one field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsValueProjection {
    /// Consumer-owned stable field identity.
    pub entry_id: SettingsEntryId,
    /// Stored user override in this scope, if present.
    pub configured: Option<SettingsOpaqueValue>,
    /// Value currently used by the product.
    pub effective: SettingsOpaqueValue,
    /// Compiled product default.
    pub compiled_default: SettingsOpaqueValue,
    /// Source selected for the effective value.
    pub effective_source: SettingsEffectiveSource,
    /// Managed-policy provenance and constraints, if present.
    pub policy: Option<SettingsPolicyProjection>,
    /// Current presentation and mutation permission.
    pub editability: SettingsEditability,
    /// Checked source diagnostics.
    pub source_diagnostics: Vec<SettingsSourceDiagnostic>,
}

/// Stable category for a scope that cannot provide normal authority.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum SettingsRecoveryCode {
    /// Persisted state is corrupt.
    Corrupt,
    /// Persisted state uses a future schema.
    FutureSchema,
    /// The source authority is temporarily unavailable.
    AuthorityUnavailable,
    /// A restore or repair operation is active.
    RecoveryInProgress,
    /// The source requires an explicit recovery action.
    RecoveryRequired,
}

/// Checked recovery state for one settings scope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsRecoveryState {
    /// Stable recovery category.
    pub code: SettingsRecoveryCode,
    /// Optional consumer-owned recovery diagnostics.
    pub diagnostic: Option<SettingsOpaqueValue>,
}

/// Runtime activation state after durable settings mutation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum SettingsActivationState {
    /// Persistence succeeded but runtime activation remains outstanding.
    Pending,
    /// The host reports that runtime activation completed.
    Satisfied,
}

/// Runtime activation requirement independent of persistence success.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsActivationRequirement {
    /// Consumer-registered activation target.
    pub target_id: SettingsActivationTargetId,
    /// Current activation state.
    pub state: SettingsActivationState,
}

/// Generation, revision, and host-issued token identifying checked authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsAuthorityExpectation {
    /// Sealed registry generation used by the client.
    pub registry_generation: SettingsRegistryGeneration,
    /// Authoritative scope revision.
    pub scope_revision: SettingsScopeRevision,
    /// Opaque host-issued authority token.
    pub authority_token: SettingsAuthorityToken,
}

/// Checked authoritative values for one registered scope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsScopeSnapshot {
    /// Exact protocol version.
    pub protocol_version: SettingsProtocolVersion,
    /// Registered scope.
    pub scope_id: SettingsScopeId,
    /// Authority identity for stale-write rejection.
    pub authority: SettingsAuthorityExpectation,
    /// Checked field projections.
    pub values: Vec<SettingsValueProjection>,
    /// Recovery state, if normal projection is unavailable or degraded.
    pub recovery: Option<SettingsRecoveryState>,
    /// Runtime activation work still associated with this scope.
    pub activation_requirements: Vec<SettingsActivationRequirement>,
}
