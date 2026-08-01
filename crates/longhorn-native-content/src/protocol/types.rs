use longhorn_core::{NativeContentIslandId, NativeContentRequestId, NativeContentRevision};
use serde::{Deserialize, Serialize};

use crate::{
    ApplyReceipt, AttachGeneration, ContentSizeDecision, ContentSizeProposal,
    ContentSizeProposalReceipt, DesiredState, DesiredUpdate, DesiredUpdateReceipt,
    HostDestroyReceipt, ObservationReceipt, ObservedState,
};

use super::{
    NativeContentAuthorityEpoch, NativeContentClientEpoch, NativeContentProtocolRejection,
    NativeContentProtocolVersion,
};

/// Current authority and state cursor carried by every changed event.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct NativeContentCursor {
    /// Host authority lifetime.
    pub authority_epoch: NativeContentAuthorityEpoch,
    /// Current renderer session.
    pub client_epoch: NativeContentClientEpoch,
    /// Canonical island identity.
    pub island_id: NativeContentIslandId,
    /// Current attach attempt, distinct from renderer session.
    pub attach_generation: AttachGeneration,
    /// Current desired revision.
    pub desired_revision: NativeContentRevision,
    /// Current observed revision.
    pub observed_revision: NativeContentRevision,
}

/// Complete product-neutral renderer snapshot.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct NativeContentSnapshot {
    /// Exact protocol line.
    pub protocol_version: NativeContentProtocolVersion,
    /// Current authority and revision cursor.
    pub cursor: NativeContentCursor,
    /// Current desired coordination state.
    pub desired: DesiredState,
    /// Current fresh native observation.
    pub observed: ObservedState,
    /// Generation invalidated by host destruction, when any.
    pub invalidated_generation: Option<AttachGeneration>,
}

/// Listener-first request for a fresh renderer session.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct NativeContentConnectRequest {
    /// Exact protocol line.
    pub protocol_version: NativeContentProtocolVersion,
    /// Correlation only; never replay authority.
    pub request_id: NativeContentRequestId,
    /// Canonical island identity, never a Tauri label.
    pub island_id: NativeContentIslandId,
}

/// Request for a current snapshot within one renderer session.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct NativeContentSnapshotRequest {
    /// Exact protocol line.
    pub protocol_version: NativeContentProtocolVersion,
    /// Correlation only.
    pub request_id: NativeContentRequestId,
    /// Canonical island identity.
    pub island_id: NativeContentIslandId,
    /// Current host-issued renderer session.
    pub client_epoch: NativeContentClientEpoch,
}

/// Revision- and session-bound desired-state replacement.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct NativeContentDesiredUpdateRequest {
    /// Exact protocol line.
    pub protocol_version: NativeContentProtocolVersion,
    /// Correlation only.
    pub request_id: NativeContentRequestId,
    /// Canonical island identity.
    pub island_id: NativeContentIslandId,
    /// Current host-issued renderer session.
    pub client_epoch: NativeContentClientEpoch,
    /// Desired revision checked atomically.
    pub expected_desired_revision: NativeContentRevision,
    /// Complete desired-state replacement.
    pub update: DesiredUpdate,
}

/// Consumer decision for one mechanism-originated size proposal.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct NativeContentContentSizeDecisionRequest {
    /// Exact protocol line.
    pub protocol_version: NativeContentProtocolVersion,
    /// Correlation only.
    pub request_id: NativeContentRequestId,
    /// Canonical island identity.
    pub island_id: NativeContentIslandId,
    /// Current host-issued renderer session.
    pub client_epoch: NativeContentClientEpoch,
    /// Exact proposal being decided.
    pub proposal: ContentSizeProposal,
    /// Consumer-owned acceptance policy outcome.
    pub decision: ContentSizeDecision,
}

/// Connect command outcome.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum NativeContentConnectResult {
    /// Fresh renderer authority was issued.
    Connected {
        /// Echoed request correlation.
        request_id: NativeContentRequestId,
        /// Current authoritative state.
        snapshot: Box<NativeContentSnapshot>,
    },
    /// Compatibility or island admission failed.
    Rejected {
        /// Echoed request correlation.
        request_id: NativeContentRequestId,
        /// Stable typed rejection.
        rejection: NativeContentProtocolRejection,
    },
}

