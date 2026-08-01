//! Strict product-neutral renderer and transport protocol.

use std::{error::Error, fmt};

use longhorn_core::{
    NotificationActionReferenceId, NotificationAuthorityId, NotificationCauseId, NotificationId,
    NotificationLedgerRevision, NotificationProducerToken, NotificationReplacementKey,
    NotificationRequestId, NotificationSourceId,
};
use serde::{Deserialize, Serialize};

use crate::{
    NotificationAction, NotificationActionLabel, NotificationAdd, NotificationAuthorityCursor,
    NotificationAuthorityEpoch, NotificationClear, NotificationClearTarget, NotificationDraft,
    NotificationLedger, NotificationLedgerError, NotificationLedgerLimits,
    NotificationMutationReceipt, NotificationReadState, NotificationRecord, NotificationRemoval,
    NotificationRemovalReason, NotificationRemovalReceipt, NotificationReplace,
    NotificationRetentionChange, NotificationRetentionClass, NotificationSeen,
    NotificationSeverity, NotificationSummary, NotificationTitle,
};

/// Current exact notification protocol line.
pub const NOTIFICATION_PROTOCOL_VERSION: u32 = 1;
/// Default record count returned by mutation results and subscriptions.
pub const NOTIFICATION_DEFAULT_PAGE_SIZE: u64 = 100;

/// Exact notification protocol version.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct NotificationProtocolVersion(u32);

impl NotificationProtocolVersion {
    /// Current exact protocol version.
    pub const CURRENT: Self = Self(NOTIFICATION_PROTOCOL_VERSION);

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Serialized authority identity and live epoch.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationAuthorityProjection {
    /// Stable authority identity.
    pub authority_id: NotificationAuthorityId,
    /// Nonzero authority lifetime.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub authority_epoch: u64,
}

impl NotificationAuthorityProjection {
    fn from_cursor(cursor: &NotificationAuthorityCursor) -> Self {
        Self {
            authority_id: cursor.authority_id().clone(),
            authority_epoch: cursor.authority_epoch().get(),
        }
    }

    fn into_cursor(self) -> Result<NotificationAuthorityCursor, NotificationProtocolInputError> {
        let epoch = NotificationAuthorityEpoch::new(self.authority_epoch)
            .map_err(|_| NotificationProtocolInputError::AuthorityEpoch)?;
        Ok(NotificationAuthorityCursor::new(self.authority_id, epoch))
    }
}

/// Product-neutral severity on the wire.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum NotificationSeverityProjection {
    /// Routine information.
    Info,
    /// Successful completion.
    Success,
    /// Recoverable concern.
    Warning,
    /// Failed or degraded behavior.
    Error,
    /// Urgent failure requiring attention.
    Critical,
}

impl From<NotificationSeverity> for NotificationSeverityProjection {
    fn from(value: NotificationSeverity) -> Self {
        match value {
            NotificationSeverity::Info => Self::Info,
            NotificationSeverity::Success => Self::Success,
            NotificationSeverity::Warning => Self::Warning,
            NotificationSeverity::Error => Self::Error,
            NotificationSeverity::Critical => Self::Critical,
        }
    }
}

impl From<NotificationSeverityProjection> for NotificationSeverity {
    fn from(value: NotificationSeverityProjection) -> Self {
        match value {
            NotificationSeverityProjection::Info => Self::Info,
            NotificationSeverityProjection::Success => Self::Success,
            NotificationSeverityProjection::Warning => Self::Warning,
            NotificationSeverityProjection::Error => Self::Error,
            NotificationSeverityProjection::Critical => Self::Critical,
        }
    }
}

/// Explicit read state on the wire.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum NotificationReadStateProjection {
    /// User has not seen the record.
    Unseen,
    /// Record was explicitly marked seen.
    Seen,
}

impl From<NotificationReadState> for NotificationReadStateProjection {
    fn from(value: NotificationReadState) -> Self {
        match value {
            NotificationReadState::Unseen => Self::Unseen,
            NotificationReadState::Seen => Self::Seen,
        }
    }
}

/// Consumer-selected retention treatment on the wire.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum NotificationRetentionClassProjection {
    /// Oldest records may be pruned.
    Standard,
    /// Automatic pruning must not remove the record.
    Protected,
}

impl From<NotificationRetentionClass> for NotificationRetentionClassProjection {
    fn from(value: NotificationRetentionClass) -> Self {
        match value {
            NotificationRetentionClass::Standard => Self::Standard,
            NotificationRetentionClass::Protected => Self::Protected,
        }
    }
}

