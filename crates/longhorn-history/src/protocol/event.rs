//! History changed events.

use std::{error::Error, fmt};

use longhorn_core::{
    HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryPlanId, HistoryRevision,
};
use serde::{Deserialize, Serialize};

use crate::{
    HistoryCommittedTransition, HistoryCommittedTransitionKind, HistoryEntryPosition,
    HistoryNavigationDirection, HistoryNavigationPosition, HistoryNavigationReceipt, HistoryPage,
    HistorySummary,
};

use super::{HistoryAuthorityEpoch, HistoryProtocolMode, HistoryProtocolVersion, HistorySnapshot};

/// Coarse payload-free committed transition kind used for live invalidation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum HistoryChangedKind {
    /// A successful product mutation recorded or coalesced.
    Record,
    /// A checked navigation plan committed.
    Navigation,
    /// Retention limits changed.
    LimitsChanged,
    /// Persisted structural history was accepted.
    Imported,
    /// Persisted history was explicitly discarded.
    DiscardedPersistence,
    /// Retained structural history was reset.
    Reset,
}

/// Non-durable live invalidation hint for one committed transition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistoryChangedEvent {
    /// Exact metadata protocol line.
    pub protocol_version: HistoryProtocolVersion,
    /// Live authority lifetime.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// Stable history identity.
    pub history_id: HistoryId,
    /// Prior in-memory revision, absent for load recovery.
    pub previous_revision: Option<HistoryRevision>,
    /// Authoritative resulting revision.
    pub committed_revision: HistoryRevision,
    /// Coarse transition category.
    pub kind: HistoryChangedKind,
}

impl HistoryChangedEvent {
    /// Projects one committed kernel transition into a live invalidation hint.
    #[must_use]
    pub fn from_transition(
        authority_epoch: HistoryAuthorityEpoch,
        transition: &HistoryCommittedTransition,
    ) -> Self {
        Self {
            protocol_version: HistoryProtocolVersion::CURRENT,
            authority_epoch,
            history_id: transition.history_id().clone(),
            previous_revision: transition.previous_revision(),
            committed_revision: transition.committed_revision(),
            kind: match transition.kind() {
                HistoryCommittedTransitionKind::Record { .. } => HistoryChangedKind::Record,
                HistoryCommittedTransitionKind::Navigation { .. } => HistoryChangedKind::Navigation,
                HistoryCommittedTransitionKind::LimitsChanged { .. } => {
                    HistoryChangedKind::LimitsChanged
                }
                HistoryCommittedTransitionKind::Imported { .. } => HistoryChangedKind::Imported,
                HistoryCommittedTransitionKind::DiscardedPersistence { .. } => {
                    HistoryChangedKind::DiscardedPersistence
                }
                HistoryCommittedTransitionKind::Reset { .. } => HistoryChangedKind::Reset,
            },
        }
    }
}

