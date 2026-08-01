//! Contract fixtures for the pure retained notification ledger.

use longhorn_core::{
    NotificationActionReferenceId, NotificationAuthorityId, NotificationCauseId, NotificationId,
    NotificationProducerToken, NotificationReplacementKey, NotificationSourceId,
};
use longhorn_notifications::{
    MAXIMUM_NOTIFICATION_ACTION_LABEL_BYTES, MAXIMUM_NOTIFICATION_ACTIONS,
    MAXIMUM_NOTIFICATION_SUMMARY_BYTES, MAXIMUM_NOTIFICATION_TITLE_BYTES, NotificationAction,
    NotificationActionLabel, NotificationAdd, NotificationAuthorityCursor,
    NotificationAuthorityEpoch, NotificationClear, NotificationClearTarget, NotificationDraft,
    NotificationLedger, NotificationLedgerError, NotificationLedgerLimits, NotificationPublishOnce,
    NotificationPublishOutcome, NotificationReadState, NotificationRemovalReason,
    NotificationReplace, NotificationRetentionChange, NotificationRetentionClass, NotificationSeen,
    NotificationSeverity, NotificationSummary, NotificationTitle,
};

fn ledger_with_limits(count: usize, weight: u64) -> NotificationLedger {
    NotificationLedger::new(
        NotificationAuthorityId::new("notifications:test").unwrap(),
        NotificationAuthorityEpoch::new(1).unwrap(),
        NotificationLedgerLimits::new(count, weight).unwrap(),
    )
}

fn ledger() -> NotificationLedger {
    ledger_with_limits(500, 32 * 1_024 * 1_024)
}

fn id(value: &str) -> NotificationId {
    NotificationId::new(value).unwrap()
}

fn draft(source: &str, title: &str, summary: &str) -> NotificationDraft {
    NotificationDraft::new(
        NotificationSourceId::new(source).unwrap(),
        NotificationSeverity::Info,
        NotificationTitle::new(title).unwrap(),
        NotificationSummary::new(summary).unwrap(),
    )
}

fn add(
    ledger: &NotificationLedger,
    notification_id: &str,
    draft: NotificationDraft,
) -> NotificationAdd {
    NotificationAdd::new(
        ledger.authority().clone(),
        ledger.revision(),
        id(notification_id),
        draft,
    )
}

#[test]
fn add_is_fresh_title_never_deduplicates_and_stale_commands_are_atomic() {
    let mut ledger = ledger();
    let first = add(
        &ledger,
        "notification:first",
        draft("source:reliability", "Connection lost", "First event"),
    );
    ledger.add(first.clone()).unwrap();
    let stale_before = ledger.clone();
    assert!(matches!(
        ledger.add(first),
        Err(NotificationLedgerError::StaleRevision { .. })
    ));
    assert_eq!(ledger, stale_before);

    ledger
        .add(add(
            &ledger,
            "notification:second",
            draft("source:reliability", "Connection lost", "Second event"),
        ))
        .unwrap();
    assert_eq!(ledger.records().len(), 2);
    assert_eq!(ledger.projection().unwrap().unseen_count(), 2);
}

