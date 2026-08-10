use longhorn_core::{
    LayoutRevision, TransferClientId, TransferHostBindingId, TransferRequestId, WindowId,
};
use longhorn_surfaces::LayoutDocument;
use serde::{Deserialize, Serialize};

use crate::{
    ClientEpoch, DragSessionId, LeaseGeneration, PanelHostBindingKind, PanelTransferCommitReceipt,
    PanelTransferError, PanelTransferErrorCode, SessionCancellationReceipt,
    SessionCancellationStatus, SessionCreationReceipt, TargetResolutionPath, TransferError,
    TransferErrorCode, TransferPayload, TransferTargetBinding,
};

use super::TransferProtocolVersion;

/// Current renderer transfer authority for the caller's managed window.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct TransferClientSnapshot {
    protocol_version: TransferProtocolVersion,
    client_id: TransferClientId,
    client_epoch: ClientEpoch,
    current_lease_generation: Option<LeaseGeneration>,
}

impl TransferClientSnapshot {
    /// Constructs a current caller-window authority snapshot.
    #[must_use]
    pub const fn new(
        client_id: TransferClientId,
        client_epoch: ClientEpoch,
        current_lease_generation: Option<LeaseGeneration>,
    ) -> Self {
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            client_id,
            client_epoch,
            current_lease_generation,
        }
    }

    /// Returns the host-issued client identity.
    #[must_use]
    pub const fn client_id(&self) -> &TransferClientId {
        &self.client_id
    }

    /// Returns the current host-issued epoch.
    #[must_use]
    pub const fn client_epoch(&self) -> ClientEpoch {
        self.client_epoch
    }

    /// Returns the current complete replacement generation.
    #[must_use]
    pub const fn current_lease_generation(&self) -> Option<LeaseGeneration> {
        self.current_lease_generation
    }
}

/// Successful host-created session response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct TransferSessionStarted {
    protocol_version: TransferProtocolVersion,
    request_id: TransferRequestId,
    payload: TransferPayload,
}

impl TransferSessionStarted {
    /// Projects the renderer-visible portion of a domain receipt.
    #[must_use]
    pub const fn from_domain(
        request_id: TransferRequestId,
        receipt: SessionCreationReceipt,
    ) -> Self {
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            payload: receipt.payload(),
        }
    }
}

/// Successful complete lease replacement response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct TransferLeaseReceipt {
    protocol_version: TransferProtocolVersion,
    request_id: TransferRequestId,
    client_id: TransferClientId,
    client_epoch: ClientEpoch,
    generation: LeaseGeneration,
    zone_count: usize,
}

impl TransferLeaseReceipt {
    /// Constructs renderer-visible lease evidence.
    #[must_use]
    pub const fn new(
        request_id: TransferRequestId,
        client_id: TransferClientId,
        client_epoch: ClientEpoch,
        generation: LeaseGeneration,
        zone_count: usize,
    ) -> Self {
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            client_id,
            client_epoch,
            generation,
            zone_count,
        }
    }
}

/// Successful idempotent cancellation response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct TransferCancelReceipt {
    protocol_version: TransferProtocolVersion,
    request_id: TransferRequestId,
    session_id: DragSessionId,
    status: SessionCancellationStatus,
}

impl TransferCancelReceipt {
    /// Projects a domain cancellation receipt.
    #[must_use]
    pub const fn from_domain(
        request_id: TransferRequestId,
        receipt: SessionCancellationReceipt,
    ) -> Self {
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            session_id: receipt.session_id(),
            status: receipt.status(),
        }
    }
}

/// Stable target-resolution evidence returned after a commit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct TransferCommittedTarget {
    path: TargetResolutionPath,
    window_id: WindowId,
    drop_zone_id: longhorn_core::DropZoneId,
    insertion_position: Option<crate::InsertionPosition>,
    binding: TransferTargetBinding,
}

impl TransferCommittedTarget {
    /// Projects stable renderer-visible evidence from a terminal domain attempt.
    #[must_use]
    pub fn from_domain(attempt: &crate::TerminalTransferAttempt) -> Self {
        Self {
            path: attempt.target().path(),
            window_id: attempt.target().window_id().clone(),
            drop_zone_id: attempt.target().zone().id().clone(),
            insertion_position: attempt.target().zone().insertion_position(),
            binding: attempt.target().zone().target().clone(),
        }
    }
}

