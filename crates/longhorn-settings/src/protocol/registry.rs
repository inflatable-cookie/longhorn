use longhorn_core::{SettingsCapabilityId, SettingsScopeId};
use serde::{Deserialize, Serialize};

use crate::{
    SettingsApplyUnitDefinition, SettingsCapabilityDefinition, SettingsLimits,
    SettingsModuleDefinition, SettingsPageDefinition, SettingsRegistry, SettingsRegistryDigest,
    SettingsRegistryGeneration, SettingsRendererDefinition, SettingsScopeDefinition,
    SettingsSectionDefinition,
};

use super::{SettingsProtocolVersion, SettingsScopeRevision};

/// Checked immutable registry projection served to renderer clients.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsRegistrySnapshot {
    /// Exact protocol version.
    pub protocol_version: SettingsProtocolVersion,
    /// Monotonic host composition generation.
    pub generation: SettingsRegistryGeneration,
    /// Digest of canonical admitted declarations.
    pub digest: SettingsRegistryDigest,
    /// Explicit defensive limits bound into this registry.
    pub limits: SettingsLimits,
    /// Capabilities present in this composition.
    pub composed_capabilities: Vec<SettingsCapabilityId>,
    /// Registered modules in navigation order.
    pub modules: Vec<SettingsModuleDefinition>,
    /// Admitted nonempty sections in navigation order.
    pub sections: Vec<SettingsSectionDefinition>,
    /// Admitted pages in navigation order.
    pub pages: Vec<SettingsPageDefinition>,
    /// Registered renderer resolver keys.
    pub renderers: Vec<SettingsRendererDefinition>,
    /// Registered readable scopes.
    pub scopes: Vec<SettingsScopeDefinition>,
    /// Registered failure-atomic apply units.
    pub apply_units: Vec<SettingsApplyUnitDefinition>,
    /// Registered composition capabilities.
    pub capabilities: Vec<SettingsCapabilityDefinition>,
}

impl From<&SettingsRegistry> for SettingsRegistrySnapshot {
    fn from(registry: &SettingsRegistry) -> Self {
        Self {
            protocol_version: SettingsProtocolVersion::CURRENT,
            generation: registry.generation(),
            digest: registry.digest().clone(),
            limits: registry.limits(),
            composed_capabilities: registry.composed_capabilities().cloned().collect(),
            modules: registry.modules().cloned().collect(),
            sections: registry.sections().cloned().collect(),
            pages: registry.pages().cloned().collect(),
            renderers: registry.renderers().cloned().collect(),
            scopes: registry.scopes().cloned().collect(),
            apply_units: registry.apply_units().cloned().collect(),
            capabilities: registry.capabilities().cloned().collect(),
        }
    }
}

/// Registry invalidation hint. Clients reload the authoritative registry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsRegistryChangedEvent {
    /// Exact protocol version.
    pub protocol_version: SettingsProtocolVersion,
    /// Latest known registry generation.
    pub registry_generation: SettingsRegistryGeneration,
}

impl From<&SettingsRegistrySnapshot> for SettingsRegistryChangedEvent {
    fn from(registry: &SettingsRegistrySnapshot) -> Self {
        Self {
            protocol_version: SettingsProtocolVersion::CURRENT,
            registry_generation: registry.generation,
        }
    }
}

/// Scope invalidation hint. Clients reload the authoritative scope snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsScopeChangedEvent {
    /// Exact protocol version.
    pub protocol_version: SettingsProtocolVersion,
    /// Registry generation under which the scope was projected.
    pub registry_generation: SettingsRegistryGeneration,
    /// Scope whose authority changed.
    pub scope_id: SettingsScopeId,
    /// Latest known scope revision.
    pub scope_revision: SettingsScopeRevision,
}

impl From<&super::SettingsScopeSnapshot> for SettingsScopeChangedEvent {
    fn from(snapshot: &super::SettingsScopeSnapshot) -> Self {
        Self {
            protocol_version: SettingsProtocolVersion::CURRENT,
            registry_generation: snapshot.authority.registry_generation,
            scope_id: snapshot.scope_id.clone(),
            scope_revision: snapshot.authority.scope_revision,
        }
    }
}
