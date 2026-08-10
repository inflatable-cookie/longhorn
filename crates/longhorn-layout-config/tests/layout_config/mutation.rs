use std::{
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

use longhorn_config::{
    ConfigDomain, ConfigStore, DomainDescriptor, DomainFilePath, DomainIssue, LoadOutcome,
    MigrationStep, MutationError, StorageClass,
};
use longhorn_core::{DomainId, SchemaVersion};
use longhorn_layout_config::{
    LayoutConfigMutationError, NoLayoutMigration, RegisteredLayoutDomain, publish_layout_mutation,
};
use longhorn_surfaces::LayoutMutationRejectionCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::support::{
    Fixture, activate_request, collapse_request, document, domain, options, registry,
    sizing_request,
};

#[test]
fn unregistered_domain_fails_before_publication() {
    let fixture = Fixture::new();
    let domain = domain();
    let error = publish_layout_mutation(
        &fixture.store(),
        &domain,
        options(Duration::from_secs(1)),
        &activate_request(7),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        LayoutConfigMutationError::Config(MutationError::Store(_))
    ));
    assert!(!fixture.path(&domain).exists());
}

#[test]
fn immediate_mutation_publishes_complete_fresh_document() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let receipt = publish_layout_mutation(
        &store,
        &domain,
        options(Duration::from_secs(1)),
        &sizing_request(7, 320_000),
    )
    .unwrap();
    assert_eq!(receipt.layout().committed_revision().get(), 8);
    assert_eq!(
        receipt.publication().domain,
        domain.descriptor().id().clone()
    );
    assert_eq!(receipt.publication().path, fixture.path(&domain));

    let LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
        panic!("published layout should load");
    };
    assert_eq!(loaded.value.revision().get(), 8);
    assert_eq!(
        loaded.value.containers()[0].sizing_slots()[0]
            .ratio()
            .millionths(),
        320_000
    );
}

#[test]
fn stale_rejection_preserves_exact_published_bytes() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    publish_layout_mutation(
        &store,
        &domain,
        options(Duration::from_secs(1)),
        &sizing_request(7, 300_000),
    )
    .unwrap();
    let before = std::fs::read(fixture.path(&domain)).unwrap();

    let error = publish_layout_mutation(
        &store,
        &domain,
        options(Duration::from_secs(1)),
        &collapse_request(7, true),
    )
    .unwrap_err();
    let LayoutConfigMutationError::Rejected(rejection) = error else {
        panic!("stale expected revision should be a layout rejection");
    };
    assert_eq!(rejection.code(), LayoutMutationRejectionCode::StaleRevision);
    assert_eq!(rejection.current_revision().get(), 8);
    assert_eq!(std::fs::read(fixture.path(&domain)).unwrap(), before);
}

#[test]
fn two_stores_admit_only_one_same_revision_mutation() {
    let fixture = Fixture::new();
    let domain = Arc::new(domain());
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for request in [sizing_request(7, 310_000), collapse_request(7, true)] {
        let domain = Arc::clone(&domain);
        let barrier = Arc::clone(&barrier);
        let roots = fixture.roots();
        let coordination = fixture.coordination();
        handles.push(thread::spawn(move || {
            let mut store = ConfigStore::new(roots, coordination);
            store.register(domain.as_ref()).unwrap();
            barrier.wait();
            publish_layout_mutation(
                &store,
                domain.as_ref(),
                options(Duration::from_secs(2)),
                &request,
            )
        }));
    }
    barrier.wait();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                Err(LayoutConfigMutationError::Rejected(rejection))
                    if rejection.code() == LayoutMutationRejectionCode::StaleRevision
            ))
            .count(),
        1
    );
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct WindowState {
    width: u32,
}

#[derive(Clone, Debug)]
struct WindowDomain {
    descriptor: DomainDescriptor,
}

impl WindowDomain {
    fn new() -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new("window.geometry").unwrap(),
                SchemaVersion::new(1).unwrap(),
                StorageClass::MachineState,
                Some(DomainFilePath::new("windows/geometry.json").unwrap()),
            )
            .unwrap(),
        }
    }
}

impl ConfigDomain for WindowDomain {
    type Value = WindowState;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        WindowState { width: 1200 }
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        serde_json::from_value(value)
            .map_err(|error| DomainIssue::new("window-decode", error.to_string()))
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        serde_json::to_value(value)
            .map_err(|error| DomainIssue::new("window-encode", error.to_string()))
    }

    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        (value.width > 0)
            .then_some(())
            .ok_or_else(|| DomainIssue::new("window-width", "width must be positive"))
    }

    fn validate_raw(
        &self,
        _schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        self.decode(value.clone())
            .and_then(|decoded| self.validate(&decoded))
    }

    fn migrate_one(
        &self,
        _from: SchemaVersion,
        _value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        Ok(None)
    }
}

#[test]
fn independent_layout_and_window_domains_cannot_replace_each_other() {
    let fixture = Fixture::new();
    let layout = Arc::new(
        RegisteredLayoutDomain::new(
            crate::support::descriptor(1),
            document(),
            registry(),
            NoLayoutMigration,
            longhorn_layout_config::LayoutBackupPolicy::Include,
        )
        .unwrap(),
    );
    let window = Arc::new(WindowDomain::new());
    let barrier = Arc::new(Barrier::new(3));

    let layout_handle = {
        let layout = Arc::clone(&layout);
        let barrier = Arc::clone(&barrier);
        let roots = fixture.roots();
        let coordination = fixture.coordination();
        thread::spawn(move || {
            let mut store = ConfigStore::new(roots, coordination);
            store.register(layout.as_ref()).unwrap();
            barrier.wait();
            publish_layout_mutation(
                &store,
                layout.as_ref(),
                options(Duration::from_secs(2)),
                &sizing_request(7, 330_000),
            )
            .unwrap();
        })
    };
    let window_handle = {
        let window = Arc::clone(&window);
        let barrier = Arc::clone(&barrier);
        let roots = fixture.roots();
        let coordination = fixture.coordination();
        thread::spawn(move || {
            let mut store = ConfigStore::new(roots, coordination);
            store.register(window.as_ref()).unwrap();
            barrier.wait();
            store
                .mutate(window.as_ref(), options(Duration::from_secs(2)), |value| {
                    value.width = 1440;
                    Ok(())
                })
                .unwrap();
        })
    };
    barrier.wait();
    layout_handle.join().unwrap();
    window_handle.join().unwrap();

    let mut store = fixture.store();
    store.register(layout.as_ref()).unwrap();
    store.register(window.as_ref()).unwrap();
    let LoadOutcome::Ready(layout_state) = store.load(layout.as_ref()).unwrap() else {
        panic!("layout state should load");
    };
    let LoadOutcome::Ready(window_state) = store.load(window.as_ref()).unwrap() else {
        panic!("window state should load");
    };
    assert_eq!(layout_state.value.revision().get(), 8);
    assert_eq!(window_state.value.width, 1440);
    assert_ne!(fixture.path(layout.as_ref()), fixture.path(window.as_ref()));
}
