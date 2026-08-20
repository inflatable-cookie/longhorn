//! Longhorn's own agent-control proof composition (Cards 230-231, 238).
//!
//! A minimal Tauri app that composes `longhorn-tauri-agent-control` behind
//! its `dev` feature — this app exists to prove the dev control surface and
//! is never shipped. Its contract-006 registry registers window-state
//! commands so the packaged freshness matrix can minimize and restore the
//! window through the same `command` tool an agent would use, plus a `ping`
//! command proving the registry round trip.
//!
//! Card 238 attaches a child webview (`preview`, the `island.html` page with
//! its own 1 Hz hue ticker at a 97° stride) to the main window — the
//! native-content-island shape from the Figmatic adoption finding. The
//! island is deliberately oversized so its right and bottom edges clip at
//! the window viewport; the freshness matrix judges both surfaces' pixels.
//! With the child attached, `webview_windows()` no longer lists `main`, so
//! every host-side lookup here goes through `get_window` (the same shape
//! `c1482daf` adopted in the plugin).

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use longhorn_command::{
    AdmittedCommandInvocation, CommandAdmissionEngine, CommandArgumentSchema, CommandAvailability,
    CommandAvailabilitySource, CommandCapabilityDefinition, CommandCapabilitySnapshot,
    CommandCapabilitySource, CommandContextDefinition, CommandContextRevision,
    CommandContextSnapshot, CommandContextSource, CommandDefinition, CommandEvidence,
    CommandExecutionOutcome, CommandExecutionRequest, CommandExecutor, CommandExecutorOutcome,
    CommandKeyword, CommandLimits, CommandRegistry, CommandRegistryBuilder,
    CommandRegistryGeneration, CommandSourceFailure, CommandTextInputPolicy, CommandVisibility,
};
use longhorn_core::{
    CommandCapabilityId, CommandCategoryId, CommandContextId, CommandEvidenceCode, CommandId,
    CommandRequestId, CommandRouteId,
};
use longhorn_tauri_agent_control::{
    AgentControlConfig, AgentControlHandle, CommandBridge, ToolError, mount_agent_control,
};
use serde_json::Value;
use tauri::{AppHandle, Manager, RunEvent};

/// Discovery and bundle identity for the proof instance.
const APP_ID: &str = "dev.example.longhorn-agent-control-proof";

fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value.parse().expect("proof id")
}

fn command(command_id: &str, label: &str, route: &str) -> CommandDefinition {
    CommandDefinition {
        id: id::<CommandId>(command_id),
        label: label.into(),
        description: Some(format!("{label} proof command")),
        category_path: vec![id::<CommandCategoryId>("commands")],
        keywords: vec![CommandKeyword::new(label.to_lowercase()).expect("keyword")],
        icon: None,
        allowed_contexts: vec![id::<CommandContextId>("global")],
        required_capabilities: vec![id::<CommandCapabilityId>("window")],
        visibility: CommandVisibility::ALL,
        text_input_policy: CommandTextInputPolicy::Blocked,
        route: id::<CommandRouteId>(route),
        arguments: CommandArgumentSchema::None,
    }
}

/// The proof registry: window-state commands the freshness matrix drives,
/// plus `ping` for the plain registry round trip.
fn proof_registry() -> CommandRegistry {
    let mut builder =
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, CommandLimits::default());
    builder
        .register_context(CommandContextDefinition {
            id: id::<CommandContextId>("global"),
            parent_id: None,
        })
        .expect("context");
    builder
        .register_capability(CommandCapabilityDefinition {
            id: id::<CommandCapabilityId>("window"),
        })
        .expect("capability");
    for definition in [
        command("proof:ping", "Ping", "proof:ping"),
        command("proof:window.minimize", "Minimize", "host:window.minimize"),
        command("proof:window.restore", "Restore", "host:window.restore"),
    ] {
        builder.register_command(definition).expect("command");
    }
    builder.seal().expect("sealed proof registry")
}

struct StaticContext(CommandContextSnapshot);

impl CommandContextSource for StaticContext {
    fn current_context(&mut self) -> Result<CommandContextSnapshot, CommandSourceFailure> {
        Ok(self.0.clone())
    }
}

struct StaticCapabilities(CommandCapabilitySnapshot);

impl CommandCapabilitySource for StaticCapabilities {
    fn current_capabilities(&mut self) -> Result<CommandCapabilitySnapshot, CommandSourceFailure> {
        Ok(self.0.clone())
    }
}

struct AlwaysAvailable;

impl CommandAvailabilitySource for AlwaysAvailable {
    fn availability(
        &mut self,
        _command: &CommandDefinition,
        _context: &CommandContextSnapshot,
        _capabilities: &CommandCapabilitySnapshot,
    ) -> Result<CommandAvailability, CommandSourceFailure> {
        Ok(CommandAvailability::available())
    }
}

/// Routes admitted invocations to the main window's native state changes.
struct ProofExecutor {
    app: AppHandle,
}

