//! Internal load, snapshot, and proposal helpers.

use longhorn_command::CommandReservedChordPolicy;
use longhorn_config::{LoadOutcome, RecoveryState, UnavailableState};

use crate::{
    CommandKeymapCommitEvidence, CommandKeymapDiagnostic, CommandKeymapLoadOrigin,
    CommandKeymapLoadOutcome, CommandKeymapMigration, CommandKeymapPatch, CommandKeymapRecovery,
    CommandKeymapRejectionCode, CommandKeymapSnapshot, CommandKeymapState,
    domain::compile_keymap_raw,
};

use super::keymap_load::KeymapLoad;
use super::{
    CommandKeymapService, CommandKeymapServiceError, Proposal, apply_patch, rejection,
    snapshot_from_effective, validate_patch,
};

impl<P, M> CommandKeymapService<P, M>
where
    P: CommandReservedChordPolicy,
    M: CommandKeymapMigration<P>,
{
    pub(super) fn project_load(
        &self,
        operation: &KeymapLoad<'_, P, M>,
        loaded: LoadOutcome<CommandKeymapState>,
    ) -> Result<CommandKeymapLoadOutcome, CommandKeymapServiceError> {
        match loaded {
            LoadOutcome::Ready(loaded) => Ok(CommandKeymapLoadOutcome::Loaded {
                snapshot: self.snapshot(
                    operation,
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
                recovery: project_recovery(recovery),
            }),
            LoadOutcome::Unavailable(unavailable) => Ok(CommandKeymapLoadOutcome::Unavailable {
                detail: unavailable_code(&unavailable).into(),
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
            LoadOutcome::Recovery(recovery) => Err(CommandKeymapServiceError::Recovery(
                project_recovery(recovery),
            )),
            LoadOutcome::Unavailable(unavailable) => {
                Err(CommandKeymapServiceError::Unavailable(unavailable))
            }
        }
    }

    pub(super) fn snapshot(
        &self,
        operation: &KeymapLoad<'_, P, M>,
        state: &CommandKeymapState,
        origin: CommandKeymapLoadOrigin,
        diagnostics: &[CommandKeymapDiagnostic],
    ) -> Result<CommandKeymapSnapshot, CommandKeymapServiceError> {
        operation
            .with_effective(state, |effective| {
                snapshot_from_effective(
                    &self.domain,
                    state.clone(),
                    effective,
                    origin,
                    diagnostics.to_vec(),
                )
            })
            .map_err(|issue| CommandKeymapServiceError::InvalidState(issue.message))
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
                Ok(revision) => revision,
                Err(_) => {
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

        let effective = match compile_keymap_raw(
            self.domain.registry(),
            self.domain.preset(&candidate.active_preset_id),
            &candidate,
            self.domain.reserved_policy(),
        ) {
            Ok(effective) => effective,
            Err(issue) => {
                return Ok(Proposal::Rejected {
                    rejection: rejection(
                        CommandKeymapRejectionCode::InvalidKeymap,
                        format!("{}: {}", issue.code, issue.message),
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
                    "command-keymap-conflict: proposed keymap contains unresolved conflicts",
                ),
                conflicts,
            });
        }
        let snapshot = snapshot_from_effective(
            &self.domain,
            candidate.clone(),
            &effective,
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

fn project_recovery(recovery: RecoveryState) -> CommandKeymapRecovery {
    CommandKeymapRecovery {
        code: recovery.kind.into(),
        detail: recovery.detail,
        source_preserved: recovery.source.is_some(),
    }
}

pub(crate) fn unavailable_code(state: &UnavailableState) -> &'static str {
    match state {
        UnavailableState::Authority { .. } => "authority-unavailable",
        UnavailableState::RestoreActive => "restore-active",
        UnavailableState::RestoreRecoveryRequired => "restore-recovery-required",
    }
}

#[cfg(test)]
mod tests {
    use longhorn_command::{
        CommandArgumentSchema, CommandBindingDefinition, CommandContextDefinition,
        CommandDefinition, CommandKeyTrigger, CommandKeymapPreset, CommandLimits,
        CommandPhysicalCode, CommandPlatformScope, CommandRegistryBuilder,
        CommandRegistryGeneration, CommandTextInputPolicy, CommandTriggerModifiers,
        CommandVisibility, NoReservedCommandChords,
    };
    use longhorn_config::{
        DomainDescriptor, DomainFilePath, DomainLocation, LoadOutcome, RecoveryKind, RecoveryState,
        StorageClass, UnavailableState,
    };
    use longhorn_core::{
        CommandCategoryId, CommandContextId, CommandId, CommandRouteId, DomainId, SchemaVersion,
    };
    use serde_json::Value;

    use crate::{
        CommandKeymapBackupPolicy, CommandKeymapLoadOutcome, CommandKeymapRecoveryCode,
        CommandKeymapService, CommandKeymapServiceError, NoCommandKeymapMigration,
        RegisteredCommandKeymapDomain,
    };

    use super::KeymapLoad;

    fn test_service() -> CommandKeymapService<NoReservedCommandChords, NoCommandKeymapMigration> {
        let mut builder = CommandRegistryBuilder::new(
            CommandRegistryGeneration::new(1),
            CommandLimits::default(),
        );
        builder
            .register_context(CommandContextDefinition {
                id: "global".parse().unwrap(),
                parent_id: None,
            })
            .unwrap();
        builder
            .register_command(CommandDefinition {
                id: "app:open".parse().unwrap(),
                label: "Open".to_owned(),
                description: None,
                category_path: vec!["general".parse::<CommandCategoryId>().unwrap()],
                keywords: Vec::new(),
                icon: None,
                allowed_contexts: vec!["global".parse::<CommandContextId>().unwrap()],
                required_capabilities: Vec::new(),
                visibility: CommandVisibility::ALL,
                text_input_policy: CommandTextInputPolicy::Blocked,
                route: "route:app:open".parse::<CommandRouteId>().unwrap(),
                arguments: CommandArgumentSchema::None,
            })
            .unwrap();
        let registry = builder.seal().unwrap();
        let preset = CommandKeymapPreset {
            id: "app:default".parse().unwrap(),
            version: SchemaVersion::new(1).unwrap(),
            bindings: vec![CommandBindingDefinition {
                id: "base:open".parse().unwrap(),
                platform: CommandPlatformScope::Any,
                trigger: CommandKeyTrigger {
                    code: CommandPhysicalCode::new("KeyO").unwrap(),
                    modifiers: CommandTriggerModifiers {
                        primary: true,
                        ..CommandTriggerModifiers::default()
                    },
                },
                context_id: "global".parse().unwrap(),
                command_id: "app:open".parse::<CommandId>().unwrap(),
                arguments: Value::Null,
            }],
        };
        let domain = RegisteredCommandKeymapDomain::new(
            DomainDescriptor::new(
                DomainId::new("commands.keymap").unwrap(),
                SchemaVersion::new(1).unwrap(),
                StorageClass::UserConfig,
                Some(DomainFilePath::new("commands/keymap.json").unwrap()),
            )
            .unwrap(),
            registry,
            vec![preset],
            "app:default".parse().unwrap(),
            NoReservedCommandChords,
            NoCommandKeymapMigration,
            CommandKeymapBackupPolicy::Include,
        )
        .unwrap();
        CommandKeymapService::new(domain)
    }

    #[test]
    fn project_load_and_ready_load_preserve_all_unavailable_codes_and_typed_recovery() {
        let service = test_service();
        let operation = KeymapLoad::new(service.domain());

        let unavailable_cases = [
            (
                UnavailableState::Authority {
                    location: DomainLocation::SecureStoreRequired,
                },
                "authority-unavailable",
            ),
            (UnavailableState::RestoreActive, "restore-active"),
            (
                UnavailableState::RestoreRecoveryRequired,
                "restore-recovery-required",
            ),
        ];

        for (state, expected_code) in unavailable_cases {
            let projected = service
                .project_load(&operation, LoadOutcome::Unavailable(state.clone()))
                .unwrap();
            assert_eq!(
                projected,
                CommandKeymapLoadOutcome::Unavailable {
                    detail: expected_code.to_owned(),
                }
            );
            let error = service
                .ready_load(LoadOutcome::Unavailable(state.clone()))
                .unwrap_err();
            assert_eq!(error, CommandKeymapServiceError::Unavailable(state));
            assert_eq!(error.to_string(), expected_code);
        }

        let recovery_state = RecoveryState {
            kind: RecoveryKind::CorruptDocument,
            path: None,
            source: None,
            detail: "corrupt json".to_owned(),
            position: None,
        };
        let error = service
            .ready_load(LoadOutcome::Recovery(recovery_state))
            .unwrap_err();
        let CommandKeymapServiceError::Recovery(recovery) = &error else {
            panic!("expected typed recovery error");
        };
        assert_eq!(recovery.code, CommandKeymapRecoveryCode::Corrupt);
        assert_eq!(recovery.detail, "corrupt json");
        assert!(!recovery.source_preserved);
        assert_eq!(error.to_string(), "corrupt json");
    }
}