impl From<NotificationRetentionClassProjection> for NotificationRetentionClass {
    fn from(value: NotificationRetentionClassProjection) -> Self {
        match value {
            NotificationRetentionClassProjection::Standard => Self::Standard,
            NotificationRetentionClassProjection::Protected => Self::Protected,
        }
    }
}

/// One bounded semantic action reference on the wire.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationActionProjection {
    /// Consumer-owned semantic reference.
    pub reference_id: NotificationActionReferenceId,
    /// Bounded presentation label.
    pub label: String,
}

impl NotificationActionProjection {
    fn from_action(action: &NotificationAction) -> Self {
        Self {
            reference_id: action.reference_id().clone(),
            label: action.label().as_str().to_owned(),
        }
    }

    fn into_action(self) -> Result<NotificationAction, NotificationProtocolInputError> {
        let label = NotificationActionLabel::new(self.label)
            .map_err(|error| NotificationProtocolInputError::Metadata(error.to_string()))?;
        Ok(NotificationAction::new(self.reference_id, label))
    }
}

/// Complete bounded consumer metadata on the wire.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationDraftProjection {
    /// Consumer-owned source identity.
    pub source_id: NotificationSourceId,
    /// Product-neutral severity.
    pub severity: NotificationSeverityProjection,
    /// Bounded title.
    pub title: String,
    /// Bounded summary.
    pub summary: String,
    /// Optional opaque cause.
    pub cause_id: Option<NotificationCauseId>,
    /// Bounded semantic actions.
    pub actions: Vec<NotificationActionProjection>,
    /// Optional explicit replacement key.
    pub replacement_key: Option<NotificationReplacementKey>,
    /// Optional producer idempotency token.
    pub producer_token: Option<NotificationProducerToken>,
    /// Consumer-selected retention treatment.
    pub retention_class: NotificationRetentionClassProjection,
    /// Optional injected wall-clock presentation time.
    #[cfg_attr(feature = "bindings", ts(type = "number | null"))]
    pub presentation_time_unix_ms: Option<i64>,
}

impl NotificationDraftProjection {
    fn from_draft(draft: &NotificationDraft) -> Self {
        Self {
            source_id: draft.source_id().clone(),
            severity: draft.severity().into(),
            title: draft.title().as_str().to_owned(),
            summary: draft.summary().as_str().to_owned(),
            cause_id: draft.cause_id().cloned(),
            actions: draft
                .actions()
                .iter()
                .map(NotificationActionProjection::from_action)
                .collect(),
            replacement_key: draft.replacement_key().cloned(),
            producer_token: draft.producer_token().cloned(),
            retention_class: draft.retention_class().into(),
            presentation_time_unix_ms: draft.presentation_time_unix_ms(),
        }
    }

    fn into_draft(self) -> Result<NotificationDraft, NotificationProtocolInputError> {
        let title = NotificationTitle::new(self.title)
            .map_err(|error| NotificationProtocolInputError::Metadata(error.to_string()))?;
        let summary = NotificationSummary::new(self.summary)
            .map_err(|error| NotificationProtocolInputError::Metadata(error.to_string()))?;
        let actions = self
            .actions
            .into_iter()
            .map(NotificationActionProjection::into_action)
            .collect::<Result<Vec<_>, _>>()?;
        let mut draft =
            NotificationDraft::new(self.source_id, self.severity.into(), title, summary)
                .with_actions(actions)
                .map_err(|error| NotificationProtocolInputError::Metadata(error.to_string()))?
                .with_retention_class(self.retention_class.into());
        if let Some(cause_id) = self.cause_id {
            draft = draft.with_cause(cause_id);
        }
        if let Some(key) = self.replacement_key {
            draft = draft.with_replacement_key(key);
        }
        if let Some(token) = self.producer_token {
            draft = draft.with_producer_token(token);
        }
        if let Some(time) = self.presentation_time_unix_ms {
            draft = draft.with_presentation_time_unix_ms(time);
        }
        Ok(draft)
    }
}

/// One authoritative retained record on the wire.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationRecordProjection {
    /// Stable record identity.
    pub notification_id: NotificationId,
    /// Retained bounded metadata.
    pub draft: NotificationDraftProjection,
    /// Monotonic insertion sequence.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub sequence: u64,
    /// Revision that last changed this record.
    pub last_changed_ledger_revision: NotificationLedgerRevision,
    /// Explicit read state.
    pub read_state: NotificationReadStateProjection,
    /// Canonical encoded metadata weight.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub encoded_metadata_weight: u64,
}

