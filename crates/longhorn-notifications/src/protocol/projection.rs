//! Projected ledger record and draft shapes.

use longhorn_core::{
    NotificationActionReferenceId, NotificationAuthorityId, NotificationCauseId, NotificationId,
    NotificationLedgerRevision, NotificationProducerToken, NotificationReplacementKey,
    NotificationSourceId,
};
use serde::{Deserialize, Serialize};

use crate::{
    NotificationAction, NotificationActionLabel, NotificationAuthorityCursor,
    NotificationAuthorityEpoch, NotificationDraft, NotificationLedgerLimits, NotificationReadState,
    NotificationRecord, NotificationRetentionClass, NotificationSeverity, NotificationSummary,
    NotificationTitle,
};

use super::*;

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
    pub(crate) fn from_cursor(cursor: &NotificationAuthorityCursor) -> Self {
        Self {
            authority_id: cursor.authority_id().clone(),
            authority_epoch: cursor.authority_epoch().get(),
        }
    }

    pub(crate) fn into_cursor(
        self,
    ) -> Result<NotificationAuthorityCursor, NotificationProtocolInputError> {
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

    pub(crate) fn into_action(self) -> Result<NotificationAction, NotificationProtocolInputError> {
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

    pub(crate) fn into_draft(self) -> Result<NotificationDraft, NotificationProtocolInputError> {
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
    pub(crate) fn from_record(record: &NotificationRecord) -> Self {
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
    pub(crate) fn from_limits(
        limits: NotificationLedgerLimits,
    ) -> Result<Self, NotificationProtocolError> {
        Ok(Self {
            maximum_notifications: u64::try_from(limits.maximum_notifications())
                .map_err(|_| NotificationProtocolError::projection("record count exceeds u64"))?,
            maximum_encoded_weight: limits.maximum_encoded_weight(),
        })
    }

    pub(crate) fn into_limits(
        self,
    ) -> Result<NotificationLedgerLimits, NotificationProtocolInputError> {
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
