//! Public command keymap service API.

use std::time::Duration;

use longhorn_command::{CommandDiscoveryRecord, CommandReservedChordPolicy};
use longhorn_config::{CheckedMutationError, ConfigStore, MutationOptions};

use crate::{
    CommandKeymapCommit, CommandKeymapCommitEvidence, CommandKeymapDiagnostic,
    CommandKeymapDurability, CommandKeymapLoadOrigin, CommandKeymapLoadOutcome,
    CommandKeymapMigration, CommandKeymapMutationOutcome, CommandKeymapMutationReceipt,
    CommandKeymapMutationResult, CommandKeymapPresetRecord, CommandKeymapPreview,
    CommandKeymapPreviewResult, CommandKeymapProtocolVersion, CommandKeymapRejectionCode,
    CommandKeymapReset, CommandKeymapSnapshot, RegisteredCommandKeymapDomain,
};

use super::{AcceptedCommit, CommandKeymapServiceError, CommitAbort, Proposal, rejection};

/// Coordinated command keymap load, preview, commit, and reset authority.
pub struct CommandKeymapService<P, M> {
    pub(crate) domain: RegisteredCommandKeymapDomain<P, M>,
}

impl<P, M> CommandKeymapService<P, M>
where
    P: CommandReservedChordPolicy,
    M: CommandKeymapMigration<P>,
{
    /// Binds one registered domain.
    #[must_use]
    pub const fn new(domain: RegisteredCommandKeymapDomain<P, M>) -> Self {
        Self { domain }
    }

    /// Returns the registered domain.
    #[must_use]
    pub const fn domain(&self) -> &RegisteredCommandKeymapDomain<P, M> {
        &self.domain
    }

    /// Projects the sealed command registry and immutable preset catalogue.
    #[must_use]
    pub fn catalogue(&self) -> crate::CommandCatalogueSnapshot {
        crate::CommandCatalogueSnapshot {
            protocol_version: CommandKeymapProtocolVersion::CURRENT,
            registry_generation: self.domain.registry().generation(),
            registry_digest: self.domain.registry().digest().clone(),
            commands: self
                .domain
                .registry()
                .commands()
                .map(CommandDiscoveryRecord::from)
                .collect(),
            presets: self
                .domain
                .presets()
                .iter()
                .map(|preset| CommandKeymapPresetRecord {
                    id: preset.id.clone(),
                    version: preset.version,
                })
                .collect(),
        }
    }

    /// Loads one authoritative state under the config coordinator.
    pub fn load(
        &self,
        store: &ConfigStore,
        lock_timeout: Duration,
    ) -> Result<CommandKeymapLoadOutcome, CommandKeymapServiceError> {
        let loaded = store
            .with_coordinated_load_set(lock_timeout, |set| set.load(&self.domain))
            .map_err(CommandKeymapServiceError::Coordination)?
            .map_err(CommandKeymapServiceError::Store)?;
        self.project_load(loaded)
    }

    /// Previews one patch against exact current coordinated state.
    pub fn preview(
        &self,
        store: &ConfigStore,
        request: &CommandKeymapPreview,
        lock_timeout: Duration,
    ) -> Result<CommandKeymapPreviewResult, CommandKeymapServiceError> {
        store
            .with_coordinated_load_set(lock_timeout, |set| {
                let loaded = set
                    .load(&self.domain)
                    .map_err(CommandKeymapServiceError::Store)?;
                let (state, origin, diagnostics) = self.ready_load(loaded)?;
                let current = self.snapshot(&state, origin, &diagnostics)?;
                if !self.matches_base(
                    request.registry_generation,
                    request.keymap_revision,
                    &request.active_preset_id,
                    request.active_preset_version,
                    &state,
                ) {
                    return Ok(CommandKeymapPreviewResult::Stale { snapshot: current });
                }

                let patch_digest = request
                    .patch
                    .digest()
                    .map_err(|error| CommandKeymapServiceError::Encoding(error.to_string()))?;
                match self.propose(&state, &request.patch, origin, &diagnostics)? {
                    Proposal::Accepted { state, snapshot } => {
                        Ok(CommandKeymapPreviewResult::Accepted {
                            evidence: CommandKeymapCommitEvidence {
                                registry_generation: self.domain.registry().generation(),
                                keymap_revision: request.keymap_revision,
                                active_preset_id: request.active_preset_id.clone(),
                                active_preset_version: request.active_preset_version,
                                patch_digest,
                            },
                            snapshot: CommandKeymapSnapshot { state, ..snapshot },
                        })
                    }
                    Proposal::Rejected {
                        rejection,
                        conflicts,
                    } => Ok(CommandKeymapPreviewResult::Rejected {
                        rejection,
                        snapshot: current,
                        conflicts,
                    }),
                }
            })
            .map_err(CommandKeymapServiceError::Coordination)?
    }

    /// Commits exactly one previously previewed patch under fresh coordination.
    pub fn commit(
        &self,
        store: &ConfigStore,
        request: &CommandKeymapCommit,
        options: MutationOptions,
    ) -> Result<CommandKeymapMutationResult, CommandKeymapServiceError> {
        let digest = request
            .patch
            .digest()
            .map_err(|error| CommandKeymapServiceError::Encoding(error.to_string()))?;
        let mutation = store.mutate_checked(&self.domain, options, |context| {
            let origin = CommandKeymapLoadOrigin::from(context.origin());
            let diagnostics = context
                .diagnostics()
                .iter()
                .map(CommandKeymapDiagnostic::from)
                .collect::<Vec<_>>();
            let current = self
                .snapshot(context.value(), origin, &diagnostics)
                .map_err(CommitAbort::Operational)?;
            if !self.matches_evidence(&request.evidence, context.value()) {
                return Err(CommitAbort::Result(Box::new(
                    CommandKeymapMutationResult::Stale { snapshot: current },
                )));
            }
            if digest != request.evidence.patch_digest {
                return Err(CommitAbort::Result(Box::new(
                    CommandKeymapMutationResult::Stale { snapshot: current },
                )));
            }

            match self
                .propose(context.value(), &request.patch, origin, &diagnostics)
                .map_err(CommitAbort::Operational)?
            {
                Proposal::Accepted { state, .. } => {
                    let previous_revision = context.value().revision;
                    *context.value_mut() = state;
                    Ok(AcceptedCommit {
                        previous_revision,
                        origin,
                        diagnostics,
                    })
                }
                Proposal::Rejected {
                    rejection,
                    conflicts,
                } => Err(CommitAbort::Result(Box::new(
                    CommandKeymapMutationResult::Rejected {
                        rejection,
                        snapshot: current,
                        conflicts,
                    },
                ))),
            }
        });

        let outcome = match mutation {
            Ok(outcome) => outcome,
            Err(CheckedMutationError::Check(CommitAbort::Result(result))) => return Ok(*result),
            Err(CheckedMutationError::Check(CommitAbort::Operational(error))) => return Err(error),
            Err(CheckedMutationError::Mutation(error)) => {
                return Err(CommandKeymapServiceError::Mutation(error));
            }
        };
        let (accepted, state, publication) = outcome.into_parts();
        let (origin, diagnostics) = if publication.is_some() {
            (CommandKeymapLoadOrigin::File, Vec::new())
        } else {
            (accepted.origin, accepted.diagnostics)
        };
        let snapshot = self.snapshot(&state, origin, &diagnostics)?;
        Ok(CommandKeymapMutationResult::Applied {
            receipt: CommandKeymapMutationReceipt {
                request_id: request.request_id.clone(),
                previous_revision: accepted.previous_revision,
                committed_revision: state.revision,
                outcome: if publication.is_some() {
                    CommandKeymapMutationOutcome::Changed
                } else {
                    CommandKeymapMutationOutcome::Unchanged
                },
                durability: publication
                    .map(|receipt| CommandKeymapDurability::from(receipt.durability))
                    .unwrap_or(CommandKeymapDurability::NotApplicable),
                patch_digest: Some(digest),
            },
            snapshot,
        })
    }

    /// Resets to the compiled default under exact current evidence.
    pub fn reset(
        &self,
        store: &ConfigStore,
        request: &CommandKeymapReset,
        options: MutationOptions,
    ) -> Result<CommandKeymapMutationResult, CommandKeymapServiceError> {
        let mutation = store.mutate_checked(&self.domain, options, |context| {
            let origin = CommandKeymapLoadOrigin::from(context.origin());
            let diagnostics = context
                .diagnostics()
                .iter()
                .map(CommandKeymapDiagnostic::from)
                .collect::<Vec<_>>();
            let current = self
                .snapshot(context.value(), origin, &diagnostics)
                .map_err(CommitAbort::Operational)?;
            if !self.matches_base(
                request.registry_generation,
                request.keymap_revision,
                &request.active_preset_id,
                request.active_preset_version,
                context.value(),
            ) {
                return Err(CommitAbort::Result(Box::new(
                    CommandKeymapMutationResult::Stale { snapshot: current },
                )));
            }
            let previous_revision = context.value().revision;
            let mut reset = self.domain.default_state().clone();
            if reset.active_preset_id != context.value().active_preset_id
                || !context.value().overrides.is_empty()
            {
                reset.revision = context.value().revision.checked_next().ok_or_else(|| {
                    CommitAbort::Result(Box::new(CommandKeymapMutationResult::Rejected {
                        rejection: rejection(
                            CommandKeymapRejectionCode::RevisionOverflow,
                            "command keymap revision cannot advance",
                        ),
                        snapshot: current.clone(),
                        conflicts: Vec::new(),
                    }))
                })?;
            } else {
                reset.revision = context.value().revision;
            }
            *context.value_mut() = reset;
            Ok(AcceptedCommit {
                previous_revision,
                origin,
                diagnostics,
            })
        });

        let outcome = match mutation {
            Ok(outcome) => outcome,
            Err(CheckedMutationError::Check(CommitAbort::Result(result))) => return Ok(*result),
            Err(CheckedMutationError::Check(CommitAbort::Operational(error))) => return Err(error),
            Err(CheckedMutationError::Mutation(error)) => {
                return Err(CommandKeymapServiceError::Mutation(error));
            }
        };
        let (accepted, state, publication) = outcome.into_parts();
        let (origin, diagnostics) = if publication.is_some() {
            (CommandKeymapLoadOrigin::File, Vec::new())
        } else {
            (accepted.origin, accepted.diagnostics)
        };
        Ok(CommandKeymapMutationResult::Applied {
            snapshot: self.snapshot(&state, origin, &diagnostics)?,
            receipt: CommandKeymapMutationReceipt {
                request_id: request.request_id.clone(),
                previous_revision: accepted.previous_revision,
                committed_revision: state.revision,
                outcome: if publication.is_some() {
                    CommandKeymapMutationOutcome::Changed
                } else {
                    CommandKeymapMutationOutcome::Unchanged
                },
                durability: publication
                    .map(|receipt| CommandKeymapDurability::from(receipt.durability))
                    .unwrap_or(CommandKeymapDurability::NotApplicable),
                patch_digest: None,
            },
        })
    }
}
