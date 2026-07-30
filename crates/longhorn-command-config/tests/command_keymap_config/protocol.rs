use std::{fs, time::Duration};

use longhorn_command::{
    CommandKeyChord, CommandKeymapOverride, CommandPlatform, CommandRegistryGeneration,
    CommandReservedChordPolicy,
};
use longhorn_command_config::{
    CommandKeymapBackupPolicy, CommandKeymapCommit, CommandKeymapLoadOrigin,
    CommandKeymapLoadOutcome, CommandKeymapMutationOutcome, CommandKeymapMutationResult,
    CommandKeymapPatch, CommandKeymapPreview, CommandKeymapPreviewResult,
    CommandKeymapRecoveryCode, CommandKeymapRejectionCode, CommandKeymapService,
    NoCommandKeymapMigration, RegisteredCommandKeymapDomain,
};
use longhorn_core::SchemaVersion;

use super::support::{
    Fixture, binding, binding_id, descriptor, domain, empty_patch, envelope, options, preset,
    preview, registry, request_id,
};

#[test]
fn default_load_and_catalogue_are_exact() {
    let domain = domain();
    let fixture = Fixture::new();
    let store = fixture.store(&domain);
    let service = CommandKeymapService::new(domain);

    let catalogue = service.catalogue();
    assert_eq!(catalogue.commands.len(), 2);
    assert_eq!(catalogue.presets.len(), 1);
    let CommandKeymapLoadOutcome::Loaded { snapshot } =
        service.load(&store, Duration::from_secs(2)).unwrap()
    else {
        panic!("expected loaded state");
    };
    assert_eq!(snapshot.origin, CommandKeymapLoadOrigin::Default);
    assert_eq!(snapshot.state.revision.get(), 0);
    assert_eq!(snapshot.bindings.len(), 2);
}

#[test]
fn patch_digest_is_order_invariant_and_contradictory_patch_is_rejected() {
    let mut first = CommandKeymapPatch {
        remove_binding_ids: vec![binding_id("base:save"), binding_id("base:open")],
        upsert_overrides: vec![
            CommandKeymapOverride::Add {
                binding: binding("override:second", "KeyK", "app:save"),
            },
            CommandKeymapOverride::Add {
                binding: binding("override:first", "KeyJ", "app:open"),
            },
        ],
        ..empty_patch()
    };
    let mut second = first.clone();
    second.remove_binding_ids.reverse();
    second.upsert_overrides.reverse();
    assert_eq!(first.digest().unwrap(), second.digest().unwrap());

    first.upsert_overrides.push(CommandKeymapOverride::Disable {
        binding_id: binding_id("base:open"),
    });
    let domain = domain();
    let fixture = Fixture::new();
    let store = fixture.store(&domain);
    let path = fixture.path(&domain);
    let service = CommandKeymapService::new(domain);
    let CommandKeymapPreviewResult::Rejected { rejection, .. } = preview(&service, &store, first)
    else {
        panic!("contradictory patch must reject");
    };
    assert_eq!(rejection.code, CommandKeymapRejectionCode::InvalidPatch);
    assert!(!path.exists());
}

