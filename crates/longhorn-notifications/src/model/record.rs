//! Authority cursor and retained notification records.

use longhorn_core::{
    NotificationActionReferenceId, NotificationAuthorityId, NotificationCauseId, NotificationId,
    NotificationLedgerRevision, NotificationProducerToken, NotificationReplacementKey,
    NotificationSourceId,
};

use crate::{
    MAXIMUM_NOTIFICATION_ACTIONS, NotificationActionLabel, NotificationAuthorityEpoch,
    NotificationLedgerLimits, NotificationSequence, NotificationSummary, NotificationTitle,
};

use super::{NotificationDraft, NotificationReadState};
/// Exact identity of one live notification authority instance.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationAuthorityCursor {
    pub(crate) authority_id: NotificationAuthorityId,
    pub(crate) authority_epoch: NotificationAuthorityEpoch,
}

impl NotificationAuthorityCursor {
    /// Constructs an authority cursor.
    #[must_use]
    pub const fn new(
        authority_id: NotificationAuthorityId,
        authority_epoch: NotificationAuthorityEpoch,
    ) -> Self {
        Self {
            authority_id,
            authority_epoch,
        }
    }

    /// Returns the stable authority identity.
    #[must_use]
    pub const fn authority_id(&self) -> &NotificationAuthorityId {
        &self.authority_id
    }

    /// Returns the live authority epoch.
    #[must_use]
    pub const fn authority_epoch(&self) -> NotificationAuthorityEpoch {
        self.authority_epoch
    }
}

/// Authoritative retained notification record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationRecord {
    pub(crate) notification_id: NotificationId,
    pub(crate) draft: NotificationDraft,
    pub(crate) sequence: NotificationSequence,
    pub(crate) last_changed_ledger_revision: NotificationLedgerRevision,
    pub(crate) read_state: NotificationReadState,
}

impl NotificationRecord {
    /// Returns stable record identity.
    #[must_use]
    pub const fn notification_id(&self) -> &NotificationId {
        &self.notification_id
    }

    /// Returns retained metadata.
    #[must_use]
    pub const fn draft(&self) -> &NotificationDraft {
        &self.draft
    }

    /// Returns insertion ordering independent from wall-clock time.
    #[must_use]
    pub const fn sequence(&self) -> NotificationSequence {
        self.sequence
    }

    /// Returns the ledger revision that last changed this record.
    #[must_use]
    pub const fn last_changed_ledger_revision(&self) -> NotificationLedgerRevision {
        self.last_changed_ledger_revision
    }

    /// Returns explicit read state.
    #[must_use]
    pub const fn read_state(&self) -> NotificationReadState {
        self.read_state
    }

    /// Returns canonical structural metadata weight used by retention.
    #[must_use]
    pub fn encoded_metadata_weight(&self) -> u64 {
        let action_bytes: usize = self
            .draft
            .actions
            .iter()
            .map(|action| action.reference_id.as_str().len() + action.label.as_str().len())
            .sum();
        let bytes = self.notification_id.as_str().len()
            + self.draft.source_id.as_str().len()
            + self.draft.title.as_str().len()
            + self.draft.summary.as_str().len()
            + self
                .draft
                .cause_id
                .as_ref()
                .map_or(0, |id| id.as_str().len())
            + self
                .draft
                .replacement_key
                .as_ref()
                .map_or(0, |id| id.as_str().len())
            + self
                .draft
                .producer_token
                .as_ref()
                .map_or(0, |id| id.as_str().len())
            + action_bytes;
        48 + u64::try_from(bytes).expect("bounded metadata length fits u64")
    }

    pub(crate) fn added(
        notification_id: NotificationId,
        draft: NotificationDraft,
        sequence: NotificationSequence,
        revision: NotificationLedgerRevision,
    ) -> Self {
        Self {
            notification_id,
            draft,
            sequence,
            last_changed_ledger_revision: revision,
            read_state: NotificationReadState::Unseen,
        }
    }

    pub(crate) fn replace(
        &mut self,
        draft: NotificationDraft,
        revision: NotificationLedgerRevision,
        mark_unseen: bool,
    ) {
        self.draft = draft;
        self.last_changed_ledger_revision = revision;
        if mark_unseen {
            self.read_state = NotificationReadState::Unseen;
        }
    }

    pub(crate) fn mark_seen(&mut self, revision: NotificationLedgerRevision) {
        self.read_state = NotificationReadState::Seen;
        self.last_changed_ledger_revision = revision;
    }
}

