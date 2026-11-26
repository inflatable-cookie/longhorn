use std::time::Duration;

use longhorn_config::{
    ConfigDomain, DebounceClock, DomainDescriptor, DomainFilePath, DurabilityRequirement,
    MutationOptions, StorageClass,
};
use longhorn_core::{
    DomainId, LayoutRequestId, LayoutSchemaId, PanelDefinitionId, PanelInstanceId, RegionFamilyId,
    RegionId, SchemaVersion, SizingSlotId, SurfaceId, SurfaceRevision, WindowId,
};
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutMutationCommand, LayoutMutationRequest,
    LayoutRatio, LayoutSchemaDefinition, PanelDefinition, PanelInstance, PanelInstancePolicy,
    ParticipatingWindow, PlacementSelector, RegionDefinition, RegionState, SizingSlotDefinition,
    SizingSlotState, SurfaceDocument, SurfaceHostPreference, SurfaceLimits, SurfaceRecord,
};
use longhorn_surfaces_config::{NoSurfaceMigration, RegisteredSurfaceDomain, SurfaceBackupPolicy};
use serde_json::Value;

pub type TestLayoutDomain = RegisteredSurfaceDomain<NoSurfaceMigration>;

pub use crate::fixture::{Fixture, envelope};

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
    RegisteredSurfaceDomain::new(
        descriptor(1),
        document(),
        registry(),
        NoSurfaceMigration,
        SurfaceBackupPolicy::Include,
    )
    .unwrap()
}

pub fn registry() -> LayoutDefinitionRegistry {
    registry_with_maximum_schemas(8)
}

pub fn registry_with_maximum_schemas(maximum_schemas: usize) -> LayoutDefinitionRegistry {
    LayoutDefinitionRegistry::new(
        SurfaceLimits::builder()
            .maximum_schemas(maximum_schemas)
            .maximum_regions_per_schema(8)
            .maximum_sizing_slots_per_schema(8)
            .maximum_panel_definitions(8)
            .maximum_surfaces(8)
            .maximum_windows(4)
            .maximum_host_preferences_per_surface(4)
            .maximum_label_bytes(64)
            .maximum_panel_instances(32)
            .maximum_panel_instances_per_region(16)
            .build()
            .unwrap(),
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

pub fn document() -> SurfaceDocument {
    let activity = instance_id("instance:activity");
    let tool = instance_id("instance:tool");
    SurfaceDocument::new(
        SurfaceRevision::new(7),
        [SurfaceRecord::new(
            surface_id("surface:primary"),
            layout_schema_id("schema:workspace"),
            None,
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
            [SurfaceHostPreference::new(
                WindowId::new("window:main").unwrap(),
                0,
            )],
        )],
        [
            PanelInstance::new(activity, definition_id("panel:activity")),
            PanelInstance::new(tool, definition_id("panel:tool")),
        ],
        [ParticipatingWindow::new(
            WindowId::new("window:main").unwrap(),
            Some(surface_id("surface:primary")),
        )],
    )
}

pub fn activate_request(expected: u64) -> LayoutMutationRequest {
    LayoutMutationRequest::new(
        request_id(&format!("request:activate:{expected}")),
        SurfaceRevision::new(expected),
        LayoutMutationCommand::ActivatePanel {
            panel_instance_id: instance_id("instance:tool"),
        },
    )
}

pub fn sizing_request(expected: u64, value: u32) -> LayoutMutationRequest {
    LayoutMutationRequest::new(
        request_id(&format!("request:size:{expected}:{value}")),
        SurfaceRevision::new(expected),
        LayoutMutationCommand::SetSizingSlot {
            surface_id: surface_id("surface:primary"),
            sizing_slot_id: slot_id("left-width"),
            ratio: ratio(value),
        },
    )
}

pub fn collapse_request(expected: u64, collapsed: bool) -> LayoutMutationRequest {
    LayoutMutationRequest::new(
        request_id(&format!("request:collapse:{expected}:{collapsed}")),
        SurfaceRevision::new(expected),
        LayoutMutationCommand::SetRegionCollapsed {
            surface_id: surface_id("surface:primary"),
            region_id: region_id("left"),
            collapsed,
        },
    )
}

pub fn sizing_command(value: u32) -> LayoutMutationCommand {
    LayoutMutationCommand::SetSizingSlot {
        surface_id: surface_id("surface:primary"),
        sizing_slot_id: slot_id("left-width"),
        ratio: ratio(value),
    }
}

pub fn collapse_command(collapsed: bool) -> LayoutMutationCommand {
    LayoutMutationCommand::SetRegionCollapsed {
        surface_id: surface_id("surface:primary"),
        region_id: region_id("left"),
        collapsed,
    }
}

pub fn activate_command() -> LayoutMutationCommand {
    LayoutMutationCommand::ActivatePanel {
        panel_instance_id: instance_id("instance:tool"),
    }
}

pub fn options(timeout: Duration) -> MutationOptions {
    MutationOptions::new(timeout, DurabilityRequirement::Atomic)
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

fn surface_id(value: &str) -> SurfaceId {
    SurfaceId::new(value).unwrap()
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
