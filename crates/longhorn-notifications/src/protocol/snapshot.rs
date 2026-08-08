//! Notification ledger snapshots and clear-target projections.

use longhorn_core::{NotificationId, NotificationLedgerRevision, NotificationRequestId};
use serde::{Deserialize, Serialize};

use crate::NotificationLedger;

use super::*;

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
