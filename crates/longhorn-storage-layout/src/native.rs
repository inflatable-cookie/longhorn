use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
};

use crate::{PlatformDirectoryFact, PlatformDirectoryFacts, TargetPlatform};

/// Environment snapshot that drives the native platform-directory facts.
///
/// Every field mirrors one host variable: a missing entry reports the facts
/// it feeds as unavailable, and an entry that is empty or relative is treated
/// as unset where the platform rule allows a default. No field is ever
/// guessed from the filesystem, the executable location, or the current
/// directory.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlatformEnvironment {
    home_dir: Option<PathBuf>,
    temp_dir: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
    xdg_data_home: Option<PathBuf>,
    xdg_state_home: Option<PathBuf>,
    xdg_cache_home: Option<PathBuf>,
    xdg_runtime_dir: Option<PathBuf>,
    windows_local_app_data: Option<PathBuf>,
    windows_roaming_app_data: Option<PathBuf>,
}

impl PlatformEnvironment {
    /// Reads the process environment once into a snapshot.
    ///
    /// `temp_dir` comes from [`env::temp_dir`], which reads the platform
    /// temporary-directory variables; every other field is one named
    /// variable.
    #[must_use]
    pub fn from_process() -> Self {
        let variable = |name: &str| env::var_os(name).map(PathBuf::from);
        Self {
            home_dir: variable("HOME"),
            temp_dir: Some(env::temp_dir()),
            xdg_config_home: variable("XDG_CONFIG_HOME"),
            xdg_data_home: variable("XDG_DATA_HOME"),
            xdg_state_home: variable("XDG_STATE_HOME"),
            xdg_cache_home: variable("XDG_CACHE_HOME"),
            xdg_runtime_dir: variable("XDG_RUNTIME_DIR"),
            windows_local_app_data: variable("LOCALAPPDATA"),
            windows_roaming_app_data: variable("APPDATA"),
        }
    }

    fn from_process_for(platform: TargetPlatform, fact: PlatformDirectoryFact) -> Self {
        let variable = |name: &str| env::var_os(name).map(PathBuf::from);
        let mut environment = Self::default();
        match (platform, fact) {
            (TargetPlatform::MacOs, PlatformDirectoryFact::Runtime)
            | (TargetPlatform::Windows, PlatformDirectoryFact::Runtime) => {
                environment.temp_dir = Some(env::temp_dir());
            }
            (TargetPlatform::MacOs, _) => environment.home_dir = variable("HOME"),
            (TargetPlatform::Windows, PlatformDirectoryFact::SharedData) => {
                environment.windows_roaming_app_data = variable("APPDATA");
            }
            (TargetPlatform::Windows, _) => {
                environment.windows_local_app_data = variable("LOCALAPPDATA");
            }
            (TargetPlatform::Linux, PlatformDirectoryFact::Config) => {
                environment.home_dir = variable("HOME");
                environment.xdg_config_home = variable("XDG_CONFIG_HOME");
            }
            (
                TargetPlatform::Linux,
                PlatformDirectoryFact::Data | PlatformDirectoryFact::SharedData,
            ) => {
                environment.home_dir = variable("HOME");
                environment.xdg_data_home = variable("XDG_DATA_HOME");
            }
            (TargetPlatform::Linux, PlatformDirectoryFact::State | PlatformDirectoryFact::Log) => {
                environment.home_dir = variable("HOME");
                environment.xdg_state_home = variable("XDG_STATE_HOME");
            }
            (TargetPlatform::Linux, PlatformDirectoryFact::Cache) => {
                environment.home_dir = variable("HOME");
                environment.xdg_cache_home = variable("XDG_CACHE_HOME");
            }
            (TargetPlatform::Linux, PlatformDirectoryFact::Runtime) => {
                environment.xdg_runtime_dir = variable("XDG_RUNTIME_DIR");
            }
        }
        environment
    }

    /// Replaces the home directory entry.
    #[must_use]
    pub fn with_home_dir(mut self, home_dir: Option<PathBuf>) -> Self {
        self.home_dir = home_dir;
        self
    }

    /// Replaces the temporary-directory entry.
    #[must_use]
    pub fn with_temp_dir(mut self, temp_dir: Option<PathBuf>) -> Self {
        self.temp_dir = temp_dir;
        self
    }

    /// Replaces the `XDG_CONFIG_HOME` entry.
    #[must_use]
    pub fn with_xdg_config_home(mut self, value: Option<PathBuf>) -> Self {
        self.xdg_config_home = value;
        self
    }

