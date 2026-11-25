use std::{error::Error, fmt};

use longhorn_config::{
    ConfigStore, DebounceClock, DebouncePolicy, DebounceStrategy, DebouncedMutation, DomainIssue,
    StoreError,
};
use longhorn_core::LayoutRequestId;
use longhorn_surfaces::{
    LayoutDefinitionRegistry, LayoutMutationCommand, LayoutMutationEngine, LayoutMutationRequest,
};

use crate::{RegisteredSurfaceDomain, SurfaceMigration, mutation::layout_rejection_issue};

/// One sizing or collapse command staged in a presentation intent.
///
/// It names no expected revision. A presentation intent applies to whatever
/// value the flush reads under the store guard, so a structural publish that
/// lands while the intent is pending cannot strand it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutPresentationRequest {
    request_id: LayoutRequestId,
    command: LayoutMutationCommand,
}

impl LayoutPresentationRequest {
    /// Returns the request identity recorded on the receipt.
    #[must_use]
    pub const fn request_id(&self) -> &LayoutRequestId {
        &self.request_id
    }

    /// Returns the sizing or collapse command.
    #[must_use]
    pub const fn command(&self) -> &LayoutMutationCommand {
        &self.command
    }
}

/// Ordered sizing/collapse commands staged for a single flush.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutPresentationIntent {
    requests: Vec<LayoutPresentationRequest>,
}

impl LayoutPresentationIntent {
    /// Starts an intent from one sizing or collapse command.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutPresentationIntentError::StructuralCommand`] for any
    /// command other than a sizing or collapse change.
    pub fn new(
        request_id: LayoutRequestId,
        command: LayoutMutationCommand,
    ) -> Result<Self, LayoutPresentationIntentError> {
        if !is_presentation(&command) {
            return Err(LayoutPresentationIntentError::StructuralCommand);
        }
        Ok(Self {
            requests: vec![LayoutPresentationRequest {
                request_id,
                command,
            }],
        })
    }

    /// Returns the complete ordered pending command sequence.
    #[must_use]
    pub fn requests(&self) -> &[LayoutPresentationRequest] {
        self.requests.as_slice()
    }
}

/// A structural request was offered to the presentation-only debounce lane.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutPresentationIntentError {
    /// Only sizing and collapse state may be debounced.
    StructuralCommand,
}

impl fmt::Display for LayoutPresentationIntentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("only sizing and collapse layout commands may be debounced")
    }
}

impl Error for LayoutPresentationIntentError {}

/// Ordered presentation intent strategy bound to one registered Surface domain's registry.
///
/// Construct a lane through [`debounced_layout_presentation`] so the strategy
/// always uses the exact [`LayoutDefinitionRegistry`] owned by the domain being
/// flushed.
#[derive(Clone, Copy, Debug)]
pub struct LayoutPresentationStrategy<'registry> {
    registry: &'registry LayoutDefinitionRegistry,
}

/// Creates a presentation-only debounce lane bound to `domain`'s definition registry.
///
/// Deriving the strategy's registry directly from `domain` prevents a debounced
/// sizing or collapse flush from normalizing against a different registry than
/// the domain being published.
///
/// # Errors
///
/// Returns [`StoreError`] if `domain` is not registered with `store` or its
/// identity does not match the registered descriptor.
pub fn debounced_layout_presentation<'config, M, C>(
    store: &'config ConfigStore,
    domain: &'config RegisteredSurfaceDomain<M>,
    clock: C,
    policy: DebouncePolicy,
) -> Result<
    DebouncedMutation<'config, RegisteredSurfaceDomain<M>, LayoutPresentationStrategy<'config>, C>,
    StoreError,
>
where
    M: SurfaceMigration,
    C: DebounceClock,
{
    DebouncedMutation::new(
        store,
        domain,
        LayoutPresentationStrategy {
            registry: domain.registry(),
        },
        clock,
        policy,
    )
}

impl<M> DebounceStrategy<RegisteredSurfaceDomain<M>> for LayoutPresentationStrategy<'_>
where
    M: SurfaceMigration,
{
    type Intent = LayoutPresentationIntent;

    fn coalesce(
        &self,
        previous: &Self::Intent,
        next: Self::Intent,
    ) -> Result<Self::Intent, DomainIssue> {
        let mut requests = previous.requests.clone();
        requests.extend(next.requests);
        Ok(LayoutPresentationIntent { requests })
    }

    fn apply(
        &self,
        intent: &Self::Intent,
        value: &mut longhorn_surfaces::SurfaceDocument,
    ) -> Result<(), DomainIssue> {
        let engine = LayoutMutationEngine::new(self.registry);
        let mut candidate = value.clone();
        for staged in &intent.requests {
            // Each command is bound to the revision the flush is reading, not to
            // one staged earlier: the store guard makes that read current.
            let request = LayoutMutationRequest::new(
                staged.request_id().clone(),
                candidate.revision(),
                staged.command().clone(),
            );
            let receipt = engine
                .apply(&candidate, &request)
                .map_err(|rejection| layout_rejection_issue(&rejection))?;
            candidate = receipt.authoritative_document().clone();
        }
        *value = candidate;
        Ok(())
    }

    fn pending_weight(&self, intent: &Self::Intent) -> usize {
        intent.requests.len()
    }
}

fn is_presentation(command: &LayoutMutationCommand) -> bool {
    matches!(
        command,
        LayoutMutationCommand::SetSizingSlot { .. }
            | LayoutMutationCommand::SetRegionCollapsed { .. }
    )
}
