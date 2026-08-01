//! Private backing-surface mechanism proof for Card 085.

mod adapter;
mod error;
mod runtime;

pub use adapter::{BackingSurfaceAdapter, BackingSurfaceSpec, InvalidatedAttachment};
pub use error::BackingSurfaceError;
pub use runtime::{
    AdapterEvent, BackingSurfaceRuntime, DetachOutcome, InputAdmission, InputRejection,
    RuntimeAttachRequest, RuntimeEvent, RuntimeEventKind, RuntimeSnapshot,
};
