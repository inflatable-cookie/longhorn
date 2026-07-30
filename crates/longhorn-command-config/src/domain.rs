use std::{collections::BTreeMap, error::Error, fmt};

use longhorn_command::{
    CommandEffectiveKeymap, CommandKeymapPreset, CommandRegistry, CommandReservedChordPolicy,
};
use longhorn_config::{
    BackupCatalog, BackupCatalogError, BackupExclusionReason, ConfigDomain, DomainDescriptor,
    DomainIssue, MigrationStep,
};
use longhorn_core::{CommandKeymapPresetId, SchemaVersion};
use serde_json::Value;

use crate::{
    CommandKeymapMigration, CommandKeymapMigrationTarget, CommandKeymapState,
    migration::keymap_issue,
};

/// Ordinary backup participation for the keymap configuration domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandKeymapBackupPolicy {
    /// Include active preset selection and sparse user overrides.
    Include,
    /// Preserve an explicit exclusion in backup evidence.
    Exclude(BackupExclusionReason),
}

/// One registered keymap domain bound to a sealed registry and preset set.
#[derive(Clone, Debug)]
pub struct RegisteredCommandKeymapDomain<P, M> {
    descriptor: DomainDescriptor,
    registry: CommandRegistry,
    presets: Vec<CommandKeymapPreset>,
    preset_index: BTreeMap<CommandKeymapPresetId, usize>,
    default: CommandKeymapState,
    reserved: P,
    migration: M,
    backup_policy: CommandKeymapBackupPolicy,
}

impl<P, M> RegisteredCommandKeymapDomain<P, M>
where
    P: CommandReservedChordPolicy,
    M: CommandKeymapMigration<P>,
{
    /// Binds complete consumer authority and validates every immutable preset.
    pub fn new(
        descriptor: DomainDescriptor,
        registry: CommandRegistry,
        presets: Vec<CommandKeymapPreset>,
        default_preset_id: CommandKeymapPresetId,
        reserved: P,
        migration: M,
        backup_policy: CommandKeymapBackupPolicy,
    ) -> Result<Self, RegisteredCommandKeymapDomainError> {
        let mut preset_index = BTreeMap::new();
        for (index, preset) in presets.iter().enumerate() {
            if preset_index.insert(preset.id.clone(), index).is_some() {
                return Err(RegisteredCommandKeymapDomainError::DuplicatePreset {
                    preset_id: preset.id.clone(),
                });
            }
            let effective = CommandEffectiveKeymap::compile(&registry, preset, &[], &reserved)
                .map_err(|error| RegisteredCommandKeymapDomainError::InvalidPreset {
                    preset_id: preset.id.clone(),
                    detail: error.to_string(),
                })?;
            if effective.has_conflicts() {
                return Err(RegisteredCommandKeymapDomainError::ConflictingPreset {
                    preset_id: preset.id.clone(),
                });
            }
        }
        if !preset_index.contains_key(&default_preset_id) {
            return Err(RegisteredCommandKeymapDomainError::UnknownDefaultPreset {
                preset_id: default_preset_id,
            });
        }
        let default = CommandKeymapState::initial(default_preset_id);
        Ok(Self {
            descriptor,
            registry,
            presets,
            preset_index,
            default,
            reserved,
            migration,
            backup_policy,
        })
    }

    /// Returns the exact injected domain descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    /// Returns the sealed command registry.
    #[must_use]
    pub const fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    /// Returns immutable presets in consumer registration order.
    #[must_use]
    pub fn presets(&self) -> &[CommandKeymapPreset] {
        &self.presets
    }

    /// Returns one immutable preset.
    #[must_use]
    pub fn preset(&self, id: &CommandKeymapPresetId) -> Option<&CommandKeymapPreset> {
        self.preset_index.get(id).map(|index| &self.presets[*index])
    }

    /// Returns the injected reserved-chord policy.
    #[must_use]
    pub const fn reserved_policy(&self) -> &P {
        &self.reserved
    }

    /// Returns the compiled initial state.
    #[must_use]
    pub const fn default_state(&self) -> &CommandKeymapState {
        &self.default
    }

    /// Returns backup participation policy.
    #[must_use]
    pub const fn backup_policy(&self) -> &CommandKeymapBackupPolicy {
        &self.backup_policy
    }

    /// Adds this exact domain to a backup catalogue.
    pub fn add_to_backup_catalog<'domain>(
        &'domain self,
        catalog: &mut BackupCatalog<'domain>,
    ) -> Result<(), BackupCatalogError> {
        match &self.backup_policy {
            CommandKeymapBackupPolicy::Include => catalog.include(self),
            CommandKeymapBackupPolicy::Exclude(reason) => catalog.exclude(self, reason.clone()),
        }
    }

    /// Compiles one validated state into its immutable effective map.
    pub fn compile_state(
        &self,
        state: &CommandKeymapState,
    ) -> Result<CommandEffectiveKeymap, DomainIssue> {
        let preset = self.preset(&state.active_preset_id).ok_or_else(|| {
            DomainIssue::new(
                "command-keymap-unknown-preset",
                format!("unknown active preset {}", state.active_preset_id),
            )
        })?;
        let effective = CommandEffectiveKeymap::compile(
            &self.registry,
            preset,
            &state.overrides,
            &self.reserved,
        )
        .map_err(keymap_issue)?;
        if effective.has_conflicts() {
            return Err(DomainIssue::new(
                "command-keymap-conflict",
                "effective keymap contains unresolved conflicts",
            ));
        }
        Ok(effective)
    }

    fn migration_target(&self) -> CommandKeymapMigrationTarget<'_, P> {
        CommandKeymapMigrationTarget::new(
            self.descriptor.schema_version(),
            &self.registry,
            &self.presets,
            &self.reserved,
        )
    }
}

