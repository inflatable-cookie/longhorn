use longhorn_core::{
    SettingsAnchorId, SettingsApplyUnitId, SettingsCapabilityId, SettingsModuleId, SettingsPageId,
    SettingsRendererId, SettingsScopeId, SettingsSectionId,
};
use serde::{Deserialize, Serialize};

/// Registered source of settings pages and authority declarations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsModuleDefinition {
    /// Stable module identity.
    pub id: SettingsModuleId,
    /// Consumer-owned navigation label.
    pub label: String,
    /// Explicit module order.
    pub order: i32,
}

/// Navigation section owned by one settings module.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsSectionDefinition {
    /// Stable section identity.
    pub id: SettingsSectionId,
    /// Owning module.
    pub module_id: SettingsModuleId,
    /// Consumer-owned navigation label.
    pub label: String,
    /// Explicit section order within the module.
    pub order: i32,
}

/// Renderer resolver key registered by the host.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsRendererDefinition {
    /// Stable renderer key.
    pub id: SettingsRendererId,
    /// Module responsible for resolving the renderer.
    pub module_id: SettingsModuleId,
}

/// Authoritative scope that can be projected into settings.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsScopeDefinition {
    /// Stable scope identity.
    pub id: SettingsScopeId,
    /// Module responsible for projecting the scope.
    pub module_id: SettingsModuleId,
}

/// Composition capability declared by one module.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsCapabilityDefinition {
    /// Stable capability identity.
    pub id: SettingsCapabilityId,
    /// Module responsible for the capability.
    pub module_id: SettingsModuleId,
}

/// Renderer-side mutation timing for one failure-atomic apply unit.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum SettingsMutationTiming {
    /// Each accepted intent is sent to authority without an explicit Apply.
    Immediate,
    /// Drafts stay local until an explicit Apply.
    Staged,
}

/// Smallest settings authority that promises failure-atomic mutation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsApplyUnitDefinition {
    /// Stable apply-unit identity.
    pub id: SettingsApplyUnitId,
    /// Module responsible for mutation semantics.
    pub module_id: SettingsModuleId,
    /// Scope mutated by this unit.
    pub scope_id: SettingsScopeId,
    /// Renderer-side mutation timing.
    pub timing: SettingsMutationTiming,
    /// Whether this unit accepts reset intents.
    pub reset_supported: bool,
}

/// Stable deep-link target within one settings page.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsAnchorDefinition {
    /// Stable anchor identity.
    pub id: SettingsAnchorId,
    /// Optional consumer-owned anchor label.
    pub label: Option<String>,
    /// Explicit anchor order.
    pub order: i32,
}

/// Optional actions surfaced for one settings page.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsPageFeatures {
    /// The page may expose reset.
    pub reset: bool,
    /// The page may expose import.
    pub import: bool,
    /// The page may expose backup.
    pub backup: bool,
    /// The page may expose restore.
    pub restore: bool,
    /// Mutations may require explicit confirmation.
    pub confirmation: bool,
}

/// Declarative settings page registered before registry seal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsPageDefinition {
    /// Stable page identity.
    pub id: SettingsPageId,
    /// Owning module.
    pub module_id: SettingsModuleId,
    /// Navigation section containing the page.
    pub section_id: SettingsSectionId,
    /// Host-resolved renderer key.
    pub renderer_id: SettingsRendererId,
    /// Consumer-owned navigation and search label.
    pub label: String,
    /// Consumer-owned search keywords.
    pub keywords: Vec<String>,
    /// Explicit page order within the section.
    pub order: i32,
    /// Stable deep-link targets.
    pub anchors: Vec<SettingsAnchorDefinition>,
    /// Capabilities required for registry admission.
    pub required_capabilities: Vec<SettingsCapabilityId>,
    /// Scopes the page may load.
    pub readable_scope_ids: Vec<SettingsScopeId>,
    /// Failure-atomic units the page may mutate.
    pub writable_apply_unit_ids: Vec<SettingsApplyUnitId>,
    /// Optional actions supported by the page.
    pub features: SettingsPageFeatures,
}