#[test]
fn exact_identity_and_state_rejections_leave_the_ledger_unchanged() {
    let mut ledger = ledger();
    ledger
        .add(add(
            &ledger,
            "notification:one",
            draft("source:test", "One", "One")
                .with_replacement_key(NotificationReplacementKey::new("key:one").unwrap()),
        ))
        .unwrap();
    let before = ledger.clone();

    assert!(matches!(
        ledger.add(add(
            &ledger,
            "notification:one",
            draft("source:test", "Duplicate", "Duplicate"),
        )),
        Err(NotificationLedgerError::DuplicateNotification { .. })
    ));
    assert_eq!(
        ledger.replace(NotificationReplace::new(
            ledger.authority().clone(),
            ledger.revision(),
            draft("source:test", "Missing key", "Missing key"),
            false,
        )),
        Err(NotificationLedgerError::MissingReplacementKey)
    );
    assert!(matches!(
        ledger.replace(NotificationReplace::new(
            ledger.authority().clone(),
            ledger.revision(),
            draft("source:test", "Unknown", "Unknown")
                .with_replacement_key(NotificationReplacementKey::new("key:unknown").unwrap()),
            false,
        )),
        Err(NotificationLedgerError::ReplacementTargetNotFound { .. })
    ));
    assert!(matches!(
        ledger.mark_seen(NotificationSeen::new(
            ledger.authority().clone(),
            ledger.revision(),
            id("notification:unknown"),
        )),
        Err(NotificationLedgerError::NotificationNotFound { .. })
    ));
    assert!(matches!(
        ledger.clear(NotificationClear::new(
            ledger.authority().clone(),
            ledger.revision(),
            NotificationClearTarget::Records(vec![id("notification:one"), id("notification:one"),]),
        )),
        Err(NotificationLedgerError::DuplicateClearTarget { .. })
    ));
    assert!(matches!(
        ledger.clear(NotificationClear::new(
            ledger.authority().clone(),
            ledger.revision(),
            NotificationClearTarget::Records(vec![
                id("notification:one");
                longhorn_notifications::MAXIMUM_RETAINED_NOTIFICATIONS
                    + 1
            ]),
        )),
        Err(NotificationLedgerError::TooManyClearTargets { .. })
    ));
    let foreign = NotificationAuthorityCursor::new(
        NotificationAuthorityId::new("notifications:foreign").unwrap(),
        NotificationAuthorityEpoch::new(1).unwrap(),
    );
    assert!(matches!(
        ledger.dismiss(foreign, ledger.revision(), id("notification:one")),
        Err(NotificationLedgerError::WrongAuthority { .. })
    ));
    assert_eq!(ledger, before);
}

#[test]
fn all_consumer_text_and_action_collections_have_hard_bounds() {
    assert!(NotificationTitle::new("x".repeat(MAXIMUM_NOTIFICATION_TITLE_BYTES + 1)).is_err());
    assert!(NotificationSummary::new("x".repeat(MAXIMUM_NOTIFICATION_SUMMARY_BYTES + 1)).is_err());
    assert!(
        NotificationActionLabel::new("x".repeat(MAXIMUM_NOTIFICATION_ACTION_LABEL_BYTES + 1))
            .is_err()
    );
    let actions = (0..=MAXIMUM_NOTIFICATION_ACTIONS)
        .map(|index| {
            NotificationAction::new(
                NotificationActionReferenceId::new(format!("action:{index}")).unwrap(),
                NotificationActionLabel::new("Action").unwrap(),
            )
        })
        .collect();
    assert!(matches!(
        draft("source:test", "Bounded", "Bounded").with_actions(actions),
        Err(NotificationLedgerError::TooManyActions { .. })
    ));
}

#[test]
fn replace_is_explicit_keeps_identity_and_preserves_seen_unless_requested() {
    let mut ledger = ledger();
    let initial = draft("loophole.render", "Rendering", "Queued")
        .with_replacement_key(NotificationReplacementKey::new("render:42").unwrap());
    ledger
        .add(add(&ledger, "notification:render-42", initial))
        .unwrap();
    ledger
        .mark_seen(NotificationSeen::new(
            ledger.authority().clone(),
            ledger.revision(),
            id("notification:render-42"),
        ))
        .unwrap();

    let replacement = draft("loophole.render", "Render complete", "Export ready")
        .with_replacement_key(NotificationReplacementKey::new("render:42").unwrap());
    let receipt = ledger
        .replace(NotificationReplace::new(
            ledger.authority().clone(),
            ledger.revision(),
            replacement.clone(),
            false,
        ))
        .unwrap();
    assert_eq!(
        receipt.record().notification_id().as_str(),
        "notification:render-42"
    );
    assert_eq!(receipt.record().read_state(), NotificationReadState::Seen);
    assert_eq!(receipt.record().draft().summary().as_str(), "Export ready");

    let receipt = ledger
        .replace(NotificationReplace::new(
            ledger.authority().clone(),
            ledger.revision(),
            replacement,
            true,
        ))
        .unwrap();
    assert_eq!(receipt.record().read_state(), NotificationReadState::Unseen);
}

