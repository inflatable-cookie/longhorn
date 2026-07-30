use longhorn_command::{
    AdmittedCommandInvocation, CommandAdmissionEngine, CommandArgumentValue, CommandAvailability,
    CommandAvailabilityReason, CommandAvailabilityReasonCode, CommandAvailabilityState,
    CommandContextRevision, CommandDiagnostic, CommandEvidence, CommandExecutionOutcome,
    CommandExecutionResult, CommandExecutor, CommandExecutorOutcome, CommandFailureCode,
    CommandFailurePhase, CommandRegistryGeneration, CommandSourceFailure,
};
use longhorn_core::{CommandAvailabilityReasonId, CommandEvidenceCode, CommandRequestId};
use serde_json::json;

use super::{
    runtime_support::{
        AvailabilityFeed, CapabilityFeed, ContextFeed, RecordingExecutor, capability_snapshot,
        context_snapshot, enabled_field_id, evidence, request, runtime_registry,
        unknown_capability_snapshot,
    },
    support::command_id,
};

fn succeeded_executor() -> RecordingExecutor {
    RecordingExecutor::new(CommandExecutorOutcome::Succeeded { evidence: None })
}

#[test]
fn stale_unknown_and_invalid_requests_fail_before_fresh_fact_loading_or_execution() {
    let registry = runtime_registry(4);
    let engine = CommandAdmissionEngine::new(&registry);
    let mut contexts = ContextFeed::new(context_snapshot(1, &["global", "project", "editor"]));
    let mut capabilities = CapabilityFeed::new(capability_snapshot(&["editing"]));
    let mut availability = AvailabilityFeed::available();
    let mut executor = succeeded_executor();

    let mut stale = request(
        &registry,
        "request:stale",
        "test:edit",
        json!({"enabled": true}),
    );
    stale.registry_generation = CommandRegistryGeneration::new(3);
    assert!(matches!(
        engine
            .execute(
                stale,
                &mut contexts,
                &mut capabilities,
                &mut availability,
                &mut executor,
            )
            .outcome(),
        CommandExecutionOutcome::StaleRegistry { .. }
    ));

    let unknown = request(&registry, "request:unknown", "test:missing", json!(null));
    assert!(matches!(
        engine
            .execute(
                unknown,
                &mut contexts,
                &mut capabilities,
                &mut availability,
                &mut executor,
            )
            .outcome(),
        CommandExecutionOutcome::UnknownCommand
    ));

    let invalid = request(
        &registry,
        "request:invalid",
        "test:edit",
        json!({"enabled": {"nested": true}}),
    );
    assert!(matches!(
        engine
            .execute(
                invalid,
                &mut contexts,
                &mut capabilities,
                &mut availability,
                &mut executor,
            )
            .outcome(),
        CommandExecutionOutcome::InvalidArguments { .. }
    ));
    assert_eq!(contexts.calls, 0);
    assert_eq!(capabilities.calls, 0);
    assert!(availability.calls.is_empty());
    assert!(executor.invocations.is_empty());
}

#[test]
fn stale_renderer_snapshot_never_authorizes_changed_context_or_lost_capability() {
    let registry = runtime_registry(2);
    let engine = CommandAdmissionEngine::new(&registry);
    let mut contexts = ContextFeed::new(context_snapshot(4, &["global", "project", "editor"]));
    let mut capabilities = CapabilityFeed::new(capability_snapshot(&["editing"]));
    let mut availability = AvailabilityFeed::available();
    let old_snapshot = engine
        .project_availability(&mut contexts, &mut capabilities, &mut availability)
        .expect("renderer snapshot");
    assert!(
        old_snapshot
            .command(&command_id("test:edit"))
            .expect("edit")
            .is_available()
    );

    contexts.current = context_snapshot(5, &["global"]);
    availability.calls.clear();
    let mut executor = succeeded_executor();
    let changed_context = engine.execute(
        request(
            &registry,
            "request:context-race",
            "test:edit",
            json!({"enabled": true}),
        ),
        &mut contexts,
        &mut capabilities,
        &mut availability,
        &mut executor,
    );
    let CommandExecutionOutcome::Unavailable {
        availability: current,
    } = changed_context.outcome()
    else {
        panic!("changed context must reject");
    };
    assert_eq!(current.state(), CommandAvailabilityState::Unavailable);
    assert_eq!(
        current.reason().expect("reason").code(),
        &CommandAvailabilityReasonCode::ContextNotAllowed
    );
    assert!(availability.calls.is_empty());
    assert!(executor.invocations.is_empty());

    contexts.current = context_snapshot(6, &["global", "project", "editor"]);
    capabilities.current = capability_snapshot(&[]);
    let lost_capability = engine.execute(
        request(
            &registry,
            "request:capability-race",
            "test:edit",
            json!({"enabled": true}),
        ),
        &mut contexts,
        &mut capabilities,
        &mut availability,
        &mut executor,
    );
    let CommandExecutionOutcome::Unavailable {
        availability: current,
    } = lost_capability.outcome()
    else {
        panic!("lost capability must reject");
    };
    assert_eq!(current.state(), CommandAvailabilityState::Unsupported);
    assert!(executor.invocations.is_empty());
}

