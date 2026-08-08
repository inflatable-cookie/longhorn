//! Operation change events.

use longhorn_core::{OperationCatalogueRevision, OperationId, OperationRequestId};
use serde::{Deserialize, Serialize};

use super::*;

/// Authoritative event summary kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum OperationChangedKind {
    /// Management mutation committed.
    Mutation,
    /// Cancellation changed authority state.
    Cancellation,
}

/// Non-durable request-correlated authority invalidation hint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationChangedEvent {
    /// Exact protocol line.
    pub protocol_version: OperationProtocolVersion,
    /// Request correlation identity.
    pub request_id: OperationRequestId,
    /// Live authority cursor.
    pub authority: OperationAuthorityProjection,
    /// Catalogue revision before commit.
    pub previous_catalogue_revision: OperationCatalogueRevision,
    /// Catalogue revision after commit.
    pub committed_catalogue_revision: OperationCatalogueRevision,
    /// Directly targeted operation when applicable.
    pub operation_id: Option<OperationId>,
    /// Change source.
    pub kind: OperationChangedKind,
}

impl OperationChangedEvent {
    /// Projects an event only for a committed management mutation.
    #[must_use]
    pub fn from_mutation(result: &OperationMutationResult) -> Option<Self> {
        let OperationMutationResult::Committed {
            request_id,
            snapshot,
            receipt,
        } = result
        else {
            return None;
        };
        Some(Self {
            protocol_version: OperationProtocolVersion::CURRENT,
            request_id: request_id.clone(),
            authority: snapshot.authority.clone(),
            previous_catalogue_revision: receipt.previous_catalogue_revision(),
            committed_catalogue_revision: receipt.committed_catalogue_revision(),
            operation_id: receipt.operation_id().cloned(),
            kind: OperationChangedKind::Mutation,
        })
    }

    /// Projects an event only when cancellation advanced catalogue state.
    #[must_use]
    pub fn from_cancellation(result: &OperationCancellationResult) -> Option<Self> {
        let OperationCancellationResult::Committed {
            request_id,
            snapshot,
            receipt,
            ..
        } = result
        else {
            return None;
        };
        if receipt.committed_catalogue_revision == receipt.previous_catalogue_revision {
            return None;
        }
        Some(Self {
            protocol_version: OperationProtocolVersion::CURRENT,
            request_id: request_id.clone(),
            authority: snapshot.authority.clone(),
            previous_catalogue_revision: receipt.previous_catalogue_revision,
            committed_catalogue_revision: receipt.committed_catalogue_revision,
            operation_id: Some(receipt.operation_id.clone()),
            kind: OperationChangedKind::Cancellation,
        })
    }
}
