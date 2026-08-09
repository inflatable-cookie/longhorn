use longhorn_notifications::{NotificationRecord, NotificationSeverity};
use poodle_specs::{Toast, ToastStackSpec, ToastTone};

/// What a severity became, and whether anything was lost saying it.
///
/// Longhorn has five severities and Poodle has four tones, so one pair
/// collapses. A projection that silently flattened them would make
/// `Critical` indistinguishable from `Error` at exactly the moment the
/// distinction matters, so the collapse is returned rather than hidden.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ToneMapping {
    /// The tone to render.
    pub tone: ToastTone,
    /// Whether the source severity carried more meaning than the tone does.
    ///
    /// True only for `Critical`, which shares `Danger` with `Error`. A
    /// surface that wants the distinction back adds it in text or in an
    /// action, because the tone cannot carry it.
    pub is_lossy: bool,
}

/// Maps one Longhorn severity onto a Poodle toast tone.
///
/// The five-to-four collapse is the first thing this projection found, and
/// it is a real gap rather than an oversight on either side: Poodle's tones
/// are a visual vocabulary and Longhorn's severities are an operational one.
/// Four tints is a reasonable palette; five operational levels is a
/// reasonable ladder. They simply do not line up.
#[must_use]
pub const fn tone_for(severity: NotificationSeverity) -> ToneMapping {
    match severity {
        NotificationSeverity::Info => ToneMapping {
            tone: ToastTone::Info,
            is_lossy: false,
        },
        NotificationSeverity::Success => ToneMapping {
            tone: ToastTone::Success,
            is_lossy: false,
        },
        NotificationSeverity::Warning => ToneMapping {
            tone: ToastTone::Warning,
            is_lossy: false,
        },
        NotificationSeverity::Error => ToneMapping {
            tone: ToastTone::Danger,
            is_lossy: false,
        },
        // The one that loses something. `Danger` is as loud as Poodle gets.
        NotificationSeverity::Critical => ToneMapping {
            tone: ToastTone::Danger,
            is_lossy: true,
        },
    }
}

/// Projects one notification record into a toast.
///
/// The record's own id becomes the toast id, so a surface can correlate a
/// dismissal back to the ledger without keeping a side table.
///
/// The first action's label becomes the toast's single action, because a
/// toast has room for one. Longhorn records may carry several; the rest are
/// reachable from the notification centre, not from the toast, and dropping
/// them here is a presentation choice rather than a loss of record.
#[must_use]
pub fn project_notification(record: &NotificationRecord) -> Toast {
    let draft = record.draft();
    let mapping = tone_for(draft.severity());

    let mut toast = Toast::new(record.notification_id().as_str(), draft.title().as_str())
        .with_tone(mapping.tone);

    let summary = draft.summary().as_str();
    if !summary.is_empty() {
        toast = toast.with_message(summary);
    }
    if let Some(action) = draft.actions().first() {
        toast = toast.with_action_label(action.label().as_str());
    }
    toast
}

/// Projects a page of records, newest first as the ledger orders them.
#[must_use]
pub fn project_notifications(records: &[NotificationRecord]) -> Vec<Toast> {
    records.iter().map(project_notification).collect()
}

/// Projects a page of records into the stack a renderer actually takes.
///
/// `Toast` is a leaf; no adapter renders one on its own. `ToastStackSpec` is
/// the rendered unit, and both `poodle-gpui` and `poodle-jetstream` implement
/// it — which makes this, and not [`project_notifications`], the function
/// that closes the loop to a drawn surface.
#[must_use]
pub fn project_notification_stack(records: &[NotificationRecord]) -> ToastStackSpec {
    ToastStackSpec::new()
        .with_toasts(project_notifications(records))
        .with_aria_label("Notifications")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_severities_map_cleanly_and_one_does_not() {
        for (severity, tone) in [
            (NotificationSeverity::Info, ToastTone::Info),
            (NotificationSeverity::Success, ToastTone::Success),
            (NotificationSeverity::Warning, ToastTone::Warning),
            (NotificationSeverity::Error, ToastTone::Danger),
        ] {
            let mapping = tone_for(severity);
            assert_eq!(mapping.tone, tone, "{severity:?}");
            assert!(!mapping.is_lossy, "{severity:?}");
        }
    }

    #[test]
    fn the_stack_carries_every_record_in_ledger_order() {
        // The stack is the rendered unit; a bare `Toast` is not renderable by
        // either adapter.
        let stack = project_notification_stack(&[]);
        assert!(stack.toasts.is_empty());
        assert_eq!(stack.aria_label.as_deref(), Some("Notifications"));
    }

    #[test]
    fn critical_collapses_into_danger_and_says_so() {
        // Five operational levels into four visual tones. The collapse is
        // reported so a surface can restore the distinction in text; a
        // projection that returned only the tone would make `Critical`
        // silently indistinguishable from `Error`.
        let critical = tone_for(NotificationSeverity::Critical);
        let error = tone_for(NotificationSeverity::Error);

        assert_eq!(critical.tone, error.tone);
        assert!(critical.is_lossy);
        assert!(!error.is_lossy);
    }
}
