//! Native-content coordination authority.

mod api;
mod receipt;
mod support;

pub use api::NativeContentCoordinator;
pub use receipt::{
    DesiredUpdateReceipt, HostDestroyOutcome, HostDestroyReceipt, ObservationReceipt,
};
pub(crate) use support::{
    compare_generation, legal_transition, require_revision, validate_desired_generation,
    validate_observation_capabilities,
};
