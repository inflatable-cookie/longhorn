use longhorn_config::{
    ConfigDomain, DomainDescriptor, DomainFilePath, DomainIssue, LoadOutcome, MigrationStep,
    StorageClass,
};
use longhorn_core::{
    DomainId, LayoutRequestId, LayoutSchemaId, PanelDefinitionId, PanelInstanceId, RegionFamilyId,
    RegionId, SchemaVersion, SurfaceRevision,
};
use longhorn_surfaces::{
    EmptyRegionPolicy, EmptyWindowPolicy, LayoutDefinitionRegistry, LayoutMutationCommand,
    LayoutMutationRequest, LayoutSchemaDefinition, PanelDefinition, PanelInstancePolicy,
    ParticipatingWindow, PlacementSelector, RegionDefinition, RegionState, SurfaceDocument,
    SurfaceHostPreference, SurfaceMutationCommand, SurfaceMutationRequest, SurfaceRecord,
};
use longhorn_surfaces_config::{
    NoSurfaceMigration, RegisteredSurfaceDomain, SurfaceBackupPolicy, publish_layout_mutation,
    publish_surface_mutation,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::support::{Fixture, descriptor, limits, options, rename_request, surface_id, window_id};

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
fn single_surface_domain_serves_surface_and_layout_mutations_independently_of_window_domain() {
    let fixture = Fixture::new();
    let surfaces = unified_surface_domain();
    let window = WindowDomain::new();
    let mut store = fixture.store();
    store.register(&surfaces).unwrap();
    store.register(&window).unwrap();

    store
        .mutate(&window, options(), |state| {
            state.width = 1440;
            Ok(())
        })
        .unwrap();

    // 1. Surface mutation advances revision 7 -> 8.
    let renamed = publish_surface_mutation(
        &store,
        &surfaces,
        options(),
        EmptyWindowPolicy::Allow,
        &rename_request(7, "Independent"),
    )
    .unwrap();
    assert_eq!(renamed.surface().committed_revision().get(), 8);

    // 2. Layout mutation on the exact same RegisteredSurfaceDomain advances 8 -> 9.
    let panel_id = PanelInstanceId::new("panel:tool:1").unwrap();
    let created = publish_layout_mutation(
        &store,
        &surfaces,
        options(),
        &LayoutMutationRequest::new(
            LayoutRequestId::new("request:create-panel").unwrap(),
            SurfaceRevision::new(8),
            LayoutMutationCommand::CreatePanel {
                panel_instance_id: panel_id.clone(),
                panel_definition_id: PanelDefinitionId::new("panel:tool").unwrap(),
                surface_id: surface_id("surface:a"),
                region_id: RegionId::new("region:main").unwrap(),
                insertion_index: 0,
            },
        ),
    )
    .unwrap();
    assert_eq!(created.layout().committed_revision().get(), 9);

    // 3. Closing surface:a removes its panel instance in the same commit (9 -> 10)
    //    and leaves the persisted document valid for reloading.
    let closed = publish_surface_mutation(
        &store,
        &surfaces,
        options(),
        EmptyWindowPolicy::Allow,
        &SurfaceMutationRequest::new(
            longhorn_core::SurfaceRequestId::new("request:close-surface-a").unwrap(),
            SurfaceRevision::new(9),
            SurfaceMutationCommand::CloseSurface {
                surface_id: surface_id("surface:a"),
            },
        ),
    )
    .unwrap();
    assert_eq!(closed.surface().committed_revision().get(), 10);

    let LoadOutcome::Ready(surface_state) = store.load(&surfaces).unwrap() else {
        panic!("Surface state should load");
    };
    let LoadOutcome::Ready(window_state) = store.load(&window).unwrap() else {
        panic!("window state should load");
    };
    assert_eq!(surface_state.value.revision().get(), 10);
    assert!(
        surface_state
            .value
            .surface(&surface_id("surface:a"))
            .is_none()
    );
    assert!(surface_state.value.panel_instance(&panel_id).is_none());
    assert_eq!(window_state.value.width, 1440);
    assert_ne!(fixture.path(&surfaces), fixture.path(&window));
}

fn unified_surface_domain() -> RegisteredSurfaceDomain<NoSurfaceMigration> {
    let schema_id = LayoutSchemaId::new("schema:test").unwrap();
    let region_id = RegionId::new("region:main").unwrap();
    let registry = LayoutDefinitionRegistry::new(
        limits(),
        [LayoutSchemaDefinition::new(
            schema_id.clone(),
            [RegionDefinition::new(
                region_id.clone(),
                RegionFamilyId::new("family:main").unwrap(),
                0,
                EmptyRegionPolicy::KeepVisible,
                false,
            )],
            [],
        )],
        [PanelDefinition::new(
            PanelDefinitionId::new("panel:tool").unwrap(),
            [PlacementSelector::Region(region_id.clone())],
            [PlacementSelector::Region(region_id.clone())],
            PanelInstancePolicy::Multiple,
            true,
            true,
        )],
    )
    .unwrap();
    let doc = SurfaceDocument::new(
        SurfaceRevision::new(7),
        [
            SurfaceRecord::new(
                surface_id("surface:a"),
                schema_id.clone(),
                Some("A".to_owned()),
                [RegionState::new(region_id.clone(), [], None, None)],
                [],
                [SurfaceHostPreference::new(window_id("window:main"), 0)],
            ),
            SurfaceRecord::new(
                surface_id("surface:b"),
                schema_id,
                Some("B".to_owned()),
                [RegionState::new(region_id, [], None, None)],
                [],
                [SurfaceHostPreference::new(window_id("window:main"), 1)],
            ),
        ],
        [],
        [ParticipatingWindow::new(
            window_id("window:main"),
            Some(surface_id("surface:a")),
        )],
    );
    RegisteredSurfaceDomain::new(
        descriptor(1),
        doc,
        registry,
        NoSurfaceMigration,
        SurfaceBackupPolicy::Include,
    )
    .unwrap()
}
