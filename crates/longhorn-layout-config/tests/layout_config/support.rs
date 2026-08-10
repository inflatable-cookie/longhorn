use std::{fs, path::PathBuf, time::Duration};

use longhorn_config::{
    ConfigDomain, ConfigStore, CoordinationAuthority, DebounceClock, DomainDescriptor,
    DomainFilePath, DomainLocation, DurabilityRequirement, MutationOptions, StorageClass,
    StorageRoots,
};
use longhorn_core::{
    DomainId, LayoutContainerId, LayoutRequestId, LayoutRevision, LayoutSchemaId,
    PanelDefinitionId, PanelInstanceId, RegionFamilyId, RegionId, SchemaVersion, SizingSlotId,
};
use longhorn_layout_config::{LayoutBackupPolicy, NoLayoutMigration, RegisteredLayoutDomain};
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutContainer, LayoutDefinitionRegistry, LayoutDocument, LayoutLimits,
    LayoutMutationCommand, LayoutMutationRequest, LayoutRatio, LayoutSchemaDefinition,
    PanelDefinition, PanelInstance, PanelInstancePolicy, PlacementSelector, RegionDefinition,
    RegionState, SizingSlotDefinition, SizingSlotState,
};
use serde_json::{Value, json};
use tempfile::TempDir;

pub type TestLayoutDomain = RegisteredLayoutDomain<NoLayoutMigration>;

pub struct Fixture {
    pub temp: TempDir,
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
        let roots = StorageRoots::new(
            root.join("config"),
            &data,
            root.join("state"),
            root.join("cache"),
            root.join("runtime"),
            root.join("log"),
            root.join("backups"),
        )
        .unwrap();
        let coordination = CoordinationAuthority::new(data).unwrap();
        Self {
            temp,
            roots,
            coordination,
        }
    }

    pub fn store(&self) -> ConfigStore {
        ConfigStore::new(self.roots.clone(), self.coordination.clone())
    }

    pub fn path<D: ConfigDomain>(&self, domain: &D) -> PathBuf {
        match self.roots.resolve(domain.descriptor()) {
            DomainLocation::File(file) => file.full_path().to_path_buf(),
            location => panic!("expected file-backed test domain, found {location:?}"),
        }
    }

    pub fn write<D: ConfigDomain>(&self, domain: &D, bytes: &[u8]) {
        let path = self.path(domain);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    pub fn roots(&self) -> StorageRoots {
        self.roots.clone()
    }

    pub fn coordination(&self) -> CoordinationAuthority {
        self.coordination.clone()
    }
}

pub fn descriptor(version: u32) -> DomainDescriptor {
    DomainDescriptor::new(
        DomainId::new("layout.workspace").unwrap(),
        SchemaVersion::new(version).unwrap(),
        StorageClass::MachineState,
        Some(DomainFilePath::new("workspace/layout.json").unwrap()),
    )
    .unwrap()
}

pub fn domain() -> TestLayoutDomain {
    RegisteredLayoutDomain::new(
        descriptor(1),
        document(),
        registry(),
        NoLayoutMigration,
        LayoutBackupPolicy::Include,
    )
    .unwrap()
}

pub fn registry() -> LayoutDefinitionRegistry {
    registry_with_maximum_schemas(8)
}

pub fn registry_with_maximum_schemas(maximum_schemas: usize) -> LayoutDefinitionRegistry {
    LayoutDefinitionRegistry::new(
        LayoutLimits::new(maximum_schemas, 8, 8, 8, 8, 32, 16).unwrap(),
        [LayoutSchemaDefinition::new(
            layout_schema_id("schema:workspace"),
            [
                RegionDefinition::new(
                    region_id("left"),
                    family_id("side"),
                    10,
                    EmptyRegionPolicy::KeepVisible,
                    true,
                ),
                RegionDefinition::new(
                    region_id("main"),
                    family_id("content"),
                    20,
                    EmptyRegionPolicy::KeepVisible,
                    false,
                ),
            ],
            [SizingSlotDefinition::new(
                slot_id("left-width"),
                10,
                ratio(100_000),
                ratio(200_000),
                ratio(500_000),
            )],
        )],
        [
            PanelDefinition::new(
                definition_id("panel:activity"),
                [PlacementSelector::Region(region_id("left"))],
                [PlacementSelector::Region(region_id("left"))],
                PanelInstancePolicy::Singleton,
                false,
                false,
            ),
            PanelDefinition::new(
                definition_id("panel:tool"),
                [PlacementSelector::Region(region_id("main"))],
                [PlacementSelector::Region(region_id("main"))],
                PanelInstancePolicy::Multiple,
                true,
                true,
            ),
        ],
    )
    .unwrap()
}

