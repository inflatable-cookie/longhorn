use longhorn_core::{
    OperationAuthorityId, OperationCatalogueRevision, OperationId, OperationKindId,
    OperationRevision, OperationScopeId,
};

use crate::{
    OperationAuthorityEpoch, OperationCancellationSupport, OperationLabel, OperationProgress,
    OperationSequence,
};

/// Closed product-neutral lifecycle for one asynchronous operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationState {
    /// Accepted work has not started.
    Queued,
    /// The consumer executor reports active work.
    Running,
    /// Cancellation was requested; the terminal fact is still unknown.
    Cancelling,
    /// The executor reports successful completion.
    Succeeded,
    /// The executor reports failed completion.
    Failed,
    /// The executor confirms work stopped without completion.
    Cancelled,
    /// The consumer confirms loss of the executor or host.
    Interrupted,
}

impl OperationState {
    /// Returns whether the state is immutable and terminal.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Interrupted
        )
    }

    /// Returns whether this state can be selected during registration.
    #[must_use]
    pub const fn is_initial(self) -> bool {
        matches!(self, Self::Queued | Self::Running)
    }

    pub(crate) const fn can_transition_to(self, next: Self) -> bool {
        match self {
            Self::Queued => matches!(
                next,
                Self::Running | Self::Failed | Self::Cancelled | Self::Interrupted
            ),
            Self::Running => matches!(
                next,
                Self::Cancelling
                    | Self::Succeeded
                    | Self::Failed
                    | Self::Cancelled
                    | Self::Interrupted
            ),
            Self::Cancelling => matches!(
                next,
                Self::Succeeded | Self::Failed | Self::Cancelled | Self::Interrupted
            ),
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Interrupted => false,
        }
    }
}

/// Exact identity of one live operation authority instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationAuthorityCursor {
    authority_id: OperationAuthorityId,
    authority_epoch: OperationAuthorityEpoch,
}

impl OperationAuthorityCursor {
    /// Constructs an authority cursor.
    #[must_use]
    pub const fn new(
        authority_id: OperationAuthorityId,
        authority_epoch: OperationAuthorityEpoch,
    ) -> Self {
        Self {
            authority_id,
            authority_epoch,
        }
    }

    /// Returns the stable authority identity.
    #[must_use]
    pub const fn authority_id(&self) -> &OperationAuthorityId {
        &self.authority_id
    }

    /// Returns the live authority epoch.
    #[must_use]
    pub const fn authority_epoch(&self) -> OperationAuthorityEpoch {
        self.authority_epoch
    }
}

/// Authoritative payload-free record for one operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationRecord {
    authority: OperationAuthorityCursor,
    operation_id: OperationId,
    kind_id: OperationKindId,
    scope_id: Option<OperationScopeId>,
    label: OperationLabel,
    cancellation_support: OperationCancellationSupport,
    retry_of: Option<OperationId>,
    sequence: OperationSequence,
    revision: OperationRevision,
    last_changed_catalogue_revision: OperationCatalogueRevision,
    state: OperationState,
    progress: OperationProgress,
}

impl OperationRecord {
    /// Returns the authority cursor that owns this record.
    #[must_use]
    pub const fn authority(&self) -> &OperationAuthorityCursor {
        &self.authority
    }

    /// Returns the stable operation identity.
    #[must_use]
    pub const fn operation_id(&self) -> &OperationId {
        &self.operation_id
    }

    /// Returns the consumer-owned operation kind.
    #[must_use]
    pub const fn kind_id(&self) -> &OperationKindId {
        &self.kind_id
    }

    /// Returns the optional consumer-owned scope.
    #[must_use]
    pub const fn scope_id(&self) -> Option<&OperationScopeId> {
        self.scope_id.as_ref()
    }

    /// Returns the bounded presentation label.
    #[must_use]
    pub const fn label(&self) -> &OperationLabel {
        &self.label
    }

    /// Returns whether the consumer executor accepts cancellation.
    #[must_use]
    pub const fn cancellation_support(&self) -> OperationCancellationSupport {
        self.cancellation_support
    }

    /// Returns the retained terminal operation this retry descends from.
    #[must_use]
    pub const fn retry_of(&self) -> Option<&OperationId> {
        self.retry_of.as_ref()
    }

