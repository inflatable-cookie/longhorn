use longhorn_core::{DomainId, SurfaceId, TransferHostBindingId, WindowId};
use longhorn_transfer::{
    EmptyDisplayTransferAttempt, TerminalTransferAttempt, TransferSourceAuthority,
    TransferTargetBinding,
};

use crate::{SurfaceTransferError, SurfaceTransferErrorCode};

pub(super) struct SurfaceSource<'a> {
    pub(super) window_id: &'a WindowId,
    pub(super) surface_id: SurfaceId,
    pub(super) host_binding_id: &'a TransferHostBindingId,
    pub(super) document_id: &'a DomainId,
    pub(super) revision: u64,
}

pub(super) struct ExistingSurfaceTarget<'a> {
    pub(super) window_id: &'a WindowId,
    pub(super) host_binding_id: &'a TransferHostBindingId,
    pub(super) document_id: &'a DomainId,
    pub(super) revision: u64,
    pub(super) insertion_index: Option<u32>,
}

pub(super) fn existing_source(
    attempt: &TerminalTransferAttempt,
) -> Result<SurfaceSource<'_>, SurfaceTransferError> {
    surface_source(attempt.source())
}

pub(super) fn empty_source(
    attempt: &EmptyDisplayTransferAttempt,
) -> Result<SurfaceSource<'_>, SurfaceTransferError> {
    surface_source(attempt.source())
}

pub(super) fn existing_target(
    attempt: &TerminalTransferAttempt,
) -> Result<ExistingSurfaceTarget<'_>, SurfaceTransferError> {
    let TransferTargetBinding::SurfaceWindow {
        host_binding_id,
        document_id,
        revision,
    } = attempt.target().zone().target()
    else {
        return Err(consumed(
            SurfaceTransferErrorCode::UnsupportedTarget,
            "terminal attempt did not resolve a Surface-window target",
        ));
    };
    Ok(ExistingSurfaceTarget {
        window_id: attempt.target().window_id(),
        host_binding_id,
        document_id,
        revision: revision.get(),
        insertion_index: attempt
            .target()
            .zone()
            .insertion_position()
            .map(|position| position.get()),
    })
}

fn surface_source(
    authority: &TransferSourceAuthority,
) -> Result<SurfaceSource<'_>, SurfaceTransferError> {
    let TransferSourceAuthority::Surface {
        source_window_id,
        subject_id,
        host_binding_id,
        document_id,
        revision,
        ..
    } = authority
    else {
        return Err(consumed(
            SurfaceTransferErrorCode::UnsupportedSource,
            "terminal attempt did not contain a Surface source",
        ));
    };
    let surface_id = SurfaceId::new(subject_id.as_str())
        .expect("Surface admission records a grammatical Surface identity");
    Ok(SurfaceSource {
        window_id: source_window_id,
        surface_id,
        host_binding_id,
        document_id,
        revision: revision.get(),
    })
}

pub(super) fn consumed(
    code: SurfaceTransferErrorCode,
    detail: impl Into<String>,
) -> SurfaceTransferError {
    SurfaceTransferError::new(code, detail).consumed()
}