pub fn document() -> LayoutDocument {
    let activity = instance_id("instance:activity");
    let tool = instance_id("instance:tool");
    LayoutDocument::new(
        LayoutRevision::new(7),
        [LayoutContainer::new(
            container_id("container:primary"),
            layout_schema_id("schema:workspace"),
            [
                RegionState::new(
                    region_id("left"),
                    [activity.clone()],
                    Some(activity.clone()),
                    Some(false),
                ),
                RegionState::new(region_id("main"), [tool.clone()], Some(tool.clone()), None),
            ],
            [SizingSlotState::new(slot_id("left-width"), ratio(200_000))],
        )],
        [
            PanelInstance::new(activity, definition_id("panel:activity")),
            PanelInstance::new(tool, definition_id("panel:tool")),
        ],
    )
}

pub fn activate_request(expected: u64) -> LayoutMutationRequest {
    LayoutMutationRequest::new(
        request_id(&format!("request:activate:{expected}")),
        LayoutRevision::new(expected),
        LayoutMutationCommand::ActivatePanel {
            panel_instance_id: instance_id("instance:tool"),
        },
    )
}

pub fn sizing_request(expected: u64, value: u32) -> LayoutMutationRequest {
    LayoutMutationRequest::new(
        request_id(&format!("request:size:{expected}:{value}")),
        LayoutRevision::new(expected),
        LayoutMutationCommand::SetSizingSlot {
            container_id: container_id("container:primary"),
            sizing_slot_id: slot_id("left-width"),
            ratio: ratio(value),
        },
    )
}

pub fn collapse_request(expected: u64, collapsed: bool) -> LayoutMutationRequest {
    LayoutMutationRequest::new(
        request_id(&format!("request:collapse:{expected}:{collapsed}")),
        LayoutRevision::new(expected),
        LayoutMutationCommand::SetRegionCollapsed {
            container_id: container_id("container:primary"),
            region_id: region_id("left"),
            collapsed,
        },
    )
}

pub fn options(timeout: Duration) -> MutationOptions {
    MutationOptions::new(timeout, DurabilityRequirement::Atomic)
}

pub fn envelope(domain: &str, version: u32, value: Value) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "domain": domain,
        "schemaVersion": version,
        "value": value,
    }))
    .unwrap()
}

pub fn current_value(domain: &TestLayoutDomain) -> Value {
    domain.encode(&document()).unwrap()
}

#[derive(Clone, Copy)]
pub struct FixedClock;

impl DebounceClock for FixedClock {
    fn now(&self) -> Duration {
        Duration::ZERO
    }
}

fn layout_schema_id(value: &str) -> LayoutSchemaId {
    LayoutSchemaId::new(value).unwrap()
}

fn container_id(value: &str) -> LayoutContainerId {
    LayoutContainerId::new(value).unwrap()
}

fn region_id(value: &str) -> RegionId {
    RegionId::new(value).unwrap()
}

fn family_id(value: &str) -> RegionFamilyId {
    RegionFamilyId::new(value).unwrap()
}

fn slot_id(value: &str) -> SizingSlotId {
    SizingSlotId::new(value).unwrap()
}

fn definition_id(value: &str) -> PanelDefinitionId {
    PanelDefinitionId::new(value).unwrap()
}

fn instance_id(value: &str) -> PanelInstanceId {
    PanelInstanceId::new(value).unwrap()
}

fn request_id(value: &str) -> LayoutRequestId {
    LayoutRequestId::new(value).unwrap()
}

fn ratio(value: u32) -> LayoutRatio {
    LayoutRatio::from_millionths(value).unwrap()
}
