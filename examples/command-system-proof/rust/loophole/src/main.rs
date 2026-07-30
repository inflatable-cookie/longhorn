use longhorn_command::{
    AdmittedCommandInvocation, CommandAdmissionEngine, CommandArgumentSchema, CommandAvailability,
    CommandAvailabilityReason, CommandAvailabilityReasonCode, CommandAvailabilitySource,
    CommandCapabilityDefinition, CommandCapabilitySnapshot, CommandCapabilitySource,
    CommandContextDefinition, CommandContextRevision, CommandContextSnapshot, CommandContextSource,
    CommandDefinition, CommandExecutionOutcome, CommandExecutionRequest, CommandExecutor,
    CommandExecutorOutcome, CommandKeyword, CommandLimits, CommandRegistry, CommandRegistryBuilder,
    CommandRegistryGeneration, CommandSourceFailure, CommandTextInputPolicy, CommandVisibility,
};
use longhorn_command_settings::{
    COMMAND_CATALOGUE_CAPABILITY_ID, KEYBINDING_SETTINGS_PAGE_ID, WRITABLE_KEYMAP_CAPABILITY_ID,
    register_command_settings,
};
use longhorn_core::{
    CommandAvailabilityReasonId, CommandCapabilityId, CommandCategoryId, CommandContextId,
    CommandId, CommandRequestId, CommandRouteId, SettingsCapabilityId,
};
use longhorn_settings::{SettingsLimits, SettingsRegistryBuilder, SettingsRegistryGeneration};
use serde_json::{Value, json};

fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value.parse().expect("proof id")
}

fn command(
    command_id: &str,
    label: &str,
    context_id: &str,
    capability_id: &str,
    route: &str,
) -> CommandDefinition {
    CommandDefinition {
        id: id::<CommandId>(command_id),
        label: label.into(),
        description: Some(format!("{label} proof command")),
        category_path: vec![id::<CommandCategoryId>("commands")],
        keywords: vec![CommandKeyword::new(label.to_lowercase()).expect("keyword")],
        icon: None,
        allowed_contexts: vec![id::<CommandContextId>(context_id)],
        required_capabilities: vec![id::<CommandCapabilityId>(capability_id)],
        visibility: CommandVisibility::ALL,
        text_input_policy: CommandTextInputPolicy::Blocked,
        route: id::<CommandRouteId>(route),
        arguments: CommandArgumentSchema::None,
    }
}

fn registry() -> CommandRegistry {
    let mut builder =
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, CommandLimits::default());
    for (context_id, parent_id) in [
        ("global", None),
        ("project", Some("global")),
        ("surface", Some("project")),
        ("region", Some("surface")),
        ("panel", Some("region")),
    ] {
        builder
            .register_context(CommandContextDefinition {
                id: id::<CommandContextId>(context_id),
                parent_id: parent_id.map(id::<CommandContextId>),
            })
            .expect("context");
    }
    for capability_id in ["transport", "editing", "panels"] {
        builder
            .register_capability(CommandCapabilityDefinition {
                id: id::<CommandCapabilityId>(capability_id),
            })
            .expect("capability");
    }
    for definition in [
        command(
            "loophole:transport.play",
            "Play",
            "project",
            "transport",
            "domain:transport.play",
        ),
        command(
            "loophole:editor.quantize",
            "Quantize",
            "surface",
            "editing",
            "domain:editor.quantize",
        ),
        command(
            "loophole:panel.close",
            "Close Panel",
            "panel",
            "panels",
            "local:panel.close",
        ),
    ] {
        builder.register_command(definition).expect("command");
    }
    builder.seal().expect("sealed Loophole registry")
}

#[derive(Clone)]
struct ContextFeed(CommandContextSnapshot);

impl CommandContextSource for ContextFeed {
    fn current_context(&mut self) -> Result<CommandContextSnapshot, CommandSourceFailure> {
        Ok(self.0.clone())
    }
}

#[derive(Clone)]
struct CapabilityFeed(CommandCapabilitySnapshot);

impl CommandCapabilitySource for CapabilityFeed {
    fn current_capabilities(&mut self) -> Result<CommandCapabilitySnapshot, CommandSourceFailure> {
        Ok(self.0.clone())
    }
}

struct AvailabilityFeed {
    available: bool,
}

impl CommandAvailabilitySource for AvailabilityFeed {
    fn availability(
        &mut self,
        _command: &CommandDefinition,
        _context: &CommandContextSnapshot,
        _capabilities: &CommandCapabilitySnapshot,
    ) -> Result<CommandAvailability, CommandSourceFailure> {
        Ok(if self.available {
            CommandAvailability::available()
        } else {
            CommandAvailability::unavailable(CommandAvailabilityReason::new(
                CommandAvailabilityReasonCode::Consumer(id::<CommandAvailabilityReasonId>(
                    "loophole:no-project",
                )),
                None,
            ))
        })
    }
}

