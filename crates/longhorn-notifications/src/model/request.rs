//! Revision-bound ledger mutation requests.

use longhorn_core::{
    NotificationActionReferenceId, NotificationAuthorityId, NotificationCauseId, NotificationId,
    NotificationLedgerRevision, NotificationProducerToken, NotificationReplacementKey,
    NotificationSourceId,
};

use crate::{
    MAXIMUM_NOTIFICATION_ACTIONS, NotificationActionLabel, NotificationAuthorityEpoch,
    NotificationLedgerLimits, NotificationSequence, NotificationSummary, NotificationTitle,
};

use super::{
    NotificationAuthorityCursor, NotificationDraft, NotificationRecord,
};
/// Revision-bound request to add a fresh record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationAdd {
    pub(crate) authority: NotificationAuthorityCursor,
    pub(crate) expected_revision: NotificationLedgerRevision,
    pub(crate) notification_id: NotificationId,
    pub(crate) draft: NotificationDraft,
}

impl NotificationAdd {
    /// Constructs an add request.
    #[must_use]
    pub const fn new(
        authority: NotificationAuthorityCursor,
        expected_revision: NotificationLedgerRevision,
        notification_id: NotificationId,
        draft: NotificationDraft,
    ) -> Self {
        Self {
            authority,
            expected_revision,
            notification_id,
            draft,
        }
    }
}

/// Revision-bound explicit replacement by source and key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationReplace {
    pub(crate) authority: NotificationAuthorityCursor,
    pub(crate) expected_revision: NotificationLedgerRevision,
    pub(crate) draft: NotificationDraft,
    pub(crate) mark_unseen: bool,
}

impl NotificationReplace {
    /// Constructs an explicit replacement request.
    #[must_use]
    pub const fn new(
        authority: NotificationAuthorityCursor,
        expected_revision: NotificationLedgerRevision,
        draft: NotificationDraft,
        mark_unseen: bool,
    ) -> Self {
        Self {
            authority,
            expected_revision,
            draft,
            mark_unseen,
        }
    }
}

/// Revision-bound request to mark one retained record seen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationSeen {
    pub(crate) authority: NotificationAuthorityCursor,
    pub(crate) expected_revision: NotificationLedgerRevision,
    pub(crate) notification_id: NotificationId,
}

impl NotificationSeen {
    /// Constructs a seen request.
    #[must_use]
    pub const fn new(
        authority: NotificationAuthorityCursor,
        expected_revision: NotificationLedgerRevision,
        notification_id: NotificationId,
    ) -> Self {
        Self {
            authority,
            expected_revision,
            notification_id,
        }
    }
}

/// Explicit clear target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotificationClearTarget {
    /// Clear every retained record.
    All,
    /// Clear exactly the supplied record ids.
    Records(Vec<NotificationId>),
}

/// Revision-bound clear request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationClear {
    pub(crate) authority: NotificationAuthorityCursor,
    pub(crate) expected_revision: NotificationLedgerRevision,
    pub(crate) target: NotificationClearTarget,
}

impl NotificationClear {
    /// Constructs a clear request.
    #[must_use]
    pub const fn new(
        authority: NotificationAuthorityCursor,
        expected_revision: NotificationLedgerRevision,
        target: NotificationClearTarget,
    ) -> Self {
        Self {
            authority,
            expected_revision,
            target,
        }
    }
}

/// Revision-bound request to change limits and prune.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationRetentionChange {
    pub(crate) authority: NotificationAuthorityCursor,
    pub(crate) expected_revision: NotificationLedgerRevision,
    pub(crate) limits: NotificationLedgerLimits,
}

impl NotificationRetentionChange {
    /// Constructs a retention change.
    #[must_use]
    pub const fn new(
        authority: NotificationAuthorityCursor,
        expected_revision: NotificationLedgerRevision,
        limits: NotificationLedgerLimits,
    ) -> Self {
        Self {
            authority,
            expected_revision,
            limits,
        }
    }
}

/// Idempotent publication request using the draft producer token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationPublishOnce {
    pub(crate) add: NotificationAdd,
}

impl NotificationPublishOnce {
    /// Constructs an idempotent publication request.
    #[must_use]
    pub const fn new(add: NotificationAdd) -> Self {
        Self { add }
    }
}

/// Result of idempotent publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotificationPublishOutcome {
    /// A fresh record was committed.
    Published(crate::NotificationMutationReceipt),
    /// The producer token already names a retained record.
    AlreadyPublished {
        /// Existing retained record.
        record: NotificationRecord,
        /// Current authoritative ledger revision.
        ledger_revision: NotificationLedgerRevision,
    },
}

