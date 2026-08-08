//! Checked linear-history navigation plans and execution.

mod error;
mod execute;
mod plan;
mod types;

#[cfg(test)]
mod tests;

pub use error::{
    HistoryNavigationExecutionError, HistoryNavigationPlanningError, HistoryNavigationRejection,
};
pub use execute::{
    HistoryNavigationReceipt, HistoryNavigationTransaction, HistoryNavigationTransactionFailure,
};
pub use plan::HistoryNavigationPlan;
pub use types::{
    HistoryNavigationDirection, HistoryNavigationPosition, HistoryNavigationRequest,
    HistoryNavigationStep, HistoryNavigationTarget,
};
