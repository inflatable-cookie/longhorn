//! Linear history state and validated authority shell.

use std::collections::VecDeque;

use longhorn_core::{HistoryId, HistoryPlanId, HistoryRevision};

use crate::{
    HistoryEntry, HistoryEntrySequence, HistoryLimits, HistoryNavigationLimits,
    HistoryProjectionLimits, HistoryRetainedBaseline,
};

use super::HistoryStateError;
use super::record::validate_state;

/// Structural state for one typed linear history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinearHistoryState<P> {
    pub(crate) history_id: HistoryId,
    pub(crate) revision: HistoryRevision,
    pub(crate) next_sequence: HistoryEntrySequence,
    pub(crate) retained_baseline: HistoryRetainedBaseline,
    pub(crate) applied: Vec<HistoryEntry<P>>,
    pub(crate) future: Vec<HistoryEntry<P>>,
}

impl<P> LinearHistoryState<P> {
    /// Constructs state that [`LinearHistory::from_state`] will validate.
    #[must_use]
    pub const fn new(
        history_id: HistoryId,
        revision: HistoryRevision,
        next_sequence: HistoryEntrySequence,
        applied: Vec<HistoryEntry<P>>,
        future: Vec<HistoryEntry<P>>,
    ) -> Self {
        Self {
            history_id,
            revision,
            next_sequence,
            retained_baseline: HistoryRetainedBaseline::EMPTY,
            applied,
            future,
        }
    }

    /// Constructs state with explicit retained-baseline evidence.
    #[must_use]
    pub const fn with_retained_baseline(
        history_id: HistoryId,
        revision: HistoryRevision,
        next_sequence: HistoryEntrySequence,
        retained_baseline: HistoryRetainedBaseline,
        applied: Vec<HistoryEntry<P>>,
        future: Vec<HistoryEntry<P>>,
    ) -> Self {
        Self {
            history_id,
            revision,
            next_sequence,
            retained_baseline,
            applied,
            future,
        }
    }

    /// Returns the history authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the structural revision.
    #[must_use]
    pub const fn revision(&self) -> HistoryRevision {
        self.revision
    }

    /// Returns the next insertion sequence.
    #[must_use]
    pub const fn next_sequence(&self) -> HistoryEntrySequence {
        self.next_sequence
    }

    /// Returns durable retained-baseline evidence.
    #[must_use]
    pub const fn retained_baseline(&self) -> &HistoryRetainedBaseline {
        &self.retained_baseline
    }

    /// Returns applied entries from oldest to current.
    #[must_use]
    pub fn applied(&self) -> &[HistoryEntry<P>] {
        &self.applied
    }

    /// Returns future entries from farthest to next-redo.
    #[must_use]
    pub fn future(&self) -> &[HistoryEntry<P>] {
        &self.future
    }
}

/// Validated mutable linear history state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinearHistory<P> {
    pub(crate) limits: HistoryLimits,
    pub(crate) navigation_limits: HistoryNavigationLimits,
    pub(crate) projection_limits: HistoryProjectionLimits,
    pub(crate) recent_committed_plan_ids: VecDeque<HistoryPlanId>,
    pub(crate) active_group: Option<crate::HistoryActiveGroup>,
    pub(crate) coalescing_open: bool,
    pub(crate) state: LinearHistoryState<P>,
}

impl<P> LinearHistory<P> {
    /// Constructs an empty history.
    #[must_use]
    pub const fn new(history_id: HistoryId, limits: HistoryLimits) -> Self {
        Self::with_runtime_limits(
            history_id,
            limits,
            HistoryNavigationLimits::DEFAULT,
            HistoryProjectionLimits::DEFAULT,
        )
    }

    /// Constructs an empty history with explicit navigation limits.
    #[must_use]
    pub const fn with_navigation_limits(
        history_id: HistoryId,
        limits: HistoryLimits,
        navigation_limits: HistoryNavigationLimits,
    ) -> Self {
        Self::with_runtime_limits(
            history_id,
            limits,
            navigation_limits,
            HistoryProjectionLimits::DEFAULT,
        )
    }

