use std::{
    fs,
    sync::{Arc, Barrier},
    time::Duration,
};

use longhorn_command::{CommandKeymapOverride, NoReservedCommandChords};
use longhorn_command_config::{
    CommandKeymapBackupPolicy, CommandKeymapCommit, CommandKeymapLoadOrigin,
    CommandKeymapLoadOutcome, CommandKeymapMigration, CommandKeymapMigrationTarget,
    CommandKeymapMutationResult, CommandKeymapPatch, CommandKeymapPreviewResult,
    CommandKeymapReset, CommandKeymapService, CommandKeymapState, RegisteredCommandKeymapDomain,
};
use longhorn_config::{
    BackupApplication, BackupCaptureOptions, BackupCatalog, BackupKind, BackupLimits,
    BackupMetadata, BackupProducer, BackupScope, DomainIssue, MigrationStep, MutationError,
};
use longhorn_core::SchemaVersion;
use serde_json::Value;

use super::support::{
    Fixture, binding, binding_id, descriptor, domain, empty_patch, envelope, options, preset,
    preview, registry, request_id,
};

#[test]
fn concurrent_commits_from_one_preview_publish_once() {
    let domain = domain();
    let fixture = Fixture::new();
    let preview_store = fixture.store(&domain);
    let preview_service = CommandKeymapService::new(domain.clone());
    let patch = disable("base:open");
    let CommandKeymapPreviewResult::Accepted { evidence, .. } =
        preview(&preview_service, &preview_store, patch.clone())
    else {
        panic!("expected accepted preview");
    };
    let request = CommandKeymapCommit {
        request_id: request_id("request:race"),
        evidence,
        patch,
    };
    let barrier = Arc::new(Barrier::new(3));
    let mut threads = Vec::new();
    for _ in 0..2 {
        let service = CommandKeymapService::new(domain.clone());
        let store = fixture.store(&domain);
        let request = request.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            service.commit(&store, &request, options()).unwrap()
        }));
    }
    barrier.wait();
    let results = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, CommandKeymapMutationResult::Applied { .. }))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, CommandKeymapMutationResult::Stale { .. }))
            .count(),
        1
    );
}

#[cfg(unix)]
#[test]
fn publication_failure_preserves_published_bytes_and_effective_state() {
    use std::os::unix::fs::PermissionsExt;

    let domain = domain();
    let fixture = Fixture::new();
    let store = fixture.store(&domain);
    let service = CommandKeymapService::new(domain.clone());
    commit_patch(&service, &store, disable("base:open"), "request:first");
    let path = fixture.path(&domain);
    let before = fs::read(&path).unwrap();
    let CommandKeymapPreviewResult::Accepted { evidence, .. } =
        preview(&service, &store, disable("base:save"))
    else {
        panic!("expected accepted second preview");
    };
    let parent = path.parent().unwrap();
    let original_mode = fs::metadata(parent).unwrap().permissions().mode();
    fs::set_permissions(parent, fs::Permissions::from_mode(0o500)).unwrap();
    let result = service.commit(
        &store,
        &CommandKeymapCommit {
            request_id: request_id("request:must-fail"),
            evidence,
            patch: disable("base:save"),
        },
        options(),
    );
    fs::set_permissions(parent, fs::Permissions::from_mode(original_mode)).unwrap();

    assert!(matches!(
        result,
        Err(
            longhorn_command_config::CommandKeymapServiceError::Mutation(
                MutationError::Publication(_)
            )
        )
    ));
    assert_eq!(fs::read(&path).unwrap(), before);
    let CommandKeymapLoadOutcome::Loaded { snapshot } =
        service.load(&store, Duration::from_secs(2)).unwrap()
    else {
        panic!("published state should remain loadable");
    };
    assert_eq!(snapshot.state.revision.get(), 1);
    assert_eq!(snapshot.bindings.len(), 1);
}

#[derive(Clone, Copy, Debug)]
struct V1Migration;

impl CommandKeymapMigration<NoReservedCommandChords> for V1Migration {
    fn validate_raw(
        &self,
        schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if schema_version.get() != 1 {
            return Err(DomainIssue::new("old-keymap-schema", "expected schema 1"));
        }
        serde_json::from_value::<CommandKeymapState>(value.clone())
            .map(|_| ())
            .map_err(|error| DomainIssue::new("old-keymap-shape", error.to_string()))
    }

    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
        target: CommandKeymapMigrationTarget<'_, NoReservedCommandChords>,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        if from.get() != 1 {
            return Ok(None);
        }
        let state = serde_json::from_value(value)
            .map_err(|error| DomainIssue::new("old-keymap-decode", error.to_string()))?;
        Ok(Some(MigrationStep {
            schema_version: target.schema_version(),
            value: target.encode_current(state)?,
        }))
    }
}

