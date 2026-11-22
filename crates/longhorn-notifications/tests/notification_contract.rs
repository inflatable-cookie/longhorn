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
        ledger.dismiss(longhorn_notifications::NotificationDismiss::new(
            foreign,
            ledger.revision(),
            id("notification:one"),
        )),
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
        ledger.publish_once(publication.clone()).unwrap(),
        NotificationPublishOutcome::AlreadyPublished {
            record: Some(_),
            ..
        }
    ));
    assert_eq!(ledger.revision(), revision);
    assert_eq!(ledger.records().len(), 1);

    // Dismissing the notification must NOT allow the same producer token to
    // re-publish it (contract 016 bounded token memory).
    ledger
        .dismiss(longhorn_notifications::NotificationDismiss::new(
            ledger.authority().clone(),
            ledger.revision(),
            id("notification:render-42"),
        ))
        .unwrap();
    assert_eq!(ledger.records().len(), 0);
    let revision_after_dismiss = ledger.revision();
    let replayed = ledger
        .publish_once(NotificationPublishOnce::new(add(
            &ledger,
            "notification:render-42-replay",
            draft("loophole.render", "Render complete", "Export ready").with_producer_token(
                NotificationProducerToken::new("operation:render-42:succeeded").unwrap(),
            ),
        )))
        .unwrap();
    assert_eq!(
        replayed,
        NotificationPublishOutcome::AlreadyPublished {
            notification_id: id("notification:render-42"),
            record: None,
            ledger_revision: revision_after_dismiss,
        }
    );
    assert_eq!(ledger.revision(), revision_after_dismiss);
    assert_eq!(ledger.records().len(), 0);
}

#[test]
fn publish_once_remains_idempotent_after_pruning_and_noop_retention_keeps_revision() {
    let mut ledger = ledger_with_limits(1, 32 * 1_024 * 1_024);
    let token = NotificationProducerToken::new("operation:pruned-1:succeeded").unwrap();
    assert!(matches!(
        ledger
            .publish_once(NotificationPublishOnce::new(add(
                &ledger,
                "notification:pruned-1",
                draft("source:test", "First", "First").with_producer_token(token.clone()),
            )))
            .unwrap(),
        NotificationPublishOutcome::Published(_)
    ));
    // Adding a second notification prunes the first.
    let add_receipt = ledger
        .add(add(
            &ledger,
            "notification:second",
            draft("source:test", "Second", "Second"),
        ))
        .unwrap();
    assert_eq!(add_receipt.pruned().len(), 1);
    let rev_before_replay = ledger.revision();
    let replay = ledger
        .publish_once(NotificationPublishOnce::new(add(
            &ledger,
            "notification:pruned-1-again",
            draft("source:test", "First", "First").with_producer_token(token),
        )))
        .unwrap();
    assert_eq!(
        replay,
        NotificationPublishOutcome::AlreadyPublished {
            notification_id: id("notification:pruned-1"),
            record: None,
            ledger_revision: rev_before_replay,
        }
    );
    assert_eq!(ledger.revision(), rev_before_replay);

    // No-op retention change must not advance the ledger revision.
    let noop = ledger
        .change_retention(NotificationRetentionChange::new(
            ledger.authority().clone(),
            ledger.revision(),
            ledger.limits(),
        ))
        .unwrap();
    assert_eq!(noop.previous_ledger_revision(), rev_before_replay);
    assert_eq!(noop.committed_ledger_revision(), rev_before_replay);
    assert!(noop.removals().is_empty());
    assert_eq!(ledger.revision(), rev_before_replay);
}

