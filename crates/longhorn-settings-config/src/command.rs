use std::time::Duration;

use longhorn_config::{CheckedMutationError, ConfigDomain, ConfigStore, MutationOptions};
use longhorn_core::SettingsEntryId;
use longhorn_settings::{
    SettingsApplyCommand, SettingsConflict, SettingsDurabilityEvidence, SettingsLoadCommand,
    SettingsLoadOutcome, SettingsMutationOutcome, SettingsMutationReceipt, SettingsMutationResult,
    SettingsOpaqueValue, SettingsRejectionCode, SettingsResetCommand,
};

use crate::{
    ConfigSettingsApplyUnit, SettingsCommittedMutation, SettingsConfigAdapter, SettingsConfigError,
    SettingsConfigLoadError,
    execution::{CheckAbort, guard_targets, rejected},
    projection::{scope_snapshot, source_diagnostics},
    validation::{MutationEnvelope, rejection},
};

enum MutationIntent<'command> {
    Apply(&'command SettingsOpaqueValue),
    Reset(&'command [SettingsEntryId]),
}

enum CheckedIntent<'command, I> {
    Apply(I),
    Reset(&'command [SettingsEntryId]),
}

impl<D, A> ConfigSettingsApplyUnit<D, A>
where
    D: ConfigDomain,
    A: SettingsConfigAdapter<D::Value>,
{
    /// Loads one checked scope through a stable coordinated read.
    pub fn load(
        &self,
        store: &ConfigStore,
        command: &SettingsLoadCommand,
        lock_timeout: Duration,
    ) -> Result<SettingsLoadOutcome, SettingsConfigError> {
        let code = if command.registry_generation != self.generation
            || command.scope_id != self.definition.scope_id
        {
            Some(SettingsRejectionCode::RegistryChanged)
        } else if !self.readable_pages.contains(&command.page_id) {
            Some(SettingsRejectionCode::Unauthorized)
        } else {
            None
        };
        if let Some(code) = code {
            return Ok(SettingsLoadOutcome::Rejected {
                rejection: rejection(code),
            });
        }

        let mut authority = self.authority.lock()?;
        let loaded = store
            .with_coordinated_load_set(lock_timeout, |set| set.load(&self.domain))
            .map_err(SettingsConfigLoadError::Coordination)
            .map_err(SettingsConfigError::Load)?
            .map_err(SettingsConfigLoadError::Store)
            .map_err(SettingsConfigError::Load)?;
        let snapshot = self.snapshot_from_load(loaded, &mut authority)?;
        Ok(SettingsLoadOutcome::Loaded { snapshot })
    }

    /// Applies immediate or staged intent through the same checked authority.
    pub fn apply(
        &self,
        store: &ConfigStore,
        command: &SettingsApplyCommand,
        options: MutationOptions,
    ) -> Result<SettingsMutationResult, SettingsConfigError> {
        self.mutate(
            store,
            options,
            MutationEnvelope {
                request_id: &command.request_id,
                page_id: &command.page_id,
                apply_unit_id: &command.apply_unit_id,
                scope_id: &command.scope_id,
                authority: &command.authority,
            },
            MutationIntent::Apply(&command.intent),
        )
    }

    /// Resets only named overrides through the bound one-domain authority.
    pub fn reset(
        &self,
        store: &ConfigStore,
        command: &SettingsResetCommand,
        options: MutationOptions,
    ) -> Result<SettingsMutationResult, SettingsConfigError> {
        self.mutate(
            store,
            options,
            MutationEnvelope {
                request_id: &command.request_id,
                page_id: &command.page_id,
                apply_unit_id: &command.apply_unit_id,
                scope_id: &command.scope_id,
                authority: &command.authority,
            },
            MutationIntent::Reset(&command.entry_ids),
        )
    }

    fn mutate(
        &self,
        store: &ConfigStore,
        options: MutationOptions,
        envelope: MutationEnvelope<'_>,
        intent: MutationIntent<'_>,
    ) -> Result<SettingsMutationResult, SettingsConfigError> {
        if let Some(result) =
            self.reject_envelope(&envelope, matches!(intent, MutationIntent::Reset(_)))
        {
            return Ok(result);
        }
        let durability = self.durability_material()?;
        let mut authority = self.authority.lock()?;
        let mutation = store.mutate_checked(&self.domain, options, |context| {
            let projection = self
                .adapter
                .project(context.value())
                .map_err(SettingsConfigError::Projection)?;
            let encoded = self.encode_for_authority(context.value())?;
            let diagnostics = source_diagnostics(context.diagnostics());
            let mut observed = projection.clone();
            observed.add_source_diagnostics(&diagnostics);
            let token = self.projection_token(&encoded, &observed)?;
            let previous_authority = authority.observe(token)?;
            let previous = scope_snapshot(envelope.scope_id, previous_authority.clone(), observed);
            if envelope.authority != &previous_authority {
                return Err(CheckAbort::Result(Box::new(
                    SettingsMutationResult::Conflict {
                        conflict: SettingsConflict {
                            expected: envelope.authority.clone(),
                            actual: previous_authority,
                        },
                        snapshot: previous,
                    },
                )));
            }

            let refuse = |rejection| rejected(rejection, &previous);
            let checked_intent = match intent {
                MutationIntent::Apply(intent) => {
                    let intent = self.adapter.decode_intent(intent).map_err(refuse)?;
                    let targets = self.adapter.targeted_entries(&intent);
                    guard_targets(projection.values(), &targets).map_err(refuse)?;
                    self.adapter
                        .validate_intent(context.value(), &intent, &projection)
                        .map_err(refuse)?;
                    self.adapter
                        .patch(context.value_mut(), &intent)
                        .map_err(refuse)?;
                    CheckedIntent::Apply(intent)
                }
                MutationIntent::Reset(ids) => {
                    guard_targets(projection.values(), ids).map_err(refuse)?;
                    self.adapter
                        .reset(context.value_mut(), ids)
                        .map_err(refuse)?;
                    CheckedIntent::Reset(ids)
                }
            };
            let mut committed = self
                .adapter
                .project(context.value())
                .map_err(SettingsConfigError::Projection)?;
            let patched = self.encode_for_authority(context.value())?;
            // An unchanged config leaves its source, including missing-file diagnostics, intact.
            if patched == encoded {
                committed.add_source_diagnostics(&diagnostics);
            }
            let token = self.projection_token(&patched, &committed)?;
            let committed_authority = authority.prepare(token)?;
            Ok((
                checked_intent,
                previous_authority,
                committed_authority,
                committed,
            ))
        });

        let outcome = match mutation {
            Ok(outcome) => outcome,
            Err(CheckedMutationError::Check(CheckAbort::Result(result))) => return Ok(*result),
            Err(CheckedMutationError::Check(CheckAbort::Operational(error))) => return Err(error),
            Err(CheckedMutationError::Mutation(error)) => {
                return self.map_mutation_error(error, &mut authority);
            }
        };
        let ((intent, previous_authority, committed_authority, projection), _value, publication) =
            outcome.into_parts();
        authority.accept(&committed_authority);
        let mutation_outcome = if publication.is_some() {
            SettingsMutationOutcome::Changed
        } else {
            SettingsMutationOutcome::Unchanged
        };
        let committed_intent = match &intent {
            CheckedIntent::Apply(intent) => SettingsCommittedMutation::Apply(intent),
            CheckedIntent::Reset(ids) => SettingsCommittedMutation::Reset(ids),
        };
        let activation = self.adapter.activation_after_commit(
            committed_intent,
            self.definition.timing,
            mutation_outcome,
            &projection,
        );
        let snapshot = scope_snapshot(
            envelope.scope_id,
            committed_authority.clone(),
            projection.with_activation(activation.clone()),
        );
        Ok(SettingsMutationResult::Applied {
            snapshot,
            receipt: SettingsMutationReceipt {
                request_id: envelope.request_id.clone(),
                page_id: envelope.page_id.clone(),
                apply_unit_id: envelope.apply_unit_id.clone(),
                scope_id: envelope.scope_id.clone(),
                previous_authority,
                committed_authority,
                outcome: mutation_outcome,
                durability: publication
                    .as_ref()
                    .map_or(SettingsDurabilityEvidence::NotApplicable, |receipt| {
                        durability.for_kind(receipt.durability)
                    }),
                activation_requirements: activation,
            },
        })
    }
}
