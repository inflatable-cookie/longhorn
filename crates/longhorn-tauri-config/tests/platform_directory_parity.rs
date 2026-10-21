//! The native facts provider and the Tauri adapter must produce identical
//! platform directory facts for the same machine snapshot, so a CLI resolves
//! exactly the paths a Tauri application with the same storage identity
//! resolves (contract 004).

use std::path::{Path, PathBuf};

use longhorn_storage_layout::{
    NativePlatformFactsError, PlatformDirectoryFact, PlatformEnvironment, TargetPlatform,
    native_platform_directory_facts, native_platform_directory_facts_from,
};
use longhorn_tauri_config::{TauriDirectorySnapshot, platform_directory_facts};

const FACTS: [PlatformDirectoryFact; 7] = [
    PlatformDirectoryFact::Config,
    PlatformDirectoryFact::Data,
    PlatformDirectoryFact::State,
    PlatformDirectoryFact::Cache,
    PlatformDirectoryFact::Log,
    PlatformDirectoryFact::Runtime,
    PlatformDirectoryFact::SharedData,
];

fn assert_parity(snapshot: TauriDirectorySnapshot, environment: &PlatformEnvironment) {
    let tauri = platform_directory_facts(snapshot);
    let native = native_platform_directory_facts_from(tauri.platform(), environment);
    for fact in FACTS {
        assert_eq!(
            native.get(fact),
            tauri.get(fact),
            "{:?} diverged on {fact:?}",
            tauri.platform()
        );
    }
}

#[test]
fn macos_native_and_tauri_facts_agree() {
    let home = PathBuf::from("/Users/example");
    let snapshot = TauriDirectorySnapshot::MacOs {
        local_data_dir: home.join("Library/Application Support"),
        cache_dir: home.join("Library/Caches"),
        home_dir: home.clone(),
        temp_dir: PathBuf::from("/private/tmp"),
    };
    let environment = PlatformEnvironment::default()
        .with_home_dir(Some(home))
        .with_temp_dir(Some(PathBuf::from("/private/tmp")));

    assert_parity(snapshot, &environment);
}

#[test]
fn linux_native_and_tauri_facts_agree_with_and_without_xdg_overrides() {
    for (home_suffix, xdg) in [("example", false), ("example", true), ("other", true)] {
        let home = PathBuf::from("/home").join(home_suffix);
        let config = if xdg {
            PathBuf::from("/xdg/config")
        } else {
            home.join(".config")
        };
        let data = if xdg {
            PathBuf::from("/xdg/data")
        } else {
            home.join(".local/share")
        };
        let state = if xdg {
            PathBuf::from("/xdg/state")
        } else {
            home.join(".local/state")
        };
        let cache = if xdg {
            PathBuf::from("/xdg/cache")
        } else {
            home.join(".cache")
        };
        let runtime = PathBuf::from("/run/user/1000");

        let snapshot = TauriDirectorySnapshot::Linux {
            config_dir: config,
            local_data_dir: data,
            state_dir: state,
            cache_dir: cache,
            runtime_dir: runtime.clone(),
        };
        let environment = PlatformEnvironment::default()
            .with_home_dir(Some(home))
            .with_temp_dir(Some(PathBuf::from("/tmp")))
            .with_xdg_config_home(xdg.then(|| PathBuf::from("/xdg/config")))
            .with_xdg_data_home(xdg.then(|| PathBuf::from("/xdg/data")))
            .with_xdg_state_home(xdg.then(|| PathBuf::from("/xdg/state")))
            .with_xdg_cache_home(xdg.then(|| PathBuf::from("/xdg/cache")))
            .with_xdg_runtime_dir(Some(runtime));

        assert_parity(snapshot, &environment);
    }
}

#[test]
fn windows_native_and_tauri_facts_agree() {
    let snapshot = TauriDirectorySnapshot::Windows {
        local_data_dir: PathBuf::from("/windows/LocalAppData"),
        roaming_data_dir: PathBuf::from("/windows/RoamingAppData"),
        temp_dir: PathBuf::from("/windows/Temp"),
    };
    let environment = PlatformEnvironment::default()
        .with_temp_dir(Some(PathBuf::from("/windows/Temp")))
        .with_windows_local_app_data(Some(PathBuf::from("/windows/LocalAppData")))
        .with_windows_roaming_app_data(Some(PathBuf::from("/windows/RoamingAppData")));

    assert_parity(snapshot, &environment);
}

#[test]
fn host_lookup_refuses_platforms_without_a_policy() {
    // The mapping itself is covered per platform above; this pins the
    // contract that an unsupported host reports typed unavailability instead
    // of inventing facts.
    if !matches!(std::env::consts::OS, "macos" | "windows" | "linux") {
        assert_eq!(
            native_platform_directory_facts().unwrap_err(),
            NativePlatformFactsError::UnsupportedPlatform
        );
    } else {
        let facts = native_platform_directory_facts().unwrap();
        assert!(matches!(
            facts.platform(),
            TargetPlatform::MacOs | TargetPlatform::Windows | TargetPlatform::Linux
        ));
        // A real host always exposes its temporary directory.
        assert!(
            facts
                .get(PlatformDirectoryFact::Runtime)
                .is_some_and(Path::is_absolute)
        );
    }
}
