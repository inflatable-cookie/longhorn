//! Narrow Tauri assembly over the generic Longhorn bridge protocol.

mod authority;
mod commands;
mod error;
mod events;
mod handler;
mod registration;

pub use authority::BridgeAuthorityProvider;
pub use commands::{
    BridgeCommandService, TauriBridgeState, longhorn_bridge_authority, longhorn_bridge_cancel,
    longhorn_bridge_command, longhorn_bridge_hello, longhorn_bridge_query, longhorn_bridge_resync,
};
pub use error::{BridgeHostError, BridgeHostErrorCode};
pub use events::{
    BRIDGE_DOMAIN_EVENT, BRIDGE_PROGRESS_EVENT, BRIDGE_TERMINAL_EVENT, BridgeEventSink,
    TauriBridgeEventSink,
};
pub use handler::BridgeHandlerAssembly;
pub use registration::{
    BridgeCancellationHandler, BridgeCommandHandler, BridgeDomainRegistry, BridgeQueryHandler,
    BridgeSnapshotHandler,
};