#[test]
fn producer_token_is_explicit_idempotency_not_add_deduplication() {
    let mut ledger = ledger();
    let token = NotificationProducerToken::new("operation:render-42:succeeded").unwrap();
    let publication = NotificationPublishOnce::new(add(
        &ledger,
        "notification:render-42",
        draft("loophole.render", "Render complete", "Export ready").with_producer_token(token),
    ));
    assert!(matches!(
        ledger.publish_once(publication.clone()).unwrap(),
        NotificationPublishOutcome::Published(_)
    ));
    let revision = ledger.revision();
    assert!(matches!(
        ledger.publish_once(publication).unwrap(),
        NotificationPublishOutcome::AlreadyPublished { .. }
    ));
    assert_eq!(ledger.revision(), revision);
    assert_eq!(ledger.records().len(), 1);
}

#[test]
fn seen_dismiss_clear_and_prune_have_distinct_receipts() {
    let mut ledger = ledger();
    for suffix in ["one", "two", "three"] {
        ledger
            .add(add(
                &ledger,
                &format!("notification:{suffix}"),
                draft("source:test", suffix, suffix),
            ))
            .unwrap();
    }
    let seen = ledger
        .mark_seen(NotificationSeen::new(
            ledger.authority().clone(),
            ledger.revision(),
            id("notification:one"),
        ))
        .unwrap();
    assert!(seen.pruned().is_empty());
    assert_eq!(ledger.records().len(), 3);
    assert_eq!(ledger.projection().unwrap().unseen_count(), 2);
    let before = ledger.clone();
    assert_eq!(
        ledger.mark_seen(NotificationSeen::new(
            ledger.authority().clone(),
            ledger.revision(),
            id("notification:one"),
        )),
        Err(NotificationLedgerError::AlreadySeen {
            notification_id: id("notification:one")
        })
    );
    assert_eq!(ledger, before);

    let dismissed = ledger
        .dismiss(
            ledger.authority().clone(),
            ledger.revision(),
            id("notification:one"),
        )
        .unwrap();
    assert_eq!(
        dismissed.removals()[0].reason(),
        NotificationRemovalReason::Dismissed
    );
    let cleared = ledger
        .clear(NotificationClear::new(
            ledger.authority().clone(),
            ledger.revision(),
            NotificationClearTarget::Records(vec![id("notification:two")]),
        ))
        .unwrap();
    assert_eq!(
        cleared.removals()[0].reason(),
        NotificationRemovalReason::Cleared
    );
    let pruned = ledger
        .change_retention(NotificationRetentionChange::new(
            ledger.authority().clone(),
            ledger.revision(),
            NotificationLedgerLimits::new(0, 0).unwrap(),
        ))
        .unwrap();
    assert_eq!(
        pruned.removals()[0].reason(),
        NotificationRemovalReason::Pruned
    );
    assert_eq!(ledger.projection().unwrap().pruned_count(), 1);
}

#[test]
fn count_and_weight_retention_are_oldest_first_finite_and_exact() {
    let mut ledger = ledger_with_limits(2, 32 * 1_024 * 1_024);
    for suffix in ["one", "two", "three"] {
        let receipt = ledger
            .add(add(
                &ledger,
                &format!("notification:{suffix}"),
                draft("source:test", suffix, suffix),
            ))
            .unwrap();
        if suffix == "three" {
            assert_eq!(receipt.pruned().len(), 1);
            assert_eq!(
                receipt.pruned()[0].notification_id().as_str(),
                "notification:one"
            );
        }
    }

    let retained_weight = ledger.projection().unwrap().retained_encoded_weight();
    let receipt = ledger
        .change_retention(NotificationRetentionChange::new(
            ledger.authority().clone(),
            ledger.revision(),
            NotificationLedgerLimits::new(2, retained_weight - 1).unwrap(),
        ))
        .unwrap();
    assert_eq!(receipt.removals().len(), 1);
    assert_eq!(
        receipt.removals()[0].notification_id().as_str(),
        "notification:two"
    );
}