/// Snapshot query outcome.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum NativeContentSnapshotResult {
    /// Current state was returned.
    Ready {
        /// Echoed request correlation.
        request_id: NativeContentRequestId,
        /// Current authoritative state.
        snapshot: Box<NativeContentSnapshot>,
    },
    /// Compatibility or session admission failed.
    Rejected {
        /// Echoed request correlation.
        request_id: NativeContentRequestId,
        /// Stable typed rejection.
        rejection: NativeContentProtocolRejection,
    },
}

/// Desired-state mutation outcome.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum NativeContentDesiredUpdateResult {
    /// Desired state was replaced atomically.
    Committed {
        /// Echoed request correlation.
        request_id: NativeContentRequestId,
        /// Current authoritative state.
        snapshot: Box<NativeContentSnapshot>,
        /// Exact desired revision change.
        receipt: DesiredUpdateReceipt,
        /// Non-durable event projection for listeners.
        event: Box<NativeContentChangedEvent>,
    },
    /// No state changed.
    Rejected {
        /// Echoed request correlation.
        request_id: NativeContentRequestId,
        /// Stable typed rejection.
        rejection: NativeContentProtocolRejection,
    },
}

/// Content-size decision outcome.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum NativeContentContentSizeDecisionResult {
    /// Proposal was current and the decision was recorded without mutating desired state.
    Decided {
        /// Echoed request correlation.
        request_id: NativeContentRequestId,
        /// Current authoritative state.
        snapshot: Box<NativeContentSnapshot>,
        /// Exact non-mutating decision evidence.
        receipt: ContentSizeProposalReceipt,
        /// Non-durable event projection for listeners.
        event: Box<NativeContentChangedEvent>,
    },
    /// No decision was admitted.
    Rejected {
        /// Echoed request correlation.
        request_id: NativeContentRequestId,
        /// Stable typed rejection.
        rejection: NativeContentProtocolRejection,
    },
}

/// Product-neutral change carried by one non-durable event.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum NativeContentChangeProjection {
    /// A renderer desired-state request committed.
    DesiredUpdated {
        /// Request that initiated the change.
        request_id: NativeContentRequestId,
        /// Exact desired revision evidence.
        receipt: DesiredUpdateReceipt,
    },
    /// A selected native adapter admitted fresh observation.
    ObservationAdmitted {
        /// Optional initiating host operation correlation.
        request_id: Option<NativeContentRequestId>,
        /// Exact observed revision evidence.
        receipt: ObservationReceipt,
    },
    /// A mechanism proposed a new semantic content size.
    ContentSizeProposed {
        /// Host operation correlation.
        request_id: NativeContentRequestId,
        /// Revision- and generation-bound proposal.
        proposal: ContentSizeProposal,
    },
    /// A consumer decision was admitted without changing desired state.
    ContentSizeDecided {
        /// Renderer request correlation.
        request_id: NativeContentRequestId,
        /// Exact decision evidence.
        receipt: ContentSizeProposalReceipt,
    },
    /// A mechanism returned complete partial-apply evidence.
    ApplyCompleted {
        /// Host operation correlation.
        request_id: NativeContentRequestId,
        /// Exact ordered operation receipt.
        receipt: ApplyReceipt,
    },
    /// The bound host was destroyed and its generation invalidated.
    HostDestroyed {
        /// Optional host lifecycle correlation.
        request_id: Option<NativeContentRequestId>,
        /// Exact invalidation evidence.
        receipt: HostDestroyReceipt,
    },
}

/// One non-durable invalidation/event projection.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct NativeContentChangedEvent {
    /// Exact protocol line.
    pub protocol_version: NativeContentProtocolVersion,
    /// Authority and state cursor after the change.
    pub cursor: NativeContentCursor,
    /// Typed product-neutral change.
    pub change: NativeContentChangeProjection,
}
