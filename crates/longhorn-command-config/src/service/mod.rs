//! Coordinated command keymap load, preview, commit, and reset authority.

mod api;
mod error;
mod internal;
mod keymap_load;
mod support;

pub use api::CommandKeymapService;
pub use error::CommandKeymapServiceError;
pub(crate) use support::{
    AcceptedCommit, CommitAbort, Proposal, apply_patch, rejection, snapshot_from_effective,
    validate_patch,
};
