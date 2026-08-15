//! Child-view execution adapter.

mod api;
mod execute;
mod receipt;
mod state;
mod util;

pub(crate) use longhorn_native_content::{compare_attached_generation, compare_generation};
pub use receipt::{
    ChildViewHostDestroyOutcome, ChildViewHostDestroyReceipt, ChildViewNavigationOutcome,
    ChildViewNavigationReceipt, ChildViewTeardownOutcome, ChildViewTeardownReceipt,
};
pub use state::ChildViewAdapter;
pub(crate) use state::{AdapterState, Attachment};
pub(crate) use util::current_attachment_mut;