#[test]
fn product_availability_rejection_skips_executor_and_uses_fresh_revision() {
    let registry = runtime_registry(3);
    let engine = CommandAdmissionEngine::new(&registry);
    let mut contexts = ContextFeed::new(context_snapshot(11, &["global", "project", "editor"]));
    let mut capabilities = CapabilityFeed::new(capability_snapshot(&["editing"]));
    let mut availability = AvailabilityFeed::available();
    availability.default = CommandAvailability::unavailable(CommandAvailabilityReason::new(
        CommandAvailabilityReasonCode::Consumer(
            CommandAvailabilityReasonId::new("editor:selection-empty").expect("reason"),
        ),
        None,
    ));
    let mut executor = succeeded_executor();
    let result = engine.execute(
        request(
            &registry,
            "request:unavailable",
            "test:edit",
            json!({"enabled": true}),
        ),
        &mut contexts,
        &mut capabilities,
        &mut availability,
        &mut executor,
    );
    assert!(matches!(
        result.outcome(),
        CommandExecutionOutcome::Unavailable { .. }
    ));
    assert_eq!(contexts.current.revision(), CommandContextRevision::new(11));
    assert!(executor.invocations.is_empty());
}

#[test]
fn admitted_invocation_carries_normalized_arguments_route_and_fresh_context() {
    let registry = runtime_registry(6);
    let engine = CommandAdmissionEngine::new(&registry);
    let mut contexts = ContextFeed::new(context_snapshot(12, &["global", "project", "editor"]));
    let mut capabilities = CapabilityFeed::new(capability_snapshot(&["editing"]));
    let mut availability = AvailabilityFeed::available();
    let mut executor = succeeded_executor();
    let result = engine.execute(
        request(
            &registry,
            "request:success",
            "test:edit",
            json!({"enabled": true}),
        ),
        &mut contexts,
        &mut capabilities,
        &mut availability,
        &mut executor,
    );
    assert!(matches!(
        result.outcome(),
        CommandExecutionOutcome::Succeeded { .. }
    ));
    assert_eq!(
        result.request_id(),
        &CommandRequestId::new("request:success").expect("request")
    );
    let invocation = &executor.invocations[0];
    assert_eq!(invocation.registry_generation(), registry.generation());
    assert_eq!(
        invocation.context_revision(),
        CommandContextRevision::new(12)
    );
    assert_eq!(invocation.matched_context_id().as_str(), "editor");
    assert_eq!(invocation.command_id().as_str(), "test:edit");
    assert_eq!(invocation.route().as_str(), "domain:editor.apply");
    assert_eq!(
        invocation.arguments().get(&enabled_field_id()),
        Some(&CommandArgumentValue::Boolean(true))
    );
}