impl<P, M> ConfigDomain for RegisteredCommandKeymapDomain<P, M>
where
    P: CommandReservedChordPolicy,
    M: CommandKeymapMigration<P>,
{
    type Value = CommandKeymapState;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        self.default.clone()
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        let state: CommandKeymapState = serde_json::from_value(value)
            .map_err(|error| DomainIssue::new("command-keymap-decode", error.to_string()))?;
        self.compile_state(&state)?;
        Ok(state)
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        self.compile_state(value)?;
        serde_json::to_value(value)
            .map_err(|error| DomainIssue::new("command-keymap-encode", error.to_string()))
    }

    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        self.compile_state(value).map(|_| ())
    }

    fn validate_raw(
        &self,
        schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if schema_version == self.descriptor.schema_version() {
            self.decode(value.clone()).map(|_| ())
        } else {
            self.migration.validate_raw(schema_version, value)
        }
    }

    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        self.migration
            .migrate_one(from, value, self.migration_target())
    }
}

/// Invalid registered keymap domain construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegisteredCommandKeymapDomainError {
    /// Preset identity repeats.
    DuplicatePreset {
        /// Repeated preset.
        preset_id: CommandKeymapPresetId,
    },
    /// Default preset is absent.
    UnknownDefaultPreset {
        /// Missing preset.
        preset_id: CommandKeymapPresetId,
    },
    /// A preset fails command, context, arguments, or modifier validation.
    InvalidPreset {
        /// Invalid preset.
        preset_id: CommandKeymapPresetId,
        /// Validation detail.
        detail: String,
    },
    /// An immutable preset contains an unresolved conflict.
    ConflictingPreset {
        /// Conflicting preset.
        preset_id: CommandKeymapPresetId,
    },
}

impl fmt::Display for RegisteredCommandKeymapDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePreset { preset_id } => {
                write!(formatter, "duplicate preset {preset_id}")
            }
            Self::UnknownDefaultPreset { preset_id } => {
                write!(formatter, "unknown default preset {preset_id}")
            }
            Self::InvalidPreset { preset_id, detail } => {
                write!(formatter, "invalid preset {preset_id}: {detail}")
            }
            Self::ConflictingPreset { preset_id } => {
                write!(
                    formatter,
                    "preset {preset_id} contains unresolved conflicts"
                )
            }
        }
    }
}

impl Error for RegisteredCommandKeymapDomainError {}
