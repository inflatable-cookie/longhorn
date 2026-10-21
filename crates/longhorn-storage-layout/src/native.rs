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
    let platform = match std::env::consts::OS {
        "macos" => TargetPlatform::MacOs,
        "windows" => TargetPlatform::Windows,
        "linux" => TargetPlatform::Linux,
        _ => return Err(NativePlatformFactsError::UnsupportedPlatform),
    };
    Ok(native_platform_directory_facts_from(
        platform,
        &PlatformEnvironment::from_process(),
    ))
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
    match platform {
        TargetPlatform::MacOs => macos_facts(environment),
        TargetPlatform::Windows => windows_facts(environment),
        TargetPlatform::Linux => linux_facts(environment),
    }
}

fn macos_facts(environment: &PlatformEnvironment) -> PlatformDirectoryFacts {
    let home = environment.home();
    let support = home.map(|home| home.join("Library").join("Application Support"));
    let cache = home.map(|home| home.join("Library").join("Caches"));
    let log = home.map(|home| home.join("Library").join("Logs"));
    let runtime = absolute(environment.temp_dir.as_ref());
    facts_with(
        TargetPlatform::MacOs,
        support.clone(),
        support.clone(),
        support.clone(),
        cache,
        log,
        runtime,
        support,
    )
}

fn windows_facts(environment: &PlatformEnvironment) -> PlatformDirectoryFacts {
    let local = absolute(environment.windows_local_app_data.as_ref());
    let roaming = absolute(environment.windows_roaming_app_data.as_ref());
    let runtime = absolute(environment.temp_dir.as_ref());
    facts_with(
        TargetPlatform::Windows,
        local.clone(),
        local.clone(),
        local.clone(),
        local.clone(),
        local,
        runtime,
        roaming,
    )
}

fn linux_facts(environment: &PlatformEnvironment) -> PlatformDirectoryFacts {
    let config = environment.xdg(environment.xdg_config_home.as_ref(), |home| {
        home.join(".config")
    });
    let data = environment.xdg(environment.xdg_data_home.as_ref(), |home| {
        home.join(".local").join("share")
    });
    let state = environment.xdg(environment.xdg_state_home.as_ref(), |home| {
        home.join(".local").join("state")
    });
    let cache = environment.xdg(environment.xdg_cache_home.as_ref(), |home| {
        home.join(".cache")
    });
    facts_with(
        TargetPlatform::Linux,
        config,
        data.clone(),
        state.clone(),
        cache,
        state,
        absolute(environment.xdg_runtime_dir.as_ref()),
        data,
    )
}

fn absolute(value: Option<&PathBuf>) -> Option<PathBuf> {
    value
        .map(PathBuf::as_path)
        .filter(|path| !path.as_os_str().is_empty() && path.is_absolute())
        .map(Path::to_path_buf)
}

#[allow(clippy::too_many_arguments)]
fn facts_with(
    platform: TargetPlatform,
    config: Option<PathBuf>,
    data: Option<PathBuf>,
    state: Option<PathBuf>,
    cache: Option<PathBuf>,
    log: Option<PathBuf>,
    runtime: Option<PathBuf>,
    shared_data: Option<PathBuf>,
) -> PlatformDirectoryFacts {
    let mut facts = PlatformDirectoryFacts::new(platform);
    for (fact, path) in [
        (PlatformDirectoryFact::Config, config),
        (PlatformDirectoryFact::Data, data),
        (PlatformDirectoryFact::State, state),
        (PlatformDirectoryFact::Cache, cache),
        (PlatformDirectoryFact::Log, log),
        (PlatformDirectoryFact::Runtime, runtime),
        (PlatformDirectoryFact::SharedData, shared_data),
    ] {
        if let Some(path) = path {
            facts = facts.with(fact, path);
        }
    }
    facts
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
}