#[test]
fn protected_records_make_unsatisfied_retention_reject_without_mutation() {
    let mut ledger = ledger_with_limits(1, 32 * 1_024 * 1_024);
    ledger
        .add(add(
            &ledger,
            "notification:protected",
            draft(
                "source:critical",
                "Storage failed",
                "Manual repair required",
            )
            .with_retention_class(NotificationRetentionClass::Protected),
        ))
        .unwrap();
    let before = ledger.clone();
    assert!(matches!(
        ledger.add(add(
            &ledger,
            "notification:new",
            draft("source:test", "New", "Must not be silently dropped")
        )),
        Err(NotificationLedgerError::RetentionUnsatisfied { .. })
    ));
    assert_eq!(ledger, before);
}

#[test]
fn pages_are_newest_first_bounded_and_report_ledger_wide_unseen() {
    let mut ledger = ledger();
    for suffix in ["one", "two", "three"] {
        ledger
            .add(add(
                &ledger,
                &format!("notification:{suffix}"),
                draft("source:test", suffix, suffix),
            ))
            .unwrap();
    }
    ledger
        .mark_seen(NotificationSeen::new(
            ledger.authority().clone(),
            ledger.revision(),
            id("notification:two"),
        ))
        .unwrap();
    let page = ledger.page(0, 2).unwrap();
    assert_eq!(
        page.records()[0].notification_id().as_str(),
        "notification:three"
    );
    assert_eq!(
        page.records()[1].notification_id().as_str(),
        "notification:two"
    );
    assert_eq!(page.unseen_count(), 2);
    assert!(page.has_more());
    assert!(matches!(
        ledger.page(0, 0),
        Err(NotificationLedgerError::InvalidPageSize(_))
    ));
}

#[test]
fn loophole_render_and_non_operation_reliability_share_one_record_shape() {
    let action = NotificationAction::new(
        NotificationActionReferenceId::new("render:open-report").unwrap(),
        NotificationActionLabel::new("Open report").unwrap(),
    );
    let render = draft("loophole.render", "Render failed", "Stem export stopped")
        .with_cause(NotificationCauseId::new("operation:render-42").unwrap())
        .with_actions(vec![action])
        .unwrap()
        .with_replacement_key(NotificationReplacementKey::new("render:42").unwrap())
        .with_producer_token(NotificationProducerToken::new("render:42:failed").unwrap());
    let reliability = draft(
        "loophole.reliability",
        "Audio device unavailable",
        "The selected output disappeared",
    )
    .with_retention_class(NotificationRetentionClass::Protected);
    let mut ledger = ledger();
    ledger
        .add(add(&ledger, "notification:render-42", render))
        .unwrap();
    ledger
        .add(add(&ledger, "notification:device-loss", reliability))
        .unwrap();
    let page = ledger.page(0, 10).unwrap();
    assert_eq!(page.records().len(), 2);
    assert_eq!(
        page.records()[0].draft().source_id().as_str(),
        "loophole.reliability"
    );
    assert_eq!(
        page.records()[1].draft().source_id().as_str(),
        "loophole.render"
    );
}

#[cfg(feature = "operation")]
mod operation {
    use super::*;
    use longhorn_core::{
        NotificationLedgerRevision, OperationAuthorityId, OperationId, OperationKindId,
    };
    use longhorn_notifications::{
        OperationNotificationObservationError, OperationNotificationPolicy,
        OperationNotificationPublication, observe_terminal_operation,
        publish_operation_notification,
    };
    use longhorn_operation::{
        OperationAuthorityEpoch, OperationCancellationSupport, OperationCatalogue,
        OperationCatalogueLimits, OperationLabel, OperationRegistration, OperationState,
        OperationTransition,
    };

    struct RenderPolicy;

