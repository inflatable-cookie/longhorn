use longhorn_core::{PanelInstanceId, TransferClientId, TransferHostBindingId, WindowId};
use longhorn_surfaces_config::LayoutConfigPublicationReceipt;

use crate::{
    ClientEpoch, DragSessionId, LiveTransferWindow, TargetSelector, TerminalTransferAttempt,
    TransferDuration,
};

use super::PanelHostBindingKind;

/// Fresh host inputs needed to admit one movable panel session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelSessionAdmission {
    source_window_id: WindowId,
    client_id: TransferClientId,
    client_epoch: ClientEpoch,
    panel_instance_id: PanelInstanceId,
    host_binding_id: TransferHostBindingId,
    lifetime: TransferDuration,
}

impl PanelSessionAdmission {
    /// Constructs one panel admission request.
    #[must_use]
    pub const fn new(
        source_window_id: WindowId,
        client_id: TransferClientId,
        client_epoch: ClientEpoch,
        panel_instance_id: PanelInstanceId,
        host_binding_id: TransferHostBindingId,
        lifetime: TransferDuration,
    ) -> Self {
        Self {
            source_window_id,
            client_id,
            client_epoch,
            panel_instance_id,
            host_binding_id,
            lifetime,
        }
    }

    pub(crate) const fn source_window_id(&self) -> &WindowId {
        &self.source_window_id
    }

    pub(crate) const fn client_id(&self) -> &TransferClientId {
        &self.client_id
    }

    pub(crate) const fn client_epoch(&self) -> ClientEpoch {
        self.client_epoch
    }

    pub(crate) const fn panel_instance_id(&self) -> &PanelInstanceId {
        &self.panel_instance_id
    }

    pub(crate) const fn host_binding_id(&self) -> &TransferHostBindingId {
        &self.host_binding_id
    }

    pub(crate) const fn lifetime(&self) -> TransferDuration {
        self.lifetime
    }
}

/// Requested first-line panel transfer operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelTransferOperation {
    /// Move one existing panel within its registered layout document.
    Move,
    /// Unsupported copy request retained as explicit rejection evidence.
    Copy,
}

/// Inputs for one terminal panel-transfer attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelTransferCommitRequest {
    session_id: DragSessionId,
    selector: TargetSelector,
    live_windows: Vec<LiveTransferWindow>,
    operation: PanelTransferOperation,
}

impl PanelTransferCommitRequest {
    /// Constructs one terminal request from fresh managed-window evidence.
    #[must_use]
    pub fn new(
        session_id: DragSessionId,
        selector: TargetSelector,
        live_windows: impl IntoIterator<Item = LiveTransferWindow>,
        operation: PanelTransferOperation,
    ) -> Self {
        Self {
            session_id,
            selector,
            live_windows: live_windows.into_iter().collect(),
            operation,
        }
    }

    pub(crate) const fn session_id(&self) -> DragSessionId {
        self.session_id
    }

    pub(crate) const fn selector(&self) -> &TargetSelector {
        &self.selector
    }

    pub(crate) fn live_windows(&self) -> &[LiveTransferWindow] {
        self.live_windows.as_slice()
    }

    pub(crate) const fn operation(&self) -> PanelTransferOperation {
        self.operation
    }
}

/// Successful consumed session and authoritative layout publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelTransferCommitReceipt {
    attempt: TerminalTransferAttempt,
    source_binding_kind: PanelHostBindingKind,
    target_binding_kind: PanelHostBindingKind,
    publication: LayoutConfigPublicationReceipt,
}

impl PanelTransferCommitReceipt {
    pub(crate) const fn new(
        attempt: TerminalTransferAttempt,
        source_binding_kind: PanelHostBindingKind,
        target_binding_kind: PanelHostBindingKind,
        publication: LayoutConfigPublicationReceipt,
    ) -> Self {
        Self {
            attempt,
            source_binding_kind,
            target_binding_kind,
            publication,
        }
    }

    /// Returns consumed session and selected target evidence.
    #[must_use]
    pub const fn attempt(&self) -> &TerminalTransferAttempt {
        &self.attempt
    }

    /// Returns the fresh source composition shape.
    #[must_use]
    pub const fn source_binding_kind(&self) -> PanelHostBindingKind {
        self.source_binding_kind
    }

    /// Returns the fresh target composition shape.
    #[must_use]
    pub const fn target_binding_kind(&self) -> PanelHostBindingKind {
        self.target_binding_kind
    }

    /// Returns the existing authoritative layout and configuration receipt.
    #[must_use]
    pub const fn publication(&self) -> &LayoutConfigPublicationReceipt {
        &self.publication
    }
}
