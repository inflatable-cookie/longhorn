//! Whole-Surface transfer with explicit target policy and window provisioning.

mod admission;
mod binding;
mod commit;
mod error;
mod policy;
mod protocol;
mod provision;
mod wire;

pub use admission::admit_surface_session;
pub use binding::{SurfaceHostBinding, SurfaceHostBindings};
pub use commit::commit_surface_transfer;
pub use error::{
    ProvisionCleanupOutcome, SurfaceProvisionFailureEvidence, SurfaceTransferError,
    SurfaceTransferErrorCode,
};
pub use policy::{
    EmptyDisplayProvisionPolicy, EmptyDisplayProvisionTarget, SurfaceTransferPolicy,
    SurfaceTransferPolicyError,
};
pub use protocol::{
    CompletedSurfaceProvision, SurfaceSessionAdmission, SurfaceTerminalAttempt,
    SurfaceTransferCommitReceipt, SurfaceTransferCommitRequest,
};
pub use provision::{
    ProvisionedSurfaceWindow, SurfaceWindowCleanupReceipt, SurfaceWindowCommitReceipt,
    SurfaceWindowProvisionFailure, SurfaceWindowProvisionReceipt, SurfaceWindowProvisionRequest,
    SurfaceWindowProvisionStage, SurfaceWindowProvisioner,
};
pub use wire::{
    SurfaceProvisioningCompletion, SurfaceSessionResponse, SurfaceSessionStartRequest,
    SurfaceTransferAbort, SurfaceTransferAbortSource, SurfaceTransferCommand,
    SurfaceTransferCompletion, SurfaceTransferResponse, SurfaceTransferTarget,
};
