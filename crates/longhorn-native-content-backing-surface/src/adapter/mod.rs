//! Backing-surface execution adapter.

mod api;
mod execute;
mod receipt;
mod state;
mod util;

pub use receipt::{
    BackingSurfaceDetachOutcome, BackingSurfaceDetachReceipt, BackingSurfaceHostDestroyOutcome,
    BackingSurfaceHostDestroyReceipt,
};
pub use state::BackingSurfaceAdapter;
pub(crate) use state::{AdapterState, Attachment};
pub(crate) use util::{
    compare_attached_generation, compare_generation, compare_generation_allow_next,
    current_attachment, current_attachment_mut, observation, validate_snapshot,
};
