use longhorn_command::{
    CommandAdmissionEngine, CommandAvailability, CommandAvailabilityReason,
    CommandAvailabilityReasonCode, CommandAvailabilityState, CommandContextRevision,
    CommandContextSnapshot, CommandDiagnostic, CommandDiagnosticError, CommandFailureCode,
    CommandFailurePhase, MAXIMUM_COMMAND_DIAGNOSTIC_BYTES,
};
use longhorn_core::{CommandAvailabilityReasonId, CommandContextId};

use super::{
    runtime_support::{
        AvailabilityFeed, CapabilityFeed, ContextFeed, capability_snapshot, context_snapshot,
        evidence, runtime_registry, unknown_capability_snapshot,
    },
    support::{command_id, context_id},
};

#[test]
fn availability_snapshot_binds_registry_generation_and_context_revision() {
    let registry = runtime_registry(4);
    let engine = CommandAdmissionEngine::new(&registry);
    let mut contexts = ContextFeed::new(context_snapshot(9, &["global", "project", "editor"]));
    let mut capabilities = CapabilityFeed::new(capability_snapshot(&["editing"]));
    let mut availability = AvailabilityFeed::available();
    availability.by_command.insert(
        command_id("test:edit"),
        CommandAvailability::hidden(CommandAvailabilityReason::new(
            CommandAvailabilityReasonCode::Consumer(
                CommandAvailabilityReasonId::new("editor:no-selection").expect("reason"),
            ),
            None,
        )),
    );

    let snapshot = engine
        .project_availability(&mut contexts, &mut capabilities, &mut availability)
        .expect("availability snapshot");
    assert_eq!(snapshot.registry_generation(), registry.generation());
    assert_eq!(snapshot.context_revision(), CommandContextRevision::new(9));
    assert_eq!(snapshot.records().count(), 2);
    assert_eq!(
        snapshot
            .command(&command_id("test:edit"))
            .expect("edit")
            .state(),
        CommandAvailabilityState::Hidden
    );
}

#[test]
fn static_context_and_capability_rejections_skip_product_availability() {
    let registry = runtime_registry(1);
    let engine = CommandAdmissionEngine::new(&registry);
    let mut contexts = ContextFeed::new(context_snapshot(2, &["global"]));
    let mut capabilities = CapabilityFeed::new(capability_snapshot(&[]));
    let mut availability = AvailabilityFeed::available();

    let snapshot = engine
        .project_availability(&mut contexts, &mut capabilities, &mut availability)
        .expect("snapshot");
    let edit = snapshot.command(&command_id("test:edit")).expect("edit");
    assert_eq!(edit.state(), CommandAvailabilityState::Unsupported);
    assert_eq!(
        edit.reason().expect("reason").code(),
        &CommandAvailabilityReasonCode::MissingCapability
    );
    assert_eq!(availability.calls, [command_id("test:global")]);
}

#[test]
fn projection_rejects_invalid_context_unknown_capability_and_source_failure() {
    let registry = runtime_registry(1);
    let engine = CommandAdmissionEngine::new(&registry);
    let invalid_path = CommandContextSnapshot::new(
        CommandContextRevision::new(1),
        vec![context_id("global"), context_id("editor")],
    )
    .expect("locally valid path");
    let mut contexts = ContextFeed::new(invalid_path);
    let mut capabilities = CapabilityFeed::new(capability_snapshot(&[]));
    let mut availability = AvailabilityFeed::available();
    let error = engine
        .project_availability(&mut contexts, &mut capabilities, &mut availability)
        .expect_err("topology must fail");
    assert_eq!(
        error.failure().code(),
        CommandFailureCode::InvalidContextSnapshot
    );
    assert_eq!(error.failure().phase(), CommandFailurePhase::Context);

    contexts.current = context_snapshot(2, &["global"]);
    capabilities.current = unknown_capability_snapshot();
    let error = engine
        .project_availability(&mut contexts, &mut capabilities, &mut availability)
        .expect_err("unknown capability");
    assert_eq!(
        error.failure().code(),
        CommandFailureCode::UnknownCapabilityFact
    );

    capabilities.current = capability_snapshot(&[]);
    contexts.failure = Some(longhorn_command::CommandSourceFailure::new(evidence(
        "context:unavailable",
    )));
    let error = engine
        .project_availability(&mut contexts, &mut capabilities, &mut availability)
        .expect_err("source failure");
    assert_eq!(error.failure().code(), CommandFailureCode::SourceFailed);
    assert!(error.failure().evidence().is_some());
}

#[test]
fn context_snapshot_and_diagnostics_are_bounded_and_strict() {
    assert!(CommandContextSnapshot::new(CommandContextRevision::INITIAL, Vec::new()).is_err());
    assert!(
        serde_json::from_value::<CommandContextSnapshot>(serde_json::json!({
            "revision": 1,
            "path": []
        }))
        .is_err()
    );
    assert!(
        CommandContextSnapshot::new(
            CommandContextRevision::INITIAL,
            vec![CommandContextId::new("editor").expect("context")]
        )
        .is_err()
    );
    assert_eq!(
        CommandDiagnostic::new(""),
        Err(CommandDiagnosticError::Empty)
    );
    assert!(matches!(
        CommandDiagnostic::new("x".repeat(MAXIMUM_COMMAND_DIAGNOSTIC_BYTES + 1)),
        Err(CommandDiagnosticError::TooLong { .. })
    ));
    assert!(
        serde_json::from_value::<CommandAvailability>(serde_json::json!({
            "state": "available",
            "reason": {
                "code": {"kind": "contextNotAllowed"},
                "detail": null
            }
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<CommandAvailability>(serde_json::json!({
            "state": "hidden",
            "reason": null
        }))
        .is_err()
    );
}
