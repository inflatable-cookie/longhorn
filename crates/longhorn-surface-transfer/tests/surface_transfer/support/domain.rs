use std::{fs, time::Duration};

use longhorn_config::{
    ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath, DurabilityRequirement,
    LoadOutcome, MutationOptions, StorageClass, StorageRoots,
};
use longhorn_core::{
    DisplayId, DomainId, LayoutContainerId, LayoutRevision, LayoutSchemaId, SchemaVersion,
    ScreenPoint, ScreenRect, ScreenSize, SurfaceId, SurfaceRevision, WindowId, WindowPlacement,
};
use longhorn_surface_transfer::{
    EmptyDisplayProvisionPolicy, EmptyDisplayProvisionTarget, SurfaceTransferPolicy,
};
use longhorn_surfaces::{
    EmptyWindowPolicy, ParticipatingWindow, SurfaceDocument, SurfaceHostPreference, SurfaceLimits,
    SurfaceRecord,
};
use longhorn_surfaces::{LayoutContainer, LayoutDocument};
use longhorn_surfaces_config::{NoSurfaceMigration, RegisteredSurfaceDomain, SurfaceBackupPolicy};
use tempfile::TempDir;

pub type TestDomain = RegisteredSurfaceDomain<NoSurfaceMigration>;

pub struct Fixture {
    _temp: TempDir,
    pub store: ConfigStore,
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
            _temp: temp,
            store: ConfigStore::new(roots, coordination),
        }
    }
}

pub fn domain() -> TestDomain {
    RegisteredSurfaceDomain::new(
        DomainDescriptor::new(
            DomainId::new("surfaces.workspace").unwrap(),
            SchemaVersion::new(1).unwrap(),
            StorageClass::MachineState,
            Some(DomainFilePath::new("workspace/surfaces.json").unwrap()),
        )
        .unwrap(),
        SurfaceDocument::new(
            SurfaceRevision::new(7),
            [
                SurfaceRecord::new(
                    surface_id("surface:a"),
                    container_id("container:a"),
                    Some("A".to_owned()),
                    [
                        preference("window:main", 0),
                        preference("window:target", 0),
                        preference("window:new", 0),
                    ],
                ),
                SurfaceRecord::new(
                    surface_id("surface:b"),
                    container_id("container:b"),
                    Some("B".to_owned()),
                    [preference("window:main", 1)],
                ),
            ],
            [
                ParticipatingWindow::new(window_id("window:main"), Some(surface_id("surface:a"))),
                ParticipatingWindow::new(window_id("window:new"), None),
                ParticipatingWindow::new(window_id("window:target"), None),
            ],
        ),
        SurfaceLimits::new(16, 8, 4, 64).unwrap(),
        NoSurfaceMigration,
        SurfaceBackupPolicy::Include,
    )
    .unwrap()
}

pub fn layout_document() -> LayoutDocument {
    layout(["container:a", "container:b"])
}

pub fn policy() -> SurfaceTransferPolicy {
    SurfaceTransferPolicy::provisioning_disabled(
        [window_id("window:target")],
        EmptyWindowPolicy::Reject,
    )
}

pub fn policy_with_provision() -> SurfaceTransferPolicy {
    SurfaceTransferPolicy::new(
        [window_id("window:new")],
        EmptyWindowPolicy::Reject,
        EmptyDisplayProvisionPolicy::Enabled(vec![EmptyDisplayProvisionTarget::new(
            DisplayId::new("display:secondary").unwrap(),
            ScreenRect::new(ScreenPoint::new(1000, 0), ScreenSize::new(1000, 800)),
            window_id("window:new"),
            WindowPlacement::new(ScreenPoint::new(1100, 100), ScreenSize::new(700, 500)),
            None,
        )]),
    )
    .unwrap()
}

pub fn load_surface(store: &ConfigStore, domain: &TestDomain) -> SurfaceDocument {
    let LoadOutcome::Ready(loaded) = store.load(domain).unwrap() else {
        panic!("Surface domain should be ready");
    };
    loaded.value
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

fn container_id(value: &str) -> LayoutContainerId {
    LayoutContainerId::new(value).unwrap()
}

fn preference(window: &str, order: u32) -> SurfaceHostPreference {
    SurfaceHostPreference::new(window_id(window), order)
}

fn layout<const N: usize>(containers: [&str; N]) -> LayoutDocument {
    LayoutDocument::new(
        LayoutRevision::new(3),
        containers.into_iter().map(|id| {
            LayoutContainer::new(
                container_id(id),
                LayoutSchemaId::new("schema:test").unwrap(),
                [],
                [],
            )
        }),
        [],
    )
}
