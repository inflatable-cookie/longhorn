#[path = "backup/adapters.rs"]
mod adapters;
#[path = "backup/archive.rs"]
mod archive;
#[path = "backup/coordination.rs"]
mod coordination;
#[path = "backup/restore.rs"]
mod restore;

use std::{fs, time::Duration};

use longhorn_config::{
    BackupAdapter, BackupAdapterCapabilities, BackupAdapterCapture, BackupAdapterCaptureMode,
    BackupAdapterCaptureRequest, BackupAdapterError, BackupAdapterId, BackupAdapterInspectRequest,
    BackupAdapterRestoreOutcome, BackupAdapterRestoreParticipation, BackupAdapterRestorePreview,
    BackupAdapterRestoreRequest, BackupApplication, BackupCaptureError, BackupCaptureOptions,
    BackupCatalog, BackupExclusionReason, BackupKind, BackupLimits, BackupMetadata, BackupProducer,
    BackupScope, BackupSourceIssue, BackupSourceState, ConfigStore, StorageClass,
};
use longhorn_core::{DomainId, SchemaVersion};
use serde_json::json;

use crate::common::{Fixture, PreferencesDomain, config_domain, document};

struct UnavailableAdapter {
    id: BackupAdapterId,
    capabilities: BackupAdapterCapabilities,
}

impl UnavailableAdapter {
    fn new() -> Self {
        Self {
            id: BackupAdapterId::new("sqlite-native-v1").unwrap(),
            capabilities: BackupAdapterCapabilities::new(
                BackupAdapterCaptureMode::CoordinatedBounded,
                BackupAdapterRestoreParticipation::Separate,
            ),
        }
    }
}

impl BackupAdapter for UnavailableAdapter {
    fn id(&self) -> &BackupAdapterId {
        &self.id
    }

    fn capabilities(&self) -> &BackupAdapterCapabilities {
        &self.capabilities
    }

    fn capture(
        &self,
        _request: BackupAdapterCaptureRequest<'_>,
    ) -> Result<BackupAdapterCapture, BackupAdapterError> {
        Err(BackupAdapterError::Unavailable)
    }

    fn inspect(
        &self,
        _request: BackupAdapterInspectRequest<'_>,
    ) -> Result<BackupAdapterRestorePreview, BackupAdapterError> {
        Err(BackupAdapterError::Unavailable)
    }

    fn restore(
        &self,
        _request: BackupAdapterRestoreRequest<'_>,
    ) -> Result<BackupAdapterRestoreOutcome, BackupAdapterError> {
        Err(BackupAdapterError::Unavailable)
    }
}

pub(super) fn metadata() -> BackupMetadata {
    BackupMetadata::new(
        "archive-2026-07-28",
        BackupKind::Operational,
        "2026-07-28T12:00:00Z",
        BackupApplication::new("com.example.desktop", "1.2.3").unwrap(),
        BackupProducer::new("longhorn-config", "0.1.0").unwrap(),
    )
    .unwrap()
}

pub(super) fn options(limits: BackupLimits) -> BackupCaptureOptions {
    BackupCaptureOptions::new(Duration::from_secs(2), limits)
}

pub(super) fn capture(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    scope: &BackupScope,
) -> Result<longhorn_config::BackupSnapshot, BackupCaptureError> {
    store.capture_backup(catalog, scope, metadata(), options(BackupLimits::default()))
}

#[test]
fn policy_is_complete_unique_and_has_safe_default_exclusions() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let user = config_domain();
    store.register(&user).unwrap();

    let empty = BackupCatalog::new();
    assert!(matches!(
        capture(&store, &empty, &BackupScope::AllRegistered),
        Err(BackupCaptureError::MissingPolicy { domain })
            if domain == DomainId::new("example.preferences").unwrap()
    ));

    let mut duplicate = BackupCatalog::new();
    duplicate.include(&user).unwrap();
    assert!(duplicate.include(&user).is_err());

    let secret = PreferencesDomain::new("z.secret", StorageClass::Secret, None, 3);
    let cache =
        PreferencesDomain::new("a.cache", StorageClass::Cache, Some("backup/cache.json"), 3);
    let runtime = PreferencesDomain::new(
        "m.runtime",
        StorageClass::Runtime,
        Some("backup/runtime.json"),
        3,
    );
    let log = PreferencesDomain::new("n.log", StorageClass::Log, Some("backup/log.json"), 3);
    let mut excluded_store = fixture.store();
    for domain in [&secret, &cache, &runtime, &log] {
        excluded_store.register(domain).unwrap();
    }

    let snapshot = capture(
        &excluded_store,
        &BackupCatalog::new(),
        &BackupScope::AllRegistered,
    )
    .unwrap();
    let ids = snapshot
        .manifest()
        .exclusions()
        .iter()
        .map(|entry| entry.domain().as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["a.cache", "m.runtime", "n.log", "z.secret"]);
    assert!(snapshot.manifest().domains().is_empty());
    assert_eq!(snapshot.receipt().excluded_domains(), 4);
}

