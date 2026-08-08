//! Validated mutable authority for one finite retained notification ledger.

mod model;
mod mutate;
mod support;

#[cfg(test)]
mod tests;

pub use model::NotificationLedger;
pub(crate) use support::{
    encoded_weight, increment_pruned_count, prune_to_limits, unseen_count, validate_clear_targets,
};