impl CommandExecutor for ProofExecutor {
    fn execute(&mut self, invocation: &AdmittedCommandInvocation) -> CommandExecutorOutcome {
        // `get_window`, not `get_webview_window`: with the preview island
        // attached, `main` hosts two webviews and tauri's `webview_windows`
        // map drops it.
        let outcome = match invocation.route().as_str() {
            "proof:ping" => Ok(()),
            "host:window.minimize" => self
                .app
                .get_window("main")
                .ok_or_else(|| "no main window".to_owned())
                .and_then(|window| window.minimize().map_err(|error| error.to_string())),
            "host:window.restore" => self
                .app
                .get_window("main")
                .ok_or_else(|| "no main window".to_owned())
                .and_then(|window| window.unminimize().map_err(|error| error.to_string())),
            route => Err(format!("unrouted command route {route:?}")),
        };
        match outcome {
            Ok(()) => CommandExecutorOutcome::Succeeded { evidence: None },
            Err(_message) => CommandExecutorOutcome::Failed {
                evidence: Some(CommandEvidence::new(
                    id::<CommandEvidenceCode>("proof:window-op-failed"),
                    None,
                )),
            },
        }
    }
}

/// The control surface's bridge into the proof registry.
struct ProofCommandBridge {
    registry: CommandRegistry,
    requests: AtomicU64,
    app: Mutex<Option<AppHandle>>,
}

impl ProofCommandBridge {
    fn new(registry: CommandRegistry) -> Self {
        Self {
            registry,
            requests: AtomicU64::new(0),
            app: Mutex::new(None),
        }
    }

    fn install_handle(&self, app: AppHandle) {
        *self.app.lock().expect("bridge handle poisoned") = Some(app);
    }
}

impl CommandBridge for ProofCommandBridge {
    fn invoke_command(
        &self,
        command: &CommandId,
        argument: Option<Value>,
    ) -> Result<Option<Value>, ToolError> {
        let failure = |message: String| ToolError::CommandFailed {
            command: command.clone(),
            message,
        };
        let app = self
            .app
            .lock()
            .expect("bridge handle poisoned")
            .clone()
            .ok_or_else(|| failure("the bridge has no app handle yet".to_owned()))?;
        let sequence = self.requests.fetch_add(1, Ordering::SeqCst);
        let request = CommandExecutionRequest {
            request_id: CommandRequestId::new(format!("agent-control-{sequence}"))
                .map_err(|error| failure(format!("request id: {error}")))?,
            registry_generation: self.registry.generation(),
            command_id: command.clone(),
            arguments: argument.unwrap_or(Value::Null),
        };
        let engine = CommandAdmissionEngine::new(&self.registry);
        let mut contexts = StaticContext(
            CommandContextSnapshot::new(
                CommandContextRevision::new(sequence + 1),
                vec![id::<CommandContextId>("global")],
            )
            .map_err(|error| failure(format!("context snapshot: {error}")))?,
        );
        let mut capabilities = StaticCapabilities(
            CommandCapabilitySnapshot::new([id::<CommandCapabilityId>("window")])
                .map_err(|error| failure(format!("capability snapshot: {error}")))?,
        );
        let result = engine.execute(
            request,
            &mut contexts,
            &mut capabilities,
            &mut AlwaysAvailable,
            &mut ProofExecutor { app },
        );
        match result.outcome() {
            CommandExecutionOutcome::Succeeded { .. } => Ok(None),
            outcome => Err(failure(format!("{outcome:?}"))),
        }
    }
}

/// Keeps the mounted control server reachable at exit.
struct ProofState {
    agent_control: Mutex<Option<AgentControlHandle>>,
}

fn main() {
    let bridge = Arc::new(ProofCommandBridge::new(proof_registry()));
    let app = tauri::Builder::default()
        .setup(move |app| {
            bridge.install_handle(app.handle().clone());
            let agent_control =
                mount_agent_control(app.handle(), AgentControlConfig::new(APP_ID), bridge)?;
            app.manage(ProofState {
                agent_control: Mutex::new(Some(agent_control)),
            });
            // Card 238: attach the preview island child webview. Oversized
            // on purpose — the window is 720x480 logical, so the island's
            // right and bottom edges clip at the viewport.
            let main_window = app.get_window("main").expect("main window");
            main_window
                .add_child(
                    tauri::webview::WebviewBuilder::new(
                        "preview",
                        tauri::WebviewUrl::App("island.html".into()),
                    ),
                    tauri::LogicalPosition::new(360.0, 120.0),
                    tauri::LogicalSize::new(400.0, 400.0),
                )
                .expect("attach preview island");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("could not build the agent-control proof app");
    app.run(|app, event| {
        // macOS quit delivers `Exit` without a preceding `ExitRequested`,
        // so shutdown hooks both; the `Option::take` makes the second fire
        // a no-op.
        if let RunEvent::ExitRequested { .. } | RunEvent::Exit = event
            && let Some(state) = app.try_state::<ProofState>()
            && let Some(agent_control) = state.agent_control.lock().expect("state poisoned").take()
        {
            // Clean exit removes the discovery file; a crash leaves it
            // stale-detectable by dead pid (contract 022).
            let _ = agent_control.shutdown();
        }
    });
}