impl NotificationRecordProjection {
    fn from_record(record: &NotificationRecord) -> Self {
        Self {
            notification_id: record.notification_id().clone(),
            draft: NotificationDraftProjection::from_draft(record.draft()),
            sequence: record.sequence().get(),
            last_changed_ledger_revision: record.last_changed_ledger_revision(),
            read_state: record.read_state().into(),
            encoded_metadata_weight: record.encoded_metadata_weight(),
        }
    }
}

/// Finite ledger limits on the wire.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationLedgerLimitsProjection {
    /// Maximum retained record count.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub maximum_notifications: u64,
    /// Maximum retained canonical encoded weight.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub maximum_encoded_weight: u64,
}

impl NotificationLedgerLimitsProjection {
    fn from_limits(limits: NotificationLedgerLimits) -> Result<Self, NotificationProtocolError> {
        Ok(Self {
            maximum_notifications: u64::try_from(limits.maximum_notifications())
                .map_err(|_| NotificationProtocolError::projection("record count exceeds u64"))?,
            maximum_encoded_weight: limits.maximum_encoded_weight(),
        })
    }

    fn into_limits(self) -> Result<NotificationLedgerLimits, NotificationProtocolInputError> {
        let maximum = usize::try_from(self.maximum_notifications)
            .map_err(|_| NotificationProtocolInputError::Limits)?;
        NotificationLedgerLimits::new(maximum, self.maximum_encoded_weight)
            .map_err(|_| NotificationProtocolInputError::Limits)
    }
}

/// One bounded newest-first page on the wire.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationPageProjection {
    /// Requested newest-first offset.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub offset: u64,
    /// Total retained record count.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub total_count: u64,
    /// Whether older records were omitted.
    pub has_more: bool,
    /// Newest-first retained records.
    pub records: Vec<NotificationRecordProjection>,
}

/// One authoritative bounded notification snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationSnapshot {
    /// Exact protocol line.
    pub protocol_version: NotificationProtocolVersion,
    /// Live authority cursor.
    pub authority: NotificationAuthorityProjection,
    /// Authoritative ledger revision.
    pub ledger_revision: NotificationLedgerRevision,
    /// Current finite limits.
    pub limits: NotificationLedgerLimitsProjection,
    /// Total retained record count.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub retained_count: u64,
    /// Exact unseen record count.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub unseen_count: u64,
    /// Retained canonical encoded weight.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub retained_encoded_weight: u64,
    /// Cumulative automatic prune count.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub pruned_count: u64,
    /// Bounded newest-first page.
    pub page: NotificationPageProjection,
}

impl NotificationSnapshot {
    /// Projects one authoritative ledger page without presentation state.
    pub fn from_ledger(
        ledger: &NotificationLedger,
        offset: u64,
        limit: u64,
    ) -> Result<Self, NotificationProtocolError> {
        let offset_usize = usize::try_from(offset)
            .map_err(|_| NotificationProtocolError::input("page offset exceeds usize"))?;
        let limit_usize = usize::try_from(limit)
            .map_err(|_| NotificationProtocolError::input("page size exceeds usize"))?;
        let projection = ledger
            .projection()
            .map_err(|error| NotificationProtocolError::projection(error.to_string()))?;
        let page = ledger
            .page(offset_usize, limit_usize)
            .map_err(|error| NotificationProtocolError::input(error.to_string()))?;
        Ok(Self {
            protocol_version: NotificationProtocolVersion::CURRENT,
            authority: NotificationAuthorityProjection::from_cursor(projection.authority()),
            ledger_revision: projection.ledger_revision(),
            limits: NotificationLedgerLimitsProjection::from_limits(projection.limits())?,
            retained_count: project_usize(projection.retained_count())?,
            unseen_count: project_usize(projection.unseen_count())?,
            retained_encoded_weight: projection.retained_encoded_weight(),
            pruned_count: projection.pruned_count(),
            page: NotificationPageProjection {
                offset,
                total_count: project_usize(page.total_count())?,
                has_more: page.has_more(),
                records: page
                    .records()
                    .iter()
                    .map(NotificationRecordProjection::from_record)
                    .collect(),
            },
        })
    }
}

/// Correlated bounded snapshot query.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationSnapshotQuery {
    /// Exact protocol line.
    pub protocol_version: NotificationProtocolVersion,
    /// Correlation identity.
    pub request_id: NotificationRequestId,
    /// Newest-first record offset.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub offset: u64,
    /// Bounded requested record count.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub limit: u64,
}

/// Correlated snapshot response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationSnapshotResponse {
    /// Echoed request identity.
    pub request_id: NotificationRequestId,
    /// Authoritative snapshot page.
    pub snapshot: NotificationSnapshot,
}

