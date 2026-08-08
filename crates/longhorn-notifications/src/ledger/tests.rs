//! Unit tests for ledger overflow and prune guards.

use longhorn_core::{
    NotificationAuthorityId, NotificationId, NotificationLedgerRevision, NotificationSourceId,
};

use crate::{
    NotificationAdd, NotificationAuthorityEpoch, NotificationDraft, NotificationLedgerError,
    NotificationLedgerLimits, NotificationSequence, NotificationSeverity, NotificationSummary,
    NotificationTitle,
};

use super::NotificationLedger;

fn ledger(maximum: usize) -> NotificationLedger {
    NotificationLedger::new(
        NotificationAuthorityId::new("notifications:overflow").unwrap(),
        NotificationAuthorityEpoch::new(1).unwrap(),
        NotificationLedgerLimits::new(maximum, 1 << 20).unwrap(),
    )
}

fn request(ledger: &NotificationLedger, suffix: &str) -> NotificationAdd {
    NotificationAdd::new(
        ledger.authority().clone(),
        ledger.revision(),
        NotificationId::new(format!("notification:{suffix}")).unwrap(),
        NotificationDraft::new(
            NotificationSourceId::new("source:test").unwrap(),
            NotificationSeverity::Info,
            NotificationTitle::new(suffix).unwrap(),
            NotificationSummary::new(suffix).unwrap(),
        ),
    )
}

#[test]
fn revision_sequence_and_prune_count_overflow_reject_atomically() {
    let mut revision = ledger(2);
    revision.revision = NotificationLedgerRevision::new(u64::MAX);
    let before = revision.clone();
    assert_eq!(
        revision.add(request(&revision, "revision")),
        Err(NotificationLedgerError::RevisionOverflow)
    );
    assert_eq!(revision, before);

    let mut sequence = ledger(2);
    sequence.next_sequence = NotificationSequence::for_test(u64::MAX);
    let before = sequence.clone();
    assert_eq!(
        sequence.add(request(&sequence, "sequence")),
        Err(NotificationLedgerError::SequenceOverflow)
    );
    assert_eq!(sequence, before);

    let mut pruned = ledger(1);
    pruned.add(request(&pruned, "oldest")).unwrap();
    pruned.pruned_count = u64::MAX;
    let before = pruned.clone();
    assert_eq!(
        pruned.add(request(&pruned, "newest")),
        Err(NotificationLedgerError::PrunedCountOverflow)
    );
    assert_eq!(pruned, before);
}