/// Successful authoritative same-document panel move.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct PanelTransferCompletion {
    protocol_version: TransferProtocolVersion,
    request_id: TransferRequestId,
    session_id: DragSessionId,
    source_host_binding_id: TransferHostBindingId,
    target_host_binding_id: TransferHostBindingId,
    source_binding_kind: PanelHostBindingKind,
    target_binding_kind: PanelHostBindingKind,
    previous_revision: LayoutRevision,
    committed_revision: LayoutRevision,
    authoritative_document: LayoutDocument,
    target: TransferCommittedTarget,
}

impl PanelTransferCompletion {
    /// Projects one domain commit without configuration-store internals.
    #[must_use]
    pub fn from_domain(
        request_id: TransferRequestId,
        receipt: &PanelTransferCommitReceipt,
    ) -> Self {
        let attempt = receipt.attempt();
        let layout = receipt.publication().layout();
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            session_id: attempt.session_id(),
            source_host_binding_id: attempt.source().host_binding_id().clone(),
            target_host_binding_id: attempt.target().zone().target().host_binding_id().clone(),
            source_binding_kind: receipt.source_binding_kind(),
            target_binding_kind: receipt.target_binding_kind(),
            previous_revision: layout.previous_revision(),
            committed_revision: layout.committed_revision(),
            authoritative_document: layout.authoritative_document().clone(),
            target: TransferCommittedTarget::from_domain(attempt),
        }
    }
}

/// Stable domain that rejected one transfer protocol request.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case", tag = "domain", content = "code")]
pub enum TransferAbortSource {
    /// Session, epoch, lease, or target coordination failed.
    Transfer(TransferErrorCode),
    /// Panel admission or authoritative panel commit failed.
    Panel(PanelTransferErrorCode),
}

/// Typed renderer-visible transfer rejection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct TransferAbort {
    protocol_version: TransferProtocolVersion,
    request_id: TransferRequestId,
    source: TransferAbortSource,
    message: String,
    retryable: bool,
    session_consumed: bool,
}

impl TransferAbort {
    /// Constructs one host-side transfer-domain rejection.
    #[must_use]
    pub fn host_transfer(
        request_id: TransferRequestId,
        code: TransferErrorCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            source: TransferAbortSource::Transfer(code),
            message: message.into(),
            retryable: false,
            session_consumed: false,
        }
    }

    /// Projects host-side lease geometry rejection before publication.
    #[must_use]
    pub fn invalid_lease(request_id: TransferRequestId, message: impl Into<String>) -> Self {
        Self::host_transfer(request_id, TransferErrorCode::InvalidLease, message)
    }

    /// Projects a coordinator failure conservatively as non-retryable.
    #[must_use]
    pub fn from_transfer(request_id: TransferRequestId, error: &TransferError) -> Self {
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            source: TransferAbortSource::Transfer(error.code()),
            message: error.detail().to_owned(),
            retryable: false,
            session_consumed: error.session_consumed(),
        }
    }

    /// Projects a panel adapter failure conservatively as non-retryable.
    #[must_use]
    pub fn from_panel(request_id: TransferRequestId, error: &PanelTransferError) -> Self {
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            source: TransferAbortSource::Panel(error.code()),
            message: error.detail().to_owned(),
            retryable: false,
            session_consumed: error.session_consumed(),
        }
    }
}

/// Panel-session admission response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "status")]
pub enum TransferSessionResponse {
    /// A bounded host-created session is ready.
    Started {
        /// Renderer-visible id-only session evidence.
        session: TransferSessionStarted,
    },
    /// Admission failed without partial session authority.
    Aborted {
        /// Typed rejection.
        abort: TransferAbort,
    },
}

/// Complete replacement lease response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "status")]
pub enum TransferLeaseResponse {
    /// The complete lease replaced prior authority.
    Published {
        /// Renderer-visible replacement evidence.
        lease: TransferLeaseReceipt,
    },
    /// The complete replacement failed atomically.
    Aborted {
        /// Typed rejection.
        abort: TransferAbort,
    },
}

/// Session cancellation response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "status")]
pub enum TransferCancelResponse {
    /// The named session is cancelled.
    Cancelled {
        /// Idempotent cancellation evidence.
        cancellation: TransferCancelReceipt,
    },
    /// The named session could not be cancelled.
    Aborted {
        /// Typed rejection.
        abort: TransferAbort,
    },
}

/// Terminal panel-transfer response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "status")]
pub enum PanelTransferResponse {
    /// The authoritative panel move committed once.
    Committed {
        /// Current layout snapshot and target evidence.
        completion: Box<PanelTransferCompletion>,
    },
    /// The terminal attempt aborted.
    Aborted {
        /// Typed rejection and consumption evidence.
        abort: TransferAbort,
    },
}
