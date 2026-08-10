use std::{error::Error, fmt};

use longhorn_config::{DebounceStrategy, DomainIssue};
use longhorn_surfaces::{
    LayoutDefinitionRegistry, LayoutMutationCommand, LayoutMutationEngine, LayoutMutationRequest,
    SurfaceDocument,
};

use crate::layout_mutation::rejection_issue;

/// One or more ordered sizing/collapse requests staged for a single flush.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutPresentationIntent {
    requests: Vec<LayoutMutationRequest>,
}

impl LayoutPresentationIntent {
    /// Starts an intent from one sizing or collapse request.
    pub fn new(request: LayoutMutationRequest) -> Result<Self, LayoutPresentationIntentError> {
        if !is_presentation(request.command()) {
            return Err(LayoutPresentationIntentError::StructuralCommand);
        }
        Ok(Self {
            requests: vec![request],
        })
    }

    /// Returns the complete ordered pending request sequence.
    #[must_use]
    pub fn requests(&self) -> &[LayoutMutationRequest] {
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

/// Ordered presentation intent strategy over fresh authoritative state.
#[derive(Clone, Copy, Debug)]
pub struct LayoutPresentationStrategy<'registry> {
    registry: &'registry LayoutDefinitionRegistry,
}

impl<'registry> LayoutPresentationStrategy<'registry> {
    /// Binds presentation mutation to one immutable definition registry.
    #[must_use]
    pub const fn new(registry: &'registry LayoutDefinitionRegistry) -> Self {
        Self { registry }
    }
}

impl<D> DebounceStrategy<D> for LayoutPresentationStrategy<'_>
where
    D: longhorn_config::ConfigDomain<Value = SurfaceDocument>,
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

    fn apply(&self, intent: &Self::Intent, value: &mut D::Value) -> Result<(), DomainIssue> {
        let engine = LayoutMutationEngine::new(self.registry);
        let mut candidate = value.clone();
        for request in &intent.requests {
            let receipt = engine
                .apply(&candidate, request)
                .map_err(|rejection| rejection_issue(&rejection))?;
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
