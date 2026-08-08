//! Internal load, snapshot, and proposal helpers.

use longhorn_command::{CommandEffectiveKeymap, CommandReservedChordPolicy};
use longhorn_config::LoadOutcome;

use crate::{
    CommandKeymapCommitEvidence, CommandKeymapDiagnostic, CommandKeymapLoadOrigin,
    CommandKeymapLoadOutcome, CommandKeymapMigration, CommandKeymapPatch, CommandKeymapRecovery,
    CommandKeymapRejectionCode, CommandKeymapSnapshot, CommandKeymapState,
};

use super::{
    CommandKeymapService, CommandKeymapServiceError, Proposal, apply_patch, rejection,
    snapshot_from_effective, validate_patch,
};

impl<P, M> CommandKeymapService<P, M>
where
    P: CommandReservedChordPolicy,
    M: CommandKeymapMigration<P>,
{
    pub(crate) fn project_load(
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

    pub(crate) fn ready_load(
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

    pub(crate) fn snapshot(
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

    pub(crate) fn propose(
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

    pub(crate) fn matches_evidence(
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

    pub(crate) fn matches_base(
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
