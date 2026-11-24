//! Isolated-window execution adapter.

mod api;
mod execute;
mod state;

pub(crate) use longhorn_native_content::{compare_attached_generation, compare_generation};
pub use state::{AdmittedIsolatedContentRequest, IsolatedWindowAdapter};
pub(crate) use state::{Attachment, MAX_PENDING_CONTENT_REQUESTS};
