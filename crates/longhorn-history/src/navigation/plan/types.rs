//! Immutable typed navigation plan.

use longhorn_core::{HistoryId, HistoryPlanId, HistoryRevision};

use crate::{
    HistoryNavigationDirection, HistoryNavigationPosition, HistoryNavigationStep,
    HistoryNavigationTarget,
};

/// Immutable typed navigation plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryNavigationPlan<P> {
    pub(crate) history_id: HistoryId,
    pub(crate) plan_id: HistoryPlanId,
    pub(crate) source_revision: HistoryRevision,
    pub(crate) target: HistoryNavigationTarget,
    pub(crate) direction: HistoryNavigationDirection,
    pub(crate) source_position: HistoryNavigationPosition,
    pub(crate) target_position: HistoryNavigationPosition,
    pub(crate) steps: Vec<HistoryNavigationStep<P>>,
}

impl<P> HistoryNavigationPlan<P> {
    /// Returns the owning history authority.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the injected plan identity.
    #[must_use]
    pub const fn plan_id(&self) -> &HistoryPlanId {
        &self.plan_id
    }

    /// Returns the exact source revision.
    #[must_use]
    pub const fn source_revision(&self) -> HistoryRevision {
        self.source_revision
    }

    /// Returns the stable requested target.
    #[must_use]
    pub const fn target(&self) -> &HistoryNavigationTarget {
        &self.target
    }

    /// Returns the planned direction.
    #[must_use]
    pub const fn direction(&self) -> HistoryNavigationDirection {
        self.direction
    }

    /// Returns the exact source position.
    #[must_use]
    pub const fn source_position(&self) -> &HistoryNavigationPosition {
        &self.source_position
    }

    /// Returns the exact target position.
    #[must_use]
    pub const fn target_position(&self) -> &HistoryNavigationPosition {
        &self.target_position
    }

    /// Returns the complete ordered typed payload batch.
    #[must_use]
    pub fn steps(&self) -> &[HistoryNavigationStep<P>] {
        &self.steps
    }
}
