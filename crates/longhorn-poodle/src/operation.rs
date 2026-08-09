//! Projects operation catalogue entries into Poodle status and progress
//! specs.
//!
//! The Svelte tier's counterpart is `operation/poodle/projectors.ts` — 82
//! lines, entirely pure, and the closest thing in the whole tier to a
//! projection with nothing else mixed in. It is also the clearest case of the
//! pattern the previous two domains found: a rule about Longhorn's own enums,
//! written once in TypeScript, with no Rust statement of it anywhere.

use longhorn_operation::{
    OperationCancellationSupportProjection, OperationEntryProjection,
    OperationOverallProgressProjection, OperationStateProjection,
};
use poodle_specs::{ProgressSpec, StatusIndicatorSpec, StatusTone};

/// The tone that carries an operation's lifecycle state.
///
/// Seven states into six tones: `Cancelling` and `Interrupted` are both
/// `Warning`. Unlike the notification severity collapse, this one loses
/// nothing, because `status_indicator` always emits the label alongside the
/// tone — the two states are never distinguished by colour alone. A caller
/// that renders the tone without the label has changed the claim and owns the
/// consequence.
#[must_use]
pub const fn state_tone(state: OperationStateProjection) -> StatusTone {
    match state {
        OperationStateProjection::Queued => StatusTone::Pending,
        OperationStateProjection::Running => StatusTone::Info,
        OperationStateProjection::Cancelling | OperationStateProjection::Interrupted => {
            StatusTone::Warning
        }
        OperationStateProjection::Succeeded => StatusTone::Success,
        OperationStateProjection::Failed => StatusTone::Danger,
        // Cancelled is neutral, not danger: the operator asked for it.
        OperationStateProjection::Cancelled => StatusTone::Neutral,
    }
}

/// The operator-facing name of a lifecycle state.
#[must_use]
pub const fn state_label(state: OperationStateProjection) -> &'static str {
    match state {
        OperationStateProjection::Queued => "Queued",
        OperationStateProjection::Running => "Running",
        OperationStateProjection::Cancelling => "Cancelling",
        OperationStateProjection::Succeeded => "Succeeded",
        OperationStateProjection::Failed => "Failed",
        OperationStateProjection::Cancelled => "Cancelled",
        OperationStateProjection::Interrupted => "Interrupted",
    }
}

/// The status indicator for one operation's current state.
#[must_use]
pub fn status_indicator(state: OperationStateProjection) -> StatusIndicatorSpec {
    StatusIndicatorSpec::new()
        .with_status(state_tone(state))
        .with_label(state_label(state))
}

/// Projects overall progress into a progress bar.
///
/// Units keep their own totals rather than being normalised, so a bar showing
/// "3 of 7" reports the executor's own count and not a percentage derived
/// from it. Normalised progress is the only arm that computes anything, and
/// it rounds only the text — `value` stays exact.
#[must_use]
pub fn progress(overall: OperationOverallProgressProjection) -> ProgressSpec {
    match overall {
        OperationOverallProgressProjection::Indeterminate => ProgressSpec {
            is_indeterminate: true,
            ..ProgressSpec::default()
        },
        OperationOverallProgressProjection::Units { completed, total } => ProgressSpec {
            value: Some(completed),
            max: total,
            value_text: Some(format!("{completed} of {total}")),
            ..ProgressSpec::default()
        },
        OperationOverallProgressProjection::Normalized { value } => ProgressSpec {
            value: Some(value),
            max: 1.0,
            value_text: Some(format!("{}%", (value * 100.0).round())),
            ..ProgressSpec::default()
        },
    }
}

/// Whether a cancel control should be offered for this entry.
#[must_use]
pub fn can_cancel(entry: &OperationEntryProjection) -> bool {
    cancel_is_offered(entry.cancellation_support, entry.state)
}

/// The rule behind [`can_cancel`], over the two fields that decide it.
///
/// Both halves matter. An executor that never accepts cancellation must not
/// be offered one, and an operation past `Running` cannot be cancelled even
/// by an executor that supports it — including `Cancelling`, where the
/// request is already in flight and a second one says nothing new.
///
/// Split out from `can_cancel` because an `OperationEntryProjection` carries
/// eleven other fields, none of which this decision reads. A test that had to
/// invent revisions and an authority cursor to ask one question would be
/// testing the fixture.
#[must_use]
pub const fn cancel_is_offered(
    support: OperationCancellationSupportProjection,
    state: OperationStateProjection,
) -> bool {
    matches!(support, OperationCancellationSupportProjection::Supported)
        && matches!(
            state,
            OperationStateProjection::Queued | OperationStateProjection::Running
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cancelled_operation_is_neutral_rather_than_a_failure() {
        // The operator asked for it, so it is not bad news.
        assert_eq!(
            state_tone(OperationStateProjection::Cancelled),
            StatusTone::Neutral
        );
        assert_eq!(
            state_tone(OperationStateProjection::Failed),
            StatusTone::Danger
        );
    }

    #[test]
    fn the_two_warning_states_are_still_told_apart_by_the_label() {
        let cancelling = status_indicator(OperationStateProjection::Cancelling);
        let interrupted = status_indicator(OperationStateProjection::Interrupted);

        assert_eq!(cancelling.status, interrupted.status);
        assert_ne!(cancelling.label, interrupted.label);
    }

    #[test]
    fn unit_progress_keeps_the_executors_own_totals() {
        let spec = progress(OperationOverallProgressProjection::Units {
            completed: 3.0,
            total: 7.0,
        });

        assert_eq!(spec.value, Some(3.0));
        assert_eq!(spec.max, 7.0);
        assert_eq!(spec.value_text.as_deref(), Some("3 of 7"));
        assert!(!spec.is_indeterminate);
    }

    #[test]
    fn normalized_progress_rounds_only_the_text() {
        let spec = progress(OperationOverallProgressProjection::Normalized { value: 0.256 });

        assert_eq!(spec.value, Some(0.256));
        assert_eq!(spec.max, 1.0);
        assert_eq!(spec.value_text.as_deref(), Some("26%"));
    }

    #[test]
    fn cancellation_needs_both_executor_support_and_a_live_state() {
        use OperationCancellationSupportProjection::{Supported, Unsupported};
        use OperationStateProjection::{
            Cancelled, Cancelling, Failed, Interrupted, Queued, Running, Succeeded,
        };

        for state in [Queued, Running] {
            assert!(cancel_is_offered(Supported, state), "{state:?}");
            assert!(!cancel_is_offered(Unsupported, state), "{state:?}");
        }
        // Cancelling included: the request is already in flight.
        for state in [Cancelling, Succeeded, Failed, Cancelled, Interrupted] {
            assert!(!cancel_is_offered(Supported, state), "{state:?}");
        }
    }

    #[test]
    fn indeterminate_progress_reports_no_value_at_all() {
        let spec = progress(OperationOverallProgressProjection::Indeterminate);

        assert!(spec.is_indeterminate);
        assert_eq!(spec.value, None);
        assert_eq!(spec.value_text, None);
    }
}
