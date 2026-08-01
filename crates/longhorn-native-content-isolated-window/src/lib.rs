//! Process-isolated native-window execution for native-content coordination.
//!
//! The crate coordinates one consumer-owned content process through injected
//! lifecycle ports. It owns no plugin ABI, product authorization, outer-window
//! placement, raw native handle, or renderer protocol.

mod adapter;
mod error;
mod policy;
mod protocol;
mod runtime;

pub use adapter::IsolatedWindowAdapter;
pub use error::IsolatedWindowError;
pub use policy::{ISOLATED_WINDOW_CAPABILITIES, IsolatedWindowSpec};
pub use protocol::{
    HelperCommand, HelperCommandKind, HelperMessage, HelperMessageKind, HelperSnapshot,
    ISOLATED_WINDOW_HELPER_PROTOCOL_VERSION, IsolatedContentRequest, IsolatedContentRequestKind,
    IsolatedWindowHelperProtocolVersion,
};
pub use runtime::{
    IsolatedWindowAdapterEvent, IsolatedWindowRuntime, IsolatedWindowRuntimeEvent,
    IsolatedWindowRuntimeEventKind, RuntimeAttachRequest, TeardownOutcome,
};
