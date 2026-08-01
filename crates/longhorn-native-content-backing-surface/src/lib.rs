//! Backing-surface execution for native-content coordination.
//!
//! The crate coordinates consumer-owned full-host native storage while the
//! desired viewport remains a separate output and interaction clip. It owns no
//! GPU renderer, semantic input payload, raw native handle, or UI framework.

mod adapter;
mod error;
mod policy;
mod runtime;

pub use adapter::{
    BackingSurfaceAdapter, BackingSurfaceDetachOutcome, BackingSurfaceDetachReceipt,
    BackingSurfaceHostDestroyOutcome, BackingSurfaceHostDestroyReceipt,
};
pub use error::BackingSurfaceError;
pub use policy::{BACKING_SURFACE_CAPABILITIES, BackingSurfaceSpec};
pub use runtime::{
    BackingSurfaceAdapterEvent, BackingSurfaceRuntime, BackingSurfaceRuntimeEvent,
    BackingSurfaceRuntimeEventKind, BackingSurfaceSnapshot, InputAdmission, InputRejection,
    RuntimeAttachRequest,
};
