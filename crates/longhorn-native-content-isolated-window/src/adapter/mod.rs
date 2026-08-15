//! Isolated-window execution adapter.

mod api;
mod execute;
mod state;
mod util;

pub(crate) use longhorn_native_content::{compare_attached_generation, compare_generation};
pub use state::IsolatedWindowAdapter;
pub(crate) use state::{AdapterState, Attachment, MAX_PENDING_CONTENT_REQUESTS};
pub(crate) use util::current_attachment_mut;
