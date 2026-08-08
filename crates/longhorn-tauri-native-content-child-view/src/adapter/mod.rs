//! Child-view execution adapter.

mod api;
mod execute;
mod receipt;
mod state;
mod util;

pub use receipt::{
    ChildViewHostDestroyOutcome, ChildViewHostDestroyReceipt, ChildViewNavigationOutcome,
    ChildViewNavigationReceipt, ChildViewTeardownOutcome, ChildViewTeardownReceipt,
};
pub use state::ChildViewAdapter;
pub(crate) use state::{AdapterState, Attachment};
pub(crate) use util::{
    compare_attached_generation, compare_generation, compare_generation_allow_next,
    current_attachment_mut,
};