    /// Returns the insertion sequence.
    #[must_use]
    pub const fn sequence(&self) -> OperationSequence {
        self.sequence
    }

    /// Returns the current operation revision.
    #[must_use]
    pub const fn revision(&self) -> OperationRevision {
        self.revision
    }

    /// Returns the catalogue revision that last changed this record.
    #[must_use]
    pub const fn last_changed_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.last_changed_catalogue_revision
    }

    /// Returns the current lifecycle state.
    #[must_use]
    pub const fn state(&self) -> OperationState {
        self.state
    }

    /// Returns current bounded progress.
    #[must_use]
    pub const fn progress(&self) -> &OperationProgress {
        &self.progress
    }

    /// Returns whether the record remains active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        !self.state.is_terminal()
    }

    /// Returns canonical structural metadata weight used by retention.
    #[must_use]
    pub fn encoded_metadata_weight(&self) -> u64 {
        let string_bytes = self.authority.authority_id().as_str().len()
            + self.operation_id.as_str().len()
            + self.kind_id.as_str().len()
            + self.scope_id.as_ref().map_or(0, |id| id.as_str().len())
            + self.label.as_str().len()
            + self.retry_of.as_ref().map_or(0, |id| id.as_str().len())
            + self.progress.phase().map_or(0, |phase| {
                phase.phase_id().as_str().len() + phase.label().as_str().len()
            });

        // Fixed-width numbers, enum/option tags, and progress numeric fields.
        80 + u64::try_from(string_bytes).expect("bounded metadata length fits u64")
    }

    pub(crate) fn registered(
        authority: OperationAuthorityCursor,
        request: OperationRegistration,
        sequence: OperationSequence,
        catalogue_revision: OperationCatalogueRevision,
    ) -> Self {
        Self {
            authority,
            operation_id: request.operation_id,
            kind_id: request.kind_id,
            scope_id: request.scope_id,
            label: request.label,
            cancellation_support: request.cancellation_support,
            retry_of: request.retry_of,
            sequence,
            revision: OperationRevision::INITIAL,
            last_changed_catalogue_revision: catalogue_revision,
            state: request.initial_state,
            progress: OperationProgress::INITIAL,
        }
    }

    pub(crate) fn commit_transition(
        &mut self,
        revision: OperationRevision,
        catalogue_revision: OperationCatalogueRevision,
        state: OperationState,
    ) {
        self.revision = revision;
        self.last_changed_catalogue_revision = catalogue_revision;
        self.state = state;
    }

    pub(crate) fn commit_progress(
        &mut self,
        revision: OperationRevision,
        catalogue_revision: OperationCatalogueRevision,
        progress: OperationProgress,
    ) {
        self.revision = revision;
        self.last_changed_catalogue_revision = catalogue_revision;
        self.progress = progress;
    }

    #[cfg(test)]
    pub(crate) fn set_revision_for_test(&mut self, revision: OperationRevision) {
        self.revision = revision;
    }
}

/// Consumer-decided registration of queued or already-running work.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationRegistration {
    pub(crate) authority: OperationAuthorityCursor,
    pub(crate) expected_catalogue_revision: OperationCatalogueRevision,
    pub(crate) operation_id: OperationId,
    pub(crate) kind_id: OperationKindId,
    pub(crate) scope_id: Option<OperationScopeId>,
    pub(crate) label: OperationLabel,
    pub(crate) initial_state: OperationState,
    pub(crate) cancellation_support: OperationCancellationSupport,
    pub(crate) retry_of: Option<OperationId>,
}

impl OperationRegistration {
    /// Constructs a registration request.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        authority: OperationAuthorityCursor,
        expected_catalogue_revision: OperationCatalogueRevision,
        operation_id: OperationId,
        kind_id: OperationKindId,
        scope_id: Option<OperationScopeId>,
        label: OperationLabel,
        initial_state: OperationState,
        cancellation_support: OperationCancellationSupport,
        retry_of: Option<OperationId>,
    ) -> Self {
        Self {
            authority,
            expected_catalogue_revision,
            operation_id,
            kind_id,
            scope_id,
            label,
            initial_state,
            cancellation_support,
            retry_of,
        }
    }
}

/// Revision-bound request for one lifecycle transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationTransition {
    pub(crate) authority: OperationAuthorityCursor,
    pub(crate) operation_id: OperationId,
    pub(crate) expected_operation_revision: OperationRevision,
    pub(crate) next_state: OperationState,
}