    /// Replaces the `XDG_DATA_HOME` entry.
    #[must_use]
    pub fn with_xdg_data_home(mut self, value: Option<PathBuf>) -> Self {
        self.xdg_data_home = value;
        self
    }

    /// Replaces the `XDG_STATE_HOME` entry.
    #[must_use]
    pub fn with_xdg_state_home(mut self, value: Option<PathBuf>) -> Self {
        self.xdg_state_home = value;
        self
    }

    /// Replaces the `XDG_CACHE_HOME` entry.
    #[must_use]
    pub fn with_xdg_cache_home(mut self, value: Option<PathBuf>) -> Self {
        self.xdg_cache_home = value;
        self
    }

    /// Replaces the `XDG_RUNTIME_DIR` entry.
    #[must_use]
    pub fn with_xdg_runtime_dir(mut self, value: Option<PathBuf>) -> Self {
        self.xdg_runtime_dir = value;
        self
    }

    /// Replaces the Windows known `LocalAppData` entry.
    #[must_use]
    pub fn with_windows_local_app_data(mut self, value: Option<PathBuf>) -> Self {
        self.windows_local_app_data = value;
        self
    }

    /// Replaces the Windows known roaming `AppData` entry.
    #[must_use]
    pub fn with_windows_roaming_app_data(mut self, value: Option<PathBuf>) -> Self {
        self.windows_roaming_app_data = value;
        self
    }

    fn home(&self) -> Option<&Path> {
        self.home_dir
            .as_deref()
            .filter(|home| !home.as_os_str().is_empty() && home.is_absolute())
    }

    /// Applies one XDG variable with the spec's absolute-path rule.
    fn xdg(
        &self,
        value: Option<&PathBuf>,
        default: impl FnOnce(&Path) -> PathBuf,
    ) -> Option<PathBuf> {
        value
            .map(PathBuf::as_path)
            .filter(|path| !path.as_os_str().is_empty() && path.is_absolute())
            .map_or_else(|| self.home().map(default), |path| Some(path.to_path_buf()))
    }
}

/// Native platform-directory failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativePlatformFactsError {
    /// This host has no Longhorn platform-directory policy.
    UnsupportedPlatform,
}

impl core::fmt::Display for NativePlatformFactsError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("no Longhorn platform-directory policy for this host")
    }
}

impl Error for NativePlatformFactsError {}

/// Reads the host environment once and maps it to platform directory facts.
///
/// The mapping is the same one the Tauri adapter supplies, so an application
/// with the same storage identity resolves identical paths with or without a
/// webview host. A missing `HOME`, an unset `XDG_RUNTIME_DIR`, or a missing
/// Windows known-folder variable leaves its facts unavailable; nothing is
/// invented as a home dotfile.
pub fn native_platform_directory_facts() -> Result<PlatformDirectoryFacts, NativePlatformFactsError>
{
    let platform = native_target_platform()?;
    Ok(native_platform_directory_facts_from(
        platform,
        &PlatformEnvironment::from_process(),
    ))
}

/// Reads only the environment entries needed for one native directory fact.
///
/// The returned fact set contains at most the requested fact, and can be
/// passed directly to [`crate::StorageLayoutRequest`]. If the host has no
/// value for the requested fact, the set is empty for that platform and
/// resolving a root that needs it reports `MissingPlatformFact`.
pub fn native_platform_directory_fact(
    fact: PlatformDirectoryFact,
) -> Result<PlatformDirectoryFacts, NativePlatformFactsError> {
    let platform = native_target_platform()?;
    Ok(native_platform_directory_fact_from(
        platform,
        fact,
        &PlatformEnvironment::from_process_for(platform, fact),
    ))
}

fn native_target_platform() -> Result<TargetPlatform, NativePlatformFactsError> {
    match std::env::consts::OS {
        "macos" => Ok(TargetPlatform::MacOs),
        "windows" => Ok(TargetPlatform::Windows),
        "linux" => Ok(TargetPlatform::Linux),
        _ => Err(NativePlatformFactsError::UnsupportedPlatform),
    }
}

