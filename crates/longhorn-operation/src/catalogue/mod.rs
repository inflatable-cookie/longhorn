//! Validated mutable authority for one finite operation catalogue.

mod model;
mod mutate;
mod project;
mod support;

#[cfg(test)]
mod tests;

pub use model::OperationCatalogue;
pub(crate) use support::{
    next_operation_revision, prune_terminal, terminal_weight, validate_progress, validate_teardown,
};
