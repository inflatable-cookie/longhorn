use longhorn_config::{
    ConfigDomain, DomainDescriptor, DomainFilePath, DomainIssue, LoadOutcome, MigrationStep,
    StorageClass,
};
use longhorn_core::{DomainId, LayoutSchemaId, RegionFamilyId, RegionId, SchemaVersion};
use longhorn_layout::{
    EmptyRegionPolicy, LayoutContainer, LayoutDefinitionRegistry, LayoutDocument, LayoutLimits,
    LayoutSchemaDefinition, RegionDefinition, RegionState,
};
use longhorn_layout_config::{LayoutBackupPolicy, NoLayoutMigration, RegisteredLayoutDomain};
use longhorn_surfaces::EmptyWindowPolicy;
use longhorn_surfaces_config::publish_surface_mutation;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::support::{
    Fixture, container_id, document, domain, options, rename_request, surface_id,
};

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
fn surface_layout_and_window_domains_publish_independently() {
    let fixture = Fixture::new();
    let surfaces = domain();
    let layout = layout_domain();
    let window = WindowDomain::new();
    let mut store = fixture.store();
    store.register(&surfaces).unwrap();
    store.register(&layout).unwrap();
    store.register(&window).unwrap();

    store
        .mutate(&layout, options(), |_document| Ok(()))
        .unwrap();
    store
        .mutate(&window, options(), |state| {
            state.width = 1440;
            Ok(())
        })
        .unwrap();
    publish_surface_mutation(
        &store,
        &surfaces,
        options(),
        &layout_document(),
        EmptyWindowPolicy::Allow,
        &rename_request(7, "Independent"),
    )
    .unwrap();

    let LoadOutcome::Ready(surface_state) = store.load(&surfaces).unwrap() else {
        panic!("Surface state should load");
    };
    let LoadOutcome::Ready(layout_state) = store.load(&layout).unwrap() else {
        panic!("layout state should load");
    };
    let LoadOutcome::Ready(window_state) = store.load(&window).unwrap() else {
        panic!("window state should load");
    };
    assert_eq!(
        surface_state
            .value
            .surface(&surface_id("surface:a"))
            .unwrap()
            .label(),
        Some("Independent")
    );
    assert_eq!(layout_state.value.revision().get(), 3);
    assert_eq!(window_state.value.width, 1440);
    assert_ne!(fixture.path(&surfaces), fixture.path(&layout));
    assert_ne!(fixture.path(&surfaces), fixture.path(&window));
    assert_ne!(fixture.path(&layout), fixture.path(&window));
    assert_eq!(document().revision().get(), 7);
}

fn layout_domain() -> RegisteredLayoutDomain<NoLayoutMigration> {
    RegisteredLayoutDomain::new(
        DomainDescriptor::new(
            DomainId::new("layout.workspace").unwrap(),
            SchemaVersion::new(1).unwrap(),
            StorageClass::MachineState,
            Some(DomainFilePath::new("workspace/layout.json").unwrap()),
        )
        .unwrap(),
        layout_document(),
        layout_registry(),
        NoLayoutMigration,
        LayoutBackupPolicy::Include,
    )
    .unwrap()
}

fn layout_registry() -> LayoutDefinitionRegistry {
    LayoutDefinitionRegistry::new(
        LayoutLimits::new(2, 2, 2, 2, 8, 8, 8).unwrap(),
        [LayoutSchemaDefinition::new(
            LayoutSchemaId::new("schema:test").unwrap(),
            [RegionDefinition::new(
                RegionId::new("region:main").unwrap(),
                RegionFamilyId::new("family:main").unwrap(),
                0,
                EmptyRegionPolicy::KeepVisible,
                false,
            )],
            [],
        )],
        [],
    )
    .unwrap()
}

fn layout_document() -> LayoutDocument {
    LayoutDocument::new(
        longhorn_core::LayoutRevision::new(3),
        ["container:a", "container:b", "container:c"]
            .into_iter()
            .map(|id| {
                LayoutContainer::new(
                    container_id(id),
                    LayoutSchemaId::new("schema:test").unwrap(),
                    [RegionState::new(
                        RegionId::new("region:main").unwrap(),
                        [],
                        None,
                        None,
                    )],
                    [],
                )
            }),
        [],
    )
}
