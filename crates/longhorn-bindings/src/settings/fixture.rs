use std::error::Error;

use longhorn_settings::{
    SettingsApplyCommand, SettingsConflict, SettingsDurabilityEvidence, SettingsLimits,
    SettingsLoadCommand, SettingsLoadOutcome, SettingsMutationOutcome, SettingsMutationReceipt,
    SettingsMutationResult, SettingsProtocolVersion, SettingsRecoveryState,
    SettingsRegistryChangedEvent, SettingsRegistryGeneration, SettingsRegistrySnapshot,
    SettingsRejection, SettingsRejectionCode, SettingsResetCommand, SettingsScopeChangedEvent,
    SettingsScopeRevision, SettingsScopeSnapshot,
};
use serde::Serialize;
use serde_json::{Value, json};

mod authority;
mod ids;
mod registry;

use authority::{activation_requirements, authority, opaque, recovery_states, snapshot};
use ids::{entry_id, page_id, request_id, scope_id, unit_id};
use registry::registry;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GoldenFixture {
    protocol_version: u16,
    registry: SettingsRegistrySnapshot,
    snapshots: Vec<SettingsScopeSnapshot>,
    load_commands: Vec<SettingsLoadCommand>,
    apply_commands: Vec<SettingsApplyCommand>,
    reset_commands: Vec<SettingsResetCommand>,
    load_outcomes: Vec<SettingsLoadOutcome>,
    mutation_results: Vec<SettingsMutationResult>,
    recovery_states: Vec<SettingsRecoveryState>,
    registry_events: Vec<SettingsRegistryChangedEvent>,
    scope_events: Vec<SettingsScopeChangedEvent>,
    incompatibility: IncompatibilityFixture,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IncompatibilityFixture {
    future_protocol_version: u16,
    unknown_mutation_status: Value,
    unknown_load_status: Value,
    unknown_durability_kind: Value,
    unknown_rejection_code: Value,
    unknown_recovery_code: Value,
    unknown_editability: Value,
}

pub fn render(rejection_codes: &[String]) -> Result<String, Box<dyn Error>> {
    let limits = SettingsLimits::default();
    let registry = registry(limits);
    let current = authority(3, "authority:current");
    let previous = authority(2, "authority:previous");
    let snapshot = snapshot(limits, current.clone());
    let rejected = rejection_codes
        .iter()
        .map(|code| {
            let code: SettingsRejectionCode =
                serde_json::from_value(Value::String(code.clone())).expect("generated code");
            SettingsMutationResult::Rejected {
                rejection: SettingsRejection {
                    code,
                    diagnostic: Some(opaque(limits, json!({"reason": "fixture"}))),
                },
                snapshot: Some(snapshot.clone()),
            }
        })
        .collect::<Vec<_>>();
    let receipt = SettingsMutationReceipt {
        request_id: request_id("request:apply"),
        page_id: page_id("app:audio"),
        apply_unit_id: unit_id("app:audio"),
        scope_id: scope_id("app:preferences"),
        previous_authority: previous.clone(),
        committed_authority: current.clone(),
        outcome: SettingsMutationOutcome::Changed,
        durability: SettingsDurabilityEvidence::Confirmed {
            evidence: Some(opaque(limits, json!({"generation": 3}))),
        },
        activation_requirements: activation_requirements(),
    };
    let apply = SettingsApplyCommand {
        protocol_version: SettingsProtocolVersion::CURRENT,
        request_id: request_id("request:apply"),
        page_id: page_id("app:audio"),
        apply_unit_id: unit_id("app:audio"),
        scope_id: scope_id("app:preferences"),
        authority: previous.clone(),
        intent: opaque(
            limits,
            json!({"kind": "setOutput", "outputId": "device:main"}),
        ),
    };
    let reset = SettingsResetCommand {
        protocol_version: SettingsProtocolVersion::CURRENT,
        request_id: request_id("request:reset"),
        page_id: page_id("app:audio"),
        apply_unit_id: unit_id("app:audio"),
        scope_id: scope_id("app:preferences"),
        authority: previous.clone(),
        entry_ids: vec![entry_id("audio:output")],
    };
    let mut mutation_results = vec![
        SettingsMutationResult::Applied {
            snapshot: snapshot.clone(),
            receipt,
        },
        SettingsMutationResult::Applied {
            snapshot: snapshot.clone(),
            receipt: SettingsMutationReceipt {
                request_id: request_id("request:unchanged"),
                page_id: page_id("app:audio"),
                apply_unit_id: unit_id("app:audio"),
                scope_id: scope_id("app:preferences"),
                previous_authority: current.clone(),
                committed_authority: current.clone(),
                outcome: SettingsMutationOutcome::Unchanged,
                durability: SettingsDurabilityEvidence::NotApplicable,
                activation_requirements: vec![],
            },
        },
        SettingsMutationResult::Conflict {
            conflict: SettingsConflict {
                expected: previous.clone(),
                actual: current.clone(),
            },
            snapshot: snapshot.clone(),
        },
    ];
    mutation_results.extend(rejected);
    let fixture = GoldenFixture {
        protocol_version: 1,
        registry,
        snapshots: vec![snapshot.clone()],
        load_commands: vec![
            SettingsLoadCommand {
                protocol_version: SettingsProtocolVersion::CURRENT,
                request_id: request_id("request:load"),
                registry_generation: SettingsRegistryGeneration::new(7),
                scope_id: scope_id("app:preferences"),
                known_authority: None,
            },
            SettingsLoadCommand {
                protocol_version: SettingsProtocolVersion::CURRENT,
                request_id: request_id("request:reload"),
                registry_generation: SettingsRegistryGeneration::new(7),
                scope_id: scope_id("app:preferences"),
                known_authority: Some(current.clone()),
            },
        ],
        apply_commands: vec![apply],
        reset_commands: vec![reset],
        load_outcomes: vec![
            SettingsLoadOutcome::Loaded {
                snapshot: snapshot.clone(),
            },
            SettingsLoadOutcome::Rejected {
                rejection: SettingsRejection {
                    code: SettingsRejectionCode::Unauthorized,
                    diagnostic: None,
                },
            },
        ],
        mutation_results,
        recovery_states: recovery_states(limits),
        registry_events: vec![SettingsRegistryChangedEvent {
            protocol_version: SettingsProtocolVersion::CURRENT,
            registry_generation: SettingsRegistryGeneration::new(7),
        }],
        scope_events: vec![SettingsScopeChangedEvent {
            protocol_version: SettingsProtocolVersion::CURRENT,
            registry_generation: SettingsRegistryGeneration::new(7),
            scope_id: scope_id("app:preferences"),
            scope_revision: SettingsScopeRevision::new(3),
        }],
        incompatibility: IncompatibilityFixture {
            future_protocol_version: 2,
            unknown_mutation_status: json!({"status": "merged"}),
            unknown_load_status: json!({"status": "cached"}),
            unknown_durability_kind: json!({"kind": "eventual"}),
            unknown_rejection_code: json!("futurePolicy"),
            unknown_recovery_code: json!("futureRecovery"),
            unknown_editability: json!("delegated"),
        },
    };
    Ok(format!("{}\n", serde_json::to_string_pretty(&fixture)?))
}