/// Maps one environment snapshot to platform directory facts for one platform.
///
/// The mapping per platform:
///
/// - macOS derives every non-runtime fact from `$HOME`: `Library/Application
///   Support` for config, data, state, and shared data; `Library/Caches` for
///   cache; `Library/Logs` for log. Runtime is the temporary directory.
/// - Windows derives config, data, state, cache, and log from
///   `LocalAppData`, shared data from roaming `AppData`, and runtime from the
///   temporary directory.
/// - Linux applies the XDG spec defaults `$HOME/.config`,
///   `$HOME/.local/share`, `$HOME/.local/state`, and `$HOME/.cache`, keeps
///   log at the state base, takes runtime only from `XDG_RUNTIME_DIR` (the
///   spec defines no default), and uses the data base as shared data.
#[must_use]
pub fn native_platform_directory_facts_from(
    platform: TargetPlatform,
    environment: &PlatformEnvironment,
) -> PlatformDirectoryFacts {
    let mut facts = PlatformDirectoryFacts::new(platform);
    for fact in [
        PlatformDirectoryFact::Config,
        PlatformDirectoryFact::Data,
        PlatformDirectoryFact::State,
        PlatformDirectoryFact::Cache,
        PlatformDirectoryFact::Log,
        PlatformDirectoryFact::Runtime,
        PlatformDirectoryFact::SharedData,
    ] {
        if let Some(path) = native_directory_path_from(platform, fact, environment) {
            facts = facts.with(fact, path);
        }
    }
    facts
}

/// Maps one environment snapshot to a fact for one platform.
///
/// The returned set contains only `fact` when its source is valid; otherwise
/// it is empty. A relative XDG value is ignored in favour of the spec default,
/// except `XDG_RUNTIME_DIR`, which has no default.
#[must_use]
pub fn native_platform_directory_fact_from(
    platform: TargetPlatform,
    fact: PlatformDirectoryFact,
    environment: &PlatformEnvironment,
) -> PlatformDirectoryFacts {
    let mut facts = PlatformDirectoryFacts::new(platform);
    if let Some(path) = native_directory_path_from(platform, fact, environment) {
        facts = facts.with(fact, path);
    }
    facts
}

fn native_directory_path_from(
    platform: TargetPlatform,
    fact: PlatformDirectoryFact,
    environment: &PlatformEnvironment,
) -> Option<PathBuf> {
    match (platform, fact) {
        (TargetPlatform::MacOs, PlatformDirectoryFact::Runtime)
        | (TargetPlatform::Windows, PlatformDirectoryFact::Runtime) => {
            absolute(environment.temp_dir.as_ref())
        }
        (TargetPlatform::MacOs, PlatformDirectoryFact::Cache) => environment
            .home()
            .map(|home| home.join("Library").join("Caches")),
        (TargetPlatform::MacOs, PlatformDirectoryFact::Log) => environment
            .home()
            .map(|home| home.join("Library").join("Logs")),
        (TargetPlatform::MacOs, _) => environment
            .home()
            .map(|home| home.join("Library").join("Application Support")),
        (TargetPlatform::Windows, PlatformDirectoryFact::SharedData) => {
            absolute(environment.windows_roaming_app_data.as_ref())
        }
        (TargetPlatform::Windows, _) => absolute(environment.windows_local_app_data.as_ref()),
        (TargetPlatform::Linux, PlatformDirectoryFact::Config) => environment
            .xdg(environment.xdg_config_home.as_ref(), |home| {
                home.join(".config")
            }),
        (
            TargetPlatform::Linux,
            PlatformDirectoryFact::Data | PlatformDirectoryFact::SharedData,
        ) => environment.xdg(environment.xdg_data_home.as_ref(), |home| {
            home.join(".local").join("share")
        }),
        (TargetPlatform::Linux, PlatformDirectoryFact::State | PlatformDirectoryFact::Log) => {
            environment.xdg(environment.xdg_state_home.as_ref(), |home| {
                home.join(".local").join("state")
            })
        }
        (TargetPlatform::Linux, PlatformDirectoryFact::Cache) => environment
            .xdg(environment.xdg_cache_home.as_ref(), |home| {
                home.join(".cache")
            }),
        (TargetPlatform::Linux, PlatformDirectoryFact::Runtime) => {
            absolute(environment.xdg_runtime_dir.as_ref())
        }
    }
}

