use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    time::Duration,
};

use longhorn_command::{
    CommandDiscoveryRecord, CommandEffectiveKeymap, CommandKeymapConflict,
    CommandReservedChordPolicy,
};
use longhorn_config::{
    CheckedMutationError, ConfigStore, CoordinatedLoadError, LoadOutcome, MutationError,
    MutationOptions, StoreError,
};

use crate::{
    CommandKeymapCommit, CommandKeymapCommitEvidence, CommandKeymapDiagnostic,
    CommandKeymapDurability, CommandKeymapLoadOrigin, CommandKeymapLoadOutcome,
    CommandKeymapMigration, CommandKeymapMutationOutcome, CommandKeymapMutationReceipt,
    CommandKeymapMutationResult, CommandKeymapPatch, CommandKeymapPresetRecord,
    CommandKeymapPreview, CommandKeymapPreviewResult, CommandKeymapProtocolVersion,
    CommandKeymapRecovery, CommandKeymapRejection, CommandKeymapRejectionCode, CommandKeymapReset,
    CommandKeymapSnapshot, CommandKeymapState, RegisteredCommandKeymapDomain,
    protocol::override_binding_id,
};

/// Coordinated command keymap load, preview, commit, and reset authority.
pub struct CommandKeymapService<P, M> {
    domain: RegisteredCommandKeymapDomain<P, M>,
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

    fn project_load(
        &self,
        loaded: LoadOutcome<CommandKeymapState>,
    ) -> Result<CommandKeymapLoadOutcome, CommandKeymapServiceError> {
        match loaded {
            LoadOutcome::Ready(loaded) => Ok(CommandKeymapLoadOutcome::Loaded {
                snapshot: self.snapshot(
                    &loaded.value,
                    loaded.origin.into(),
                    &loaded
                        .diagnostics
                        .iter()
                        .map(CommandKeymapDiagnostic::from)
                        .collect::<Vec<_>>(),
                )?,
            }),
            LoadOutcome::Recovery(recovery) => Ok(CommandKeymapLoadOutcome::Recovery {
                recovery: CommandKeymapRecovery {
                    code: recovery.kind.into(),
                    detail: recovery.detail,
                    source_preserved: recovery.source.is_some(),
                },
            }),
            LoadOutcome::Unavailable(unavailable) => Ok(CommandKeymapLoadOutcome::Unavailable {
                detail: format!("{unavailable:?}"),
            }),
        }
    }

    fn ready_load(
        &self,
        loaded: LoadOutcome<CommandKeymapState>,
    ) -> Result<
        (
            CommandKeymapState,
            CommandKeymapLoadOrigin,
            Vec<CommandKeymapDiagnostic>,
        ),
        CommandKeymapServiceError,
    > {
        match loaded {
            LoadOutcome::Ready(loaded) => Ok((
                loaded.value,
                loaded.origin.into(),
                loaded
                    .diagnostics
                    .iter()
                    .map(CommandKeymapDiagnostic::from)
                    .collect(),
            )),
            LoadOutcome::Recovery(recovery) => {
                Err(CommandKeymapServiceError::Recovery(recovery.detail))
            }
            LoadOutcome::Unavailable(unavailable) => Err(CommandKeymapServiceError::Unavailable(
                format!("{unavailable:?}"),
            )),
        }
    }

    fn snapshot(
        &self,
        state: &CommandKeymapState,
        origin: CommandKeymapLoadOrigin,
        diagnostics: &[CommandKeymapDiagnostic],
    ) -> Result<CommandKeymapSnapshot, CommandKeymapServiceError> {
        let preset = self
            .domain
            .preset(&state.active_preset_id)
            .ok_or_else(|| CommandKeymapServiceError::InvalidState("unknown preset".into()))?;
        let effective = CommandEffectiveKeymap::compile(
            self.domain.registry(),
            preset,
            &state.overrides,
            self.domain.reserved_policy(),
        )
        .map_err(|error| CommandKeymapServiceError::InvalidState(error.to_string()))?;
        Ok(snapshot_from_effective(
            &self.domain,
            state.clone(),
            preset.version,
            effective,
            origin,
            diagnostics.to_vec(),
        ))
    }

