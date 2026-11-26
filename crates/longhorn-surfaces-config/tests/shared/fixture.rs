//! Temporary storage roots and a configuration store for both test binaries.

use std::{fs, path::PathBuf};

use longhorn_config::{
    ConfigDomain, ConfigStore, CoordinationAuthority, DomainLocation, StorageRootsResolve,
};
use longhorn_storage_layout::StorageRoots;
use serde_json::{Value, json};
use tempfile::TempDir;

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

pub fn envelope(domain: &str, version: u32, value: Value) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "domain": domain,
        "schemaVersion": version,
        "value": value,
    }))
    .unwrap()
}
