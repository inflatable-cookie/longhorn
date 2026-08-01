use longhorn_core::{ClientSize, NativeContentFailureCode, NativeContentRevision};
use serde::{Deserialize, Serialize};

use crate::{AttachGeneration, CoordinationError, DesiredState};

/// One mechanism-originated proposal for a new semantic content size.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ContentSizeProposal {
    generation: AttachGeneration,
    desired_revision: NativeContentRevision,
    size: ClientSize,
}

impl ContentSizeProposal {
    /// Constructs a revision- and generation-bound content-size proposal.
    #[must_use]
    pub const fn new(
        generation: AttachGeneration,
        desired_revision: NativeContentRevision,
        size: ClientSize,
    ) -> Self {
        Self {
            generation,
            desired_revision,
            size,
        }
    }

    /// Returns the proposal attach generation.
    #[must_use]
    pub const fn generation(self) -> AttachGeneration {
        self.generation
    }
    /// Returns the desired revision used by the proposer.
    #[must_use]
    pub const fn desired_revision(self) -> NativeContentRevision {
        self.desired_revision
    }
    /// Returns proposed semantic content size.
    #[must_use]
    pub const fn size(self) -> ClientSize {
        self.size
    }
}

/// Consumer decision for one content-size proposal.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ContentSizeDecision {
    /// Accept the requested size exactly.
    Accepted,
    /// Accept a consumer-constrained semantic size.
    Constrained {
        /// Consumer-authorized replacement size.
        size: ClientSize,
    },
    /// Reject without changing desired state.
    Rejected {
        /// Stable consumer-owned rejection category.
        code: NativeContentFailureCode,
    },
}

/// Exact non-mutating decision evidence for a content-size proposal.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ContentSizeProposalReceipt {
    proposal: ContentSizeProposal,
    decision: ContentSizeDecision,
    accepted_size: Option<ClientSize>,
}

impl ContentSizeProposalReceipt {
    /// Returns original mechanism proposal.
    #[must_use]
    pub const fn proposal(&self) -> ContentSizeProposal {
        self.proposal
    }
    /// Returns consumer decision.
    #[must_use]
    pub const fn decision(&self) -> &ContentSizeDecision {
        &self.decision
    }
    /// Returns the semantic size authorized for a later desired update.
    #[must_use]
    pub const fn accepted_size(&self) -> Option<ClientSize> {
        self.accepted_size
    }
}

pub(crate) fn decide_content_size(
    desired: &DesiredState,
    proposal: ContentSizeProposal,
    decision: ContentSizeDecision,
) -> Result<ContentSizeProposalReceipt, CoordinationError> {
    validate_content_size_proposal(desired, proposal)?;

    let accepted_size = match &decision {
        ContentSizeDecision::Accepted => Some(proposal.size),
        ContentSizeDecision::Constrained { size } => Some(*size),
        ContentSizeDecision::Rejected { .. } => None,
    };
    Ok(ContentSizeProposalReceipt {
        proposal,
        decision,
        accepted_size,
    })
}

pub(crate) fn validate_content_size_proposal(
    desired: &DesiredState,
    proposal: ContentSizeProposal,
) -> Result<(), CoordinationError> {
    if !desired.capabilities().accepts_content_size_requests() {
        return Err(CoordinationError::ContentSizeRequestsUnsupported);
    }
    if proposal.generation < desired.generation() {
        return Err(CoordinationError::StaleGeneration {
            current: desired.generation(),
            supplied: proposal.generation,
        });
    }
    if proposal.generation > desired.generation() {
        return Err(CoordinationError::FutureGeneration {
            current: desired.generation(),
            supplied: proposal.generation,
        });
    }
    if proposal.desired_revision != desired.revision() {
        return Err(CoordinationError::StaleRevision {
            current: desired.revision(),
            supplied: proposal.desired_revision,
        });
    }
    Ok(())
}
