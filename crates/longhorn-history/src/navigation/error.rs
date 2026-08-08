//! Navigation planning and execution failures.

use std::{error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryPlanId, HistoryRevision};

/// Failed navigation planning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryNavigationPlanningError<E> {
    /// The request did not target current history.
    StaleRevision {
        /// Requested revision.
        expected: HistoryRevision,
        /// Current revision.
        actual: HistoryRevision,
    },
    /// A recently committed plan id was reused.
    DuplicatePlanId(HistoryPlanId),
    /// No applied entry can be undone.
    NothingToUndo,
    /// No future entry can be redone.
    NothingToRedo,
    /// Checkout named no retained entry.
    UnknownEntry(HistoryEntryId),
    /// The requested route exceeded the configured batch bound.
    TooManySteps {
        /// Configured maximum.
        maximum: usize,
        /// Required steps.
        actual: usize,
    },
    /// The history revision cannot advance.
    RevisionOverflow,
    /// Consumer inverse policy rejected one entry.
    Policy {
        /// Entry whose inverse failed.
        entry_id: HistoryEntryId,
        /// Consumer failure.
        error: E,
    },
}

impl<E: fmt::Display> fmt::Display for HistoryNavigationPlanningError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleRevision { expected, actual } => write!(
                formatter,
                "expected history revision {}; current revision is {}",
                expected.get(),
                actual.get()
            ),
            Self::DuplicatePlanId(plan_id) => {
                write!(
                    formatter,
                    "history plan id {plan_id} was recently committed"
                )
            }
            Self::NothingToUndo => formatter.write_str("history has no applied entry to undo"),
            Self::NothingToRedo => formatter.write_str("history has no future entry to redo"),
            Self::UnknownEntry(entry_id) => {
                write!(formatter, "history entry {entry_id} is not retained")
            }
            Self::TooManySteps { maximum, actual } => write!(
                formatter,
                "history navigation requires {actual} steps; maximum is {maximum}"
            ),
            Self::RevisionOverflow => formatter.write_str("history revision cannot advance"),
            Self::Policy { entry_id, error } => {
                write!(formatter, "history inverse for {entry_id} failed: {error}")
            }
        }
    }
}

impl<E> Error for HistoryNavigationPlanningError<E> where E: Error + 'static {}

/// Structural rejection before consumer product apply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryNavigationRejection {
    /// The plan identity was already committed recently.
    DuplicatePlan,
    /// The plan belongs to another history authority.
    ForeignHistory,
    /// The plan source revision is no longer current.
    StaleRevision {
        /// Planned source revision.
        expected: HistoryRevision,
        /// Current revision.
        actual: HistoryRevision,
    },
    /// Current applied/future position differs from the plan source.
    SourcePositionChanged,
    /// Direction, target, steps, or entry identities are inconsistent.
    InvalidPlan,
    /// The route exceeds the current configured batch bound.
    TooManySteps {
        /// Current maximum.
        maximum: usize,
        /// Planned steps.
        actual: usize,
    },
    /// The history revision cannot advance.
    RevisionOverflow,
}

impl fmt::Display for HistoryNavigationRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePlan => formatter.write_str("history plan was already committed"),
            Self::ForeignHistory => formatter.write_str("history plan belongs to another history"),
            Self::StaleRevision { expected, actual } => write!(
                formatter,
                "history plan revision {} is stale; current revision is {}",
                expected.get(),
                actual.get()
            ),
            Self::SourcePositionChanged => {
                formatter.write_str("history plan source position changed")
            }
            Self::InvalidPlan => formatter.write_str("history plan is structurally invalid"),
            Self::TooManySteps { maximum, actual } => write!(
                formatter,
                "history plan has {actual} steps; current maximum is {maximum}"
            ),
            Self::RevisionOverflow => formatter.write_str("history revision cannot advance"),
        }
    }
}

impl Error for HistoryNavigationRejection {}

/// Failed checked navigation execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryNavigationExecutionError<E> {
    /// Plan validation failed before product apply.
    Rejected {
        /// Rejected plan identity.
        plan_id: HistoryPlanId,
        /// Structural rejection.
        rejection: HistoryNavigationRejection,
    },
    /// Product apply failed and exact rollback succeeded.
    RolledBack {
        /// Plan identity.
        plan_id: HistoryPlanId,
        /// Product apply failure.
        error: E,
    },
    /// Product apply and exact rollback both failed.
    RollbackFailed {
        /// Plan identity.
        plan_id: HistoryPlanId,
        /// Product apply failure.
        error: E,
        /// Rollback failure.
        rollback_error: E,
    },
}

impl<E: fmt::Display> fmt::Display for HistoryNavigationExecutionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected { plan_id, rejection } => {
                write!(formatter, "history plan {plan_id} rejected: {rejection}")
            }
            Self::RolledBack { plan_id, error } => {
                write!(formatter, "history plan {plan_id} rolled back: {error}")
            }
            Self::RollbackFailed {
                plan_id,
                error,
                rollback_error,
            } => write!(
                formatter,
                "history plan {plan_id} failed: {error}; rollback failed: {rollback_error}"
            ),
        }
    }
}

impl<E> Error for HistoryNavigationExecutionError<E> where E: Error + 'static {}
