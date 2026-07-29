use longhorn_core::{
    DisplayId, ScreenPoint, SurfaceRevision, TransferHostBindingId, TransferRequestId, WindowId,
};
use longhorn_surfaces::{SurfaceDocument, SurfaceMutationRejectionCode};
use longhorn_transfer::{
    DragSessionId, TransferCommittedTarget, TransferErrorCode, TransferProtocolVersion,
    TransferSessionStarted,
};
use serde::{Deserialize, Serialize};

use crate::{
    SurfaceTerminalAttempt, SurfaceTransferCommitReceipt, SurfaceTransferError,
    SurfaceTransferErrorCode,
};

/// Completed hidden creation, placement, readiness, and host commit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SurfaceProvisioningCompletion {
    window_id: WindowId,
    host_binding_id: TransferHostBindingId,
    display_id: DisplayId,
}

impl SurfaceProvisioningCompletion {
    fn from_domain(provision: &crate::CompletedSurfaceProvision) -> Self {
        debug_assert_eq!(
            provision.provision().window_id(),
            provision.commit().window_id()
        );
        Self {
            window_id: provision.provision().window_id().clone(),
            host_binding_id: provision.provision().host_binding_id().clone(),
            display_id: provision.provision().display_id().clone(),
        }
    }
}

/// Stable target evidence for one successful whole-Surface move.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub enum SurfaceTransferTarget {
    /// An existing managed window and current leased zone resolved.
    Existing {
        /// Complete shared target evidence.
        target: TransferCommittedTarget,
    },
    /// Empty-display policy provisioned and committed a host.
    Provisioned {
        /// Fresh screen-DIP point outside all managed windows.
        #[cfg_attr(feature = "bindings", ts(type = "{ x: number; y: number }"))]
        drop_point: ScreenPoint,
        /// Completed host lifecycle evidence.
        provisioning: SurfaceProvisioningCompletion,
    },
}

/// Successful authoritative whole-Surface move.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SurfaceTransferCompletion {
    protocol_version: TransferProtocolVersion,
    request_id: TransferRequestId,
    session_id: DragSessionId,
    source_host_binding_id: TransferHostBindingId,
    target_host_binding_id: TransferHostBindingId,
    previous_revision: SurfaceRevision,
    committed_revision: SurfaceRevision,
    authoritative_document: SurfaceDocument,
    target: SurfaceTransferTarget,
}

impl SurfaceTransferCompletion {
    /// Projects one domain commit without configuration-store internals.
    #[must_use]
    pub fn from_domain(
        request_id: TransferRequestId,
        receipt: &SurfaceTransferCommitReceipt,
    ) -> Self {
        let surface = receipt.publication().surface();
        let target = match receipt.attempt() {
            SurfaceTerminalAttempt::Existing(attempt) => SurfaceTransferTarget::Existing {
                target: TransferCommittedTarget::from_domain(attempt),
            },
            SurfaceTerminalAttempt::EmptyDisplay(attempt) => {
                let provisioning = receipt
                    .provisioning()
                    .expect("successful empty-display commit retains provisioning evidence");
                SurfaceTransferTarget::Provisioned {
                    drop_point: attempt.screen_point(),
                    provisioning: SurfaceProvisioningCompletion::from_domain(provisioning),
                }
            }
        };
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            session_id: receipt.attempt().session_id(),
            source_host_binding_id: receipt.source_host_binding_id().clone(),
            target_host_binding_id: receipt.target_host_binding_id().clone(),
            previous_revision: surface.previous_revision(),
            committed_revision: surface.committed_revision(),
            authoritative_document: surface.authoritative_document().clone(),
            target,
        }
    }
}

/// Stable domain that rejected a whole-Surface request.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case", tag = "domain", content = "code")]
pub enum SurfaceTransferAbortSource {
    /// Session, epoch, lease, or target coordination failed.
    Transfer(TransferErrorCode),
    /// Whole-Surface admission, policy, publication, or host lifecycle failed.
    SurfaceTransfer(SurfaceTransferErrorCode),
}

/// Typed renderer-visible whole-Surface rejection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SurfaceTransferAbort {
    protocol_version: TransferProtocolVersion,
    request_id: TransferRequestId,
    source: SurfaceTransferAbortSource,
    surface_code: Option<SurfaceMutationRejectionCode>,
    message: String,
    retryable: bool,
    session_consumed: bool,
    reconciliation_required: bool,
}

impl SurfaceTransferAbort {
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
            source: SurfaceTransferAbortSource::Transfer(code),
            surface_code: None,
            message: message.into(),
            retryable: false,
            session_consumed: false,
            reconciliation_required: false,
        }
    }

    /// Projects an adapter failure conservatively as non-retryable.
    #[must_use]
    pub fn from_domain(request_id: TransferRequestId, error: &SurfaceTransferError) -> Self {
        let source = error.transfer_code().map_or_else(
            || SurfaceTransferAbortSource::SurfaceTransfer(error.code()),
            SurfaceTransferAbortSource::Transfer,
        );
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            source,
            surface_code: error.surface_code(),
            message: error.detail().to_owned(),
            retryable: false,
            session_consumed: error.session_consumed(),
            reconciliation_required: error.code()
                == SurfaceTransferErrorCode::HostReconciliationRequired,
        }
    }
}

/// Whole-Surface session-admission response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "status")]
pub enum SurfaceSessionResponse {
    /// A bounded host-created session is ready.
    Started {
        /// Renderer-visible id-only session evidence.
        session: TransferSessionStarted,
    },
    /// Admission failed without partial session authority.
    Aborted {
        /// Typed rejection.
        abort: SurfaceTransferAbort,
    },
}

/// Terminal whole-Surface transfer response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "status")]
pub enum SurfaceTransferResponse {
    /// The authoritative whole-Surface move committed once.
    Committed {
        /// Current Surface snapshot and target evidence.
        completion: Box<SurfaceTransferCompletion>,
    },
    /// The terminal attempt aborted.
    Aborted {
        /// Typed rejection and reconciliation evidence.
        abort: SurfaceTransferAbort,
    },
}
