//! Composition entry point: mounts the control server inside the app.
//!
//! [`mount_agent_control`] resolves the discovery directory through the
//! contract 004 storage resolver at the Tauri path edge, spawns a background
//! thread owning a tokio runtime, and serves the core
//! `serve_control_surface` there — token generation, 127.0.0.1 ephemeral
//! bind, discovery publication, and clean-exit removal are the core's
//! lifecycle, not reimplemented here. The returned [`AgentControlHandle`] is
//! the app's shutdown lever: call [`AgentControlHandle::shutdown`] from the
//! run-event callback — hook `RunEvent::Exit` (and `ExitRequested` where it
//! fires; a macOS quit delivers `Exit` alone) — so a clean exit removes the
//! discovery file; a crash leaves it stale-detectable by dead pid (contract
//! 022).

use std::{
    collections::BTreeSet,
    error::Error,
    fmt, io,
    path::PathBuf,
    sync::Arc,
    thread::{self, JoinHandle},
};

use longhorn_agent_control::{
    ControlServerConfig, DiscoveryError, SelectionRegistry, ServeError, ServeReceipt,
    resolve_discovery_dir, resolve_discovery_dir_with_state_override, serve_control_surface,
};
use longhorn_tauri_config::{TauriDirectorySnapshot, platform_directory_facts};
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::oneshot;

use crate::{bridge::CommandBridge, handler::TauriControlHandler, shim::SHIM_SOURCE};

/// Tauri directory lookup that can fail while mounting agent control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AgentControlPathOperation {
    /// Resolve the configuration directory.
    ConfigDir,
    /// Resolve the local data directory.
    LocalDataDir,
    /// Resolve the roaming data directory.
    RoamingDataDir,
    /// Resolve the cache directory.
    CacheDir,
    /// Resolve the home directory.
    HomeDir,
    /// Resolve the temporary directory.
    TempDir,
}

impl fmt::Display for AgentControlPathOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::ConfigDir => "config_dir",
            Self::LocalDataDir => "local_data_dir",
            Self::RoamingDataDir => "roaming_data_dir",
            Self::CacheDir => "cache_dir",
            Self::HomeDir => "home_dir",
            Self::TempDir => "temp_dir",
        };
        formatter.write_str(name)
    }
}

fn agent_control_plugin<R: Runtime>(registry: SelectionRegistry) -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("longhorn-agent-control")
        .setup(move |app, _api| {
            app.manage(registry);
            Ok(())
        })
        // Runs at document-start for webviews created after plugin registration.
        .js_init_script(SHIM_SOURCE.to_owned())
        // Webviews created before `mount_agent_control` (e.g. from tauri.conf.json)
        // do not receive retroactive plugin init scripts. On Wry/WKWebView,
        // `Started` fires at navigation commit so evaluating on every load
        // event installs the ring before document scripts finish; `Finished`
        // covers any late context and `installAgentControlShim` is idempotent.
        .on_page_load(|webview, _payload| {
            let _ = webview.eval(SHIM_SOURCE);
        })
        .build()
}

/// What one mount needs from the composing app.
#[derive(Clone, Debug)]
pub struct AgentControlConfig {
    /// Canonical application id written to the discovery file.
    pub app_id: String,
    /// Loopback port to bind; 0 (the default) asks the OS for an ephemeral
    /// port, published through the discovery file.
    pub port: u16,
    /// Explicit state-root override for the discovery directory —
    /// deployment and test policy per contract 004, recorded by the
    /// resolver as an explicit override rather than a parallel root.
    /// `None` (the default) resolves the platform state root.
    pub state_root: Option<PathBuf>,
    /// Child-webview labels opted in as semantic targets. Empty (the
    /// default) is today's UI-webview-only behavior. The set is fixed at
    /// mount; there is no runtime mutation.
    pub semantic_children: BTreeSet<String>,
}

