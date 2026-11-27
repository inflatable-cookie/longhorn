use std::{cell::Cell, error::Error, fs, time::Duration};

use longhorn_command::{
    CommandKeyChord, CommandKeymapOverride, CommandPlatform, CommandRegistryGeneration,
    CommandReservedChordPolicy, NoReservedCommandChords,
};
use longhorn_command_config::{
    CommandKeymapBackupPolicy, CommandKeymapCommit, CommandKeymapDiagnostic,
    CommandKeymapLoadOrigin, CommandKeymapLoadOutcome, CommandKeymapMutationOutcome,
    CommandKeymapMutationResult, CommandKeymapPatch, CommandKeymapPreview,
    CommandKeymapPreviewResult, CommandKeymapProtocolVersion, CommandKeymapRecoveryCode,
    CommandKeymapRejectionCode, CommandKeymapReset, CommandKeymapService,
    CommandKeymapServiceError, NoCommandKeymapMigration, RegisteredCommandKeymapDomain,
};
use longhorn_config::{
    DomainDescriptor, DomainFilePath, DomainLocation, LoadDiagnostic, LoadDiagnosticCode,
    StorageClass, StoreError, UnavailableState,
};
use longhorn_core::{DomainId, SchemaVersion};
use longhorn_storage_layout::RootKind;
use serde_json::json;

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
    assert_eq!(snapshot.diagnostics.len(), 1);
    assert_eq!(snapshot.diagnostics[0].code, "missing");

    let diagnostic = CommandKeymapDiagnostic::from(&LoadDiagnostic {
        code: LoadDiagnosticCode::Missing,
        message: "keymap file absent".to_owned(),
    });
    assert_eq!(diagnostic.code, "missing");
    assert_eq!(diagnostic.detail, "keymap file absent");
}

#[test]
fn file_load_compiles_overrides_once_and_rechecks_on_the_next_load() {
    #[derive(Default)]
    struct ObserveOverride {
        checks: Cell<u32>,
        reject: Cell<bool>,
    }

    impl CommandReservedChordPolicy for ObserveOverride {
        fn is_reserved(&self, platform: CommandPlatform, chord: &CommandKeyChord) -> bool {
            if platform == CommandPlatform::MacOs && chord.code.as_str() == "KeyJ" {
                self.checks.set(self.checks.get() + 1);
                return self.reject.get();
            }
            false
        }
    }

    let domain = RegisteredCommandKeymapDomain::new(
        descriptor(1),
        registry(1),
        vec![preset(
            "app:default",
            1,
            vec![binding("base:open", "KeyO", "app:open")],
        )],
        super::support::id("app:default"),
        ObserveOverride::default(),
        NoCommandKeymapMigration,
        CommandKeymapBackupPolicy::Include,
    )
    .unwrap();
    let fixture = Fixture::new();
    let store = fixture.store(&domain);
    let mut state = domain.default_state().clone();
    state.overrides.push(CommandKeymapOverride::Add {
        binding: binding("override:open", "KeyJ", "app:open"),
    });
    fixture.write(&domain, &envelope(1, serde_json::to_value(state).unwrap()));
    let service = CommandKeymapService::new(domain);

    let CommandKeymapLoadOutcome::Loaded { snapshot } =
        service.load(&store, Duration::from_secs(2)).unwrap()
    else {
        panic!("expected the compiled keymap");
    };
    assert_eq!(snapshot.origin, CommandKeymapLoadOrigin::File);
    assert_eq!(snapshot.bindings.len(), 2);
    assert_eq!(service.domain().reserved_policy().checks.get(), 1);

    service.domain().reserved_policy().reject.set(true);
    let CommandKeymapLoadOutcome::Recovery { recovery } =
        service.load(&store, Duration::from_secs(2)).unwrap()
    else {
        panic!("expected the changed reservation to block the load");
    };
    assert_eq!(recovery.code, CommandKeymapRecoveryCode::InvalidValue);
    assert!(recovery.detail.contains("command-keymap-reserved-chord"));
    assert!(recovery.source_preserved);
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
                protocol_version: CommandKeymapProtocolVersion::CURRENT,
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
                protocol_version: CommandKeymapProtocolVersion::CURRENT,
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
                protocol_version: CommandKeymapProtocolVersion::CURRENT,
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
                protocol_version: CommandKeymapProtocolVersion::CURRENT,
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
    assert!(
        rejection
            .detail
            .starts_with("command-keymap-unknown-command:")
    );

    let unknown_preset = CommandKeymapPatch {
        active_preset_id: Some(super::support::id("app:missing")),
        ..empty_patch()
    };
    let CommandKeymapPreviewResult::Rejected { rejection, .. } =
        preview(&service, &store, unknown_preset)
    else {
        panic!("unknown preset must reject");
    };
    assert_eq!(rejection.code, CommandKeymapRejectionCode::InvalidKeymap);
    assert!(
        rejection
            .detail
            .starts_with("command-keymap-unknown-preset:")
    );

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
    assert!(rejection.detail.starts_with("command-keymap-conflict:"));
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
                protocol_version: CommandKeymapProtocolVersion::CURRENT,
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
                protocol_version: CommandKeymapProtocolVersion::CURRENT,
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
    assert!(
        rejection
            .detail
            .starts_with("command-keymap-reserved-chord:")
    );
    assert!(!path.exists());
}