/// Explicit clear target on the wire.
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum NotificationClearTargetProjection {
    All,
    Records {
        notification_ids: Vec<NotificationId>,
    },
}

/// Revision-bound notification mutation command.
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum NotificationMutationCommand {
    Add {
        request_id: NotificationRequestId,
        protocol_version: NotificationProtocolVersion,
        authority: NotificationAuthorityProjection,
        expected_ledger_revision: NotificationLedgerRevision,
        notification_id: NotificationId,
        draft: NotificationDraftProjection,
    },
    Replace {
        request_id: NotificationRequestId,
        protocol_version: NotificationProtocolVersion,
        authority: NotificationAuthorityProjection,
        expected_ledger_revision: NotificationLedgerRevision,
        draft: NotificationDraftProjection,
        mark_unseen: bool,
    },
    MarkSeen {
        request_id: NotificationRequestId,
        protocol_version: NotificationProtocolVersion,
        authority: NotificationAuthorityProjection,
        expected_ledger_revision: NotificationLedgerRevision,
        notification_id: NotificationId,
    },
    Dismiss {
        request_id: NotificationRequestId,
        protocol_version: NotificationProtocolVersion,
        authority: NotificationAuthorityProjection,
        expected_ledger_revision: NotificationLedgerRevision,
        notification_id: NotificationId,
    },
    Clear {
        request_id: NotificationRequestId,
        protocol_version: NotificationProtocolVersion,
        authority: NotificationAuthorityProjection,
        expected_ledger_revision: NotificationLedgerRevision,
        target: NotificationClearTargetProjection,
    },
    ChangeRetention {
        request_id: NotificationRequestId,
        protocol_version: NotificationProtocolVersion,
        authority: NotificationAuthorityProjection,
        expected_ledger_revision: NotificationLedgerRevision,
        limits: NotificationLedgerLimitsProjection,
    },
}

impl NotificationMutationCommand {
    /// Returns request correlation identity.
    #[must_use]
    pub const fn request_id(&self) -> &NotificationRequestId {
        match self {
            Self::Add { request_id, .. }
            | Self::Replace { request_id, .. }
            | Self::MarkSeen { request_id, .. }
            | Self::Dismiss { request_id, .. }
            | Self::Clear { request_id, .. }
            | Self::ChangeRetention { request_id, .. } => request_id,
        }
    }

    const fn protocol_version(&self) -> NotificationProtocolVersion {
        match self {
            Self::Add {
                protocol_version, ..
            }
            | Self::Replace {
                protocol_version, ..
            }
            | Self::MarkSeen {
                protocol_version, ..
            }
            | Self::Dismiss {
                protocol_version, ..
            }
            | Self::Clear {
                protocol_version, ..
            }
            | Self::ChangeRetention {
                protocol_version, ..
            } => *protocol_version,
        }
    }
}

/// Distinct removal reason on the wire.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum NotificationRemovalReasonProjection {
    /// Explicit single-record dismissal.
    Dismissed,
    /// Explicit clear transition.
    Cleared,
    /// Finite retention prune.
    Pruned,
}

/// Exact removed record on the wire.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationRemovalProjection {
    /// Complete removed record.
    pub record: NotificationRecordProjection,
    /// Distinct removal reason.
    pub reason: NotificationRemovalReasonProjection,
}

impl From<&NotificationRemoval> for NotificationRemovalProjection {
    fn from(value: &NotificationRemoval) -> Self {
        Self {
            record: NotificationRecordProjection::from_record(value.record()),
            reason: match value.reason() {
                NotificationRemovalReason::Dismissed => {
                    NotificationRemovalReasonProjection::Dismissed
                }
                NotificationRemovalReason::Cleared => NotificationRemovalReasonProjection::Cleared,
                NotificationRemovalReason::Pruned => NotificationRemovalReasonProjection::Pruned,
            },
        }
    }
}

/// Exact successful mutation receipt on the wire.
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum NotificationMutationReceiptProjection {
    Added {
        record: NotificationRecordProjection,
        previous_ledger_revision: NotificationLedgerRevision,
        committed_ledger_revision: NotificationLedgerRevision,
        pruned: Vec<NotificationRemovalProjection>,
    },
    Replaced {
        record: NotificationRecordProjection,
        previous_ledger_revision: NotificationLedgerRevision,
        committed_ledger_revision: NotificationLedgerRevision,
        pruned: Vec<NotificationRemovalProjection>,
    },
    Seen {
        record: NotificationRecordProjection,
        previous_ledger_revision: NotificationLedgerRevision,
        committed_ledger_revision: NotificationLedgerRevision,
    },
    Removed {
        previous_ledger_revision: NotificationLedgerRevision,
        committed_ledger_revision: NotificationLedgerRevision,
        removals: Vec<NotificationRemovalProjection>,
    },
    RetentionChanged {
        previous_limits: NotificationLedgerLimitsProjection,
        committed_limits: NotificationLedgerLimitsProjection,
        previous_ledger_revision: NotificationLedgerRevision,
        committed_ledger_revision: NotificationLedgerRevision,
        removals: Vec<NotificationRemovalProjection>,
    },
}

