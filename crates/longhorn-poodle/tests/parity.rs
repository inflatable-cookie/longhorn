//! Checks this crate's projections against the cross-backend parity fixture.
//!
//! `fixtures/parity/projection-v1.json` states what a surface must produce for
//! a given fact, in either language. `longhorn-poodle-svelte` checks the same
//! file, so a mapping changed on one side and not the other fails a gate
//! rather than waiting to be noticed by eye.
//!
//! The fixture is hand-written and neither side generates it. Wording is
//! generated (Card 170) because there one source should decide; behaviour is
//! not, because the value is two independent implementations agreeing. See
//! card 171.

use longhorn_config::RestoreDomainCompatibilityProjection;
use longhorn_notifications::NotificationSeverity;
use longhorn_operation::{
    OperationCancellationSupportProjection, OperationOverallProgressProjection,
    OperationStateProjection,
};
use longhorn_poodle::{config, operation, tone_for};
use poodle_specs::{StatusTone, ToastTone};
use serde_json::Value;

fn fixture() -> Value {
    let raw = include_str!("../../../fixtures/parity/projection-v1.json");
    serde_json::from_str(raw).expect("parity fixture is valid JSON")
}

fn cases(key: &str) -> Vec<Value> {
    fixture()[key]
        .as_array()
        .unwrap_or_else(|| panic!("fixture has no `{key}` cases"))
        .clone()
}

fn text(case: &Value, field: &str) -> String {
    case[field]
        .as_str()
        .unwrap_or_else(|| panic!("case {case} has no string `{field}`"))
        .to_owned()
}

fn severity(wire: &str) -> NotificationSeverity {
    NotificationSeverity::ALL
        .into_iter()
        .find(|candidate| candidate.wire_name() == wire)
        .unwrap_or_else(|| panic!("unknown severity `{wire}`"))
}

fn state(wire: &str) -> OperationStateProjection {
    OperationStateProjection::ALL
        .into_iter()
        .find(|candidate| candidate.wire_name() == wire)
        .unwrap_or_else(|| panic!("unknown operation state `{wire}`"))
}

fn toast_tone_name(tone: ToastTone) -> &'static str {
    match tone {
        ToastTone::Info => "info",
        ToastTone::Success => "success",
        ToastTone::Warning => "warning",
        ToastTone::Danger => "danger",
    }
}

fn status_tone_name(tone: StatusTone) -> &'static str {
    match tone {
        StatusTone::Neutral => "neutral",
        StatusTone::Info => "info",
        StatusTone::Success => "success",
        StatusTone::Warning => "warning",
        StatusTone::Danger => "danger",
        StatusTone::Pending => "pending",
    }
}

#[test]
fn every_severity_maps_as_the_fixture_states() {
    let cases = cases("notificationSeverityTone");
    assert_eq!(cases.len(), NotificationSeverity::ALL.len());

    for case in cases {
        let mapping = tone_for(severity(&text(&case, "severity")));
        assert_eq!(toast_tone_name(mapping.tone), text(&case, "tone"), "{case}");
        assert_eq!(
            mapping.is_lossy,
            case["isLossy"].as_bool().expect("isLossy"),
            "{case}"
        );
    }
}

#[test]
fn every_operation_state_maps_as_the_fixture_states() {
    let cases = cases("operationState");
    assert_eq!(cases.len(), OperationStateProjection::ALL.len());

    for case in cases {
        let state = state(&text(&case, "state"));
        assert_eq!(
            status_tone_name(operation::state_tone(state)),
            text(&case, "tone"),
            "{case}"
        );
        assert_eq!(
            operation::state_label(state),
            text(&case, "label"),
            "{case}"
        );
    }
}

#[test]
fn every_progress_shape_becomes_the_bar_the_fixture_states() {
    for case in cases("operationProgress") {
        let overall: OperationOverallProgressProjection =
            serde_json::from_value(case["progress"].clone()).expect("progress");
        let spec = operation::progress(overall);

        assert_eq!(
            spec.is_indeterminate,
            case["indeterminate"].as_bool().expect("indeterminate"),
            "{case}"
        );
        assert_eq!(spec.value, case["value"].as_f64(), "{case}");
        assert_eq!(spec.max, case["max"].as_f64().expect("max"), "{case}");
        assert_eq!(
            spec.value_text.as_deref(),
            case["valueText"].as_str(),
            "{case}"
        );
    }
}

