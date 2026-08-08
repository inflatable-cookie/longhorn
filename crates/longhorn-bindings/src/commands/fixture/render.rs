use std::error::Error;

use longhorn_command::{
    CommandContextRevision, CommandContextSnapshot, CommandEffectiveKeymap, CommandKeyboardMode,
    CommandKeymapOverride, CommandPlatform, CommandSurface, NoReservedCommandChords,
};
use longhorn_command_config::{
    CommandCatalogueChangedEvent, CommandCatalogueSnapshot, CommandKeymapChangedEvent,
    CommandKeymapCommit, CommandKeymapCommitEvidence, CommandKeymapDiagnostic,
    CommandKeymapDurability, CommandKeymapLoadOrigin, CommandKeymapLoadOutcome,
    CommandKeymapMutationOutcome, CommandKeymapMutationReceipt, CommandKeymapMutationResult,
    CommandKeymapPatch, CommandKeymapPresetRecord, CommandKeymapPreview,
    CommandKeymapPreviewResult, CommandKeymapProtocolVersion, CommandKeymapRecovery,
    CommandKeymapRecoveryCode, CommandKeymapRejection, CommandKeymapRejectionCode,
    CommandKeymapReset, CommandKeymapRevision, CommandKeymapSnapshot, CommandKeymapState,
};
use longhorn_core::{CommandContextId, CommandId, SchemaVersion};
use serde_json::{Value, json, to_value};

use super::*;