impl NotificationMutationReceiptProjection {
    /// Returns revision before the transition.
    #[must_use]
    pub const fn previous_ledger_revision(&self) -> NotificationLedgerRevision {
        match self {
            Self::Added {
                previous_ledger_revision,
                ..
            }
            | Self::Replaced {
                previous_ledger_revision,
                ..
            }
            | Self::Seen {
                previous_ledger_revision,
                ..
            }
            | Self::Removed {
                previous_ledger_revision,
                ..
            }
            | Self::RetentionChanged {
                previous_ledger_revision,
                ..
            } => *previous_ledger_revision,
        }
    }

    /// Returns revision after the transition.
    #[must_use]
    pub const fn committed_ledger_revision(&self) -> NotificationLedgerRevision {
        match self {
            Self::Added {
                committed_ledger_revision,
                ..
            }
            | Self::Replaced {
                committed_ledger_revision,
                ..
            }
            | Self::Seen {
                committed_ledger_revision,
                ..
            }
            | Self::Removed {
                committed_ledger_revision,
                ..
            }
            | Self::RetentionChanged {
                committed_ledger_revision,
                ..
            } => *committed_ledger_revision,
        }
    }

    /// Returns every directly changed or removed record identity.
    #[must_use]
    pub fn affected_notification_ids(&self) -> Vec<NotificationId> {
        match self {
            Self::Added { record, pruned, .. } | Self::Replaced { record, pruned, .. } => {
                let mut ids = vec![record.notification_id.clone()];
                ids.extend(
                    pruned
                        .iter()
                        .map(|removal| removal.record.notification_id.clone()),
                );
                ids
            }
            Self::Seen { record, .. } => vec![record.notification_id.clone()],
            Self::Removed { removals, .. } | Self::RetentionChanged { removals, .. } => removals
                .iter()
                .map(|removal| removal.record.notification_id.clone())
                .collect(),
        }
    }
}

/// Stable checked mutation rejection category.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum NotificationRejectionCode {
    IncompatibleProtocol,
    InvalidCommand,
    AuthorityMismatch,
    LedgerRevisionMismatch,
    DuplicateNotification,
    UnknownNotification,
    DuplicateReplacementKey,
    MissingReplacementKey,
    ReplacementTargetNotFound,
    DuplicateProducerToken,
    MissingProducerToken,
    AlreadySeen,
    DuplicateClearTarget,
    ClearTargetNotFound,
    RetentionUnsatisfied,
    CapacityOverflow,
}

/// Checked rejection with fresh-snapshot guidance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationRejection {
    /// Stable category.
    pub code: NotificationRejectionCode,
    /// Product-neutral diagnostic.
    pub detail: String,
    /// Whether caller should load fresh authority before retry.
    pub refresh_required: bool,
}

/// Successful or checked-rejected notification mutation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status",
    deny_unknown_fields
)]
pub enum NotificationMutationResult {
    /// Authority committed the mutation.
    Committed {
        /// Echoed request identity.
        request_id: NotificationRequestId,
        /// Fresh authoritative first page.
        snapshot: NotificationSnapshot,
        /// Exact mutation receipt.
        receipt: Box<NotificationMutationReceiptProjection>,
    },
    /// Authority rejected without mutation.
    Rejected {
        /// Echoed request identity.
        request_id: NotificationRequestId,
        /// Unchanged authoritative first page.
        snapshot: NotificationSnapshot,
        /// Checked rejection.
        rejection: NotificationRejection,
    },
}

/// Authoritative event summary kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum NotificationChangedKind {
    /// Fresh record addition.
    Added,
    /// Explicit replacement.
    Replaced,
    /// Explicit mark-seen transition.
    Seen,
    /// Explicit dismissal.
    Dismissed,
    /// Explicit clear transition.
    Cleared,
    /// Retention limits changed.
    RetentionChanged,
}