impl AgentControlConfig {
    /// Config for `app_id` on an ephemeral port with the platform state root.
    #[must_use]
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            port: 0,
            state_root: None,
            semantic_children: BTreeSet::new(),
        }
    }

    /// Pins the loopback port instead of asking for an ephemeral one.
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Overrides the discovery state root (deployment/test policy).
    #[must_use]
    pub fn with_state_root(mut self, state_root: PathBuf) -> Self {
        self.state_root = Some(state_root);
        self
    }

    /// Names one child-webview label as a semantic target. Repeatable.
    /// The label is the host's own webview label; it must match a hosted
    /// webview at call time. Opting in asserts the child's content is the
    /// app's own to drive.
    #[must_use]
    pub fn with_semantic_child(mut self, label: impl Into<String>) -> Self {
        self.semantic_children.insert(label.into());
        self
    }
}

/// Mount-time failure: the discovery directory could not be resolved before
/// the server thread was spawned.
#[derive(Debug)]
pub enum AgentControlMountError {
    /// A Tauri path-edge lookup failed.
    Path {
        /// Which directory lookup failed.
        operation: AgentControlPathOperation,
        /// Underlying Tauri error.
        source: tauri::Error,
    },
    /// Discovery directory resolution rejected the resolved facts.
    Discovery(DiscoveryError),
    /// Registering the Tauri plugin failed (selection registry and init script
    /// would not be installed).
    Plugin(tauri::Error),
}

impl fmt::Display for AgentControlMountError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path { operation, source } => {
                write!(formatter, "tauri path lookup {operation} failed: {source}")
            }
            Self::Discovery(source) => write!(formatter, "{source}"),
            Self::Plugin(source) => {
                write!(
                    formatter,
                    "agent-control plugin registration failed: {source}"
                )
            }
        }
    }
}

impl Error for AgentControlMountError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Path { source, .. } | Self::Plugin(source) => Some(source),
            Self::Discovery(source) => Some(source),
        }
    }
}

/// Shutdown-time failure: the server thread could not be signaled or the
/// serve run itself failed (including discovery removal on the way out).
#[derive(Debug)]
pub enum AgentControlShutdownError {
    /// The handle had no server thread left to join. `shutdown` consumes the
    /// handle, so this is not produced by a server that exited early: that
    /// case joins and reports the thread's own result.
    AlreadyShutdown,
    /// The server thread panicked.
    ThreadPanicked,
    /// The serve run failed; carries the core's typed error.
    Serve(ServeError),
}

impl fmt::Display for AgentControlShutdownError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyShutdown => write!(formatter, "control server shutdown already began"),
            Self::ThreadPanicked => write!(formatter, "control server thread panicked"),
            Self::Serve(source) => write!(formatter, "{source}"),
        }
    }
}

impl Error for AgentControlShutdownError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serve(source) => Some(source),
            Self::AlreadyShutdown | Self::ThreadPanicked => None,
        }
    }
}

/// The app's lever on the mounted control server.
///
/// Dropping the handle without [`AgentControlHandle::shutdown`] detaches the
/// server thread: the process exit then strands the discovery file, which
/// the stale-pid path covers. Clean shutdown is the honest path — take it
/// from the app's run-event callback. Hook `RunEvent::Exit` (and
/// `ExitRequested` where it fires): on macOS a quit delivers `Exit`
/// without a preceding `ExitRequested`, so `ExitRequested` alone misses
/// the removal.
pub struct AgentControlHandle {
    shutdown: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<Result<ServeReceipt, ServeError>>>,
}

