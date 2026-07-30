use std::time::Duration;

use longhorn_config::{CheckedMutationError, ConfigDomain, ConfigStore, MutationOptions};
use longhorn_settings::{
    SettingsApplyCommand, SettingsConflict, SettingsDurabilityEvidence, SettingsLoadCommand,
    SettingsLoadOutcome, SettingsMutationOutcome, SettingsMutationReceipt, SettingsMutationResult,
    SettingsRejectionCode, SettingsResetCommand,
};

use crate::{
    ConfigSettingsApplyUnit, SettingsCommittedMutation, SettingsConfigAdapter, SettingsConfigError,
    SettingsConfigLoadError,
    execution::{AcceptedMutation, AcceptedReset, CheckAbort, guard_targets, rejected},
    projection::scope_snapshot,
    validation::rejection,
};

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
        if command.registry_generation != self.generation
            || command.scope_id != self.definition.scope_id
        {
            return Ok(SettingsLoadOutcome::Rejected {
                rejection: rejection(SettingsRejectionCode::RegistryChanged),
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
        if let Some(result) = self.reject_apply_envelope(command) {
            return Ok(result);
        }
        let durability = self.durability_material()?;
        let mut authority = self.authority.lock()?;
        let mutation = store.mutate_checked(&self.domain, options, |context| {
            let previous = self
                .ready_snapshot(context.value(), context.diagnostics(), &mut authority)
                .map_err(CheckAbort::Operational)?;
            if command.authority != previous.authority {
                return Err(CheckAbort::Result(Box::new(
                    SettingsMutationResult::Conflict {
                        conflict: SettingsConflict {
                            expected: command.authority.clone(),
                            actual: previous.authority.clone(),
                        },
                        snapshot: previous,
                    },
                )));
            }

            let intent = self
                .adapter
                .decode_intent(&command.intent)
                .map_err(|rejection| rejected(rejection, previous.clone()))?;
            let targets = self.adapter.targeted_entries(&intent);
            guard_targets(previous.values.as_slice(), &targets)
                .map_err(|rejection| rejected(rejection, previous.clone()))?;
            let projection = self
                .adapter
                .project(context.value())
                .map_err(SettingsConfigError::Projection)
                .map_err(CheckAbort::Operational)?;
            self.adapter
                .validate_intent(context.value(), &intent, &projection)
                .map_err(|rejection| rejected(rejection, previous.clone()))?;
            self.adapter
                .patch(context.value_mut(), &intent)
                .map_err(|rejection| rejected(rejection, previous.clone()))?;
            let committed = self
                .project_committed(context.value())
                .map_err(CheckAbort::Operational)?;
            Ok(AcceptedMutation {
                previous,
                committed,
                intent,
            })
        });

        let outcome = match mutation {
            Ok(outcome) => outcome,
            Err(CheckedMutationError::Check(CheckAbort::Result(result))) => return Ok(*result),
            Err(CheckedMutationError::Check(CheckAbort::Operational(error))) => return Err(error),
            Err(CheckedMutationError::Mutation(error)) => {
                return self.map_mutation_error(error, &mut authority);
            }
        };
        let (accepted, _value, publication) = outcome.into_parts();
        let mutation_outcome = if publication.is_some() {
            SettingsMutationOutcome::Changed
        } else {
            SettingsMutationOutcome::Unchanged
        };
        let activation = self.adapter.activation_after_commit(
            SettingsCommittedMutation::Apply(&accepted.intent),
            self.definition.timing,
            mutation_outcome,
            &accepted.committed,
        );
        let committed_token = accepted.committed.token.clone();
        let committed_projection = accepted.committed.with_activation(activation.clone());
        let committed_authority = authority.observe(committed_token)?;
        let snapshot = scope_snapshot(
            &self.definition.scope_id,
            committed_authority.clone(),
            committed_projection,
        );
        Ok(SettingsMutationResult::Applied {
            snapshot,
            receipt: SettingsMutationReceipt {
                request_id: command.request_id.clone(),
                page_id: command.page_id.clone(),
                apply_unit_id: command.apply_unit_id.clone(),
                scope_id: command.scope_id.clone(),
                previous_authority: accepted.previous.authority,
                committed_authority,
                outcome: mutation_outcome,
                durability: publication
                    .as_ref()
                    .map(|receipt| durability.for_kind(receipt.durability))
                    .unwrap_or(SettingsDurabilityEvidence::NotApplicable),
                activation_requirements: activation,
            },
        })
    }

    /// Resets only named overrides through the bound one-domain authority.
    pub fn reset(
        &self,
        store: &ConfigStore,
        command: &SettingsResetCommand,
        options: MutationOptions,
    ) -> Result<SettingsMutationResult, SettingsConfigError> {
        if let Some(result) = self.reject_reset_envelope(command) {
            return Ok(result);
        }
        let durability = self.durability_material()?;
        let mut authority = self.authority.lock()?;
        let mutation = store.mutate_checked(&self.domain, options, |context| {
            let previous = self
                .ready_snapshot(context.value(), context.diagnostics(), &mut authority)
                .map_err(CheckAbort::Operational)?;
            if command.authority != previous.authority {
                return Err(CheckAbort::Result(Box::new(
                    SettingsMutationResult::Conflict {
                        conflict: SettingsConflict {
                            expected: command.authority.clone(),
                            actual: previous.authority.clone(),
                        },
                        snapshot: previous,
                    },
                )));
            }
            guard_targets(previous.values.as_slice(), &command.entry_ids)
                .map_err(|rejection| rejected(rejection, previous.clone()))?;
            self.adapter
                .reset(context.value_mut(), &command.entry_ids)
                .map_err(|rejection| rejected(rejection, previous.clone()))?;
            let committed = self
                .project_committed(context.value())
                .map_err(CheckAbort::Operational)?;
            Ok(AcceptedReset {
                previous,
                committed,
            })
        });

        let outcome = match mutation {
            Ok(outcome) => outcome,
            Err(CheckedMutationError::Check(CheckAbort::Result(result))) => return Ok(*result),
            Err(CheckedMutationError::Check(CheckAbort::Operational(error))) => return Err(error),
            Err(CheckedMutationError::Mutation(error)) => {
                return self.map_mutation_error(error, &mut authority);
            }
        };
        let (accepted, _value, publication) = outcome.into_parts();
        let mutation_outcome = if publication.is_some() {
            SettingsMutationOutcome::Changed
        } else {
            SettingsMutationOutcome::Unchanged
        };
        let activation = self.adapter.activation_after_commit(
            SettingsCommittedMutation::Reset(&command.entry_ids),
            self.definition.timing,
            mutation_outcome,
            &accepted.committed,
        );
        let committed_token = accepted.committed.token.clone();
        let committed_projection = accepted.committed.with_activation(activation.clone());
        let committed_authority = authority.observe(committed_token)?;
        let snapshot = scope_snapshot(
            &self.definition.scope_id,
            committed_authority.clone(),
            committed_projection,
        );
        Ok(SettingsMutationResult::Applied {
            snapshot,
            receipt: SettingsMutationReceipt {
                request_id: command.request_id.clone(),
                page_id: command.page_id.clone(),
                apply_unit_id: command.apply_unit_id.clone(),
                scope_id: command.scope_id.clone(),
                previous_authority: accepted.previous.authority,
                committed_authority,
                outcome: mutation_outcome,
                durability: publication
                    .as_ref()
                    .map(|receipt| durability.for_kind(receipt.durability))
                    .unwrap_or(SettingsDurabilityEvidence::NotApplicable),
                activation_requirements: activation,
            },
        })
    }
}