#[test]
fn executor_terminals_remain_distinct_and_request_correlated() {
    let registry = runtime_registry(1);
    let cases = [
        (
            CommandExecutorOutcome::Succeeded { evidence: None },
            "succeeded",
        ),
        (
            CommandExecutorOutcome::Unauthorized { evidence: None },
            "unauthorized",
        ),
        (
            CommandExecutorOutcome::Cancelled { evidence: None },
            "cancelled",
        ),
        (
            CommandExecutorOutcome::Rejected { evidence: None },
            "rejected",
        ),
        (CommandExecutorOutcome::Failed { evidence: None }, "failed"),
        (
            CommandExecutorOutcome::Indeterminate { evidence: None },
            "indeterminate",
        ),
    ];
    for (terminal, expected) in cases {
        let mut contexts = ContextFeed::new(context_snapshot(1, &["global"]));
        let mut capabilities = CapabilityFeed::new(capability_snapshot(&[]));
        let mut availability = AvailabilityFeed::available();
        let mut executor = RecordingExecutor::new(terminal);
        let result = CommandAdmissionEngine::new(&registry).execute(
            request(
                &registry,
                &format!("request:{expected}"),
                "test:global",
                json!(null),
            ),
            &mut contexts,
            &mut capabilities,
            &mut availability,
            &mut executor,
        );
        let actual = match result.outcome() {
            CommandExecutionOutcome::Succeeded { .. } => "succeeded",
            CommandExecutionOutcome::Unauthorized { .. } => "unauthorized",
            CommandExecutionOutcome::Cancelled { .. } => "cancelled",
            CommandExecutionOutcome::Rejected { .. } => "rejected",
            CommandExecutionOutcome::Failed { failure }
                if failure.phase() == CommandFailurePhase::Executor =>
            {
                "failed"
            }
            CommandExecutionOutcome::Indeterminate { .. } => "indeterminate",
            outcome => panic!("unexpected terminal {outcome:?}"),
        };
        assert_eq!(actual, expected);
        assert_eq!(result.request_id().as_str(), format!("request:{expected}"));
        assert_eq!(executor.invocations.len(), 1);
    }
}

