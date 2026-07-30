use std::fs;

use longhorn_config::{ConfigStore, CoordinationAuthority, StorageRoots};
use tempfile::TempDir;

pub(crate) struct Fixture {
    pub(crate) temp: TempDir,
    roots: StorageRoots,
    coordination: CoordinationAuthority,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        Self::at(TempDir::new().unwrap())
    }

    pub(crate) fn at(temp: TempDir) -> Self {
        let root = temp.path();
        let config = root.join("config");
        let data = root.join("data");
        let state = root.join("state");
        let cache = root.join("cache");
        let runtime = root.join("runtime");
        let log = root.join("log");
        let backup = root.join("backup");
        for path in [&config, &data, &state, &cache, &runtime, &log, &backup] {
            fs::create_dir_all(path).unwrap();
        }
        let roots =
            StorageRoots::new(config, data.clone(), state, cache, runtime, log, backup).unwrap();
        let coordination = CoordinationAuthority::new(data).unwrap();
        Self {
            temp,
            roots,
            coordination,
        }
    }

    pub(crate) fn store(&self) -> ConfigStore {
        ConfigStore::new(self.roots.clone(), self.coordination.clone())
    }

    pub(crate) fn config_path(&self) -> std::path::PathBuf {
        self.temp.path().join("config/preferences/settings.json")
    }

    pub(crate) fn store_at(root: &std::path::Path) -> ConfigStore {
        let config = root.join("config");
        let data = root.join("data");
        let state = root.join("state");
        let cache = root.join("cache");
        let runtime = root.join("runtime");
        let log = root.join("log");
        let backup = root.join("backup");
        let roots =
            StorageRoots::new(config, data.clone(), state, cache, runtime, log, backup).unwrap();
        let coordination = CoordinationAuthority::new(data).unwrap();
        ConfigStore::new(roots, coordination)
    }
}
