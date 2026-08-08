//! Protocol execution over OperationCatalogue.

mod api;
mod mutation;
mod project;
mod reject;

pub(crate) use mutation::execute_mutation;
pub(crate) use project::{project_teardown_outcome, project_teardown_resolution};