#[test]
fn preview_commit_and_repeat_are_bound_by_digest_and_revision() {
    let domain = domain();
    let fixture = Fixture::new();
    let store = fixture.store(&domain);
    let service = CommandKeymapService::new(domain);
    let patch = CommandKeymapPatch {
        upsert_overrides: vec![CommandKeymapOverride::Disable {
            binding_id: binding_id("base:open"),
        }],
        ..empty_patch()
    };
    let CommandKeymapPreviewResult::Accepted { evidence, snapshot } =
        preview(&service, &store, patch.clone())
    else {
        panic!("expected accepted preview");
    };
    assert_eq!(snapshot.state.revision.get(), 1);
    assert_eq!(snapshot.bindings.len(), 1);

    let result = service
        .commit(
            &store,
            &CommandKeymapCommit {
                request_id: request_id("request:first"),
                evidence: evidence.clone(),
                patch: patch.clone(),
            },
            options(),
        )
        .unwrap();
    let CommandKeymapMutationResult::Applied { snapshot, receipt } = result else {
        panic!("expected applied commit");
    };
    assert_eq!(receipt.outcome, CommandKeymapMutationOutcome::Changed);
    assert_eq!(receipt.patch_digest, Some(evidence.patch_digest.clone()));
    assert_eq!(snapshot.state.revision.get(), 1);

    let stale = service
        .commit(
            &store,
            &CommandKeymapCommit {
                request_id: request_id("request:repeat"),
                evidence,
                patch,
            },
            options(),
        )
        .unwrap();
    assert!(matches!(stale, CommandKeymapMutationResult::Stale { .. }));
}

#[test]
fn changed_patch_and_changed_registry_evidence_do_not_publish() {
    let domain = domain();
    let fixture = Fixture::new();
    let store = fixture.store(&domain);
    let path = fixture.path(&domain);
    let service = CommandKeymapService::new(domain);
    let CommandKeymapPreviewResult::Accepted { evidence, .. } =
        preview(&service, &store, empty_patch())
    else {
        panic!("expected accepted preview");
    };
    let different = CommandKeymapPatch {
        upsert_overrides: vec![CommandKeymapOverride::Disable {
            binding_id: binding_id("base:open"),
        }],
        ..empty_patch()
    };
    let changed = service
        .commit(
            &store,
            &CommandKeymapCommit {
                request_id: request_id("request:changed"),
                evidence,
                patch: different,
            },
            options(),
        )
        .unwrap();
    assert!(matches!(changed, CommandKeymapMutationResult::Stale { .. }));
    assert!(!path.exists());

    let current = service.load(&store, Duration::from_secs(2)).unwrap();
    let CommandKeymapLoadOutcome::Loaded { snapshot } = current else {
        panic!("expected current");
    };
    let stale = service
        .preview(
            &store,
            &CommandKeymapPreview {
                registry_generation: CommandRegistryGeneration::new(99),
                keymap_revision: snapshot.state.revision,
                active_preset_id: snapshot.state.active_preset_id,
                active_preset_version: snapshot.active_preset_version,
                patch: empty_patch(),
            },
            Duration::from_secs(2),
        )
        .unwrap();
    assert!(matches!(stale, CommandKeymapPreviewResult::Stale { .. }));
    assert!(!path.exists());
}

#[test]
fn invalid_unknown_and_conflicting_overrides_never_publish() {
    let domain = domain();
    let fixture = Fixture::new();
    let store = fixture.store(&domain);
    let path = fixture.path(&domain);
    let service = CommandKeymapService::new(domain);

    let invalid = CommandKeymapPatch {
        upsert_overrides: vec![CommandKeymapOverride::Add {
            binding: binding("override:unknown", "KeyU", "app:missing"),
        }],
        ..empty_patch()
    };
    let CommandKeymapPreviewResult::Rejected { rejection, .. } = preview(&service, &store, invalid)
    else {
        panic!("unknown command must reject");
    };
    assert_eq!(rejection.code, CommandKeymapRejectionCode::InvalidKeymap);

    let conflict = CommandKeymapPatch {
        upsert_overrides: vec![CommandKeymapOverride::Add {
            binding: binding("override:conflict", "KeyO", "app:save"),
        }],
        ..empty_patch()
    };
    let CommandKeymapPreviewResult::Rejected {
        rejection,
        conflicts,
        ..
    } = preview(&service, &store, conflict)
    else {
        panic!("ambiguous keymap must reject");
    };
    assert_eq!(rejection.code, CommandKeymapRejectionCode::Conflict);
    assert!(!conflicts.is_empty());
    assert!(!path.exists());
}

