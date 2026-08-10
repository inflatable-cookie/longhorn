use longhorn_core::{DomainId, RegionId, SurfaceId, TransferHostBindingId, WindowId};

use crate::{TerminalTransferAttempt, TransferSourceAuthority, TransferTargetBinding};

use super::{PanelTransferError, PanelTransferErrorCode, consumed};

pub(super) struct PanelSource<'a> {
    pub(super) window_id: &'a WindowId,
    pub(super) subject_id: &'a str,
    pub(super) host_binding_id: &'a TransferHostBindingId,
    pub(super) document_id: &'a DomainId,
    pub(super) revision: u64,
    pub(super) surface_id: &'a SurfaceId,
    pub(super) region_id: &'a RegionId,
}

pub(super) struct PanelTarget<'a> {
    pub(super) window_id: &'a WindowId,
    pub(super) host_binding_id: &'a TransferHostBindingId,
    pub(super) document_id: &'a DomainId,
    pub(super) revision: u64,
    pub(super) surface_id: &'a SurfaceId,
    pub(super) region_id: &'a RegionId,
}

pub(super) fn panel_source(
    attempt: &TerminalTransferAttempt,
) -> Result<PanelSource<'_>, PanelTransferError> {
    let TransferSourceAuthority::Panel {
        source_window_id,
        subject_id,
        host_binding_id,
        document_id,
        revision,
        surface_id,
        region_id,
        ..
    } = attempt.source()
    else {
        return Err(consumed(
            PanelTransferErrorCode::UnsupportedSource,
            "terminal attempt did not contain a panel source",
        ));
    };
    Ok(PanelSource {
        window_id: source_window_id,
        subject_id: subject_id.as_str(),
        host_binding_id,
        document_id,
        revision: revision.get(),
        surface_id,
        region_id,
    })
}

pub(super) fn panel_target(
    attempt: &TerminalTransferAttempt,
) -> Result<PanelTarget<'_>, PanelTransferError> {
    let TransferTargetBinding::PanelRegion {
        host_binding_id,
        document_id,
        revision,
        surface_id,
        region_id,
    } = attempt.target().zone().target()
    else {
        return Err(consumed(
            PanelTransferErrorCode::UnsupportedTarget,
            "terminal attempt did not resolve a panel-region target",
        ));
    };
    Ok(PanelTarget {
        window_id: attempt.target().window_id(),
        host_binding_id,
        document_id,
        revision: revision.get(),
        surface_id,
        region_id,
    })
}