    fn propose(
        &self,
        current: &CommandKeymapState,
        patch: &CommandKeymapPatch,
        origin: CommandKeymapLoadOrigin,
        diagnostics: &[CommandKeymapDiagnostic],
    ) -> Result<Proposal, CommandKeymapServiceError> {
        let patch = match validate_patch(patch) {
            Ok(patch) => patch,
            Err(detail) => {
                return Ok(Proposal::Rejected {
                    rejection: rejection(CommandKeymapRejectionCode::InvalidPatch, detail),
                    conflicts: Vec::new(),
                });
            }
        };
        let mut candidate = current.clone();
        apply_patch(&mut candidate, &patch);
        let changed = candidate.active_preset_id != current.active_preset_id
            || candidate.overrides != current.overrides;
        candidate.revision = if changed {
            match current.revision.checked_next() {
                Some(revision) => revision,
                None => {
                    return Ok(Proposal::Rejected {
                        rejection: rejection(
                            CommandKeymapRejectionCode::RevisionOverflow,
                            "command keymap revision cannot advance",
                        ),
                        conflicts: Vec::new(),
                    });
                }
            }
        } else {
            current.revision
        };

        let Some(preset) = self.domain.preset(&candidate.active_preset_id) else {
            return Ok(Proposal::Rejected {
                rejection: rejection(
                    CommandKeymapRejectionCode::InvalidKeymap,
                    format!("unknown active preset {}", candidate.active_preset_id),
                ),
                conflicts: Vec::new(),
            });
        };
        let effective = match CommandEffectiveKeymap::compile(
            self.domain.registry(),
            preset,
            &candidate.overrides,
            self.domain.reserved_policy(),
        ) {
            Ok(effective) => effective,
            Err(error) => {
                return Ok(Proposal::Rejected {
                    rejection: rejection(
                        CommandKeymapRejectionCode::InvalidKeymap,
                        error.to_string(),
                    ),
                    conflicts: Vec::new(),
                });
            }
        };
        let conflicts = effective.conflicts().cloned().collect::<Vec<_>>();
        if !conflicts.is_empty() {
            return Ok(Proposal::Rejected {
                rejection: rejection(
                    CommandKeymapRejectionCode::Conflict,
                    "proposed keymap contains unresolved conflicts",
                ),
                conflicts,
            });
        }
        let snapshot = snapshot_from_effective(
            &self.domain,
            candidate.clone(),
            preset.version,
            effective,
            origin,
            diagnostics.to_vec(),
        );
        Ok(Proposal::Accepted {
            state: candidate,
            snapshot,
        })
    }

    fn matches_evidence(
        &self,
        evidence: &CommandKeymapCommitEvidence,
        state: &CommandKeymapState,
    ) -> bool {
        self.matches_base(
            evidence.registry_generation,
            evidence.keymap_revision,
            &evidence.active_preset_id,
            evidence.active_preset_version,
            state,
        )
    }

    fn matches_base(
        &self,
        registry_generation: longhorn_command::CommandRegistryGeneration,
        keymap_revision: crate::CommandKeymapRevision,
        active_preset_id: &longhorn_core::CommandKeymapPresetId,
        active_preset_version: longhorn_core::SchemaVersion,
        state: &CommandKeymapState,
    ) -> bool {
        registry_generation == self.domain.registry().generation()
            && keymap_revision == state.revision
            && active_preset_id == &state.active_preset_id
            && self
                .domain
                .preset(&state.active_preset_id)
                .is_some_and(|preset| preset.version == active_preset_version)
    }
}