fn absolute(value: Option<&PathBuf>) -> Option<PathBuf> {
    value
        .map(PathBuf::as_path)
        .filter(|path| !path.as_os_str().is_empty() && path.is_absolute())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(value: &str) -> Option<PathBuf> {
        Some(PathBuf::from(value))
    }

    fn macos_home(value: &str) -> PlatformEnvironment {
        PlatformEnvironment::default()
            .with_home_dir(home(value))
            .with_temp_dir(home("/private/tmp"))
    }

    fn linux_home(value: &str) -> PlatformEnvironment {
        PlatformEnvironment::default().with_home_dir(home(value))
    }

    fn fact(facts: &PlatformDirectoryFacts, fact: PlatformDirectoryFact) -> PathBuf {
        facts.get(fact).map(Path::to_path_buf).unwrap()
    }

    #[test]
    fn full_macos_snapshot_maps_every_fact_from_home() {
        let facts = native_platform_directory_facts_from(
            TargetPlatform::MacOs,
            &macos_home("/Users/example"),
        );

        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Config),
            PathBuf::from("/Users/example/Library/Application Support")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Data),
            PathBuf::from("/Users/example/Library/Application Support")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::State),
            PathBuf::from("/Users/example/Library/Application Support")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Cache),
            PathBuf::from("/Users/example/Library/Caches")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Log),
            PathBuf::from("/Users/example/Library/Logs")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Runtime),
            PathBuf::from("/private/tmp")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::SharedData),
            PathBuf::from("/Users/example/Library/Application Support")
        );
    }

    #[test]
    fn missing_home_reports_facts_unavailable_and_keeps_runtime() {
        let facts = native_platform_directory_facts_from(
            TargetPlatform::MacOs,
            &PlatformEnvironment::default().with_temp_dir(home("/private/tmp")),
        );

        for unavailable in [
            PlatformDirectoryFact::Config,
            PlatformDirectoryFact::Data,
            PlatformDirectoryFact::State,
            PlatformDirectoryFact::Cache,
            PlatformDirectoryFact::Log,
            PlatformDirectoryFact::SharedData,
        ] {
            assert_eq!(facts.get(unavailable), None, "{unavailable:?} invented");
        }
        assert_eq!(
            facts.get(PlatformDirectoryFact::Runtime),
            Some(Path::new("/private/tmp"))
        );
    }

    #[test]
    fn full_linux_snapshot_prefers_xdg_over_spec_defaults() {
        let environment = linux_home("/home/example")
            .with_temp_dir(home("/tmp"))
            .with_xdg_config_home(home("/xdg/config"))
            .with_xdg_data_home(home("/xdg/data"))
            .with_xdg_state_home(home("/xdg/state"))
            .with_xdg_cache_home(home("/xdg/cache"))
            .with_xdg_runtime_dir(home("/run/user/1000"));
        let facts = native_platform_directory_facts_from(TargetPlatform::Linux, &environment);

        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Config),
            PathBuf::from("/xdg/config")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Data),
            PathBuf::from("/xdg/data")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::State),
            PathBuf::from("/xdg/state")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Cache),
            PathBuf::from("/xdg/cache")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Log),
            PathBuf::from("/xdg/state")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Runtime),
            PathBuf::from("/run/user/1000")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::SharedData),
            PathBuf::from("/xdg/data")
        );
    }

    #[test]
    fn unset_xdg_variables_fall_back_to_spec_defaults() {
        let facts = native_platform_directory_facts_from(
            TargetPlatform::Linux,
            &linux_home("/home/example"),
        );

        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Config),
            PathBuf::from("/home/example/.config")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Data),
            PathBuf::from("/home/example/.local/share")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::State),
            PathBuf::from("/home/example/.local/state")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Cache),
            PathBuf::from("/home/example/.cache")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::Log),
            PathBuf::from("/home/example/.local/state")
        );
        assert_eq!(
            fact(&facts, PlatformDirectoryFact::SharedData),
            PathBuf::from("/home/example/.local/share")
        );
    }

    #[test]
    fn xdg_runtime_dir_has_no_default_and_relative_entries_are_ignored() {
        let unset = native_platform_directory_facts_from(
            TargetPlatform::Linux,
            &linux_home("/home/example"),
        );
        assert_eq!(unset.get(PlatformDirectoryFact::Runtime), None);

        let relative = native_platform_directory_facts_from(
            TargetPlatform::Linux,
            &linux_home("/home/example").with_xdg_runtime_dir(home("run/user")),
        );
        assert_eq!(relative.get(PlatformDirectoryFact::Runtime), None);

        let relative_home = native_platform_directory_facts_from(
            TargetPlatform::Linux,
            &linux_home("home/example"),
        );
        assert_eq!(relative_home.get(PlatformDirectoryFact::Config), None);

        let empty_home = native_platform_directory_facts_from(
            TargetPlatform::Linux,
            &PlatformEnvironment::default().with_home_dir(Some(PathBuf::new())),
        );
        assert_eq!(empty_home.get(PlatformDirectoryFact::Data), None);
    }

    #[test]
    fn windows_maps_local_app_data_and_roaming_shared_data() {
        let environment = PlatformEnvironment::default()
            .with_temp_dir(home("/windows/Temp"))
            .with_windows_local_app_data(home("/windows/LocalAppData"))
            .with_windows_roaming_app_data(home("/windows/RoamingAppData"));
        let facts = native_platform_directory_facts_from(TargetPlatform::Windows, &environment);

        for fact_kind in [
            PlatformDirectoryFact::Config,
            PlatformDirectoryFact::Data,
            PlatformDirectoryFact::State,
            PlatformDirectoryFact::Cache,
            PlatformDirectoryFact::Log,
        ] {
            assert_eq!(
                facts.get(fact_kind),
                Some(Path::new("/windows/LocalAppData")),
                "{fact_kind:?}"
            );
        }
        assert_eq!(
            facts.get(PlatformDirectoryFact::Runtime),
            Some(Path::new("/windows/Temp"))
        );
        assert_eq!(
            facts.get(PlatformDirectoryFact::SharedData),
            Some(Path::new("/windows/RoamingAppData"))
        );
    }

    #[test]
    fn missing_windows_variables_report_unavailable() {
        let facts = native_platform_directory_facts_from(
            TargetPlatform::Windows,
            &PlatformEnvironment::default().with_temp_dir(home("/windows/Temp")),
        );

        for unavailable in [
            PlatformDirectoryFact::Config,
            PlatformDirectoryFact::Data,
            PlatformDirectoryFact::State,
            PlatformDirectoryFact::Cache,
            PlatformDirectoryFact::Log,
            PlatformDirectoryFact::SharedData,
        ] {
            assert_eq!(facts.get(unavailable), None, "{unavailable:?} invented");
        }
    }

    #[test]
    fn lazy_fact_matches_the_eager_provider_for_existing_environment_shapes() {
        let snapshots = [
            (TargetPlatform::MacOs, macos_home("/Users/example")),
            (
                TargetPlatform::MacOs,
                PlatformEnvironment::default().with_temp_dir(home("/private/tmp")),
            ),
            (
                TargetPlatform::Linux,
                linux_home("/home/example")
                    .with_xdg_config_home(home("/xdg/config"))
                    .with_xdg_data_home(home("/xdg/data"))
                    .with_xdg_state_home(home("/xdg/state"))
                    .with_xdg_cache_home(home("/xdg/cache"))
                    .with_xdg_runtime_dir(home("/run/user/1000")),
            ),
            (TargetPlatform::Linux, linux_home("/home/example")),
            (
                TargetPlatform::Linux,
                linux_home("/home/example").with_xdg_runtime_dir(home("run/user")),
            ),
            (TargetPlatform::Linux, linux_home("home/example")),
            (
                TargetPlatform::Linux,
                PlatformEnvironment::default().with_home_dir(Some(PathBuf::new())),
            ),
            (
                TargetPlatform::Windows,
                PlatformEnvironment::default()
                    .with_temp_dir(home("/windows/Temp"))
                    .with_windows_local_app_data(home("/windows/LocalAppData"))
                    .with_windows_roaming_app_data(home("/windows/RoamingAppData")),
            ),
            (
                TargetPlatform::Windows,
                PlatformEnvironment::default().with_temp_dir(home("/windows/Temp")),
            ),
        ];

        for (platform, environment) in snapshots {
            let eager = native_platform_directory_facts_from(platform, &environment);
            for fact in [
                PlatformDirectoryFact::Config,
                PlatformDirectoryFact::Data,
                PlatformDirectoryFact::State,
                PlatformDirectoryFact::Cache,
                PlatformDirectoryFact::Log,
                PlatformDirectoryFact::Runtime,
                PlatformDirectoryFact::SharedData,
            ] {
                let lazy = native_platform_directory_fact_from(platform, fact, &environment);
                assert_eq!(lazy.get(fact), eager.get(fact), "{platform:?} {fact:?}");
                for other in [
                    PlatformDirectoryFact::Config,
                    PlatformDirectoryFact::Data,
                    PlatformDirectoryFact::State,
                    PlatformDirectoryFact::Cache,
                    PlatformDirectoryFact::Log,
                    PlatformDirectoryFact::Runtime,
                    PlatformDirectoryFact::SharedData,
                ] {
                    if other != fact {
                        assert_eq!(lazy.get(other), None, "lazy call returned {other:?}");
                    }
                }
            }
        }
    }
}
