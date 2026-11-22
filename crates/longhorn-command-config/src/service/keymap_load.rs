//! Shares a compiled keymap across one coordinated store operation.

use std::cell::RefCell;

use longhorn_command::{CommandEffectiveKeymap, CommandReservedChordPolicy};
use longhorn_config::{ConfigDomain, DomainDescriptor, DomainIssue, MigrationStep};
use longhorn_core::SchemaVersion;
use serde_json::Value;

use crate::domain::{decode_state, encode_state};
use crate::{CommandKeymapMigration, CommandKeymapState, RegisteredCommandKeymapDomain};

pub(super) struct KeymapLoad<'a, P, M> {
    domain: &'a RegisteredCommandKeymapDomain<P, M>,
    compiled: RefCell<Option<(CommandKeymapState, CommandEffectiveKeymap)>>,
}

impl<'a, P: CommandReservedChordPolicy, M: CommandKeymapMigration<P>> KeymapLoad<'a, P, M> {
    pub(super) fn new(domain: &'a RegisteredCommandKeymapDomain<P, M>) -> Self {
        Self {
            domain,
            compiled: RefCell::new(None),
        }
    }

    pub(super) fn with_effective<T>(
        &self,
        state: &CommandKeymapState,
        project: impl FnOnce(&CommandEffectiveKeymap) -> T,
    ) -> Result<T, DomainIssue> {
        let mut compiled = self.compiled.borrow_mut();
        if let Some((cached, effective)) = compiled.as_ref()
            && cached == state
        {
            return Ok(project(effective));
        }
        let effective = self.domain.compile_state(state)?;
        let output = project(&effective);
        *compiled = Some((state.clone(), effective));
        Ok(output)
    }
}

impl<P: CommandReservedChordPolicy, M: CommandKeymapMigration<P>> ConfigDomain
    for KeymapLoad<'_, P, M>
{
    type Value = CommandKeymapState;

    fn descriptor(&self) -> &DomainDescriptor {
        self.domain.descriptor()
    }

    fn default_value(&self) -> Self::Value {
        self.domain.default_value()
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        let state = decode_state(value)?;
        self.validate(&state)?;
        Ok(state)
    }

    fn encode(&self, state: &Self::Value) -> Result<Value, DomainIssue> {
        self.validate(state)?;
        encode_state(state)
    }

    fn validate(&self, state: &Self::Value) -> Result<(), DomainIssue> {
        self.with_effective(state, |_| ())
    }

    fn validate_raw(&self, version: SchemaVersion, value: &Value) -> Result<(), DomainIssue> {
        if version == self.descriptor().schema_version() {
            self.decode(value.clone()).map(|_| ())
        } else {
            self.domain.validate_raw(version, value)
        }
    }

    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        self.domain.migrate_one(from, value)
    }
}