fn snapshot_from_effective<P, M>(
    domain: &RegisteredCommandKeymapDomain<P, M>,
    state: CommandKeymapState,
    preset_version: longhorn_core::SchemaVersion,
    effective: CommandEffectiveKeymap,
    origin: CommandKeymapLoadOrigin,
    diagnostics: Vec<CommandKeymapDiagnostic>,
) -> CommandKeymapSnapshot
where
    P: CommandReservedChordPolicy,
    M: CommandKeymapMigration<P>,
{
    CommandKeymapSnapshot {
        protocol_version: CommandKeymapProtocolVersion::CURRENT,
        registry_generation: domain.registry().generation(),
        registry_digest: domain.registry().digest().clone(),
        state,
        active_preset_version: preset_version,
        bindings: effective.bindings().cloned().collect(),
        conflicts: effective.conflicts().cloned().collect(),
        origin,
        diagnostics,
    }
}

fn validate_patch(patch: &CommandKeymapPatch) -> Result<CommandKeymapPatch, String> {
    let canonical = patch.canonical();
    let removes = canonical.remove_binding_ids.iter().collect::<BTreeSet<_>>();
    if removes.len() != canonical.remove_binding_ids.len() {
        return Err("patch contains duplicate remove binding ids".into());
    }
    let mut upserts = BTreeSet::new();
    for directive in &canonical.upsert_overrides {
        let binding_id = override_binding_id(directive);
        if !upserts.insert(binding_id.clone()) {
            return Err(format!(
                "patch contains duplicate upsert binding id {binding_id}"
            ));
        }
        if removes.contains(&binding_id) {
            return Err(format!(
                "patch both removes and upserts binding id {binding_id}"
            ));
        }
    }
    Ok(canonical)
}

fn apply_patch(state: &mut CommandKeymapState, patch: &CommandKeymapPatch) {
    if let Some(preset_id) = &patch.active_preset_id {
        state.active_preset_id = preset_id.clone();
    }
    let mut overrides = if patch.clear_overrides {
        BTreeMap::new()
    } else {
        state
            .overrides
            .iter()
            .cloned()
            .map(|directive| (override_binding_id(&directive), directive))
            .collect()
    };
    for binding_id in &patch.remove_binding_ids {
        overrides.remove(binding_id);
    }
    for directive in &patch.upsert_overrides {
        overrides.insert(override_binding_id(directive), directive.clone());
    }
    state.overrides = overrides.into_values().collect();
}

fn rejection(
    code: CommandKeymapRejectionCode,
    detail: impl Into<String>,
) -> CommandKeymapRejection {
    CommandKeymapRejection {
        code,
        detail: detail.into(),
    }
}

enum Proposal {
    Accepted {
        state: CommandKeymapState,
        snapshot: CommandKeymapSnapshot,
    },
    Rejected {
        rejection: CommandKeymapRejection,
        conflicts: Vec<CommandKeymapConflict>,
    },
}

struct AcceptedCommit {
    previous_revision: crate::CommandKeymapRevision,
    origin: CommandKeymapLoadOrigin,
    diagnostics: Vec<CommandKeymapDiagnostic>,
}

enum CommitAbort {
    Result(Box<CommandKeymapMutationResult>),
    Operational(CommandKeymapServiceError),
}

/// Operational failure outside normal stale or rejected domain results.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandKeymapServiceError {
    /// Coordinated read authority failed.
    Coordination(CoordinatedLoadError),
    /// Domain registration changed or is missing.
    Store(StoreError),
    /// Coordinated mutation or publication failed.
    Mutation(MutationError),
    /// Source is in explicit recovery.
    Recovery(String),
    /// Required storage authority is unavailable.
    Unavailable(String),
    /// Current supposedly-valid state could not project.
    InvalidState(String),
    /// Canonical patch encoding failed.
    Encoding(String),
}

impl fmt::Display for CommandKeymapServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coordination(error) => error.fmt(formatter),
            Self::Store(error) => error.fmt(formatter),
            Self::Mutation(error) => error.fmt(formatter),
            Self::Recovery(detail)
            | Self::Unavailable(detail)
            | Self::InvalidState(detail)
            | Self::Encoding(detail) => formatter.write_str(detail),
        }
    }
}

impl Error for CommandKeymapServiceError {}
