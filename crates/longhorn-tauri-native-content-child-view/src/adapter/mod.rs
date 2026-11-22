//! Child-view execution adapter.

mod api;
mod execute;
mod receipt;
mod state;

pub(crate) use longhorn_native_content::{compare_attached_generation, compare_generation};
pub use receipt::{
    ChildViewHostDestroyOutcome, ChildViewHostDestroyReceipt, ChildViewNavigationOutcome,
    ChildViewNavigationReceipt, ChildViewTeardownOutcome, ChildViewTeardownReceipt,
};
pub(crate) use state::Attachment;
pub use state::ChildViewAdapter;