#[test]
fn cancel_eligibility_matches_the_fixture() {
    for case in cases("cancelEligibility") {
        let support = match text(&case, "support").as_str() {
            "supported" => OperationCancellationSupportProjection::Supported,
            "unsupported" => OperationCancellationSupportProjection::Unsupported,
            other => panic!("unknown cancellation support `{other}`"),
        };

        assert_eq!(
            operation::cancel_is_offered(support, state(&text(&case, "state"))),
            case["canCancel"].as_bool().expect("canCancel"),
            "{case}"
        );
    }
}

#[test]
fn every_restore_classification_labels_and_gates_as_the_fixture_states() {
    let cases = cases("restoreCompatibility");
    assert_eq!(
        cases.len(),
        RestoreDomainCompatibilityProjection::TEMPLATES.len()
    );

    for case in cases {
        let compatibility: RestoreDomainCompatibilityProjection =
            serde_json::from_value(case["compatibility"].clone()).expect("compatibility");

        assert_eq!(
            config::compatibility_label(&compatibility),
            text(&case, "label"),
            "{case}"
        );

        let domain = domain_with(compatibility);
        assert_eq!(
            config::can_use_archive(&domain),
            case["canUseArchive"].as_bool().expect("canUseArchive"),
            "{case}"
        );
    }
}

#[test]
fn a_toast_carries_the_first_action_and_no_more() {
    use longhorn_core::{
        NotificationActionReferenceId, NotificationAuthorityId, NotificationId,
        NotificationSourceId,
    };
    use longhorn_notifications::{
        NotificationAction, NotificationActionLabel, NotificationAdd, NotificationAuthorityEpoch,
        NotificationDraft, NotificationLedger, NotificationLedgerLimits, NotificationSummary,
        NotificationTitle,
    };

    for case in cases("toastAction") {
        let labels: Vec<String> = case["actions"]
            .as_array()
            .expect("actions")
            .iter()
            .map(|value| value.as_str().expect("label").to_owned())
            .collect();

        let actions: Vec<NotificationAction> = labels
            .iter()
            .enumerate()
            .map(|(index, label)| {
                NotificationAction::new(
                    NotificationActionReferenceId::new(format!("action:{index}")).expect("id"),
                    NotificationActionLabel::new(label.clone()).expect("label"),
                )
            })
            .collect();

        let mut ledger = NotificationLedger::new(
            NotificationAuthorityId::new("notifications:parity").expect("authority"),
            NotificationAuthorityEpoch::new(1).expect("epoch"),
            NotificationLedgerLimits::new(8, 1_024 * 1_024).expect("limits"),
        );
        let draft = NotificationDraft::new(
            NotificationSourceId::new("parity").expect("source"),
            NotificationSeverity::Info,
            NotificationTitle::new("Title").expect("title"),
            NotificationSummary::new("Summary").expect("summary"),
        )
        .with_actions(actions)
        .expect("actions");
        let add = NotificationAdd::new(
            ledger.authority().clone(),
            ledger.revision(),
            NotificationId::new("parity:1").expect("id"),
            draft,
        );
        ledger.add(add).expect("add");

        let record = ledger.records().next().expect("record");
        let toast = longhorn_poodle::project_notification(record);

        assert_eq!(
            toast.action_label.as_deref(),
            case["actionLabel"].as_str(),
            "{case}"
        );
    }
}

fn domain_with(
    compatibility: RestoreDomainCompatibilityProjection,
) -> longhorn_config::RestoreDomainInspectionProjection {
    longhorn_config::RestoreDomainInspectionProjection {
        domain_id: "settings".to_owned(),
        storage_class: "sqlite".to_owned(),
        consistency_group: "primary".to_owned(),
        adapter: "builtin".to_owned(),
        source_state: "present".to_owned(),
        source_schema_version: Some(3),
        target_schema_version: Some(4),
        compatibility,
    }
}
