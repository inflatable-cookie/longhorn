use longhorn_core::ClientSize;
use serde::{Deserialize, Serialize};

use crate::{
    AttachGeneration, CoordinationError, DesiredState, NativeContentFailureCode,
    NativeContentRevision,
};

/// One mechanism-originated proposal for a new semantic content size.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
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

    /// Returns proposed semantic content size.
    #[must_use]
    pub const fn size(self) -> ClientSize {
        self.size
    }
}

/// Consumer decision for one content-size proposal.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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

/// Validates and records a consumer decision without mutating desired state.
pub fn decide_content_size(
    desired: &DesiredState,
    proposal: ContentSizeProposal,
    decision: ContentSizeDecision,
) -> Result<ContentSizeProposalReceipt, CoordinationError> {
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