impl AgentControlHandle {
    /// Signals graceful shutdown and joins the server thread, returning what
    /// the completed run bound and served.
    ///
    /// Always joins. A startup failure, such as a bind on a port already in
    /// use, ends the server thread before any shutdown is requested; the join
    /// is how that original failure reaches the caller.
    ///
    /// Blocks until the server finishes: axum's graceful shutdown drains
    /// in-flight requests, then the core removes the discovery file before
    /// the thread returns. Call this from the run-event callback — hook
    /// `RunEvent::Exit` (and `ExitRequested` where it fires) — before the
    /// process exits.
    pub fn shutdown(mut self) -> Result<ServeReceipt, AgentControlShutdownError> {
        // The receiver is gone once the server thread has returned, so a
        // failed send is expected after a startup failure. The join below
        // reports the thread's real outcome, so the send result is not
        // needed.
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        let thread = self
            .thread
            .take()
            .ok_or(AgentControlShutdownError::AlreadyShutdown)?;
        thread
            .join()
            .map_err(|_| AgentControlShutdownError::ThreadPanicked)?
            .map_err(AgentControlShutdownError::Serve)
    }
}

/// Mounts the control server inside the app and returns its shutdown handle.
///
/// The server binds 127.0.0.1 only, generates its per-instance token, and
/// publishes the discovery file once bound; all of that happens on the
/// spawned thread, so a bind or token failure surfaces at
/// [`AgentControlHandle::shutdown`], not here. Mount-time failures are
/// limited to resolving the discovery directory.
pub fn mount_agent_control<R: Runtime>(
    app: &AppHandle<R>,
    config: AgentControlConfig,
    commands: Arc<dyn CommandBridge>,
) -> Result<AgentControlHandle, AgentControlMountError> {
    let facts = platform_directory_facts(directory_snapshot(app)?);
    let discovery_dir = match &config.state_root {
        Some(state_root) => resolve_discovery_dir_with_state_override(&facts, state_root),
        None => resolve_discovery_dir(&facts),
    }
    .map_err(AgentControlMountError::Discovery)?;

    let registry = SelectionRegistry::new(format!("{}:{}", config.app_id, std::process::id()));

    // Initialization script for windows created after this mount; eval for
    // windows that already exist (the proof app and the mount fixtures).
    app.plugin(agent_control_plugin(registry.clone()))
        .map_err(AgentControlMountError::Plugin)?;
    // Walk every webview of every window: `webview_windows()` excludes any
    // window hosting a child webview with a different label (Figmatic
    // adoption finding, 2026-08-20).
    for (_, window) in app.windows() {
        for webview in window.webviews() {
            let _ = webview.eval(SHIM_SOURCE);
        }
    }

    let handler = TauriControlHandler::new(app.clone(), commands, config.semantic_children);
    let server_config = ControlServerConfig {
        app_id: config.app_id,
        discovery_dir,
        port: config.port,
    };
    let registry_for_server = registry.clone();
    Ok(spawn_server(move |signaled| {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(source) => {
                return Err(ServeError::Io(io::Error::other(format!(
                    "tokio runtime must build: {source}"
                ))));
            }
        };
        runtime.block_on(serve_control_surface(
            server_config,
            handler,
            registry_for_server,
            async move {
                let _ = signaled.await;
            },
        ))
    }))
}

/// Spawns the server thread and pairs it with the handle that shuts it down.
///
/// `serve` runs on the thread and receives the shutdown signal. Whatever it
/// returns, startup failure included, is what [`AgentControlHandle::shutdown`]
/// reports once it joins the thread.
fn spawn_server<Serve>(serve: Serve) -> AgentControlHandle
where
    Serve: FnOnce(oneshot::Receiver<()>) -> Result<ServeReceipt, ServeError> + Send + 'static,
{
    let (shutdown, signaled) = oneshot::channel::<()>();
    AgentControlHandle {
        shutdown: Some(shutdown),
        thread: Some(thread::spawn(move || serve(signaled))),
    }
}