#[test]
fn protocol_version_rejects_unsupported_versions_and_versions_requests() {
    assert_eq!(
        serde_json::from_str::<CommandKeymapProtocolVersion>("1").unwrap(),
        CommandKeymapProtocolVersion::CURRENT
    );
    assert!(serde_json::from_str::<CommandKeymapProtocolVersion>("0").is_err());
    assert!(serde_json::from_str::<CommandKeymapProtocolVersion>("2").is_err());

    let preview_payload = json!({
        "protocolVersion": 2,
        "registryGeneration": 1,
        "keymapRevision": 0,
        "activePresetId": "app:default",
        "activePresetVersion": 1,
        "patch": empty_patch(),
    });
    assert!(serde_json::from_value::<CommandKeymapPreview>(preview_payload).is_err());

    let commit_payload = json!({
        "protocolVersion": 2,
        "requestId": "request:1",
        "evidence": {
            "registryGeneration": 1,
            "keymapRevision": 0,
            "activePresetId": "app:default",
            "activePresetVersion": 1,
            "patchDigest": "0".repeat(64),
        },
        "patch": empty_patch(),
    });
    assert!(serde_json::from_value::<CommandKeymapCommit>(commit_payload).is_err());

    let reset_payload = json!({
        "protocolVersion": 2,
        "requestId": "request:1",
        "registryGeneration": 1,
        "keymapRevision": 0,
        "activePresetId": "app:default",
        "activePresetVersion": 1,
    });
    assert!(serde_json::from_value::<CommandKeymapReset>(reset_payload).is_err());
}

#[test]
fn unavailable_and_recovery_preview_return_typed_errors_and_sources() {
    let project_domain = RegisteredCommandKeymapDomain::new(
        DomainDescriptor::new(
            DomainId::new("commands.keymap").unwrap(),
            SchemaVersion::new(1).unwrap(),
            StorageClass::ProjectShared,
            Some(DomainFilePath::new("commands/keymap.json").unwrap()),
        )
        .unwrap(),
        registry(1),
        vec![preset(
            "app:default",
            1,
            vec![binding("base:open", "KeyO", "app:open")],
        )],
        super::support::id("app:default"),
        NoReservedCommandChords,
        NoCommandKeymapMigration,
        CommandKeymapBackupPolicy::Include,
    )
    .unwrap();
    let fixture = Fixture::new();
    let store = fixture.store(&project_domain);
    let service = CommandKeymapService::new(project_domain);

    let outcome = service.load(&store, Duration::from_secs(2)).unwrap();
    assert_eq!(
        outcome,
        CommandKeymapLoadOutcome::Unavailable {
            detail: "authority-unavailable".to_owned(),
        }
    );

    let err = service
        .preview(
            &store,
            &CommandKeymapPreview {
                protocol_version: CommandKeymapProtocolVersion::CURRENT,
                registry_generation: CommandRegistryGeneration::new(1),
                keymap_revision: Default::default(),
                active_preset_id: super::support::id("app:default"),
                active_preset_version: SchemaVersion::new(1).unwrap(),
                patch: empty_patch(),
            },
            Duration::from_secs(2),
        )
        .unwrap_err();
    assert_eq!(
        err,
        CommandKeymapServiceError::Unavailable(UnavailableState::Authority {
            location: DomainLocation::RootRequired {
                root: RootKind::Project,
                relative_path: DomainFilePath::new("commands/keymap.json").unwrap(),
            },
        })
    );
    assert_eq!(err.to_string(), "authority-unavailable");
    assert!(err.source().is_none());

    let user_domain = domain();
    let user_fixture = Fixture::new();
    let user_store = user_fixture.store(&user_domain);
    user_fixture.write(&user_domain, b"{broken");
    let user_service = CommandKeymapService::new(user_domain);
    let recovery_err = user_service
        .preview(
            &user_store,
            &CommandKeymapPreview {
                protocol_version: CommandKeymapProtocolVersion::CURRENT,
                registry_generation: CommandRegistryGeneration::new(1),
                keymap_revision: Default::default(),
                active_preset_id: super::support::id("app:default"),
                active_preset_version: SchemaVersion::new(1).unwrap(),
                patch: empty_patch(),
            },
            Duration::from_secs(2),
        )
        .unwrap_err();
    let CommandKeymapServiceError::Recovery(recovery) = &recovery_err else {
        panic!("expected typed recovery error");
    };
    assert_eq!(recovery.code, CommandKeymapRecoveryCode::Corrupt);
    assert!(recovery.source_preserved);
    assert!(recovery_err.source().is_none());

    let store_err = CommandKeymapServiceError::Store(StoreError::NotRegistered {
        id: DomainId::new("commands.keymap").unwrap(),
    });
    assert!(store_err.source().is_some());
    assert!(
        CommandKeymapServiceError::InvalidState("bad".into())
            .source()
            .is_none()
    );
    assert!(
        CommandKeymapServiceError::Encoding("bad".into())
            .source()
            .is_none()
    );
}