    /// Constructs an empty history with all pure runtime limits explicit.
    #[must_use]
    pub const fn with_runtime_limits(
        history_id: HistoryId,
        limits: HistoryLimits,
        navigation_limits: HistoryNavigationLimits,
        projection_limits: HistoryProjectionLimits,
    ) -> Self {
        Self {
            limits,
            navigation_limits,
            projection_limits,
            recent_committed_plan_ids: VecDeque::new(),
            active_group: None,
            coalescing_open: false,
            state: LinearHistoryState {
                history_id,
                revision: HistoryRevision::INITIAL,
                next_sequence: HistoryEntrySequence::FIRST,
                retained_baseline: HistoryRetainedBaseline::EMPTY,
                applied: Vec::new(),
                future: Vec::new(),
            },
        }
    }

    /// Validates and accepts decoded structural state.
    pub fn from_state(
        limits: HistoryLimits,
        state: LinearHistoryState<P>,
    ) -> Result<Self, HistoryStateError> {
        Self::from_state_with_navigation_limits(limits, HistoryNavigationLimits::DEFAULT, state)
    }

    /// Validates decoded state with explicit transient navigation limits.
    pub fn from_state_with_navigation_limits(
        limits: HistoryLimits,
        navigation_limits: HistoryNavigationLimits,
        state: LinearHistoryState<P>,
    ) -> Result<Self, HistoryStateError> {
        Self::from_state_with_runtime_limits(
            limits,
            navigation_limits,
            HistoryProjectionLimits::DEFAULT,
            state,
        )
    }

    /// Validates decoded state with all transient runtime limits explicit.
    pub fn from_state_with_runtime_limits(
        limits: HistoryLimits,
        navigation_limits: HistoryNavigationLimits,
        projection_limits: HistoryProjectionLimits,
        state: LinearHistoryState<P>,
    ) -> Result<Self, HistoryStateError> {
        validate_state(limits, &state)?;
        Ok(Self {
            limits,
            navigation_limits,
            projection_limits,
            recent_committed_plan_ids: VecDeque::new(),
            active_group: None,
            coalescing_open: false,
            state,
        })
    }

    /// Consumes the authority and returns its structural state.
    #[must_use]
    pub fn into_state(self) -> LinearHistoryState<P> {
        self.state
    }

    /// Returns configured limits.
    #[must_use]
    pub const fn limits(&self) -> HistoryLimits {
        self.limits
    }

    /// Returns transient navigation and duplicate-plan limits.
    #[must_use]
    pub const fn navigation_limits(&self) -> HistoryNavigationLimits {
        self.navigation_limits
    }

    /// Returns configured projection limits.
    #[must_use]
    pub const fn projection_limits(&self) -> HistoryProjectionLimits {
        self.projection_limits
    }

    /// Returns the history authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.state.history_id
    }

    /// Returns the current structural revision.
    #[must_use]
    pub const fn revision(&self) -> HistoryRevision {
        self.state.revision
    }

    /// Returns the next insertion sequence.
    #[must_use]
    pub const fn next_sequence(&self) -> HistoryEntrySequence {
        self.state.next_sequence
    }

    /// Returns durable retained-baseline evidence.
    #[must_use]
    pub const fn retained_baseline(&self) -> &HistoryRetainedBaseline {
        &self.state.retained_baseline
    }

    /// Returns applied entries from oldest to current.
    #[must_use]
    pub fn applied(&self) -> &[HistoryEntry<P>] {
        &self.state.applied
    }

    /// Returns future entries from farthest to next-redo.
    #[must_use]
    pub fn future(&self) -> &[HistoryEntry<P>] {
        &self.state.future
    }

    /// Returns the current entry.
    #[must_use]
    pub fn current(&self) -> Option<&HistoryEntry<P>> {
        self.state.applied.last()
    }

    /// Returns the next entry that undo navigation would target.
    #[must_use]
    pub fn next_undo(&self) -> Option<&HistoryEntry<P>> {
        self.state.applied.last()
    }

    /// Returns the next entry that redo navigation would apply.
    #[must_use]
    pub fn next_redo(&self) -> Option<&HistoryEntry<P>> {
        self.state.future.last()
    }
}