#[test]
fn explicit_exclusion_custom_adapter_and_descriptor_drift_are_typed() {
    let fixture = Fixture::new();
    let registered = config_domain();
    let changed = PreferencesDomain::new(
        "example.preferences",
        StorageClass::UserConfig,
        Some("different/preferences.json"),
        3,
    );
    let mut store = fixture.store();
    store.register(&registered).unwrap();

    let mut excluded = BackupCatalog::new();
    excluded
        .exclude(
            &registered,
            BackupExclusionReason::new("user-chose-not-to-back-up").unwrap(),
        )
        .unwrap();
    let snapshot = capture(&store, &excluded, &BackupScope::AllRegistered).unwrap();
    assert_eq!(
        snapshot.manifest().exclusions()[0].reason(),
        "user-chose-not-to-back-up"
    );

    let adapter = UnavailableAdapter::new();
    let mut custom = BackupCatalog::new();
    custom.custom(&registered, &adapter).unwrap();
    assert!(matches!(
        capture(&store, &custom, &BackupScope::AllRegistered),
        Err(BackupCaptureError::AdapterFailed { adapter, .. })
            if adapter == "sqlite-native-v1"
    ));

    let mut drifted = BackupCatalog::new();
    drifted.include(&changed).unwrap();
    assert!(matches!(
        capture(&store, &drifted, &BackupScope::AllRegistered),
        Err(BackupCaptureError::DescriptorChanged { .. })
    ));
}

#[test]
fn manifest_and_payloads_are_stable_and_match_exact_current_source() {
    let fixture = Fixture::new();
    let alpha = PreferencesDomain::new(
        "a.preferences",
        StorageClass::UserConfig,
        Some("backup/a.json"),
        3,
    );
    let zulu = PreferencesDomain::new(
        "z.preferences",
        StorageClass::MachineState,
        Some("backup/z.json"),
        3,
    );
    let alpha_bytes = document(
        "a.preferences",
        3,
        json!({"name": "alpha", "enabled": true}),
    );
    let zulu_bytes = document(
        "z.preferences",
        3,
        json!({"name": "zulu", "enabled": false}),
    );
    fixture.write(&alpha, &alpha_bytes);
    fixture.write(&zulu, &zulu_bytes);

    let mut store = fixture.store();
    store.register(&zulu).unwrap();
    store.register(&alpha).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog.include(&zulu).unwrap();
    catalog.include(&alpha).unwrap();
    let snapshot = capture(&store, &catalog, &BackupScope::AllRegistered).unwrap();

    let domain_ids = snapshot
        .manifest()
        .domains()
        .iter()
        .map(|entry| entry.domain().as_str())
        .collect::<Vec<_>>();
    let payload_ids = snapshot
        .payloads()
        .iter()
        .map(|entry| entry.domain().as_str())
        .collect::<Vec<_>>();
    assert_eq!(domain_ids, ["a.preferences", "z.preferences"]);
    assert_eq!(payload_ids, domain_ids);
    assert_eq!(snapshot.payloads()[0].bytes(), alpha_bytes);
    assert_eq!(snapshot.payloads()[1].bytes(), zulu_bytes);

    let evidence = &snapshot.manifest().domains()[0].payloads()[0];
    assert_eq!(evidence.byte_length(), alpha_bytes.len() as u64);
    assert_eq!(
        evidence.sha256(),
        &longhorn_config::Sha256Digest::from_bytes(&alpha_bytes)
    );
    assert_eq!(
        evidence.path().as_str(),
        "longhorn/domains/a.preferences.json"
    );
    assert_eq!(
        snapshot.manifest().domains()[0].source_schema_version(),
        Some(SchemaVersion::new(3).unwrap())
    );
    assert_eq!(
        snapshot.manifest().consistency_groups()[0].id(),
        "longhorn-config-store"
    );
    assert_eq!(
        snapshot.manifest().consistency_groups()[0].mode(),
        longhorn_config::BackupConsistencyMode::CoordinatedBounded
    );
}

#[test]
fn absent_source_stays_absent_without_materializing_defaults() {
    let fixture = Fixture::new();
    let domain = config_domain();
    let path = fixture.path_for(&domain);
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog.include(&domain).unwrap();

    let snapshot = capture(&store, &catalog, &BackupScope::AllRegistered).unwrap();
    let manifest_domain = &snapshot.manifest().domains()[0];
    assert_eq!(manifest_domain.state(), BackupSourceState::Absent);
    assert_eq!(manifest_domain.source_schema_version(), None);
    assert!(manifest_domain.payloads().is_empty());
    assert!(snapshot.payloads().is_empty());
    assert!(!path.exists());
}