#[test]
fn migration_projects_source_versions_and_preserves_exact_bytes() {
    let fixture = Fixture::new();
    let current = RegisteredCommandKeymapDomain::new(
        descriptor(2),
        registry(1),
        vec![preset(
            "app:default",
            1,
            vec![
                binding("base:open", "KeyO", "app:open"),
                binding("base:save", "KeyS", "app:save"),
            ],
        )],
        super::support::id("app:default"),
        NoReservedCommandChords,
        V1Migration,
        CommandKeymapBackupPolicy::Include,
    )
    .unwrap();
    let bytes = envelope(
        1,
        serde_json::to_value(CommandKeymapState::initial(super::support::id(
            "app:default",
        )))
        .unwrap(),
    );
    fixture.write(&current, &bytes);
    let store = fixture.store(&current);
    let service = CommandKeymapService::new(current.clone());

    let CommandKeymapLoadOutcome::Loaded { snapshot } =
        service.load(&store, Duration::from_secs(2)).unwrap()
    else {
        panic!("explicit migration should load");
    };
    assert_eq!(
        snapshot.origin,
        CommandKeymapLoadOrigin::Migrated {
            from: SchemaVersion::new(1).unwrap(),
            to: SchemaVersion::new(2).unwrap(),
        }
    );
    assert_eq!(fs::read(fixture.path(&current)).unwrap(), bytes);
}

#[test]
fn reset_returns_to_compiled_default_and_advances_revision_once() {
    let domain = domain();
    let fixture = Fixture::new();
    let store = fixture.store(&domain);
    let service = CommandKeymapService::new(domain);
    commit_patch(&service, &store, disable("base:open"), "request:first");
    let CommandKeymapLoadOutcome::Loaded { snapshot } =
        service.load(&store, Duration::from_secs(2)).unwrap()
    else {
        panic!("expected published state");
    };
    let result = service
        .reset(
            &store,
            &CommandKeymapReset {
                request_id: request_id("request:reset"),
                registry_generation: snapshot.registry_generation,
                keymap_revision: snapshot.state.revision,
                active_preset_id: snapshot.state.active_preset_id,
                active_preset_version: snapshot.active_preset_version,
            },
            options(),
        )
        .unwrap();
    let CommandKeymapMutationResult::Applied { snapshot, receipt } = result else {
        panic!("expected applied reset");
    };
    assert_eq!(receipt.previous_revision.get(), 1);
    assert_eq!(receipt.committed_revision.get(), 2);
    assert_eq!(snapshot.bindings.len(), 2);
    assert!(snapshot.state.overrides.is_empty());
}

#[test]
fn included_keymap_domain_participates_in_ordinary_backup_capture() {
    let domain = domain();
    let fixture = Fixture::new();
    let store = fixture.store(&domain);
    let service = CommandKeymapService::new(domain.clone());
    commit_patch(&service, &store, disable("base:open"), "request:backup");
    let mut catalog = BackupCatalog::new();
    domain.add_to_backup_catalog(&mut catalog).unwrap();

    let snapshot = store
        .capture_backup(
            &catalog,
            &BackupScope::AllRegistered,
            BackupMetadata::new(
                "command-keymap-fixture",
                BackupKind::Operational,
                "2026-07-30T12:00:00Z",
                BackupApplication::new("test.longhorn", "0.1.0").unwrap(),
                BackupProducer::new("longhorn-command-config", "0.1.0").unwrap(),
            )
            .unwrap(),
            BackupCaptureOptions::new(Duration::from_secs(2), BackupLimits::default()),
        )
        .unwrap();
    assert_eq!(snapshot.manifest().domains().len(), 1);
    assert_eq!(
        snapshot.manifest().domains()[0].domain(),
        domain.descriptor().id()
    );
    assert!(snapshot.manifest().exclusions().is_empty());
}

fn disable(binding: &str) -> CommandKeymapPatch {
    CommandKeymapPatch {
        upsert_overrides: vec![CommandKeymapOverride::Disable {
            binding_id: binding_id(binding),
        }],
        ..empty_patch()
    }
}

fn commit_patch(
    service: &super::support::TestService,
    store: &longhorn_config::ConfigStore,
    patch: CommandKeymapPatch,
    request: &str,
) {
    let CommandKeymapPreviewResult::Accepted { evidence, .. } =
        preview(service, store, patch.clone())
    else {
        panic!("expected accepted preview");
    };
    assert!(matches!(
        service
            .commit(
                store,
                &CommandKeymapCommit {
                    request_id: request_id(request),
                    evidence,
                    patch,
                },
                options(),
            )
            .unwrap(),
        CommandKeymapMutationResult::Applied { .. }
    ));
}
