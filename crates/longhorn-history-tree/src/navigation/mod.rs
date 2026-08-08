//! Checked fork-history navigation plans and execution.

mod error;
mod execute;
mod plan;
mod transaction;
mod types;

pub use error::ForkNavigationError;
pub(crate) use execute::shared_depth;
pub use transaction::{ForkNavigationReceipt, ForkNavigationTransaction};
pub use types::{ForkNavigationPlan, ForkNavigationTarget};
