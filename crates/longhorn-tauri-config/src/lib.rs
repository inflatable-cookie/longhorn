//! Dependency-light adapter for directory paths supplied by Tauri applications.
//!
//! Consumer applications obtain these paths through Tauri's application path
//! API, then pass the snapshot into Longhorn. This keeps Longhorn independent of
//! the consumer's exact Tauri version and preserves the workspace Rust floor.

use std::path::PathBuf;

use longhorn_config::{PlatformDirectoryFacts, TargetPlatform};

/// Raw desktop directory paths obtained at a Tauri application edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TauriDirectorySnapshot {
    /// macOS paths from `local_data_dir`, `cache_dir`, `home_dir`, and `temp_dir`.
    MacOs {
        /// Durable local application-data base.
        local_data_dir: PathBuf,
        /// Native cache base.
        cache_dir: PathBuf,
        /// User home used to derive `Library/Logs`.
        home_dir: PathBuf,
        /// Temporary runtime base.
        temp_dir: PathBuf,
    },
    /// Windows paths from `local_data_dir` and `temp_dir`.
    Windows {
        /// Durable local application-data base.
        local_data_dir: PathBuf,
        /// Temporary runtime base.
        temp_dir: PathBuf,
    },
    /// Linux paths from Tauri plus the host's XDG state lookup.
    Linux {
        /// XDG configuration base.
        config_dir: PathBuf,
        /// XDG local-data base.
        local_data_dir: PathBuf,
        /// XDG state base supplied explicitly because Tauri has no state API.
        state_dir: PathBuf,
        /// XDG cache base.
        cache_dir: PathBuf,
        /// XDG runtime base.
        runtime_dir: PathBuf,
    },
}

/// Converts Tauri-supplied paths into pure platform directory facts.
///
/// No application identity leaf is included. The pure storage layout resolver
/// appends the canonical application id or explicit stable storage name.
#[must_use]
pub fn platform_directory_facts(snapshot: TauriDirectorySnapshot) -> PlatformDirectoryFacts {
    match snapshot {
        TauriDirectorySnapshot::MacOs {
            local_data_dir,
            cache_dir,
            home_dir,
            temp_dir,
        } => PlatformDirectoryFacts::complete(
            TargetPlatform::MacOs,
            local_data_dir.clone(),
            local_data_dir.clone(),
            local_data_dir,
            cache_dir,
            home_dir.join("Library").join("Logs"),
            temp_dir,
        ),
        TauriDirectorySnapshot::Windows {
            local_data_dir,
            temp_dir,
        } => PlatformDirectoryFacts::complete(
            TargetPlatform::Windows,
            local_data_dir.clone(),
            local_data_dir.clone(),
            local_data_dir.clone(),
            local_data_dir.clone(),
            local_data_dir,
            temp_dir,
        ),
        TauriDirectorySnapshot::Linux {
            config_dir,
            local_data_dir,
            state_dir,
            cache_dir,
            runtime_dir,
        } => PlatformDirectoryFacts::complete(
            TargetPlatform::Linux,
            config_dir,
            local_data_dir,
            state_dir.clone(),
            cache_dir,
            state_dir,
            runtime_dir,
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use longhorn_config::{PlatformDirectoryFact, TargetPlatform};

    use super::*;

    #[test]
    fn macos_maps_tauri_paths_without_an_app_leaf() {
        let facts = platform_directory_facts(TauriDirectorySnapshot::MacOs {
            local_data_dir: "/Users/example/Library/Application Support".into(),
            cache_dir: "/Users/example/Library/Caches".into(),
            home_dir: "/Users/example".into(),
            temp_dir: "/private/tmp".into(),
        });

        assert_eq!(facts.platform(), TargetPlatform::MacOs);
        assert_eq!(
            facts.get(PlatformDirectoryFact::Config),
            Some(Path::new("/Users/example/Library/Application Support"))
        );
        assert_eq!(
            facts.get(PlatformDirectoryFact::Log),
            Some(Path::new("/Users/example/Library/Logs"))
        );
    }

    #[test]
    fn windows_uses_local_data_for_non_runtime_facts() {
        let facts = platform_directory_facts(TauriDirectorySnapshot::Windows {
            local_data_dir: "/windows/LocalAppData".into(),
            temp_dir: "/windows/Temp".into(),
        });

        for fact in [
            PlatformDirectoryFact::Config,
            PlatformDirectoryFact::Data,
            PlatformDirectoryFact::State,
            PlatformDirectoryFact::Cache,
            PlatformDirectoryFact::Log,
        ] {
            assert_eq!(facts.get(fact), Some(Path::new("/windows/LocalAppData")));
        }
        assert_eq!(
            facts.get(PlatformDirectoryFact::Runtime),
            Some(Path::new("/windows/Temp"))
        );
    }

    #[test]
    fn linux_keeps_xdg_lifecycle_bases_distinct() {
        let facts = platform_directory_facts(TauriDirectorySnapshot::Linux {
            config_dir: "/home/example/.config".into(),
            local_data_dir: "/home/example/.local/share".into(),
            state_dir: "/home/example/.local/state".into(),
            cache_dir: "/home/example/.cache".into(),
            runtime_dir: "/run/user/1000".into(),
        });

        assert_eq!(facts.platform(), TargetPlatform::Linux);
        assert_eq!(
            facts.get(PlatformDirectoryFact::State),
            Some(Path::new("/home/example/.local/state"))
        );
        assert_eq!(
            facts.get(PlatformDirectoryFact::Log),
            Some(Path::new("/home/example/.local/state"))
        );
    }
}