#[test]
fn older_valid_source_is_captured_exactly_without_rewrite() {
    let fixture = Fixture::new();
    let domain = config_domain();
    let bytes = document("example.preferences", 1, json!({"label": "legacy-name"}));
    let path = fixture.write(&domain, &bytes);
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog.include(&domain).unwrap();

    let snapshot = capture(&store, &catalog, &BackupScope::AllRegistered).unwrap();
    let entry = &snapshot.manifest().domains()[0];
    assert_eq!(entry.state(), BackupSourceState::Present);
    assert_eq!(
        entry.source_schema_version(),
        Some(SchemaVersion::new(1).unwrap())
    );
    assert_eq!(snapshot.payloads()[0].bytes(), bytes);
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn future_and_corrupt_readable_sources_are_preserved_non_restorable() {
    let fixture = Fixture::new();
    let future = config_domain();
    let future_bytes = document(
        "example.preferences",
        99,
        json!({"name": "future", "enabled": true}),
    );
    fixture.write(&future, &future_bytes);
    let mut future_store = fixture.store();
    future_store.register(&future).unwrap();
    let mut future_catalog = BackupCatalog::new();
    future_catalog.include(&future).unwrap();
    let future_snapshot =
        capture(&future_store, &future_catalog, &BackupScope::AllRegistered).unwrap();
    let future_entry = &future_snapshot.manifest().domains()[0];
    assert_eq!(future_entry.state(), BackupSourceState::SourcePreserved);
    assert_eq!(
        future_entry.source_issue(),
        Some(BackupSourceIssue::FutureSchema)
    );
    assert_eq!(
        future_entry.source_schema_version(),
        Some(SchemaVersion::new(99).unwrap())
    );
    assert_eq!(future_snapshot.payloads()[0].bytes(), future_bytes);

    let corrupt = PreferencesDomain::new(
        "other.preferences",
        StorageClass::UserConfig,
        Some("backup/corrupt.json"),
        3,
    );
    let corrupt_bytes = b"{ this is readable but invalid json".to_vec();
    fixture.write(&corrupt, &corrupt_bytes);
    let mut corrupt_store = fixture.store();
    corrupt_store.register(&corrupt).unwrap();
    let mut corrupt_catalog = BackupCatalog::new();
    corrupt_catalog.include(&corrupt).unwrap();
    let corrupt_snapshot = capture(
        &corrupt_store,
        &corrupt_catalog,
        &BackupScope::AllRegistered,
    )
    .unwrap();
    let corrupt_entry = &corrupt_snapshot.manifest().domains()[0];
    assert_eq!(corrupt_entry.state(), BackupSourceState::SourcePreserved);
    assert_eq!(
        corrupt_entry.source_issue(),
        Some(BackupSourceIssue::CorruptDocument)
    );
    assert_eq!(corrupt_entry.source_schema_version(), None);
    assert_eq!(corrupt_snapshot.payloads()[0].bytes(), corrupt_bytes);
}

#[test]
fn unreadable_required_source_fails_the_whole_capture() {
    let fixture = Fixture::new();
    let domain = config_domain();
    fs::create_dir_all(fixture.path_for(&domain)).unwrap();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog.include(&domain).unwrap();

    assert!(matches!(
        capture(&store, &catalog, &BackupScope::AllRegistered),
        Err(BackupCaptureError::Unreadable { .. })
    ));
}

#[test]
fn per_domain_and_total_bounds_reject_before_snapshot_return() {
    let fixture = Fixture::new();
    let alpha = PreferencesDomain::new(
        "a.bound",
        StorageClass::UserConfig,
        Some("backup/bound-a.json"),
        3,
    );
    let beta = PreferencesDomain::new(
        "b.bound",
        StorageClass::UserConfig,
        Some("backup/bound-b.json"),
        3,
    );
    let alpha_bytes = document(
        "a.bound",
        3,
        json!({"name": "alpha-bound", "enabled": true}),
    );
    let beta_bytes = document("b.bound", 3, json!({"name": "beta-bound", "enabled": true}));
    fixture.write(&alpha, &alpha_bytes);
    fixture.write(&beta, &beta_bytes);
    let mut store = fixture.store();
    store.register(&alpha).unwrap();
    store.register(&beta).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog.include(&alpha).unwrap();
    catalog.include(&beta).unwrap();

    let per_domain =
        BackupLimits::new(alpha_bytes.len() - 1, alpha_bytes.len() + beta_bytes.len()).unwrap();
    assert!(matches!(
        store.capture_backup(
            &catalog,
            &BackupScope::AllRegistered,
            metadata(),
            options(per_domain)
        ),
        Err(BackupCaptureError::DomainTooLarge { .. })
    ));

    let max_source = alpha_bytes.len().max(beta_bytes.len());
    let total_limit = alpha_bytes.len() + beta_bytes.len() - 1;
    let total = BackupLimits::new(max_source, total_limit).unwrap();
    assert!(matches!(
        store.capture_backup(
            &catalog,
            &BackupScope::AllRegistered,
            metadata(),
            options(total)
        ),
        Err(BackupCaptureError::TotalTooLarge {
            limit,
            observed,
            ..
        }) if limit == total_limit && observed == alpha_bytes.len() + beta_bytes.len()
    ));
}