#[test]
fn source_and_fresh_fact_failures_are_typed_and_never_call_executor() {
    let registry = runtime_registry(1);
    let engine = CommandAdmissionEngine::new(&registry);
    let mut contexts = ContextFeed::new(context_snapshot(1, &["global", "project", "editor"]));
    let mut capabilities = CapabilityFeed::new(capability_snapshot(&["editing"]));
    let mut availability = AvailabilityFeed::available();
    let mut executor = succeeded_executor();

    contexts.failure = Some(CommandSourceFailure::new(evidence("context:failed")));
    let result = engine.execute(
        request(
            &registry,
            "request:context-source",
            "test:edit",
            json!({"enabled": true}),
        ),
        &mut contexts,
        &mut capabilities,
        &mut availability,
        &mut executor,
    );
    let CommandExecutionOutcome::Failed { failure } = result.outcome() else {
        panic!("source failure");
    };
    assert_eq!(failure.phase(), CommandFailurePhase::Context);
    assert_eq!(failure.code(), CommandFailureCode::SourceFailed);
    assert!(failure.evidence().is_some());

    contexts.failure = None;
    capabilities.current = unknown_capability_snapshot();
    let result = engine.execute(
        request(
            &registry,
            "request:unknown-capability",
            "test:edit",
            json!({"enabled": true}),
        ),
        &mut contexts,
        &mut capabilities,
        &mut availability,
        &mut executor,
    );
    let CommandExecutionOutcome::Failed { failure } = result.outcome() else {
        panic!("capability failure");
    };
    assert_eq!(failure.phase(), CommandFailurePhase::Capability);
    assert_eq!(failure.code(), CommandFailureCode::UnknownCapabilityFact);

    capabilities.current = capability_snapshot(&["editing"]);
    capabilities.failure = Some(CommandSourceFailure::new(evidence("capability:failed")));
    let result = engine.execute(
        request(
            &registry,
            "request:capability-source",
            "test:edit",
            json!({"enabled": true}),
        ),
        &mut contexts,
        &mut capabilities,
        &mut availability,
        &mut executor,
    );
    let CommandExecutionOutcome::Failed { failure } = result.outcome() else {
        panic!("capability source failure");
    };
    assert_eq!(failure.phase(), CommandFailurePhase::Capability);

    capabilities.failure = None;
    availability.failure = Some(CommandSourceFailure::new(evidence("availability:failed")));
    let result = engine.execute(
        request(
            &registry,
            "request:availability-source",
            "test:edit",
            json!({"enabled": true}),
        ),
        &mut contexts,
        &mut capabilities,
        &mut availability,
        &mut executor,
    );
    let CommandExecutionOutcome::Failed { failure } = result.outcome() else {
        panic!("availability source failure");
    };
    assert_eq!(failure.phase(), CommandFailurePhase::Availability);
    assert!(executor.invocations.is_empty());
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TypedDomainOperation {
    ApplyEditor { enabled: bool },
}

struct LocalExecutor {
    invocation: Option<AdmittedCommandInvocation>,
}

impl CommandExecutor for LocalExecutor {
    fn execute(&mut self, invocation: &AdmittedCommandInvocation) -> CommandExecutorOutcome {
        self.invocation = Some(invocation.clone());
        CommandExecutorOutcome::Succeeded { evidence: None }
    }
}

struct TypedDomainExecutor {
    invocation: Option<AdmittedCommandInvocation>,
    operation: Option<TypedDomainOperation>,
}

impl CommandExecutor for TypedDomainExecutor {
    fn execute(&mut self, invocation: &AdmittedCommandInvocation) -> CommandExecutorOutcome {
        self.invocation = Some(invocation.clone());
        if invocation.route().as_str() != "domain:editor.apply" {
            return CommandExecutorOutcome::Rejected { evidence: None };
        }
        let Some(CommandArgumentValue::Boolean(enabled)) =
            invocation.arguments().get(&enabled_field_id())
        else {
            return CommandExecutorOutcome::Rejected { evidence: None };
        };
        self.operation = Some(TypedDomainOperation::ApplyEditor { enabled: *enabled });
        CommandExecutorOutcome::Succeeded { evidence: None }
    }
}

#[test]
fn renderer_local_and_typed_domain_routes_receive_the_same_admitted_invocation() {
    let registry = runtime_registry(8);
    let engine = CommandAdmissionEngine::new(&registry);
    let mut contexts = ContextFeed::new(context_snapshot(21, &["global", "project", "editor"]));
    let mut capabilities = CapabilityFeed::new(capability_snapshot(&["editing"]));
    let mut availability = AvailabilityFeed::available();
    let invocation = engine
        .admit(
            request(
                &registry,
                "request:route",
                "test:edit",
                json!({"enabled": true}),
            ),
            &mut contexts,
            &mut capabilities,
            &mut availability,
        )
        .expect("admitted");

    let mut local = LocalExecutor { invocation: None };
    let mut domain = TypedDomainExecutor {
        invocation: None,
        operation: None,
    };
    local.execute(&invocation);
    domain.execute(&invocation);

    assert_eq!(local.invocation, domain.invocation);
    assert_eq!(
        domain.operation,
        Some(TypedDomainOperation::ApplyEditor { enabled: true })
    );
}

#[test]
fn executor_failure_preserves_immutable_registry_and_fresh_context_facts() {
    let registry = runtime_registry(2);
    let digest = registry.digest().clone();
    let engine = CommandAdmissionEngine::new(&registry);
    let original_context = context_snapshot(4, &["global"]);
    let mut contexts = ContextFeed::new(original_context.clone());
    let mut capabilities = CapabilityFeed::new(capability_snapshot(&[]));
    let mut availability = AvailabilityFeed::available();
    let mut executor = RecordingExecutor::new(CommandExecutorOutcome::Failed {
        evidence: Some(evidence("local:failed")),
    });
    let result = engine.execute(
        request(&registry, "request:failed", "test:global", json!(null)),
        &mut contexts,
        &mut capabilities,
        &mut availability,
        &mut executor,
    );
    let CommandExecutionOutcome::Failed { failure } = result.outcome() else {
        panic!("executor failure");
    };
    assert_eq!(failure.phase(), CommandFailurePhase::Executor);
    assert_eq!(registry.digest(), &digest);
    assert_eq!(contexts.current, original_context);
}

#[test]
fn bounded_product_evidence_is_preserved_without_interpretation() {
    let registry = runtime_registry(1);
    let engine = CommandAdmissionEngine::new(&registry);
    let mut contexts = ContextFeed::new(context_snapshot(1, &["global"]));
    let mut capabilities = CapabilityFeed::new(capability_snapshot(&[]));
    let mut availability = AvailabilityFeed::available();
    let invocation = engine
        .admit(
            request(&registry, "request:evidence", "test:global", json!(null)),
            &mut contexts,
            &mut capabilities,
            &mut availability,
        )
        .expect("admitted");
    let evidence = CommandEvidence::new(
        CommandEvidenceCode::new("domain:semantic-rejection").expect("code"),
        Some(CommandDiagnostic::new("selection is locked").expect("detail")),
    );
    let result = CommandAdmissionEngine::complete(
        &invocation,
        CommandExecutorOutcome::Rejected {
            evidence: Some(evidence.clone()),
        },
    );
    assert_eq!(result.request_id(), invocation.request_id());
    assert_eq!(
        result.outcome(),
        &CommandExecutionOutcome::Rejected {
            evidence: Some(evidence)
        }
    );
    let encoded = serde_json::to_value(&result).expect("serialize");
    let decoded: CommandExecutionResult = serde_json::from_value(encoded).expect("deserialize");
    assert_eq!(decoded, result);
}
