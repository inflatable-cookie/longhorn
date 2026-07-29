use std::{error::Error, fmt};

use longhorn_surfaces::SurfaceMutationRejectionCode;
use longhorn_surfaces_config::SurfaceConfigPublicationReceipt;
use longhorn_transfer::{TransferError, TransferErrorCode};
use serde::{Deserialize, Serialize};

use crate::{
    SurfaceWindowCleanupReceipt, SurfaceWindowProvisionFailure, SurfaceWindowProvisionReceipt,
};

/// Stable whole-Surface transfer rejection category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SurfaceTransferErrorCode {
    /// The host-binding snapshot was internally inconsistent.
    InvalidBindingSnapshot,
    /// A required current host binding was absent.
    UnknownHostBinding,
    /// A current host binding no longer matched recorded authority.
    StaleHostBinding,
    /// The registered Surface document could not be loaded.
    SurfaceLoadFailed,
    /// The registered Surface document was not ready for authoritative use.
    SurfaceUnavailable,
    /// The named source Surface no longer existed.
    UnknownSurface,
    /// The source Surface no longer occupied its recorded primary host.
    SourceChanged,
    /// A terminal attempt did not contain a Surface source.
    UnsupportedSource,
    /// A normal attempt did not resolve a Surface-window target.
    UnsupportedTarget,
    /// Source or target named another registered Surface domain.
    CrossDocument,
    /// Recorded Surface revision evidence was no longer current.
    StaleSurfaceRevision,
    /// Consumer policy no longer allowed the target.
    IneligibleTarget,
    /// The target no longer participated or was not declared by the Surface.
    TargetChanged,
    /// Advisory insertion evidence was invalid against current target state.
    InvalidInsertionPosition,
    /// Empty-display provisioning was disabled.
    EmptyDisplayDisabled,
    /// No consumer-approved display target contained the point.
    NoEmptyDisplayTarget,
    /// Multiple consumer-approved display targets contained the point.
    AmbiguousEmptyDisplayTarget,
    /// The injected host failed before a prepared target existed.
    ProvisionFailed,
    /// The provisioner returned authority for another target.
    ProvisionReceiptMismatch,
    /// The authoritative Surface mutation rejected for another typed reason.
    SurfaceMutationRejected,
    /// Coordinated Surface publication failed.
    PublicationFailed,
    /// Durable Surface state committed but the host could not finalize it.
    HostReconciliationRequired,
    /// Session or lease authority rejected the attempt.
    TransferRejected,
}

/// Cleanup result after failed Surface publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProvisionCleanupOutcome {
    /// Prepared native state was removed.
    Succeeded(SurfaceWindowCleanupReceipt),
    /// Cleanup authority remains unresolved.
    Failed(SurfaceWindowProvisionFailure),
}

/// Provisioning evidence attached to a typed failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SurfaceProvisionFailureEvidence {
    /// Creation, placement, or readiness failed before Surface publication.
    ProvisionFailed(SurfaceWindowProvisionFailure),
    /// A prepared host contradicted the admitted target and was cleaned up.
    PreparedTargetRejected {
        /// Prepared target returned by the host.
        provision: SurfaceWindowProvisionReceipt,
        /// Explicit cleanup outcome.
        cleanup: ProvisionCleanupOutcome,
    },
    /// Surface publication failed and cleanup was attempted.
    PublicationFailed {
        /// Hidden prepared target.
        provision: SurfaceWindowProvisionReceipt,
        /// Explicit cleanup outcome.
        cleanup: ProvisionCleanupOutcome,
    },
    /// Surface publication committed but host commit failed.
    ReconciliationRequired {
        /// Hidden prepared target.
        provision: SurfaceWindowProvisionReceipt,
        /// Authoritative committed Surface state.
        publication: Box<SurfaceConfigPublicationReceipt>,
        /// Failed host commit.
        failure: SurfaceWindowProvisionFailure,
    },
}

/// Typed whole-Surface admission, commit, or provisioning failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceTransferError {
    code: SurfaceTransferErrorCode,
    detail: String,
    session_consumed: bool,
    transfer_code: Option<TransferErrorCode>,
    surface_code: Option<SurfaceMutationRejectionCode>,
    provisioning: Option<Box<SurfaceProvisionFailureEvidence>>,
}

impl SurfaceTransferError {
    pub(crate) fn new(code: SurfaceTransferErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            session_consumed: false,
            transfer_code: None,
            surface_code: None,
            provisioning: None,
        }
    }

    pub(crate) fn consumed(mut self) -> Self {
        self.session_consumed = true;
        self
    }

    pub(crate) fn from_transfer(error: TransferError) -> Self {
        Self {
            code: SurfaceTransferErrorCode::TransferRejected,
            detail: error.detail().to_owned(),
            session_consumed: error.session_consumed(),
            transfer_code: Some(error.code()),
            surface_code: None,
            provisioning: None,
        }
    }

    pub(crate) fn with_surface_code(mut self, code: SurfaceMutationRejectionCode) -> Self {
        self.surface_code = Some(code);
        self
    }

    pub(crate) fn with_provisioning(mut self, evidence: SurfaceProvisionFailureEvidence) -> Self {
        self.provisioning = Some(Box::new(evidence));
        self
    }

    pub(crate) fn reconciliation_required(mut self, detail: impl Into<String>) -> Self {
        self.code = SurfaceTransferErrorCode::HostReconciliationRequired;
        self.detail = detail.into();
        self
    }

    /// Returns the stable adapter rejection category.
    #[must_use]
    pub const fn code(&self) -> SurfaceTransferErrorCode {
        self.code
    }

    /// Returns diagnostic detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Returns whether the first terminal session attempt was consumed.
    #[must_use]
    pub const fn session_consumed(&self) -> bool {
        self.session_consumed
    }

    /// Returns the underlying transfer category when applicable.
    #[must_use]
    pub const fn transfer_code(&self) -> Option<TransferErrorCode> {
        self.transfer_code
    }

    /// Returns the underlying Surface mutation category when applicable.
    #[must_use]
    pub const fn surface_code(&self) -> Option<SurfaceMutationRejectionCode> {
        self.surface_code
    }

    /// Returns provisioning, cleanup, or reconciliation evidence.
    #[must_use]
    pub fn provisioning(&self) -> Option<&SurfaceProvisionFailureEvidence> {
        self.provisioning.as_deref()
    }
}

impl fmt::Display for SurfaceTransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for SurfaceTransferError {}
