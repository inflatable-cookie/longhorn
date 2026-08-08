//! Load outcome and receipt types.

use longhorn_history::HistoryPayloadCodecVersion;

use crate::ForkHistory;

/// Successful compatibility outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForkLoadOutcome {
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

/// Successful load receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForkLoadReceipt {
    pub(crate) outcome: ForkLoadOutcome,
    pub(crate) source_structural_version: u32,
    pub(crate) source_payload_version: HistoryPayloadCodecVersion,
}

impl ForkLoadReceipt {
    /// Returns whether source bytes were preserved or migrated.
    #[must_use]
    pub const fn outcome(self) -> ForkLoadOutcome {
        self.outcome
    }

    /// Returns the source structural version.
    #[must_use]
    pub const fn source_structural_version(self) -> u32 {
        self.source_structural_version
    }

    /// Returns the source payload version.
    #[must_use]
    pub const fn source_payload_version(self) -> HistoryPayloadCodecVersion {
        self.source_payload_version
    }
}

/// A fully validated graph and visible load receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkLoadResult<P> {
    pub(crate) history: ForkHistory<P>,
    pub(crate) receipt: ForkLoadReceipt,
}

impl<P> ForkLoadResult<P> {
    /// Returns the validated graph.
    #[must_use]
    pub const fn history(&self) -> &ForkHistory<P> {
        &self.history
    }

    /// Returns the compatibility receipt.
    #[must_use]
    pub const fn receipt(&self) -> ForkLoadReceipt {
        self.receipt
    }

    /// Consumes the result.
    #[must_use]
    pub fn into_parts(self) -> (ForkHistory<P>, ForkLoadReceipt) {
        (self.history, self.receipt)
    }
}
