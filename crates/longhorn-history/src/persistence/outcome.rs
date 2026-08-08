//! Load and discard receipts.

use longhorn_core::{HistoryId, HistoryRevision};

use crate::{
    HistoryCommittedTransition, HistoryCommittedTransitionKind, HistoryDiscardReason,
    HistoryLimits, HistoryNavigationLimits, HistoryProjectionLimits, LinearHistory,
};

use super::{HistoryLoadError, HistoryPayloadCodecFamily, HistoryPayloadCodecVersion};

/// Visible successful compatibility outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryLoadOutcome {
    /// Current structural and payload versions were preserved.
    Preserved,
    /// One or both independent version families migrated.
    Migrated {
        /// Structural migration ran.
        structural: bool,
        /// Payload migration ran.
        payload: bool,
    },
}

/// Successful checked load receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryLoadReceipt {
    pub(crate) outcome: HistoryLoadOutcome,
    pub(crate) source_structural_version: u32,
    pub(crate) structural_version: u32,
    pub(crate) payload_codec_family: HistoryPayloadCodecFamily,
    pub(crate) source_payload_codec_version: HistoryPayloadCodecVersion,
    pub(crate) payload_codec_version: HistoryPayloadCodecVersion,
    pub(crate) transition: HistoryCommittedTransition,
}

impl HistoryLoadReceipt {
    /// Returns whether bytes were preserved or visibly migrated.
    #[must_use]
    pub const fn outcome(&self) -> HistoryLoadOutcome {
        self.outcome
    }

    /// Returns the structural version found in source bytes.
    #[must_use]
    pub const fn source_structural_version(&self) -> u32 {
        self.source_structural_version
    }

    /// Returns the accepted structural version.
    #[must_use]
    pub const fn structural_version(&self) -> u32 {
        self.structural_version
    }

    /// Returns the registered payload codec family.
    #[must_use]
    pub const fn payload_codec_family(&self) -> &HistoryPayloadCodecFamily {
        &self.payload_codec_family
    }

    /// Returns the payload codec version found in source bytes.
    #[must_use]
    pub const fn source_payload_codec_version(&self) -> HistoryPayloadCodecVersion {
        self.source_payload_codec_version
    }

    /// Returns the accepted payload codec version.
    #[must_use]
    pub const fn payload_codec_version(&self) -> HistoryPayloadCodecVersion {
        self.payload_codec_version
    }

    /// Returns the committed import transition.
    #[must_use]
    pub const fn transition(&self) -> &HistoryCommittedTransition {
        &self.transition
    }
}

/// Fully validated authority plus its visible load receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryLoadResult<P> {
    pub(crate) history: LinearHistory<P>,
    pub(crate) receipt: HistoryLoadReceipt,
}

/// Checked load result across codec, policy, and structural migration hooks.
pub type HistoryLoadAttempt<P, CE, PE, ME> =
    Result<HistoryLoadResult<P>, HistoryLoadError<CE, PE, ME>>;

impl<P> HistoryLoadResult<P> {
    /// Returns the validated authority.
    #[must_use]
    pub const fn history(&self) -> &LinearHistory<P> {
        &self.history
    }

    /// Returns the preserve or migration receipt.
    #[must_use]
    pub const fn receipt(&self) -> &HistoryLoadReceipt {
        &self.receipt
    }

    /// Consumes the result into the validated authority and receipt.
    #[must_use]
    pub fn into_parts(self) -> (LinearHistory<P>, HistoryLoadReceipt) {
        (self.history, self.receipt)
    }
}

/// Explicit discard-history recovery receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryDiscardRecoveryReceipt {
    pub(crate) history_id: HistoryId,
    pub(crate) reason: HistoryDiscardReason,
    pub(crate) transition: HistoryCommittedTransition,
}

impl HistoryDiscardRecoveryReceipt {
    /// Returns the replacement authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the caller-owned visible discard reason.
    #[must_use]
    pub const fn reason(&self) -> HistoryDiscardReason {
        self.reason
    }

    /// Returns the committed discard transition.
    #[must_use]
    pub const fn transition(&self) -> &HistoryCommittedTransition {
        &self.transition
    }
}

/// Explicit fresh authority produced after a visible discard decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryDiscardRecovery<P> {
    history: LinearHistory<P>,
    receipt: HistoryDiscardRecoveryReceipt,
}

impl<P> HistoryDiscardRecovery<P> {
    /// Returns the fresh empty authority.
    #[must_use]
    pub const fn history(&self) -> &LinearHistory<P> {
        &self.history
    }

    /// Returns explicit discard evidence.
    #[must_use]
    pub const fn receipt(&self) -> &HistoryDiscardRecoveryReceipt {
        &self.receipt
    }

    /// Consumes the recovery into the fresh authority and receipt.
    #[must_use]
    pub fn into_parts(self) -> (LinearHistory<P>, HistoryDiscardRecoveryReceipt) {
        (self.history, self.receipt)
    }
}

/// Deliberately discards unusable persisted history and creates a fresh authority.
#[must_use]
pub fn discard_persisted_history<P>(
    history_id: HistoryId,
    limits: HistoryLimits,
    navigation_limits: HistoryNavigationLimits,
    projection_limits: HistoryProjectionLimits,
    reason: HistoryDiscardReason,
) -> HistoryDiscardRecovery<P> {
    let history = LinearHistory::with_runtime_limits(
        history_id.clone(),
        limits,
        navigation_limits,
        projection_limits,
    );
    let transition = HistoryCommittedTransition::new(
        history_id.clone(),
        None,
        HistoryRevision::INITIAL,
        HistoryCommittedTransitionKind::DiscardedPersistence { reason },
    );
    HistoryDiscardRecovery {
        history,
        receipt: HistoryDiscardRecoveryReceipt {
            history_id,
            reason,
            transition,
        },
    }
}
