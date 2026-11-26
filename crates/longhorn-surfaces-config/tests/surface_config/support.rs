use std::{fs, time::Duration};

use longhorn_config::{
    DomainDescriptor, DomainFilePath, DurabilityRequirement, MutationOptions, StorageClass,
};
use longhorn_core::{
    DomainId, LayoutSchemaId, SchemaVersion, SurfaceId, SurfaceRequestId, SurfaceRevision, WindowId,
};
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutSchemaDefinition, ParticipatingWindow,
    RegionDefinition, RegionState, SurfaceDocument, SurfaceHostPreference, SurfaceLimits,
    SurfaceMutationCommand, SurfaceMutationRequest, SurfaceRecord,
};
use longhorn_surfaces_config::{NoSurfaceMigration, RegisteredSurfaceDomain, SurfaceBackupPolicy};

pub type TestSurfaceDomain = RegisteredSurfaceDomain<NoSurfaceMigration>;

pub use crate::fixture::{Fixture, envelope};

pub fn descriptor(version: u32) -> DomainDescriptor {
    DomainDescriptor::new(
        DomainId::new("surfaces.workspace").unwrap(),
        SchemaVersion::new(version).unwrap(),
        StorageClass::MachineState,
        Some(DomainFilePath::new("workspace/surfaces.json").unwrap()),
    )
    .unwrap()
}

pub fn domain() -> TestSurfaceDomain {
    RegisteredSurfaceDomain::new(
        descriptor(1),
        document(),
        registry(),
        NoSurfaceMigration,
        SurfaceBackupPolicy::Include,
    )
    .unwrap()
}

pub fn limits() -> SurfaceLimits {
    SurfaceLimits::builder()
        .maximum_schemas(8)
        .maximum_regions_per_schema(8)
        .maximum_sizing_slots_per_schema(8)
        .maximum_panel_definitions(64)
        .maximum_surfaces(16)
        .maximum_windows(4)
        .maximum_host_preferences_per_surface(4)
        .maximum_label_bytes(64)
        .maximum_panel_instances(64)
        .maximum_panel_instances_per_region(16)
        .build()
        .unwrap()
}

pub fn document() -> SurfaceDocument {
    SurfaceDocument::new(
        SurfaceRevision::new(7),
        [surface("surface:a", "A", 0), surface("surface:b", "B", 1)],
        [],
        [ParticipatingWindow::new(
            window_id("window:main"),
            Some(surface_id("surface:a")),
        )],
    )
}

pub fn registry() -> LayoutDefinitionRegistry {
    LayoutDefinitionRegistry::new(
        limits(),
        [LayoutSchemaDefinition::new(
            LayoutSchemaId::new("schema:test").unwrap(),
            [RegionDefinition::new(
                longhorn_core::RegionId::new("region:main").expect("region id is valid"),
                longhorn_core::RegionFamilyId::new("family:main").expect("family id is valid"),
                0,
                EmptyRegionPolicy::KeepVisible,
                false,
            )],
            [],
        )],
        [],
    )
    .expect("registry is valid")
}

pub fn rename_request(expected: u64, label: &str) -> SurfaceMutationRequest {
    let request_label = label.to_ascii_lowercase().replace(' ', "-");
    SurfaceMutationRequest::new(
        SurfaceRequestId::new(format!("request:rename:{expected}:{request_label}")).unwrap(),
        SurfaceRevision::new(expected),
        SurfaceMutationCommand::RenameSurface {
            surface_id: surface_id("surface:a"),
            label: Some(label.to_owned()),
        },
    )
}

pub fn options() -> MutationOptions {
    MutationOptions::new(Duration::from_secs(2), DurabilityRequirement::Atomic)
}

pub fn surface_id(value: &str) -> SurfaceId {
    SurfaceId::new(value).unwrap()
}

pub fn window_id(value: &str) -> WindowId {
    WindowId::new(value).unwrap()
}

pub fn surface(id: &str, label: &str, order: u32) -> SurfaceRecord {
    SurfaceRecord::new(
        surface_id(id),
        LayoutSchemaId::new("schema:test").unwrap(),
        Some(label.to_owned()),
        [RegionState::new(
            longhorn_core::RegionId::new("region:main").unwrap(),
            [],
            None,
            None,
        )],
        [],
        [SurfaceHostPreference::new(window_id("window:main"), order)],
    )
}

/// Places a restore journal the store cannot read. The store reports that as
/// recovery-required, which blocks bare loads exactly like an active restore.
pub fn write_unreadable_restore_journal(fixture: &Fixture) {
    let path = fixture
        .temp
        .path()
        .join("data")
        .join(".longhorn")
        .join("restore")
        .join("journal.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"{ not a journal").unwrap();
}