/// Non-durable request-correlated authority invalidation hint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationChangedEvent {
    /// Exact protocol line.
    pub protocol_version: NotificationProtocolVersion,
    /// Request correlation identity.
    pub request_id: NotificationRequestId,
    /// Live authority cursor.
    pub authority: NotificationAuthorityProjection,
    /// Ledger revision before commit.
    pub previous_ledger_revision: NotificationLedgerRevision,
    /// Ledger revision after commit.
    pub committed_ledger_revision: NotificationLedgerRevision,
    /// Every directly changed or removed record id.
    pub affected_notification_ids: Vec<NotificationId>,
    /// Change source.
    pub kind: NotificationChangedKind,
}

impl NotificationChangedEvent {
    /// Projects an event only for a revision-advancing commit.
    #[must_use]
    pub fn from_mutation(result: &NotificationMutationResult) -> Option<Self> {
        let NotificationMutationResult::Committed {
            request_id,
            snapshot,
            receipt,
        } = result
        else {
            return None;
        };
        if receipt.previous_ledger_revision() == receipt.committed_ledger_revision() {
            return None;
        }
        let kind = match receipt.as_ref() {
            NotificationMutationReceiptProjection::Added { .. } => NotificationChangedKind::Added,
            NotificationMutationReceiptProjection::Replaced { .. } => {
                NotificationChangedKind::Replaced
            }
            NotificationMutationReceiptProjection::Seen { .. } => NotificationChangedKind::Seen,
            NotificationMutationReceiptProjection::Removed { removals, .. } => {
                if removals
                    .iter()
                    .all(|removal| removal.reason == NotificationRemovalReasonProjection::Dismissed)
                {
                    NotificationChangedKind::Dismissed
                } else {
                    NotificationChangedKind::Cleared
                }
            }
            NotificationMutationReceiptProjection::RetentionChanged { .. } => {
                NotificationChangedKind::RetentionChanged
            }
        };
        Some(Self {
            protocol_version: NotificationProtocolVersion::CURRENT,
            request_id: request_id.clone(),
            authority: snapshot.authority.clone(),
            previous_ledger_revision: receipt.previous_ledger_revision(),
            committed_ledger_revision: receipt.committed_ledger_revision(),
            affected_notification_ids: receipt.affected_notification_ids(),
            kind,
        })
    }
}

impl NotificationLedger {
    /// Executes one strict bounded snapshot query.
    pub fn execute_protocol_snapshot(
        &self,
        query: NotificationSnapshotQuery,
    ) -> Result<NotificationSnapshotResponse, NotificationProtocolError> {
        if query.protocol_version != NotificationProtocolVersion::CURRENT {
            return Err(NotificationProtocolError::incompatible());
        }
        Ok(NotificationSnapshotResponse {
            request_id: query.request_id,
            snapshot: NotificationSnapshot::from_ledger(self, query.offset, query.limit)?,
        })
    }

    /// Executes one strict mutation and returns fresh first-page authority.
    pub fn execute_protocol_mutation(
        &mut self,
        command: NotificationMutationCommand,
    ) -> Result<NotificationMutationResult, NotificationProtocolError> {
        let request_id = command.request_id().clone();
        if command.protocol_version() != NotificationProtocolVersion::CURRENT {
            return self.rejected_mutation(request_id, incompatible_rejection());
        }
        let result = execute_mutation(self, command);
        match result {
            Ok(receipt) => Ok(NotificationMutationResult::Committed {
                request_id,
                snapshot: NotificationSnapshot::from_ledger(
                    self,
                    0,
                    NOTIFICATION_DEFAULT_PAGE_SIZE,
                )?,
                receipt: Box::new(receipt),
            }),
            Err(rejection) => self.rejected_mutation(request_id, rejection),
        }
    }

    fn rejected_mutation(
        &self,
        request_id: NotificationRequestId,
        rejection: NotificationRejection,
    ) -> Result<NotificationMutationResult, NotificationProtocolError> {
        Ok(NotificationMutationResult::Rejected {
            request_id,
            snapshot: NotificationSnapshot::from_ledger(self, 0, NOTIFICATION_DEFAULT_PAGE_SIZE)?,
            rejection,
        })
    }
}

