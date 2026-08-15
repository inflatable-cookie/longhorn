//! Backing-surface execution adapter.

mod api;
mod execute;
mod receipt;
mod state;
mod util;

pub(crate) use longhorn_native_content::{compare_attached_generation, compare_generation};
pub use receipt::{
    BackingSurfaceDetachOutcome, BackingSurfaceDetachReceipt, BackingSurfaceHostDestroyOutcome,
    BackingSurfaceHostDestroyReceipt,
};
pub use state::BackingSurfaceAdapter;
pub(crate) use state::{AdapterState, Attachment};
pub(crate) use util::{
    current_attachment, current_attachment_mut, observation, reject_invalidated, validate_snapshot,
};
