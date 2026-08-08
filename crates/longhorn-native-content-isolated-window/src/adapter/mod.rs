//! Isolated-window execution adapter.

mod api;
mod execute;
mod state;
mod util;

pub use state::IsolatedWindowAdapter;
pub(crate) use state::{AdapterState, Attachment, MAX_PENDING_CONTENT_REQUESTS};
pub(crate) use util::{
    compare_attached_generation, compare_generation, compare_generation_allow_next,
    current_attachment_mut,
};
