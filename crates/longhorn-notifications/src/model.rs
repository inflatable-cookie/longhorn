use longhorn_core::{
    NotificationActionReferenceId, NotificationAuthorityId, NotificationCauseId, NotificationId,
    NotificationLedgerRevision, NotificationProducerToken, NotificationReplacementKey,
    NotificationSourceId,
};

use crate::{
    MAXIMUM_NOTIFICATION_ACTIONS, NotificationActionLabel, NotificationAuthorityEpoch,
    NotificationLedgerLimits, NotificationSequence, NotificationSummary, NotificationTitle,
};

/// Closed, product-neutral notification severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationSeverity {
    /// Routine information.
    Info,
    /// Successful completion.
    Success,
    /// Recoverable concern.
    Warning,
    /// Failed work or degraded behavior.
    Error,
    /// Urgent failure requiring attention.
    Critical,
}

/// Explicit read state independent from retention and presentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationReadState {
    /// The user has not seen the retained record.
    Unseen,
    /// The record was explicitly marked seen.
    Seen,
}

/// Consumer-selected retention treatment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationRetentionClass {
    /// Oldest records may be pruned to satisfy limits.
    Standard,
    /// Automatic pruning must not remove the record.
    Protected,
}

/// Bounded semantic action data. It contains no executable callback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationAction {
    reference_id: NotificationActionReferenceId,
    label: NotificationActionLabel,
}

impl NotificationAction {
    /// Constructs a semantic action reference.
    #[must_use]
    pub const fn new(
        reference_id: NotificationActionReferenceId,
        label: NotificationActionLabel,
    ) -> Self {
        Self {
            reference_id,
            label,
        }
    }

    /// Returns the consumer-owned semantic action reference.
    #[must_use]
    pub const fn reference_id(&self) -> &NotificationActionReferenceId {
        &self.reference_id
    }

    /// Returns the presentation label.
    #[must_use]
    pub const fn label(&self) -> &NotificationActionLabel {
        &self.label
    }
}

/// Consumer-supplied bounded notification metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationDraft {
    source_id: NotificationSourceId,
    severity: NotificationSeverity,
    title: NotificationTitle,
    summary: NotificationSummary,
    cause_id: Option<NotificationCauseId>,
    actions: Vec<NotificationAction>,
    replacement_key: Option<NotificationReplacementKey>,
    producer_token: Option<NotificationProducerToken>,
    retention_class: NotificationRetentionClass,
    presentation_time_unix_ms: Option<i64>,
}

impl NotificationDraft {
    /// Constructs the required record metadata with standard retention.
    #[must_use]
    pub const fn new(
        source_id: NotificationSourceId,
        severity: NotificationSeverity,
        title: NotificationTitle,
        summary: NotificationSummary,
    ) -> Self {
        Self {
            source_id,
            severity,
            title,
            summary,
            cause_id: None,
            actions: Vec::new(),
            replacement_key: None,
            producer_token: None,
            retention_class: NotificationRetentionClass::Standard,
            presentation_time_unix_ms: None,
        }
    }

    /// Attaches an opaque cause reference.
    #[must_use]
    pub fn with_cause(mut self, cause_id: NotificationCauseId) -> Self {
        self.cause_id = Some(cause_id);
        self
    }

    /// Attaches bounded semantic action references.
    pub fn with_actions(
        mut self,
        actions: Vec<NotificationAction>,
    ) -> Result<Self, crate::NotificationLedgerError> {
        if actions.len() > MAXIMUM_NOTIFICATION_ACTIONS {
            return Err(crate::NotificationLedgerError::TooManyActions {
                maximum: MAXIMUM_NOTIFICATION_ACTIONS,
                actual: actions.len(),
            });
        }
        self.actions = actions;
        Ok(self)
    }

    /// Selects an explicit replacement key.
    #[must_use]
    pub fn with_replacement_key(mut self, key: NotificationReplacementKey) -> Self {
        self.replacement_key = Some(key);
        self
    }

    /// Selects a durable idempotent producer token.
    #[must_use]
    pub fn with_producer_token(mut self, token: NotificationProducerToken) -> Self {
        self.producer_token = Some(token);
        self
    }

    /// Selects consumer-owned retention treatment.
    #[must_use]
    pub const fn with_retention_class(mut self, class: NotificationRetentionClass) -> Self {
        self.retention_class = class;
        self
    }

    /// Attaches an injected wall-clock presentation time.
    #[must_use]
    pub const fn with_presentation_time_unix_ms(mut self, time: i64) -> Self {
        self.presentation_time_unix_ms = Some(time);
        self
    }

    /// Returns the source identity.
    #[must_use]
    pub const fn source_id(&self) -> &NotificationSourceId {
        &self.source_id
    }

