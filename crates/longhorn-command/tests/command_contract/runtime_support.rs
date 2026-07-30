use std::collections::BTreeMap;

use longhorn_command::{
    AdmittedCommandInvocation, CommandArgumentField, CommandArgumentKind, CommandArgumentSchema,
    CommandAvailability, CommandAvailabilitySource, CommandCapabilityDefinition,
    CommandCapabilitySnapshot, CommandCapabilitySource, CommandContextDefinition,
    CommandContextRevision, CommandContextSnapshot, CommandContextSource, CommandDefinition,
    CommandEvidence, CommandExecutionRequest, CommandExecutor, CommandExecutorOutcome,
    CommandLimits, CommandRegistry, CommandRegistryBuilder, CommandRegistryGeneration,
    CommandSourceFailure, CommandTextInputPolicy, CommandVisibility,
};
use longhorn_core::{
    CommandCapabilityId, CommandCategoryId, CommandEvidenceCode, CommandFieldId, CommandId,
    CommandRequestId, CommandRouteId,
};
use serde_json::Value;

use super::support::{capability_id, command_id, context_id, field_id};

pub(crate) fn runtime_registry(generation: u64) -> CommandRegistry {
    let mut builder = CommandRegistryBuilder::new(
        CommandRegistryGeneration::new(generation),
        CommandLimits::default(),
    );
    for definition in [
        CommandContextDefinition {
            id: context_id("global"),
            parent_id: None,
        },
        CommandContextDefinition {
            id: context_id("project"),
            parent_id: Some(context_id("global")),
        },
        CommandContextDefinition {
            id: context_id("editor"),
            parent_id: Some(context_id("project")),
        },
    ] {
        builder.register_context(definition).expect("context");
    }
    builder
        .register_capability(CommandCapabilityDefinition {
            id: capability_id("editing"),
        })
        .expect("capability");
    builder
        .register_command(CommandDefinition {
            id: command_id("test:global"),
            label: "Global".to_owned(),
            description: None,
            category_path: vec![CommandCategoryId::new("general").expect("category")],
            keywords: Vec::new(),
            icon: None,
            allowed_contexts: vec![context_id("global")],
            required_capabilities: Vec::new(),
            visibility: CommandVisibility::ALL,
            text_input_policy: CommandTextInputPolicy::Allowed,
            route: CommandRouteId::new("local:global").expect("route"),
            arguments: CommandArgumentSchema::None,
        })
        .expect("global command");
    builder
        .register_command(CommandDefinition {
            id: command_id("test:edit"),
            label: "Edit".to_owned(),
            description: None,
            category_path: vec![CommandCategoryId::new("edit").expect("category")],
            keywords: Vec::new(),
            icon: None,
            allowed_contexts: vec![context_id("editor")],
            required_capabilities: vec![capability_id("editing")],
            visibility: CommandVisibility::ALL,
            text_input_policy: CommandTextInputPolicy::Blocked,
            route: CommandRouteId::new("domain:editor.apply").expect("route"),
            arguments: CommandArgumentSchema::Object {
                fields: vec![CommandArgumentField {
                    id: field_id("enabled"),
                    required: true,
                    default: None,
                    kind: CommandArgumentKind::Boolean,
                }],
            },
        })
        .expect("edit command");
    builder.seal().expect("runtime registry")
}

pub(crate) fn context_snapshot(revision: u64, path: &[&str]) -> CommandContextSnapshot {
    CommandContextSnapshot::new(
        CommandContextRevision::new(revision),
        path.iter().map(|value| context_id(value)).collect(),
    )
    .expect("context snapshot")
}

pub(crate) fn capability_snapshot(values: &[&str]) -> CommandCapabilitySnapshot {
    CommandCapabilitySnapshot::new(values.iter().map(|value| capability_id(value)))
        .expect("capability snapshot")
}

pub(crate) fn request(
    registry: &CommandRegistry,
    request_id: &str,
    command_id: &str,
    arguments: Value,
) -> CommandExecutionRequest {
    CommandExecutionRequest {
        request_id: CommandRequestId::new(request_id).expect("request id"),
        registry_generation: registry.generation(),
        command_id: CommandId::new(command_id).expect("command id"),
        arguments,
    }
}

pub(crate) fn evidence(code: &str) -> CommandEvidence {
    CommandEvidence::new(CommandEvidenceCode::new(code).expect("evidence code"), None)
}

pub(crate) struct ContextFeed {
    pub current: CommandContextSnapshot,
    pub failure: Option<CommandSourceFailure>,
    pub calls: usize,
}

impl ContextFeed {
    pub(crate) fn new(current: CommandContextSnapshot) -> Self {
        Self {
            current,
            failure: None,
            calls: 0,
        }
    }
}

impl CommandContextSource for ContextFeed {
    fn current_context(&mut self) -> Result<CommandContextSnapshot, CommandSourceFailure> {
        self.calls += 1;
        self.failure
            .clone()
            .map_or_else(|| Ok(self.current.clone()), Err)
    }
}

pub(crate) struct CapabilityFeed {
    pub current: CommandCapabilitySnapshot,
    pub failure: Option<CommandSourceFailure>,
    pub calls: usize,
}

impl CapabilityFeed {
    pub(crate) fn new(current: CommandCapabilitySnapshot) -> Self {
        Self {
            current,
            failure: None,
            calls: 0,
        }
    }
}

impl CommandCapabilitySource for CapabilityFeed {
    fn current_capabilities(&mut self) -> Result<CommandCapabilitySnapshot, CommandSourceFailure> {
        self.calls += 1;
        self.failure
            .clone()
            .map_or_else(|| Ok(self.current.clone()), Err)
    }
}

pub(crate) struct AvailabilityFeed {
    pub default: CommandAvailability,
    pub by_command: BTreeMap<CommandId, CommandAvailability>,
    pub failure: Option<CommandSourceFailure>,
    pub calls: Vec<CommandId>,
}

impl AvailabilityFeed {
    pub(crate) fn available() -> Self {
        Self {
            default: CommandAvailability::available(),
            by_command: BTreeMap::new(),
            failure: None,
            calls: Vec::new(),
        }
    }
}

impl CommandAvailabilitySource for AvailabilityFeed {
    fn availability(
        &mut self,
        command: &CommandDefinition,
        _context: &CommandContextSnapshot,
        _capabilities: &CommandCapabilitySnapshot,
    ) -> Result<CommandAvailability, CommandSourceFailure> {
        self.calls.push(command.id.clone());
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        Ok(self
            .by_command
            .get(&command.id)
            .cloned()
            .unwrap_or_else(|| self.default.clone()))
    }
}

pub(crate) struct RecordingExecutor {
    pub outcome: CommandExecutorOutcome,
    pub invocations: Vec<AdmittedCommandInvocation>,
}

impl RecordingExecutor {
    pub(crate) fn new(outcome: CommandExecutorOutcome) -> Self {
        Self {
            outcome,
            invocations: Vec::new(),
        }
    }
}

impl CommandExecutor for RecordingExecutor {
    fn execute(&mut self, invocation: &AdmittedCommandInvocation) -> CommandExecutorOutcome {
        self.invocations.push(invocation.clone());
        self.outcome.clone()
    }
}

pub(crate) fn unknown_capability_snapshot() -> CommandCapabilitySnapshot {
    CommandCapabilitySnapshot::new([CommandCapabilityId::new("unknown").expect("capability")])
        .expect("snapshot")
}

pub(crate) fn enabled_field_id() -> CommandFieldId {
    field_id("enabled")
}