fn execute_mutation(
    ledger: &mut NotificationLedger,
    command: NotificationMutationCommand,
) -> Result<NotificationMutationReceiptProjection, NotificationRejection> {
    match command {
        NotificationMutationCommand::Add {
            authority,
            expected_ledger_revision,
            notification_id,
            draft,
            ..
        } => {
            let receipt = ledger
                .add(NotificationAdd::new(
                    authority
                        .into_cursor()
                        .map_err(NotificationRejection::from)?,
                    expected_ledger_revision,
                    notification_id,
                    draft.into_draft().map_err(NotificationRejection::from)?,
                ))
                .map_err(NotificationRejection::from)?;
            Ok(project_record_receipt(receipt, true, false))
        }
        NotificationMutationCommand::Replace {
            authority,
            expected_ledger_revision,
            draft,
            mark_unseen,
            ..
        } => {
            let receipt = ledger
                .replace(NotificationReplace::new(
                    authority
                        .into_cursor()
                        .map_err(NotificationRejection::from)?,
                    expected_ledger_revision,
                    draft.into_draft().map_err(NotificationRejection::from)?,
                    mark_unseen,
                ))
                .map_err(NotificationRejection::from)?;
            Ok(project_record_receipt(receipt, false, false))
        }
        NotificationMutationCommand::MarkSeen {
            authority,
            expected_ledger_revision,
            notification_id,
            ..
        } => {
            let receipt = ledger
                .mark_seen(NotificationSeen::new(
                    authority
                        .into_cursor()
                        .map_err(NotificationRejection::from)?,
                    expected_ledger_revision,
                    notification_id,
                ))
                .map_err(NotificationRejection::from)?;
            Ok(project_record_receipt(receipt, false, true))
        }
        NotificationMutationCommand::Dismiss {
            authority,
            expected_ledger_revision,
            notification_id,
            ..
        } => ledger
            .dismiss(
                authority
                    .into_cursor()
                    .map_err(NotificationRejection::from)?,
                expected_ledger_revision,
                notification_id,
            )
            .map(project_removal_receipt)
            .map_err(NotificationRejection::from),
        NotificationMutationCommand::Clear {
            authority,
            expected_ledger_revision,
            target,
            ..
        } => {
            let target = match target {
                NotificationClearTargetProjection::All => NotificationClearTarget::All,
                NotificationClearTargetProjection::Records { notification_ids } => {
                    NotificationClearTarget::Records(notification_ids)
                }
            };
            ledger
                .clear(NotificationClear::new(
                    authority
                        .into_cursor()
                        .map_err(NotificationRejection::from)?,
                    expected_ledger_revision,
                    target,
                ))
                .map(project_removal_receipt)
                .map_err(NotificationRejection::from)
        }
        NotificationMutationCommand::ChangeRetention {
            authority,
            expected_ledger_revision,
            limits,
            ..
        } => {
            let previous_limits = NotificationLedgerLimitsProjection::from_limits(ledger.limits())
                .expect("validated notification limits fit u64");
            let receipt = ledger
                .change_retention(NotificationRetentionChange::new(
                    authority
                        .into_cursor()
                        .map_err(NotificationRejection::from)?,
                    expected_ledger_revision,
                    limits.into_limits().map_err(NotificationRejection::from)?,
                ))
                .map_err(NotificationRejection::from)?;
            Ok(NotificationMutationReceiptProjection::RetentionChanged {
                previous_limits,
                committed_limits: NotificationLedgerLimitsProjection::from_limits(ledger.limits())
                    .expect("validated notification limits fit u64"),
                previous_ledger_revision: receipt.previous_ledger_revision(),
                committed_ledger_revision: receipt.committed_ledger_revision(),
                removals: receipt.removals().iter().map(Into::into).collect(),
            })
        }
    }
}

fn project_record_receipt(
    receipt: NotificationMutationReceipt,
    added: bool,
    seen: bool,
) -> NotificationMutationReceiptProjection {
    let record = NotificationRecordProjection::from_record(receipt.record());
    if seen {
        NotificationMutationReceiptProjection::Seen {
            record,
            previous_ledger_revision: receipt.previous_ledger_revision(),
            committed_ledger_revision: receipt.committed_ledger_revision(),
        }
    } else if added {
        NotificationMutationReceiptProjection::Added {
            record,
            previous_ledger_revision: receipt.previous_ledger_revision(),
            committed_ledger_revision: receipt.committed_ledger_revision(),
            pruned: receipt.pruned().iter().map(Into::into).collect(),
        }
    } else {
        NotificationMutationReceiptProjection::Replaced {
            record,
            previous_ledger_revision: receipt.previous_ledger_revision(),
            committed_ledger_revision: receipt.committed_ledger_revision(),
            pruned: receipt.pruned().iter().map(Into::into).collect(),
        }
    }
}

fn project_removal_receipt(
    receipt: NotificationRemovalReceipt,
) -> NotificationMutationReceiptProjection {
    NotificationMutationReceiptProjection::Removed {
        previous_ledger_revision: receipt.previous_ledger_revision(),
        committed_ledger_revision: receipt.committed_ledger_revision(),
        removals: receipt.removals().iter().map(Into::into).collect(),
    }
}