    impl OperationNotificationPolicy for RenderPolicy {
        fn publication(
            &self,
            operation: &longhorn_operation::OperationRecord,
            _receipt: &longhorn_operation::OperationTransitionReceipt,
        ) -> Option<OperationNotificationPublication> {
            Some(OperationNotificationPublication::new(
                NotificationId::new(format!(
                    "notification:{}",
                    operation.operation_id().as_str().replace(':', "-")
                ))
                .unwrap(),
                draft(
                    "loophole.render",
                    "Render failed",
                    "Inspect the render report",
                )
                .with_producer_token(
                    NotificationProducerToken::new(format!("{}:failed", operation.operation_id()))
                        .unwrap(),
                ),
            ))
        }
    }

    fn committed_failure() -> (
        OperationCatalogue,
        longhorn_operation::OperationTransitionReceipt,
    ) {
        let mut catalogue = OperationCatalogue::new(
            OperationAuthorityId::new("operations:test").unwrap(),
            OperationAuthorityEpoch::new(1).unwrap(),
            OperationCatalogueLimits::default(),
        );
        catalogue
            .register(OperationRegistration::new(
                catalogue.authority().clone(),
                catalogue.revision(),
                OperationId::new("operation:render-42").unwrap(),
                OperationKindId::new("loophole.render").unwrap(),
                None,
                OperationLabel::new("Render stems").unwrap(),
                OperationState::Running,
                OperationCancellationSupport::Supported,
                None,
            ))
            .unwrap();
        let receipt = catalogue
            .transition(OperationTransition::new(
                catalogue.authority().clone(),
                OperationId::new("operation:render-42").unwrap(),
                catalogue
                    .operation(&OperationId::new("operation:render-42").unwrap())
                    .unwrap()
                    .revision(),
                OperationState::Failed,
            ))
            .unwrap();
        (catalogue, receipt)
    }

    #[test]
    fn terminal_projection_is_optional_idempotent_and_failure_isolated() {
        let (catalogue, receipt) = committed_failure();
        let committed = catalogue.clone();
        let operation = catalogue
            .operation(&OperationId::new("operation:render-42").unwrap())
            .unwrap();
        let publication = observe_terminal_operation(&RenderPolicy, operation, &receipt)
            .unwrap()
            .unwrap();

        let mut closed_ledger = ledger_with_limits(0, 0);
        assert!(matches!(
            publish_operation_notification(&mut closed_ledger, publication.clone()),
            Err(NotificationLedgerError::RetentionUnsatisfied { .. })
        ));
        assert_eq!(catalogue, committed);
        assert_eq!(
            closed_ledger.revision(),
            NotificationLedgerRevision::INITIAL
        );

        let mut ledger = ledger();
        assert!(matches!(
            publish_operation_notification(&mut ledger, publication.clone()).unwrap(),
            NotificationPublishOutcome::Published(_)
        ));
        let revision = ledger.revision();
        assert!(matches!(
            publish_operation_notification(&mut ledger, publication).unwrap(),
            NotificationPublishOutcome::AlreadyPublished { .. }
        ));
        assert_eq!(ledger.revision(), revision);
    }

    #[test]
    fn observer_rejects_non_terminal_receipts() {
        let mut active = OperationCatalogue::new(
            OperationAuthorityId::new("operations:active").unwrap(),
            OperationAuthorityEpoch::new(1).unwrap(),
            OperationCatalogueLimits::default(),
        );
        active
            .register(OperationRegistration::new(
                active.authority().clone(),
                active.revision(),
                OperationId::new("operation:active").unwrap(),
                OperationKindId::new("test").unwrap(),
                None,
                OperationLabel::new("Active").unwrap(),
                OperationState::Queued,
                OperationCancellationSupport::Supported,
                None,
            ))
            .unwrap();
        let receipt = active
            .transition(OperationTransition::new(
                active.authority().clone(),
                OperationId::new("operation:active").unwrap(),
                active
                    .operation(&OperationId::new("operation:active").unwrap())
                    .unwrap()
                    .revision(),
                OperationState::Running,
            ))
            .unwrap();
        let record = active
            .operation(&OperationId::new("operation:active").unwrap())
            .unwrap();
        assert_eq!(
            observe_terminal_operation(&RenderPolicy, record, &receipt),
            Err(OperationNotificationObservationError::NonTerminalTransition)
        );
    }
}