#[derive(Default)]
struct RouteExecutor {
    routes: Vec<String>,
}

impl CommandExecutor for RouteExecutor {
    fn execute(&mut self, invocation: &AdmittedCommandInvocation) -> CommandExecutorOutcome {
        self.routes.push(invocation.route().as_str().to_owned());
        CommandExecutorOutcome::Succeeded { evidence: None }
    }
}

fn context(revision: u64, path: &[&str]) -> CommandContextSnapshot {
    CommandContextSnapshot::new(
        CommandContextRevision::new(revision),
        path.iter()
            .map(|value| id::<CommandContextId>(value))
            .collect(),
    )
    .expect("context snapshot")
}

fn request(
    registry: &CommandRegistry,
    request_id: &str,
    command_id: &str,
) -> CommandExecutionRequest {
    CommandExecutionRequest {
        request_id: id::<CommandRequestId>(request_id),
        registry_generation: registry.generation(),
        command_id: id::<CommandId>(command_id),
        arguments: Value::Null,
    }
}

fn main() {
    let registry = registry();
    let engine = CommandAdmissionEngine::new(&registry);
    let mut capabilities = CapabilityFeed(
        CommandCapabilitySnapshot::new(
            ["transport", "editing", "panels"].map(id::<CommandCapabilityId>),
        )
        .expect("capability snapshot"),
    );
    let mut availability = AvailabilityFeed { available: true };
    let mut executor = RouteExecutor::default();

    let mut contexts = ContextFeed(context(2, &["global"]));
    let stale = engine.execute(
        request(&registry, "loophole:request.stale", "loophole:panel.close"),
        &mut contexts,
        &mut capabilities,
        &mut availability,
        &mut executor,
    );
    assert!(matches!(
        stale.outcome(),
        CommandExecutionOutcome::Unavailable { .. }
    ));
    assert!(executor.routes.is_empty());

    contexts.0 = context(3, &["global", "project"]);
    availability.available = false;
    let stale_availability = engine.execute(
        request(
            &registry,
            "loophole:request.unavailable",
            "loophole:transport.play",
        ),
        &mut contexts,
        &mut capabilities,
        &mut availability,
        &mut executor,
    );
    assert!(matches!(
        stale_availability.outcome(),
        CommandExecutionOutcome::Unavailable { .. }
    ));
    assert!(executor.routes.is_empty());

    availability.available = true;
    contexts.0 = context(4, &["global", "project", "surface", "region", "panel"]);
    let local = engine.execute(
        request(&registry, "loophole:request.local", "loophole:panel.close"),
        &mut contexts,
        &mut capabilities,
        &mut availability,
        &mut executor,
    );
    assert!(matches!(
        local.outcome(),
        CommandExecutionOutcome::Succeeded { .. }
    ));

    contexts.0 = context(5, &["global", "project"]);
    let typed = engine.execute(
        request(
            &registry,
            "loophole:request.domain",
            "loophole:transport.play",
        ),
        &mut contexts,
        &mut capabilities,
        &mut availability,
        &mut executor,
    );
    assert!(matches!(
        typed.outcome(),
        CommandExecutionOutcome::Succeeded { .. }
    ));

    let mut settings = SettingsRegistryBuilder::new(
        SettingsRegistryGeneration::INITIAL,
        SettingsLimits::default(),
    );
    register_command_settings(&mut settings).expect("command settings registration");
    let settings = settings
        .seal([
            id::<SettingsCapabilityId>(COMMAND_CATALOGUE_CAPABILITY_ID),
            id::<SettingsCapabilityId>(WRITABLE_KEYMAP_CAPABILITY_ID),
        ])
        .expect("settings registry");
    let settings_admitted = settings
        .pages()
        .any(|page| page.id.as_str() == KEYBINDING_SETTINGS_PAGE_ID);
    assert!(settings_admitted);

    let _ = longhorn_command_config::CommandKeymapProtocolVersion::CURRENT;
    let _ = longhorn_tauri_command::COMMAND_CATALOGUE_CHANGED_EVENT;

    println!(
        "{}",
        json!({
            "shape": "loophole",
            "contexts": registry.contexts().count(),
            "commands": registry.commands().map(|command| command.id.as_str()).collect::<Vec<_>>(),
            "staleContextRejected": true,
            "staleAvailabilityRejected": true,
            "executorRoutes": executor.routes,
            "settingsAdmitted": settings_admitted,
            "tauriEvent": longhorn_tauri_command::COMMAND_CATALOGUE_CHANGED_EVENT,
        })
    );
}