/// Reads the platform directory snapshot at the Tauri path edge.
fn directory_snapshot<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<TauriDirectorySnapshot, AgentControlMountError> {
    let path = app.path();
    #[cfg(target_os = "macos")]
    {
        Ok(TauriDirectorySnapshot::MacOs {
            local_data_dir: lookup(
                AgentControlPathOperation::LocalDataDir,
                path.local_data_dir(),
            )?,
            cache_dir: lookup(AgentControlPathOperation::CacheDir, path.cache_dir())?,
            home_dir: lookup(AgentControlPathOperation::HomeDir, path.home_dir())?,
            temp_dir: lookup(AgentControlPathOperation::TempDir, path.temp_dir())?,
        })
    }
    #[cfg(target_os = "windows")]
    {
        Ok(TauriDirectorySnapshot::Windows {
            local_data_dir: lookup(
                AgentControlPathOperation::LocalDataDir,
                path.local_data_dir(),
            )?,
            roaming_data_dir: lookup(
                AgentControlPathOperation::RoamingDataDir,
                path.roaming_data_dir(),
            )?,
            temp_dir: lookup(AgentControlPathOperation::TempDir, path.temp_dir())?,
        })
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Tauri has no XDG state lookup; honor XDG_STATE_HOME, then the
        // spec's default below the resolved home directory.
        let home_dir = lookup("home_dir", path.home_dir())?;
        let state_dir = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir.join(".local").join("state"));
        Ok(TauriDirectorySnapshot::Linux {
            config_dir: lookup(AgentControlPathOperation::ConfigDir, path.config_dir())?,
            local_data_dir: lookup(
                AgentControlPathOperation::LocalDataDir,
                path.local_data_dir(),
            )?,
            state_dir,
            cache_dir: lookup(AgentControlPathOperation::CacheDir, path.cache_dir())?,
            runtime_dir: path.runtime_dir().ok(),
        })
    }
}

fn lookup(
    operation: AgentControlPathOperation,
    result: Result<PathBuf, tauri::Error>,
) -> Result<PathBuf, AgentControlMountError> {
    result.map_err(|source| AgentControlMountError::Path { operation, source })
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        time::{Duration, Instant},
    };

    use super::*;

    #[test]
    fn path_operations_keep_their_labels() {
        let operations = [
            (AgentControlPathOperation::ConfigDir, "config_dir"),
            (AgentControlPathOperation::LocalDataDir, "local_data_dir"),
            (
                AgentControlPathOperation::RoamingDataDir,
                "roaming_data_dir",
            ),
            (AgentControlPathOperation::CacheDir, "cache_dir"),
            (AgentControlPathOperation::HomeDir, "home_dir"),
            (AgentControlPathOperation::TempDir, "temp_dir"),
        ];

        for (operation, label) in operations {
            assert_eq!(operation.to_string(), label);
            assert_eq!(
                format!("tauri path lookup {operation} failed"),
                format!("tauri path lookup {label} failed")
            );
        }
    }

    /// A bind failure ends the server thread before anyone asks it to stop.
    /// `shutdown` must still join it and report the original failure, not an
    /// already-shutdown error.
    #[test]
    fn shutdown_reports_startup_failure_after_the_thread_has_exited() {
        let handle = spawn_server(|_signaled| {
            Err(ServeError::Io(io::Error::new(
                io::ErrorKind::AddrInUse,
                "port taken",
            )))
        });
        wait_for_thread_exit(&handle);

        match handle.shutdown() {
            Err(AgentControlShutdownError::Serve(ServeError::Io(source))) => {
                assert_eq!(source.kind(), io::ErrorKind::AddrInUse);
            }
            other => panic!("expected the startup failure, got {other:?}"),
        }
    }

    /// The serve future owns the shutdown receiver, so a closed sender means
    /// the server thread has stopped serving.
    fn wait_for_thread_exit(handle: &AgentControlHandle) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !handle
            .shutdown
            .as_ref()
            .is_some_and(|shutdown| shutdown.is_closed())
        {
            assert!(Instant::now() < deadline, "server thread did not exit");
            thread::sleep(Duration::from_millis(5));
        }
    }
}