    /// Returns severity.
    #[must_use]
    pub const fn severity(&self) -> NotificationSeverity {
        self.severity
    }

    /// Returns title.
    #[must_use]
    pub const fn title(&self) -> &NotificationTitle {
        &self.title
    }

    /// Returns summary.
    #[must_use]
    pub const fn summary(&self) -> &NotificationSummary {
        &self.summary
    }

    /// Returns the optional cause reference.
    #[must_use]
    pub const fn cause_id(&self) -> Option<&NotificationCauseId> {
        self.cause_id.as_ref()
    }

    /// Returns semantic actions.
    #[must_use]
    pub fn actions(&self) -> &[NotificationAction] {
        &self.actions
    }

    /// Returns the explicit replacement key.
    #[must_use]
    pub const fn replacement_key(&self) -> Option<&NotificationReplacementKey> {
        self.replacement_key.as_ref()
    }

    /// Returns the idempotent producer token.
    #[must_use]
    pub const fn producer_token(&self) -> Option<&NotificationProducerToken> {
        self.producer_token.as_ref()
    }

    /// Returns retention treatment.
    #[must_use]
    pub const fn retention_class(&self) -> NotificationRetentionClass {
        self.retention_class
    }

    /// Returns the optional injected wall-clock time.
    #[must_use]
    pub const fn presentation_time_unix_ms(&self) -> Option<i64> {
        self.presentation_time_unix_ms
    }
}

/// Exact identity of one live notification authority instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationAuthorityCursor {
    authority_id: NotificationAuthorityId,
    authority_epoch: NotificationAuthorityEpoch,
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
    notification_id: NotificationId,
    draft: NotificationDraft,
    sequence: NotificationSequence,
    last_changed_ledger_revision: NotificationLedgerRevision,
    read_state: NotificationReadState,
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

/// Lightweight authoritative ledger summary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationLedgerProjection {
    pub(crate) authority: NotificationAuthorityCursor,
    pub(crate) ledger_revision: NotificationLedgerRevision,
    pub(crate) retained_count: usize,
    pub(crate) unseen_count: usize,
    pub(crate) retained_encoded_weight: u64,
    pub(crate) pruned_count: u64,
    pub(crate) limits: NotificationLedgerLimits,
}

impl NotificationLedgerProjection {
    /// Returns authority identity and epoch.
    #[must_use]
    pub const fn authority(&self) -> &NotificationAuthorityCursor {
        &self.authority
    }
    /// Returns authoritative revision.
    #[must_use]
    pub const fn ledger_revision(&self) -> NotificationLedgerRevision {
        self.ledger_revision
    }
    /// Returns retained record count.
    #[must_use]
    pub const fn retained_count(&self) -> usize {
        self.retained_count
    }
    /// Returns exact unseen record count.
    #[must_use]
    pub const fn unseen_count(&self) -> usize {
        self.unseen_count
    }
    /// Returns retained canonical encoded weight.
    #[must_use]
    pub const fn retained_encoded_weight(&self) -> u64 {
        self.retained_encoded_weight
    }
    /// Returns cumulative automatic prune count.
    #[must_use]
    pub const fn pruned_count(&self) -> u64 {
        self.pruned_count
    }
    /// Returns current finite limits.
    #[must_use]
    pub const fn limits(&self) -> NotificationLedgerLimits {
        self.limits
    }
}

/// Bounded newest-first record page with truncation evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationPage {
    pub(crate) authority: NotificationAuthorityCursor,
    pub(crate) ledger_revision: NotificationLedgerRevision,
    pub(crate) offset: usize,
    pub(crate) total_count: usize,
    pub(crate) unseen_count: usize,
    pub(crate) records: Vec<NotificationRecord>,
}

impl NotificationPage {
    /// Returns authority identity and epoch.
    #[must_use]
    pub const fn authority(&self) -> &NotificationAuthorityCursor {
        &self.authority
    }
    /// Returns authoritative revision.
    #[must_use]
    pub const fn ledger_revision(&self) -> NotificationLedgerRevision {
        self.ledger_revision
    }
    /// Returns requested newest-first offset.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }
    /// Returns total retained count.
    #[must_use]
    pub const fn total_count(&self) -> usize {
        self.total_count
    }
    /// Returns exact ledger-wide unseen count.
    #[must_use]
    pub const fn unseen_count(&self) -> usize {
        self.unseen_count
    }
    /// Returns newest-first records in this page.
    #[must_use]
    pub fn records(&self) -> &[NotificationRecord] {
        &self.records
    }
    /// Returns whether older retained records were omitted.
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.offset.saturating_add(self.records.len()) < self.total_count
    }
}
