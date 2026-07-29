//! Narrow Tauri readback, projection, and handler assembly for transfer.

mod commands;
mod error;
mod handler;
mod model;
mod projection;
mod runtime;

#[cfg(feature = "surface-transfer")]
pub use commands::{
    AssembledSurfaceTransferCommands, SurfaceTransferCommandService, TauriSurfaceTransferState,
    longhorn_transfer_commit_surface, longhorn_transfer_start_surface,
};
pub use commands::{
    AssembledTransferCommands, TRANSFER_CLIENT_CHANGED_EVENT, TauriTransferState,
    TransferCommandService, longhorn_transfer_cancel, longhorn_transfer_commit_panel,
    longhorn_transfer_publish_lease, longhorn_transfer_snapshot, longhorn_transfer_start_panel,
};
pub use error::{TransferHandlerError, TransferProjectionError, TransferRuntimeError};
#[cfg(feature = "surface-transfer")]
pub use handler::SurfaceTransferAdapter;
pub use handler::{
    PanelTransferAdapter, TransferCallerAuthority, TransferHandlerAssembly,
    TransferHandlerTeardownReceipt, TransferHandlerTeardownStatus,
};
pub use model::{ManagedTransferSnapshot, ManagedTransferWindow};
pub use projection::{project_client_point, project_client_rect};
pub use runtime::{ManagedTransferRuntime, TauriTransferRuntime};
