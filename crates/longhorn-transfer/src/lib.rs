//! Bounded framework-neutral cross-window transfer sessions and target leases.

mod coordinator;
mod error;
mod identity;
mod model;
mod panel;
mod policy;
mod wire;

pub use coordinator::{
    ClientEpochBindingStatus, CoordinatorDiscardReceipt, LeasePublicationReceipt,
    TransferCoordinator, WindowInvalidationReceipt,
};
pub use error::{TransferError, TransferErrorCode};
pub use identity::{
    ClientEpoch, DragSessionId, DragSessionIdAllocationError, DragSessionIdAllocator,
    DragSessionIdParseError, InsertionPosition, LeaseGeneration, MonotonicClock, TransferDuration,
    TransferInstant, TransferRevision,
};
pub use longhorn_core::{DropZoneId, TransferClientId, TransferHostBindingId, TransferSubjectId};
pub use model::{
    DropZone, EmptyDisplayTransferAttempt, LeasePublication, LiveTransferWindow,
    ResolvedTransferTarget, SessionCancellationReceipt, SessionCancellationStatus,
    SessionCreationReceipt, TRANSFER_PROTOCOL_VERSION, TargetResolutionPath, TargetSelector,
    TerminalTransferAttempt, TerminalTransferResolution, TransferCapability, TransferPayload,
    TransferSessionRequest, TransferSourceAuthority, TransferSubjectKind, TransferTargetBinding,
};
pub use panel::{
    PanelHostBinding, PanelHostBindingKind, PanelHostBindings, PanelSessionAdmission,
    PanelTransferCommitReceipt, PanelTransferCommitRequest, PanelTransferError,
    PanelTransferErrorCode, PanelTransferOperation, admit_panel_session, commit_panel_transfer,
};
pub use policy::{TransferLimits, TransferLimitsError};
pub use wire::{
    ClientDropZone, PanelSessionStartRequest, PanelTransferCommand, PanelTransferCompletion,
    PanelTransferResponse, TransferAbort, TransferAbortSource, TransferCancelReceipt,
    TransferCancelRequest, TransferCancelResponse, TransferClientSnapshot, TransferCommitSelector,
    TransferCommittedTarget, TransferLeaseReceipt, TransferLeaseRequest, TransferLeaseResponse,
    TransferProtocolVersion, TransferSessionResponse, TransferSessionStarted,
};