#[test]
fn corrupt_and_future_sources_preserve_recovery_evidence() {
    let domain = domain();
    let fixture = Fixture::new();
    let store = fixture.store(&domain);
    let service = CommandKeymapService::new(domain.clone());

    fixture.write(&domain, b"{broken");
    let CommandKeymapLoadOutcome::Recovery { recovery } =
        service.load(&store, Duration::from_secs(2)).unwrap()
    else {
        panic!("expected corrupt recovery");
    };
    assert_eq!(recovery.code, CommandKeymapRecoveryCode::Corrupt);
    assert!(recovery.source_preserved);

    let value = serde_json::to_value(domain.default_state()).unwrap();
    let future = envelope(2, value);
    fixture.write(&domain, &future);
    let CommandKeymapLoadOutcome::Recovery { recovery } =
        service.load(&store, Duration::from_secs(2)).unwrap()
    else {
        panic!("expected future recovery");
    };
    assert_eq!(recovery.code, CommandKeymapRecoveryCode::FutureSchema);
    assert!(recovery.source_preserved);
    assert_eq!(fs::read(fixture.path(&domain)).unwrap(), future);
}

#[test]
fn stale_preset_version_is_detected() {
    let domain = domain();
    let fixture = Fixture::new();
    let store = fixture.store(&domain);
    let service = CommandKeymapService::new(domain);
    let current = service.load(&store, Duration::from_secs(2)).unwrap();
    let CommandKeymapLoadOutcome::Loaded { snapshot } = current else {
        panic!("expected current");
    };
    let result = service
        .preview(
            &store,
            &CommandKeymapPreview {
                registry_generation: snapshot.registry_generation,
                keymap_revision: snapshot.state.revision,
                active_preset_id: snapshot.state.active_preset_id,
                active_preset_version: SchemaVersion::new(2).unwrap(),
                patch: empty_patch(),
            },
            Duration::from_secs(2),
        )
        .unwrap();
    assert!(matches!(result, CommandKeymapPreviewResult::Stale { .. }));
}

#[derive(Clone, Copy, Debug)]
struct ReservedKeyR;

impl CommandReservedChordPolicy for ReservedKeyR {
    fn is_reserved(&self, platform: CommandPlatform, chord: &CommandKeyChord) -> bool {
        platform == CommandPlatform::MacOs && chord.code.as_str() == "KeyR"
    }
}

#[test]
fn reserved_override_is_rejected_without_publication() {
    let domain = RegisteredCommandKeymapDomain::new(
        descriptor(1),
        registry(1),
        vec![preset(
            "app:default",
            1,
            vec![binding("base:open", "KeyO", "app:open")],
        )],
        super::support::id("app:default"),
        ReservedKeyR,
        NoCommandKeymapMigration,
        CommandKeymapBackupPolicy::Include,
    )
    .unwrap();
    let fixture = Fixture::new();
    let store = fixture.store(&domain);
    let path = fixture.path(&domain);
    let service = CommandKeymapService::new(domain);
    let CommandKeymapLoadOutcome::Loaded { snapshot } =
        service.load(&store, Duration::from_secs(2)).unwrap()
    else {
        panic!("expected default state");
    };
    let result = service
        .preview(
            &store,
            &CommandKeymapPreview {
                registry_generation: snapshot.registry_generation,
                keymap_revision: snapshot.state.revision,
                active_preset_id: snapshot.state.active_preset_id,
                active_preset_version: snapshot.active_preset_version,
                patch: CommandKeymapPatch {
                    upsert_overrides: vec![CommandKeymapOverride::Add {
                        binding: binding("override:reserved", "KeyR", "app:open"),
                    }],
                    ..empty_patch()
                },
            },
            Duration::from_secs(2),
        )
        .unwrap();
    let CommandKeymapPreviewResult::Rejected { rejection, .. } = result else {
        panic!("reserved override must reject");
    };
    assert_eq!(rejection.code, CommandKeymapRejectionCode::InvalidKeymap);
    assert!(!path.exists());
}
