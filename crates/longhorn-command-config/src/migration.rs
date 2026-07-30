use longhorn_command::{CommandKeymapPreset, CommandRegistry, CommandReservedChordPolicy};
use longhorn_config::{DomainIssue, MigrationStep};
use longhorn_core::SchemaVersion;
use serde_json::Value;

use crate::CommandKeymapState;

/// Current registry, preset, and codec authority exposed to migration code.
#[derive(Clone, Copy, Debug)]
pub struct CommandKeymapMigrationTarget<'target, P> {
    schema_version: SchemaVersion,
    registry: &'target CommandRegistry,
    presets: &'target [CommandKeymapPreset],
    reserved: &'target P,
}

impl<'target, P> CommandKeymapMigrationTarget<'target, P>
where
    P: CommandReservedChordPolicy,
{
    pub(crate) const fn new(
        schema_version: SchemaVersion,
        registry: &'target CommandRegistry,
        presets: &'target [CommandKeymapPreset],
        reserved: &'target P,
    ) -> Self {
        Self {
            schema_version,
            registry,
            presets,
            reserved,
        }
    }

    /// Returns the current configuration schema.
    #[must_use]
    pub const fn schema_version(self) -> SchemaVersion {
        self.schema_version
    }

    /// Returns the current sealed command registry.
    #[must_use]
    pub const fn registry(self) -> &'target CommandRegistry {
        self.registry
    }

    /// Returns current immutable presets.
    #[must_use]
    pub const fn presets(self) -> &'target [CommandKeymapPreset] {
        self.presets
    }

    /// Validates and encodes one migrated current state.
    pub fn encode_current(self, state: CommandKeymapState) -> Result<Value, DomainIssue> {
        let preset = self
            .presets
            .iter()
            .find(|preset| preset.id == state.active_preset_id)
            .ok_or_else(|| {
                DomainIssue::new(
                    "command-keymap-unknown-preset",
                    format!("unknown active preset {}", state.active_preset_id),
                )
            })?;
        let effective = longhorn_command::CommandEffectiveKeymap::compile(
            self.registry,
            preset,
            &state.overrides,
            self.reserved,
        )
        .map_err(keymap_issue)?;
        if effective.has_conflicts() {
            return Err(DomainIssue::new(
                "command-keymap-conflict",
                "migrated keymap contains unresolved conflicts",
            ));
        }
        serde_json::to_value(state)
            .map_err(|error| DomainIssue::new("command-keymap-migration-encode", error.to_string()))
    }
}

/// Consumer-owned validation and one-step migration for older keymap schemas.
pub trait CommandKeymapMigration<P> {
    /// Validates raw older-schema data before migration.
    fn validate_raw(&self, schema_version: SchemaVersion, value: &Value)
    -> Result<(), DomainIssue>;

    /// Migrates one older schema or returns `None` when unavailable.
    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
        target: CommandKeymapMigrationTarget<'_, P>,
    ) -> Result<Option<MigrationStep>, DomainIssue>;
}

/// Explicit no-migration implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoCommandKeymapMigration;

impl<P> CommandKeymapMigration<P> for NoCommandKeymapMigration {
    fn validate_raw(
        &self,
        _schema_version: SchemaVersion,
        _value: &Value,
    ) -> Result<(), DomainIssue> {
        Ok(())
    }

    fn migrate_one(
        &self,
        _from: SchemaVersion,
        _value: Value,
        _target: CommandKeymapMigrationTarget<'_, P>,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        Ok(None)
    }
}

pub(crate) fn keymap_issue(error: longhorn_command::CommandKeymapError) -> DomainIssue {
    DomainIssue::new(
        format!("command-keymap-{:?}", error.code()).to_ascii_lowercase(),
        error.detail(),
    )
}
