use std::{fs, path::PathBuf, time::Duration};

use longhorn_config::{
    ConfigDomain, ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath,
    DomainLocation, DurabilityRequirement, MutationOptions, StorageClass, StorageRoots,
};
use longhorn_core::{
    DomainId, LayoutSchemaId, SchemaVersion, SurfaceId, SurfaceRequestId, SurfaceRevision, WindowId,
};
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutLimits, LayoutSchemaDefinition,
    ParticipatingWindow, RegionDefinition, SurfaceDocument, SurfaceHostPreference, SurfaceLimits,
    SurfaceMutationCommand, SurfaceMutationRequest, SurfaceRecord,
};
use longhorn_surfaces_config::{NoSurfaceMigration, RegisteredSurfaceDomain, SurfaceBackupPolicy};
use serde_json::{Value, json};
use tempfile::TempDir;

pub type TestSurfaceDomain = RegisteredSurfaceDomain<NoSurfaceMigration>;

pub struct Fixture {
    temp: TempDir,
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
        let _keep_temp_alive = &self.temp;
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
        limits(),
        NoSurfaceMigration,
        SurfaceBackupPolicy::Include,
    )
    .unwrap()
}

pub fn limits() -> SurfaceLimits {
    SurfaceLimits::new(16, 4, 4, 64).unwrap()
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
        LayoutLimits::new(8, 8, 8, 64, 8, 64, 16).expect("layout limits are valid"),
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

pub fn envelope(domain: &str, version: u32, value: Value) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "domain": domain,
        "schemaVersion": version,
        "value": value,
    }))
    .unwrap()
}

pub fn surface_id(value: &str) -> SurfaceId {
    SurfaceId::new(value).unwrap()
}

pub fn window_id(value: &str) -> WindowId {
    WindowId::new(value).unwrap()
}

fn surface(id: &str, label: &str, order: u32) -> SurfaceRecord {
    SurfaceRecord::new(
        surface_id(id),
        LayoutSchemaId::new("schema:test").unwrap(),
        Some(label.to_owned()),
        [],
        [],
        [SurfaceHostPreference::new(window_id("window:main"), order)],
    )
}
