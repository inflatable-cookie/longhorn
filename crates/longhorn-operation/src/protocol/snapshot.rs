//! Operation catalogue snapshots and teardown-resolution projections.

use longhorn_core::{
    OperationCatalogueRevision, OperationId, OperationRequestId, OperationRevision,
};
use serde::{Deserialize, Serialize};

use crate::{OperationCatalogue, OperationCatalogueProjection};

use super::*;

/// One exact bounded catalogue snapshot.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationSnapshot {
    /// Exact protocol line.
    pub protocol_version: OperationProtocolVersion,
    /// Live authority cursor.
    pub authority: OperationAuthorityProjection,
    /// Authoritative catalogue revision.
    pub catalogue_revision: OperationCatalogueRevision,
    /// Whether controlled teardown closed this authority.
    pub closed: bool,
    /// Current finite limits.
    pub limits: OperationCatalogueLimitsProjection,
    /// Cumulative finite-retention truncation evidence.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub terminal_eviction_count: u64,
    /// Current retained terminal metadata weight.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub retained_terminal_encoded_weight: u64,
    /// Active entries in insertion order.
    pub active: Vec<OperationEntryProjection>,
    /// Terminal entries newest first.
    pub recent: Vec<OperationEntryProjection>,
}

impl OperationSnapshot {
    /// Projects one authoritative catalogue without product payloads.
    pub fn from_catalogue(
        catalogue: &OperationCatalogue,
    ) -> Result<Self, OperationProtocolProjectionError> {
        let projection = catalogue.project();
        Self::from_projection(catalogue, &projection)
    }

    fn from_projection(
        catalogue: &OperationCatalogue,
        projection: &OperationCatalogueProjection,
    ) -> Result<Self, OperationProtocolProjectionError> {
        let retained_terminal_encoded_weight = catalogue
            .retained_terminal_encoded_weight()
            .map_err(|error| OperationProtocolProjectionError(error.to_string()))?;
        Ok(Self {
            protocol_version: OperationProtocolVersion::CURRENT,
            authority: OperationAuthorityProjection::from_cursor(projection.authority()),
            catalogue_revision: projection.catalogue_revision(),
            closed: projection.is_closed(),
            limits: OperationCatalogueLimitsProjection::from_limits(catalogue.limits())?,
            terminal_eviction_count: projection.terminal_eviction_count(),
            retained_terminal_encoded_weight,
            active: projection
                .active()
                .iter()
                .map(OperationEntryProjection::from_record)
                .collect(),
            recent: projection
                .recent()
                .iter()
                .map(OperationEntryProjection::from_record)
                .collect(),
        })
    }
}

/// Correlated snapshot query.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationSnapshotQuery {
    /// Exact protocol line.
    pub protocol_version: OperationProtocolVersion,
    /// Correlation identity.
    pub request_id: OperationRequestId,
}

/// Correlated snapshot response.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationSnapshotResponse {
    /// Echoed correlation identity.
    pub request_id: OperationRequestId,
    /// Authoritative snapshot.
    pub snapshot: OperationSnapshot,
}

/// One teardown resolution carried by a management command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum OperationTeardownResolutionProjection {
    /// Commit a terminal fact.
    Complete {
        /// Operation identity.
        operation_id: OperationId,
        /// Expected operation revision.
        expected_operation_revision: OperationRevision,
        /// Consumer-proven terminal state.
        state: OperationStateProjection,
    },
    /// Transfer to another live authority.
    Transfer {
        /// Operation identity.
        operation_id: OperationId,
        /// Expected operation revision.
        expected_operation_revision: OperationRevision,
        /// Receiving authority.
        target_authority: OperationAuthorityProjection,
    },
}
