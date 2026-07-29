//! Bounded framework-neutral cross-window transfer sessions and target leases.

mod coordinator;
mod error;
mod identity;
mod model;
mod policy;

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
    DropZone, LeasePublication, LiveTransferWindow, ResolvedTransferTarget,
    SessionCancellationReceipt, SessionCancellationStatus, SessionCreationReceipt,
    TRANSFER_PROTOCOL_VERSION, TargetResolutionPath, TargetSelector, TerminalTransferAttempt,
    TransferCapability, TransferPayload, TransferSessionRequest, TransferSourceAuthority,
    TransferSubjectKind, TransferTargetBinding,
};
pub use policy::{TransferLimits, TransferLimitsError};