pub fn render() -> Result<String, Box<dyn Error>> {
    let (registry, preset) = authority()?;
    let state = CommandKeymapState::initial(preset.id.clone());
    let effective =
        CommandEffectiveKeymap::compile(&registry, &preset, &[], &NoReservedCommandChords)?;
    let snapshot = CommandKeymapSnapshot {
        protocol_version: CommandKeymapProtocolVersion::CURRENT,
        registry_generation: registry.generation(),
        registry_digest: registry.digest().clone(),
        state: state.clone(),
        active_preset_version: preset.version,
        bindings: effective.bindings().cloned().collect(),
        conflicts: Vec::new(),
        origin: CommandKeymapLoadOrigin::Default,
        diagnostics: Vec::new(),
    };
    let context = CommandContextSnapshot::new(
        CommandContextRevision::new(7),
        vec![id::<CommandContextId>("global")],
    )?;
    let mut conflict_preset = preset.clone();
    conflict_preset
        .bindings
        .push(binding("base:conflict", "KeyO", "app:save"));
    let conflict_keymap = CommandEffectiveKeymap::compile(
        &registry,
        &conflict_preset,
        &[],
        &NoReservedCommandChords,
    )?;
    let keyboard_cases = vec![
        keyboard_case(
            &effective,
            CommandPlatform::MacOs,
            keyboard_input("KeyO", false, false, false, false, false, true),
            &context,
            CommandKeyboardMode::Dispatch,
        )?,
        keyboard_case(
            &effective,
            CommandPlatform::Windows,
            keyboard_input("KeyS", true, false, false, false, false, false),
            &context,
            CommandKeyboardMode::Dispatch,
        )?,
        keyboard_case(
            &effective,
            CommandPlatform::Windows,
            keyboard_input("KeyS", false, true, false, false, false, false),
            &context,
            CommandKeyboardMode::Dispatch,
        )?,
        keyboard_case_with_reserved(
            &effective,
            CommandPlatform::MacOs,
            keyboard_input("KeyQ", false, false, false, false, false, true),
            &context,
            CommandKeyboardMode::Dispatch,
            &FixtureReservedChords,
        )?,
        keyboard_case(
            &effective,
            CommandPlatform::MacOs,
            keyboard_input("KeyO", false, false, true, false, false, true),
            &context,
            CommandKeyboardMode::Dispatch,
        )?,
        keyboard_case(
            &conflict_keymap,
            CommandPlatform::MacOs,
            keyboard_input("KeyO", false, false, false, false, false, true),
            &context,
            CommandKeyboardMode::Dispatch,
        )?,
        keyboard_case(
            &effective,
            CommandPlatform::Linux,
            keyboard_input("KeyK", false, false, false, true, false, false),
            &context,
            CommandKeyboardMode::Capture,
        )?,
        keyboard_case(
            &effective,
            CommandPlatform::Linux,
            keyboard_input("KeyZ", false, false, false, true, false, false),
            &context,
            CommandKeyboardMode::Dispatch,
        )?,
    ];
    let search_cases = ["", "open", "save", "missing"]
        .into_iter()
        .map(|query| {
            Ok(json!({
                "query": query,
                "hits": registry.search(CommandSurface::Palette, query)?,
            }))
        })
        .collect::<Result<Vec<Value>, Box<dyn Error>>>()?;
    let shortcut_cases = CommandPlatform::ALL
        .into_iter()
        .map(|platform| {
            json!({
                "platform": platform,
                "commandId": "app:open",
                "shortcuts": effective.shortcuts_for_command(
                    &id::<CommandId>("app:open"),
                    platform,
                ),
            })
        })
        .collect::<Vec<_>>();
    let patch = CommandKeymapPatch {
        active_preset_id: None,
        clear_overrides: false,
        remove_binding_ids: Vec::new(),
        upsert_overrides: vec![CommandKeymapOverride::Disable {
            binding_id: binding_id("base:open"),
        }],
    };
    let candidate_state = CommandKeymapState {
        revision: CommandKeymapRevision::new(1),
        active_preset_id: preset.id.clone(),
        overrides: patch.upsert_overrides.clone(),
    };
    let candidate_effective = CommandEffectiveKeymap::compile(
        &registry,
        &preset,
        &candidate_state.overrides,
        &NoReservedCommandChords,
    )?;
    let candidate_snapshot = CommandKeymapSnapshot {
        state: candidate_state,
        bindings: candidate_effective.bindings().cloned().collect(),
        origin: CommandKeymapLoadOrigin::File,
        ..snapshot.clone()
    };
    let evidence = CommandKeymapCommitEvidence {
        registry_generation: registry.generation(),
        keymap_revision: state.revision,
        active_preset_id: preset.id.clone(),
        active_preset_version: preset.version,
        patch_digest: patch.digest()?,
    };
    let rejection = CommandKeymapRejection {
        code: CommandKeymapRejectionCode::Conflict,
        detail: "fixture conflict".into(),
    };
    let receipt = CommandKeymapMutationReceipt {
        request_id: request_id("request:fixture"),
        previous_revision: CommandKeymapRevision::INITIAL,
        committed_revision: CommandKeymapRevision::new(1),
        outcome: CommandKeymapMutationOutcome::Changed,
        durability: CommandKeymapDurability::FileSynced,
        patch_digest: Some(evidence.patch_digest.clone()),
    };
    let fixture = json!({
        "protocolVersion": 1,
        "catalogue": to_value(CommandCatalogueSnapshot {
            protocol_version: CommandKeymapProtocolVersion::CURRENT,
            registry_generation: registry.generation(),
            registry_digest: registry.digest().clone(),
            commands: registry.commands().map(Into::into).collect(),
            presets: vec![CommandKeymapPresetRecord {
                id: preset.id.clone(),
                version: preset.version,
            }],
        })?,
        "requests": {
            "preview": to_value(CommandKeymapPreview {
                registry_generation: registry.generation(),
                keymap_revision: state.revision,
                active_preset_id: preset.id.clone(),
                active_preset_version: preset.version,
                patch: patch.clone(),
            })?,
            "commit": to_value(CommandKeymapCommit {
                request_id: request_id("request:fixture"),
                evidence: evidence.clone(),
                patch: patch.clone(),
            })?,
            "reset": to_value(CommandKeymapReset {
                request_id: request_id("request:reset"),
                registry_generation: registry.generation(),
                keymap_revision: state.revision,
                active_preset_id: preset.id.clone(),
                active_preset_version: preset.version,
            })?,
        },
        "events": {
            "catalogueChanged": to_value(CommandCatalogueChangedEvent {
                protocol_version: CommandKeymapProtocolVersion::CURRENT,
                registry_generation: registry.generation(),
            })?,
            "keymapChanged": to_value(CommandKeymapChangedEvent {
                protocol_version: CommandKeymapProtocolVersion::CURRENT,
                registry_generation: registry.generation(),
                keymap_revision: CommandKeymapRevision::new(1),
            })?,
        },
        "snapshots": [
            to_value(&snapshot)?,
            to_value(CommandKeymapSnapshot {
                origin: CommandKeymapLoadOrigin::File,
                ..snapshot.clone()
            })?,
            to_value(CommandKeymapSnapshot {
                origin: CommandKeymapLoadOrigin::Migrated {
                    from: SchemaVersion::new(1)?,
                    to: SchemaVersion::new(2)?,
                },
                diagnostics: vec![CommandKeymapDiagnostic {
                    code: "migrated".into(),
                    detail: "fixture migration".into(),
                }],
                ..snapshot.clone()
            })?,
        ],
        "previewResults": [
            to_value(CommandKeymapPreviewResult::Accepted {
                evidence: evidence.clone(),
                snapshot: candidate_snapshot.clone(),
            })?,
            to_value(CommandKeymapPreviewResult::Stale {
                snapshot: snapshot.clone(),
            })?,
            to_value(CommandKeymapPreviewResult::Rejected {
                rejection: rejection.clone(),
                snapshot: snapshot.clone(),
                conflicts: Vec::new(),
            })?,
        ],
        "loadOutcomes": [
            to_value(CommandKeymapLoadOutcome::Loaded {
                snapshot: snapshot.clone(),
            })?,
            to_value(CommandKeymapLoadOutcome::Recovery {
                recovery: CommandKeymapRecovery {
                    code: CommandKeymapRecoveryCode::Corrupt,
                    detail: "fixture corrupt source".into(),
                    source_preserved: true,
                },
            })?,
            to_value(CommandKeymapLoadOutcome::Unavailable {
                detail: "fixture unavailable".into(),
            })?,
        ],
        "mutationResults": [
            to_value(CommandKeymapMutationResult::Applied {
                snapshot: candidate_snapshot,
                receipt,
            })?,
            to_value(CommandKeymapMutationResult::Stale {
                snapshot: snapshot.clone(),
            })?,
            to_value(CommandKeymapMutationResult::Rejected {
                rejection,
                snapshot,
                conflicts: Vec::new(),
            })?,
        ],
        "semantics": {
            "context": to_value(&context)?,
            "search": search_cases,
            "shortcuts": shortcut_cases,
            "keyboard": keyboard_cases,
        },
        "incompatibility": {
            "futureProtocolVersion": 2,
            "unknownLoadStatus": "futureLoad",
            "unknownPreviewStatus": "futurePreview",
            "unknownMutationStatus": "futureMutation",
            "unknownOverrideKind": "futureOverride",
            "unknownRejectionCode": "futureRejection",
            "unknownRecoveryCode": "futureRecovery",
            "unknownDurability": "futureDurability",
            "unknownMutationOutcome": "futureOutcome",
            "unknownLoadOrigin": "futureOrigin",
            "unknownBindingSource": "futureSource",
        }
    });
    let mut rendered = serde_json::to_string_pretty(&fixture)?;
    rendered.push('\n');
    Ok(rendered)
}