#[test]
fn producer_token_fifo_window_evicts_oldest_after_maximum_remembered_tokens() {
    let cap = longhorn_notifications::MAXIMUM_REMEMBERED_PRODUCER_TOKENS;
    let mut ledger = ledger_with_limits(1, 32 * 1_024 * 1_024);
    let first_token = NotificationProducerToken::new("token:0").unwrap();
    assert!(matches!(
        ledger
            .publish_once(NotificationPublishOnce::new(add(
                &ledger,
                "notification:0",
                draft("source:test", "Zero", "Zero").with_producer_token(first_token.clone()),
            )))
            .unwrap(),
        NotificationPublishOutcome::Published(_)
    ));
    ledger
        .dismiss(longhorn_notifications::NotificationDismiss::new(
            ledger.authority().clone(),
            ledger.revision(),
            id("notification:0"),
        ))
        .unwrap();

    // Fill the remaining cap - 1 slots with distinct tokens.
    for index in 1..cap {
        let token = NotificationProducerToken::new(format!("token:{index}")).unwrap();
        assert!(matches!(
            ledger
                .publish_once(NotificationPublishOnce::new(add(
                    &ledger,
                    &format!("notification:{index}"),
                    draft("source:test", "Fill", "Fill").with_producer_token(token),
                )))
                .unwrap(),
            NotificationPublishOutcome::Published(_)
        ));
    }

    // `token:0` is still at the front of the FIFO window: replay is suppressed.
    let rev_before = ledger.revision();
    assert_eq!(
        ledger
            .publish_once(NotificationPublishOnce::new(add(
                &ledger,
                "notification:0-replay-inside-window",
                draft("source:test", "Zero", "Zero").with_producer_token(first_token.clone()),
            )))
            .unwrap(),
        NotificationPublishOutcome::AlreadyPublished {
            notification_id: id("notification:0"),
            record: None,
            ledger_revision: rev_before,
        }
    );

    // One more distinct token evicts `token:0` from the FIFO window.
    let overflow_token = NotificationProducerToken::new(format!("token:{cap}")).unwrap();
    assert!(matches!(
        ledger
            .publish_once(NotificationPublishOnce::new(add(
                &ledger,
                &format!("notification:{cap}"),
                draft("source:test", "Overflow", "Overflow").with_producer_token(overflow_token),
            )))
            .unwrap(),
        NotificationPublishOutcome::Published(_)
    ));

    // After eviction, replaying `token:0` publishes a fresh record.
    assert!(matches!(
        ledger
            .publish_once(NotificationPublishOnce::new(add(
                &ledger,
                "notification:0-after-eviction",
                draft("source:test", "Zero", "Zero").with_producer_token(first_token),
            )))
            .unwrap(),
        NotificationPublishOutcome::Published(_)
    ));
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
        .dismiss(longhorn_notifications::NotificationDismiss::new(
            ledger.authority().clone(),
            ledger.revision(),
            id("notification:one"),
        ))
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
        OperationNotificationPublication, OperationTerminalEvidence, observe_terminal_operation,
        observe_terminal_receipt, publish_operation_notification,
    };
    use longhorn_operation::{
        OperationAuthorityEpoch, OperationCancellationRequest, OperationCancellationSupport,
        OperationCatalogue, OperationCatalogueLimits, OperationLabel, OperationRegistration,
        OperationState, OperationTeardown, OperationTeardownResolution,
        OperationTeardownResolutionOutcome, OperationTransition, QueuedCancellationConfirmation,
    };

    struct RenderPolicy;

    impl OperationNotificationPolicy for RenderPolicy {
        fn publication(
            &self,
            operation: &longhorn_operation::OperationRecord,
            _evidence: OperationTerminalEvidence<'_>,
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

    #[test]
    fn observer_handles_confirmed_queued_cancellation_and_teardown_terminals() {
        let mut catalogue = OperationCatalogue::new(
            OperationAuthorityId::new("operations:terminal-paths").unwrap(),
            OperationAuthorityEpoch::new(1).unwrap(),
            OperationCatalogueLimits::default(),
        );
        catalogue
            .register(OperationRegistration::new(
                catalogue.authority().clone(),
                catalogue.revision(),
                OperationId::new("operation:queued-cancel").unwrap(),
                OperationKindId::new("loophole.render").unwrap(),
                None,
                OperationLabel::new("Queued cancel").unwrap(),
                OperationState::Queued,
                OperationCancellationSupport::Supported,
                None,
            ))
            .unwrap();
        catalogue
            .register(OperationRegistration::new(
                catalogue.authority().clone(),
                catalogue.revision(),
                OperationId::new("operation:teardown").unwrap(),
                OperationKindId::new("loophole.render").unwrap(),
                None,
                OperationLabel::new("Teardown").unwrap(),
                OperationState::Running,
                OperationCancellationSupport::Supported,
                None,
            ))
            .unwrap();

        let queued_op = catalogue
            .operation(&OperationId::new("operation:queued-cancel").unwrap())
            .unwrap();
        let cancel_receipt = catalogue
            .request_cancellation(
                OperationCancellationRequest::new(
                    catalogue.authority().clone(),
                    queued_op.operation_id().clone(),
                    queued_op.revision(),
                )
                .with_queued_confirmation(QueuedCancellationConfirmation::ConfirmedUnstarted),
            )
            .unwrap();
        let queued_record = catalogue
            .operation(&OperationId::new("operation:queued-cancel").unwrap())
            .unwrap();
        let cancel_pub = observe_terminal_operation(&RenderPolicy, queued_record, &cancel_receipt)
            .unwrap()
            .expect("confirmed queued cancel is observable");
        assert_eq!(
            cancel_pub.notification_id().as_str(),
            "notification:operation-queued-cancel"
        );

        let teardown_op = catalogue
            .operation(&OperationId::new("operation:teardown").unwrap())
            .unwrap();
        let teardown_receipt = catalogue
            .teardown(OperationTeardown::new(
                catalogue.authority().clone(),
                catalogue.revision(),
                vec![OperationTeardownResolution::new(
                    teardown_op.operation_id().clone(),
                    teardown_op.revision(),
                    OperationTeardownResolutionOutcome::Complete(OperationState::Interrupted),
                )],
            ))
            .unwrap();
        let teardown_record = catalogue
            .operation(&OperationId::new("operation:teardown").unwrap())
            .unwrap();
        let teardown_pub =
            observe_terminal_operation(&RenderPolicy, teardown_record, &teardown_receipt)
                .unwrap()
                .expect("teardown terminal outcome is observable");
        assert_eq!(
            teardown_pub.notification_id().as_str(),
            "notification:operation-teardown"
        );
    }

    #[test]
    fn observer_rejects_noop_terminal_cancellation_receipt() {
        let mut catalogue = OperationCatalogue::new(
            OperationAuthorityId::new("operations:noop-cancel").unwrap(),
            OperationAuthorityEpoch::new(1).unwrap(),
            OperationCatalogueLimits::default(),
        );
        let op_id = OperationId::new("operation:already-succeeded").unwrap();
        catalogue
            .register(OperationRegistration::new(
                catalogue.authority().clone(),
                catalogue.revision(),
                op_id.clone(),
                OperationKindId::new("loophole.render").unwrap(),
                None,
                OperationLabel::new("Done").unwrap(),
                OperationState::Running,
                OperationCancellationSupport::Supported,
                None,
            ))
            .unwrap();
        let rev = catalogue.operation(&op_id).unwrap().revision();
        catalogue
            .transition(OperationTransition::new(
                catalogue.authority().clone(),
                op_id.clone(),
                rev,
                OperationState::Succeeded,
            ))
            .unwrap();
        let done_rev = catalogue.operation(&op_id).unwrap().revision();
        let noop_cancel = catalogue
            .request_cancellation(OperationCancellationRequest::new(
                catalogue.authority().clone(),
                op_id.clone(),
                done_rev,
            ))
            .unwrap();
        assert_eq!(
            noop_cancel.outcome(),
            longhorn_operation::OperationCancellationOutcome::Terminal
        );
        let record = catalogue.operation(&op_id).unwrap();
        assert_eq!(
            observe_terminal_operation(&RenderPolicy, record, &noop_cancel),
            Err(OperationNotificationObservationError::NonTerminalTransition)
        );
        assert_eq!(
            observe_terminal_receipt(&RenderPolicy, &noop_cancel),
            Err(OperationNotificationObservationError::NonTerminalTransition)
        );
    }

    #[test]
    fn observer_handles_evicted_queued_cancel_and_teardown_under_zero_terminal_retention() {
        // Zero terminal retention evicts the completed operation during the
        // same commit; receipts carry the committed record so observation works
        // independently of retention.
        let zero_terminal = OperationCatalogueLimits::new(2, 0, 0).unwrap();
        let mut catalogue = OperationCatalogue::new(
            OperationAuthorityId::new("operations:zero-retention").unwrap(),
            OperationAuthorityEpoch::new(1).unwrap(),
            zero_terminal,
        );
        let cancel_id = OperationId::new("operation:evicted-cancel").unwrap();
        let teardown_id = OperationId::new("operation:evicted-teardown").unwrap();
        for (id, initial) in [
            (&cancel_id, OperationState::Queued),
            (&teardown_id, OperationState::Running),
        ] {
            catalogue
                .register(OperationRegistration::new(
                    catalogue.authority().clone(),
                    catalogue.revision(),
                    id.clone(),
                    OperationKindId::new("loophole.render").unwrap(),
                    None,
                    OperationLabel::new("Evicted on terminal").unwrap(),
                    initial,
                    OperationCancellationSupport::Supported,
                    None,
                ))
                .unwrap();
        }

        let cancel_rev = catalogue.operation(&cancel_id).unwrap().revision();
        let cancel_receipt = catalogue
            .request_cancellation(
                OperationCancellationRequest::new(
                    catalogue.authority().clone(),
                    cancel_id.clone(),
                    cancel_rev,
                )
                .with_queued_confirmation(QueuedCancellationConfirmation::ConfirmedUnstarted),
            )
            .unwrap();
        assert!(
            catalogue.operation(&cancel_id).is_none(),
            "zero terminal retention must evict the operation immediately"
        );
        let cancel_pub = observe_terminal_receipt(&RenderPolicy, &cancel_receipt)
            .unwrap()
            .expect("evicted queued cancel is still observable from its receipt");
        assert_eq!(
            cancel_pub.notification_id().as_str(),
            "notification:operation-evicted-cancel"
        );

        let teardown_rev = catalogue.operation(&teardown_id).unwrap().revision();
        let teardown_receipt = catalogue
            .teardown(OperationTeardown::new(
                catalogue.authority().clone(),
                catalogue.revision(),
                vec![OperationTeardownResolution::new(
                    teardown_id.clone(),
                    teardown_rev,
                    OperationTeardownResolutionOutcome::Complete(OperationState::Interrupted),
                )],
            ))
            .unwrap();
        assert!(
            catalogue.operation(&teardown_id).is_none(),
            "zero terminal retention must evict the torn-down operation immediately"
        );
        let teardown_pub = observe_terminal_receipt(
            &RenderPolicy,
            OperationTerminalEvidence::TeardownOutcome {
                receipt: &teardown_receipt,
                outcome_index: 0,
            },
        )
        .unwrap()
        .expect("evicted teardown completion is still observable from its receipt");
        assert_eq!(
            teardown_pub.notification_id().as_str(),
            "notification:operation-evicted-teardown"
        );
    }

    #[test]
    fn observer_rejects_foreign_authority_epoch_record_with_matching_operation_id() {
        let authority_id = OperationAuthorityId::new("operations:epoch-check").unwrap();
        let op_id = OperationId::new("operation:render-42").unwrap();

        let make_catalogue = |epoch: u64| {
            OperationCatalogue::new(
                authority_id.clone(),
                OperationAuthorityEpoch::new(epoch).unwrap(),
                OperationCatalogueLimits::default(),
            )
        };

        // Same authority_id and operation_id, different authority epochs:
        // transition, confirmed queued cancellation, and teardown must all
        // reject a foreign-epoch record even when state and revision match.
        let mut epoch1_transition = make_catalogue(1);
        let mut epoch2_transition = make_catalogue(2);
        for catalogue in [&mut epoch1_transition, &mut epoch2_transition] {
            catalogue
                .register(OperationRegistration::new(
                    catalogue.authority().clone(),
                    catalogue.revision(),
                    op_id.clone(),
                    OperationKindId::new("loophole.render").unwrap(),
                    None,
                    OperationLabel::new("Render stems").unwrap(),
                    OperationState::Running,
                    OperationCancellationSupport::Supported,
                    None,
                ))
                .unwrap();
            let rev = catalogue.operation(&op_id).unwrap().revision();
            catalogue
                .transition(OperationTransition::new(
                    catalogue.authority().clone(),
                    op_id.clone(),
                    rev,
                    OperationState::Failed,
                ))
                .unwrap();
        }
        let mut epoch1_fresh = make_catalogue(1);
        epoch1_fresh
            .register(OperationRegistration::new(
                epoch1_fresh.authority().clone(),
                epoch1_fresh.revision(),
                op_id.clone(),
                OperationKindId::new("loophole.render").unwrap(),
                None,
                OperationLabel::new("Render stems").unwrap(),
                OperationState::Running,
                OperationCancellationSupport::Supported,
                None,
            ))
            .unwrap();
        let rev = epoch1_fresh.operation(&op_id).unwrap().revision();
        let transition_receipt = epoch1_fresh
            .transition(OperationTransition::new(
                epoch1_fresh.authority().clone(),
                op_id.clone(),
                rev,
                OperationState::Failed,
            ))
            .unwrap();
        let foreign_transition_record = epoch2_transition.operation(&op_id).unwrap();
        assert_eq!(
            foreign_transition_record.operation_id(),
            transition_receipt.operation_id()
        );
        assert_eq!(
            foreign_transition_record.state(),
            transition_receipt.committed_state()
        );
        assert_eq!(
            foreign_transition_record.revision(),
            transition_receipt.committed_operation_revision()
        );
        assert_eq!(
            observe_terminal_operation(
                &RenderPolicy,
                foreign_transition_record,
                &transition_receipt
            ),
            Err(OperationNotificationObservationError::OperationMismatch)
        );

        let mut epoch1_cancel = make_catalogue(1);
        let mut epoch2_cancel = make_catalogue(2);
        for catalogue in [&mut epoch1_cancel, &mut epoch2_cancel] {
            catalogue
                .register(OperationRegistration::new(
                    catalogue.authority().clone(),
                    catalogue.revision(),
                    op_id.clone(),
                    OperationKindId::new("loophole.render").unwrap(),
                    None,
                    OperationLabel::new("Queued").unwrap(),
                    OperationState::Queued,
                    OperationCancellationSupport::Supported,
                    None,
                ))
                .unwrap();
        }
        let c1_rev = epoch1_cancel.operation(&op_id).unwrap().revision();
        let cancel_receipt = epoch1_cancel
            .request_cancellation(
                OperationCancellationRequest::new(
                    epoch1_cancel.authority().clone(),
                    op_id.clone(),
                    c1_rev,
                )
                .with_queued_confirmation(QueuedCancellationConfirmation::ConfirmedUnstarted),
            )
            .unwrap();
        let c2_rev = epoch2_cancel.operation(&op_id).unwrap().revision();
        epoch2_cancel
            .request_cancellation(
                OperationCancellationRequest::new(
                    epoch2_cancel.authority().clone(),
                    op_id.clone(),
                    c2_rev,
                )
                .with_queued_confirmation(QueuedCancellationConfirmation::ConfirmedUnstarted),
            )
            .unwrap();
        let foreign_cancel_record = epoch2_cancel.operation(&op_id).unwrap();
        assert_eq!(
            observe_terminal_operation(&RenderPolicy, foreign_cancel_record, &cancel_receipt),
            Err(OperationNotificationObservationError::OperationMismatch)
        );

        let mut epoch1_teardown = make_catalogue(1);
        let mut epoch2_teardown = make_catalogue(2);
        for catalogue in [&mut epoch1_teardown, &mut epoch2_teardown] {
            catalogue
                .register(OperationRegistration::new(
                    catalogue.authority().clone(),
                    catalogue.revision(),
                    op_id.clone(),
                    OperationKindId::new("loophole.render").unwrap(),
                    None,
                    OperationLabel::new("Teardown").unwrap(),
                    OperationState::Running,
                    OperationCancellationSupport::Supported,
                    None,
                ))
                .unwrap();
        }
        let t1_rev = epoch1_teardown.operation(&op_id).unwrap().revision();
        let teardown_receipt = epoch1_teardown
            .teardown(OperationTeardown::new(
                epoch1_teardown.authority().clone(),
                epoch1_teardown.revision(),
                vec![OperationTeardownResolution::new(
                    op_id.clone(),
                    t1_rev,
                    OperationTeardownResolutionOutcome::Complete(OperationState::Interrupted),
                )],
            ))
            .unwrap();
        let t2_rev = epoch2_teardown.operation(&op_id).unwrap().revision();
        epoch2_teardown
            .teardown(OperationTeardown::new(
                epoch2_teardown.authority().clone(),
                epoch2_teardown.revision(),
                vec![OperationTeardownResolution::new(
                    op_id.clone(),
                    t2_rev,
                    OperationTeardownResolutionOutcome::Complete(OperationState::Interrupted),
                )],
            ))
            .unwrap();
        let foreign_teardown_record = epoch2_teardown.operation(&op_id).unwrap();
        assert_eq!(
            observe_terminal_operation(&RenderPolicy, foreign_teardown_record, &teardown_receipt),
            Err(OperationNotificationObservationError::OperationMismatch)
        );
        assert_eq!(
            observe_terminal_operation(
                &RenderPolicy,
                foreign_teardown_record,
                OperationTerminalEvidence::TeardownOutcome {
                    receipt: &teardown_receipt,
                    outcome_index: 0,
                },
            ),
            Err(OperationNotificationObservationError::OperationMismatch)
        );
    }

    #[test]
    fn observer_teardown_outcome_batch_observation_is_linear() {
        use std::time::Instant;

        let batch_size = 2_048_usize;
        let mut catalogue = OperationCatalogue::new(
            OperationAuthorityId::new("operations:batch-teardown").unwrap(),
            OperationAuthorityEpoch::new(1).unwrap(),
            OperationCatalogueLimits::new(batch_size, 0, 0).unwrap(),
        );
        let mut resolutions = Vec::with_capacity(batch_size);
        for index in 0..batch_size {
            let op_id = OperationId::new(format!("operation:batch-{index:04}")).unwrap();
            catalogue
                .register(OperationRegistration::new(
                    catalogue.authority().clone(),
                    catalogue.revision(),
                    op_id.clone(),
                    OperationKindId::new("loophole.render").unwrap(),
                    None,
                    OperationLabel::new("Batch item").unwrap(),
                    OperationState::Running,
                    OperationCancellationSupport::Supported,
                    None,
                ))
                .unwrap();
            let rev = catalogue.operation(&op_id).unwrap().revision();
            resolutions.push(OperationTeardownResolution::new(
                op_id,
                rev,
                OperationTeardownResolutionOutcome::Complete(OperationState::Interrupted),
            ));
        }

        let receipt = catalogue
            .teardown(OperationTeardown::new(
                catalogue.authority().clone(),
                catalogue.revision(),
                resolutions,
            ))
            .unwrap();
        assert_eq!(receipt.outcomes().len(), batch_size);

        let started = Instant::now();
        for outcome_index in 0..receipt.outcomes().len() {
            let publication = observe_terminal_receipt(
                &RenderPolicy,
                OperationTerminalEvidence::TeardownOutcome {
                    receipt: &receipt,
                    outcome_index,
                },
            )
            .unwrap()
            .expect("each completed teardown outcome publishes");
            assert_eq!(
                publication.notification_id().as_str(),
                format!("notification:operation-batch-{outcome_index:04}")
            );
        }
        let elapsed = started.elapsed();
        assert!(
            elapsed.as_secs() < 2,
            "observing {batch_size} teardown outcomes by index took too long: {elapsed:?}"
        );
        assert_eq!(
            observe_terminal_receipt(
                &RenderPolicy,
                OperationTerminalEvidence::TeardownOutcome {
                    receipt: &receipt,
                    outcome_index: batch_size,
                },
            ),
            Err(OperationNotificationObservationError::OperationMismatch)
        );
    }
}
