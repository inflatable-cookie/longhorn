use std::{error::Error, fmt};

use longhorn_surfaces::LayoutMutationRejectionCode;
use serde::{Deserialize, Serialize};

use crate::{TransferError, TransferErrorCode};

/// Stable panel-transfer rejection category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PanelTransferErrorCode {
    /// The host-binding snapshot was internally inconsistent.
    InvalidBindingSnapshot,
    /// A required current host binding was absent.
    UnknownHostBinding,
    /// A current host binding no longer matched recorded authority.
    StaleHostBinding,
    /// The registered layout document could not be loaded.
    LayoutLoadFailed,
    /// The registered layout document was not ready for authoritative use.
    LayoutUnavailable,
    /// The named source panel no longer existed.
    UnknownPanel,
    /// The source panel was not movable.
    PanelNotMovable,
    /// The source panel no longer occupied its recorded placement.
    SourceChanged,
    /// A terminal attempt did not contain a panel source.
    UnsupportedSource,
    /// A terminal attempt did not resolve a panel-region target.
    UnsupportedTarget,
    /// Source and target named different registered layout domains.
    CrossDocument,
    /// Copy is not implemented by the first panel-transfer line.
    CopyUnsupported,
    /// Recorded layout revision evidence was no longer current.
    StaleLayoutRevision,
    /// The target container or region no longer existed.
    TargetChanged,
    /// Current placement policy rejected the target.
    IneligibleTarget,
    /// Current instance policy rejected the mutation.
    InstancePolicyExceeded,
    /// Advisory insertion evidence was invalid against current target state.
    InvalidInsertionPosition,
    /// The authoritative layout mutation rejected for another typed reason.
    LayoutMutationRejected,
    /// Coordinated configuration publication failed.
    PublicationFailed,
    /// Session or lease authority rejected the attempt.
    TransferRejected,
}

/// Typed panel-transfer admission or terminal failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelTransferError {
    code: PanelTransferErrorCode,
    detail: String,
    session_consumed: bool,
    transfer_code: Option<TransferErrorCode>,
    layout_code: Option<LayoutMutationRejectionCode>,
}

impl PanelTransferError {
    pub(crate) fn new(code: PanelTransferErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            session_consumed: false,
            transfer_code: None,
            layout_code: None,
        }
    }

    pub(crate) fn consumed(mut self) -> Self {
        self.session_consumed = true;
        self
    }

    pub(crate) fn from_transfer(error: TransferError) -> Self {
        Self {
            code: PanelTransferErrorCode::TransferRejected,
            detail: error.detail().to_owned(),
            session_consumed: error.session_consumed(),
            transfer_code: Some(error.code()),
            layout_code: None,
        }
    }

    pub(crate) fn from_layout_rejection(
        code: PanelTransferErrorCode,
        rejection: &longhorn_surfaces::LayoutMutationRejection,
    ) -> Self {
        Self {
            code,
            detail: rejection.detail().to_owned(),
            session_consumed: true,
            transfer_code: None,
            layout_code: Some(rejection.code()),
        }
    }

    /// Returns the stable adapter rejection category.
    #[must_use]
    pub const fn code(&self) -> PanelTransferErrorCode {
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

    /// Returns the underlying session or lease category when applicable.
    #[must_use]
    pub const fn transfer_code(&self) -> Option<TransferErrorCode> {
        self.transfer_code
    }

    /// Returns the existing layout-engine category when applicable.
    #[must_use]
    pub const fn layout_code(&self) -> Option<LayoutMutationRejectionCode> {
        self.layout_code
    }
}

impl fmt::Display for PanelTransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for PanelTransferError {}