impl From<NotificationLedgerError> for NotificationRejection {
    fn from(error: NotificationLedgerError) -> Self {
        let code = match error {
            NotificationLedgerError::WrongAuthority { .. } => {
                NotificationRejectionCode::AuthorityMismatch
            }
            NotificationLedgerError::StaleRevision { .. } => {
                NotificationRejectionCode::LedgerRevisionMismatch
            }
            NotificationLedgerError::DuplicateNotification { .. } => {
                NotificationRejectionCode::DuplicateNotification
            }
            NotificationLedgerError::NotificationNotFound { .. } => {
                NotificationRejectionCode::UnknownNotification
            }
            NotificationLedgerError::DuplicateReplacementKey { .. } => {
                NotificationRejectionCode::DuplicateReplacementKey
            }
            NotificationLedgerError::MissingReplacementKey => {
                NotificationRejectionCode::MissingReplacementKey
            }
            NotificationLedgerError::ReplacementTargetNotFound { .. } => {
                NotificationRejectionCode::ReplacementTargetNotFound
            }
            NotificationLedgerError::DuplicateProducerToken { .. } => {
                NotificationRejectionCode::DuplicateProducerToken
            }
            NotificationLedgerError::MissingProducerToken => {
                NotificationRejectionCode::MissingProducerToken
            }
            NotificationLedgerError::AlreadySeen { .. } => NotificationRejectionCode::AlreadySeen,
            NotificationLedgerError::DuplicateClearTarget { .. } => {
                NotificationRejectionCode::DuplicateClearTarget
            }
            NotificationLedgerError::ClearTargetNotFound { .. } => {
                NotificationRejectionCode::ClearTargetNotFound
            }
            NotificationLedgerError::RetentionUnsatisfied { .. } => {
                NotificationRejectionCode::RetentionUnsatisfied
            }
            NotificationLedgerError::TooManyActions { .. }
            | NotificationLedgerError::TooManyClearTargets { .. }
            | NotificationLedgerError::InvalidPageSize(_) => {
                NotificationRejectionCode::InvalidCommand
            }
            NotificationLedgerError::EncodedWeightOverflow
            | NotificationLedgerError::RevisionOverflow
            | NotificationLedgerError::SequenceOverflow
            | NotificationLedgerError::PrunedCountOverflow => {
                NotificationRejectionCode::CapacityOverflow
            }
        };
        let refresh_required = matches!(
            code,
            NotificationRejectionCode::AuthorityMismatch
                | NotificationRejectionCode::LedgerRevisionMismatch
                | NotificationRejectionCode::UnknownNotification
                | NotificationRejectionCode::ReplacementTargetNotFound
                | NotificationRejectionCode::AlreadySeen
                | NotificationRejectionCode::ClearTargetNotFound
        );
        Self {
            code,
            detail: error.to_string(),
            refresh_required,
        }
    }
}

impl From<NotificationProtocolInputError> for NotificationRejection {
    fn from(error: NotificationProtocolInputError) -> Self {
        Self {
            code: NotificationRejectionCode::InvalidCommand,
            detail: error.to_string(),
            refresh_required: false,
        }
    }
}

fn incompatible_rejection() -> NotificationRejection {
    NotificationRejection {
        code: NotificationRejectionCode::IncompatibleProtocol,
        detail: format!("notification protocol version must be {NOTIFICATION_PROTOCOL_VERSION}"),
        refresh_required: false,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum NotificationProtocolInputError {
    AuthorityEpoch,
    Metadata(String),
    Limits,
}

impl fmt::Display for NotificationProtocolInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthorityEpoch => {
                formatter.write_str("notification authority epoch must be nonzero")
            }
            Self::Metadata(detail) => formatter.write_str(detail),
            Self::Limits => formatter.write_str("notification ledger limits are invalid"),
        }
    }
}

/// A protocol query or projection could not be represented safely.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationProtocolError(String);

impl NotificationProtocolError {
    fn incompatible() -> Self {
        Self(format!(
            "notification protocol version must be {NOTIFICATION_PROTOCOL_VERSION}"
        ))
    }

    fn input(detail: impl Into<String>) -> Self {
        Self(detail.into())
    }

    fn projection(detail: impl Into<String>) -> Self {
        Self(detail.into())
    }
}

impl fmt::Display for NotificationProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for NotificationProtocolError {}

fn project_usize(value: usize) -> Result<u64, NotificationProtocolError> {
    u64::try_from(value)
        .map_err(|_| NotificationProtocolError::projection("usize value exceeds u64"))
}