impl OperationTransition {
    /// Constructs a lifecycle transition request.
    #[must_use]
    pub const fn new(
        authority: OperationAuthorityCursor,
        operation_id: OperationId,
        expected_operation_revision: OperationRevision,
        next_state: OperationState,
    ) -> Self {
        Self {
            authority,
            operation_id,
            expected_operation_revision,
            next_state,
        }
    }
}

/// Receipt for one committed registration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationRegistrationReceipt {
    pub(crate) operation: OperationRecord,
    pub(crate) previous_catalogue_revision: OperationCatalogueRevision,
    pub(crate) committed_catalogue_revision: OperationCatalogueRevision,
}

impl OperationRegistrationReceipt {
    /// Returns the registered operation.
    #[must_use]
    pub const fn operation(&self) -> &OperationRecord {
        &self.operation
    }

    /// Returns the catalogue revision before registration.
    #[must_use]
    pub const fn previous_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.previous_catalogue_revision
    }

    /// Returns the committed catalogue revision.
    #[must_use]
    pub const fn committed_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.committed_catalogue_revision
    }
}

/// Receipt for one committed lifecycle transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationTransitionReceipt {
    pub(crate) operation_id: OperationId,
    pub(crate) previous_state: OperationState,
    pub(crate) committed_state: OperationState,
    pub(crate) previous_operation_revision: OperationRevision,
    pub(crate) committed_operation_revision: OperationRevision,
    pub(crate) previous_catalogue_revision: OperationCatalogueRevision,
    pub(crate) committed_catalogue_revision: OperationCatalogueRevision,
    pub(crate) evicted: Vec<crate::OperationRemoval>,
}

impl OperationTransitionReceipt {
    /// Returns the changed operation identity.
    #[must_use]
    pub const fn operation_id(&self) -> &OperationId {
        &self.operation_id
    }
    /// Returns the state before the transition.
    #[must_use]
    pub const fn previous_state(&self) -> OperationState {
        self.previous_state
    }
    /// Returns the committed state.
    #[must_use]
    pub const fn committed_state(&self) -> OperationState {
        self.committed_state
    }
    /// Returns the operation revision before the transition.
    #[must_use]
    pub const fn previous_operation_revision(&self) -> OperationRevision {
        self.previous_operation_revision
    }
    /// Returns the committed operation revision.
    #[must_use]
    pub const fn committed_operation_revision(&self) -> OperationRevision {
        self.committed_operation_revision
    }
    /// Returns the catalogue revision before the transition.
    #[must_use]
    pub const fn previous_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.previous_catalogue_revision
    }
    /// Returns the committed catalogue revision.
    #[must_use]
    pub const fn committed_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.committed_catalogue_revision
    }
    /// Returns terminal records evicted by this transition.
    #[must_use]
    pub fn evicted(&self) -> &[crate::OperationRemoval] {
        &self.evicted
    }
}

/// Authoritative active and terminal operation projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationCatalogueProjection {
    pub(crate) authority: OperationAuthorityCursor,
    pub(crate) catalogue_revision: OperationCatalogueRevision,
    pub(crate) terminal_eviction_count: u64,
    pub(crate) closed: bool,
    pub(crate) active: Vec<OperationRecord>,
    pub(crate) recent: Vec<OperationRecord>,
}

impl OperationCatalogueProjection {
    /// Returns the exact authority cursor.
    #[must_use]
    pub const fn authority(&self) -> &OperationAuthorityCursor {
        &self.authority
    }
    /// Returns the authoritative catalogue revision.
    #[must_use]
    pub const fn catalogue_revision(&self) -> OperationCatalogueRevision {
        self.catalogue_revision
    }
    /// Returns cumulative terminal records removed by finite retention.
    #[must_use]
    pub const fn terminal_eviction_count(&self) -> u64 {
        self.terminal_eviction_count
    }
    /// Returns whether controlled teardown closed this authority.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }
    /// Returns active records in insertion order.
    #[must_use]
    pub fn active(&self) -> &[OperationRecord] {
        &self.active
    }
    /// Returns terminal records newest first.
    #[must_use]
    pub fn recent(&self) -> &[OperationRecord] {
        &self.recent
    }
}
