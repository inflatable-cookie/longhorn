use std::{fs, path::PathBuf, time::Duration};

use longhorn_config::StorageRootsResolve;
use longhorn_config::{
    ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath, DomainLocation,
    DurabilityRequirement, MutationOptions, StorageClass,
};
use longhorn_core::{
    DomainId, LayoutSchemaId, PanelDefinitionId, PanelInstanceId, RegionFamilyId, RegionId,
    SchemaVersion, SurfaceId, SurfaceRevision, WindowId,
};
use longhorn_storage_layout::StorageRoots;
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutSchemaDefinition, PanelDefinition,
    PanelInstance, PanelInstancePolicy, ParticipatingWindow, PlacementSelector, RegionDefinition,
    RegionState, SurfaceDocument, SurfaceHostPreference, SurfaceLimits, SurfaceRecord,
};
use longhorn_surfaces_config::{NoSurfaceMigration, RegisteredSurfaceDomain, SurfaceBackupPolicy};
use tempfile::TempDir;

pub type TestDomain = RegisteredSurfaceDomain<NoSurfaceMigration>;

pub struct Fixture {
    _temp: TempDir,
    roots: StorageRoots,
    coordination: CoordinationAuthority,
}

impl Fixture {
    pub fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let data = root.join("data");
        for path in [
            root.join("config"),
            data.clone(),
            root.join("state"),
            root.join("cache"),
            root.join("runtime"),
            root.join("log"),
            root.join("backups"),
        ] {
            fs::create_dir_all(path).unwrap();
        }
        Self {
            roots: StorageRoots::new(
                root.join("config"),
                &data,
                root.join("state"),
                root.join("cache"),
                root.join("runtime"),
                root.join("log"),
                root.join("backups"),
            )
            .unwrap(),
            coordination: CoordinationAuthority::new(data).unwrap(),
            _temp: temp,
        }
    }

    pub fn store(&self) -> ConfigStore {
        ConfigStore::new(self.roots.clone(), self.coordination.clone())
    }

    pub fn path(&self, domain: &TestDomain) -> PathBuf {
        match self.roots.resolve(domain.descriptor()) {
            DomainLocation::File(file) => file.full_path().to_path_buf(),
            location => panic!("expected file layout domain, found {location:?}"),
        }
    }
}

pub fn domain() -> TestDomain {
    RegisteredSurfaceDomain::new(
        DomainDescriptor::new(
            domain_id(),
            SchemaVersion::new(1).unwrap(),
            StorageClass::MachineState,
            Some(DomainFilePath::new("workspace/layout.json").unwrap()),
        )
        .unwrap(),
        document(),
        registry(),
        NoSurfaceMigration,
        SurfaceBackupPolicy::Include,
    )
    .unwrap()
}

pub fn registry() -> LayoutDefinitionRegistry {
    LayoutDefinitionRegistry::new(
        SurfaceLimits::builder()
            .maximum_schemas(4)
            .maximum_regions_per_schema(4)
            .maximum_sizing_slots_per_schema(4)
            .maximum_panel_definitions(4)
            .maximum_surfaces(4)
            .maximum_windows(4)
            .maximum_host_preferences_per_surface(4)
            .maximum_label_bytes(64)
            .maximum_panel_instances(16)
            .maximum_panel_instances_per_region(8)
            .build()
            .unwrap(),
        [LayoutSchemaDefinition::new(
            schema_id(),
            [
                RegionDefinition::new(
                    main_region(),
                    RegionFamilyId::new("family:content").unwrap(),
                    10,
                    EmptyRegionPolicy::KeepVisible,
                    false,
                ),
                RegionDefinition::new(
                    side_region(),
                    RegionFamilyId::new("family:side").unwrap(),
                    20,
                    EmptyRegionPolicy::KeepVisible,
                    true,
                ),
            ],
            [],
        )],
        [
            PanelDefinition::new(
                tool_definition(),
                [PlacementSelector::Region(main_region())],
                [PlacementSelector::Region(main_region())],
                PanelInstancePolicy::Multiple,
                true,
                true,
            ),
            PanelDefinition::new(
                fixed_definition(),
                [PlacementSelector::Region(side_region())],
                [PlacementSelector::Region(side_region())],
                PanelInstancePolicy::Singleton,
                false,
                false,
            ),
        ],
    )
    .unwrap()
}

pub fn document() -> SurfaceDocument {
    let tool = tool_panel();
    let fixed = fixed_panel();
    let window = WindowId::new("window:main").unwrap();
    SurfaceDocument::new(
        SurfaceRevision::new(7),
        [
            SurfaceRecord::new(
                source_container(),
                schema_id(),
                None,
                [
                    RegionState::new(main_region(), [tool.clone()], Some(tool.clone()), None),
                    RegionState::new(
                        side_region(),
                        [fixed.clone()],
                        Some(fixed.clone()),
                        Some(false),
                    ),
                ],
                [],
                [SurfaceHostPreference::new(window.clone(), 0)],
            ),
            SurfaceRecord::new(
                target_container(),
                schema_id(),
                None,
                [
                    RegionState::new(main_region(), [], None, None),
                    RegionState::new(side_region(), [], None, Some(false)),
                ],
                [],
                [SurfaceHostPreference::new(window.clone(), 1)],
            ),
        ],
        [
            PanelInstance::new(tool, tool_definition()),
            PanelInstance::new(fixed, fixed_definition()),
        ],
        [ParticipatingWindow::new(window, Some(source_container()))],
    )
}

pub fn options() -> MutationOptions {
    MutationOptions::new(Duration::from_secs(1), DurabilityRequirement::Atomic)
}

pub fn domain_id() -> DomainId {
    DomainId::new("layout.workspace").unwrap()
}

pub fn source_container() -> SurfaceId {
    SurfaceId::new("surface:source").unwrap()
}

pub fn target_container() -> SurfaceId {
    SurfaceId::new("surface:target").unwrap()
}

pub fn main_region() -> RegionId {
    RegionId::new("region:main").unwrap()
}

pub fn side_region() -> RegionId {
    RegionId::new("region:side").unwrap()
}

pub fn tool_panel() -> PanelInstanceId {
    PanelInstanceId::new("panel:tool:one").unwrap()
}

pub fn fixed_panel() -> PanelInstanceId {
    PanelInstanceId::new("panel:fixed:one").unwrap()
}

fn schema_id() -> LayoutSchemaId {
    LayoutSchemaId::new("schema:workspace").unwrap()
}

fn tool_definition() -> PanelDefinitionId {
    PanelDefinitionId::new("panel:tool").unwrap()
}

fn fixed_definition() -> PanelDefinitionId {
    PanelDefinitionId::new("panel:fixed").unwrap()
}
