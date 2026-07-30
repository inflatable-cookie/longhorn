use longhorn_core::HistoryGroupId;

/// Structural decision for two adjacent compatible typed payloads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryCoalesce<P> {
    /// Keep the incoming payload as a separate entry.
    KeepSeparate,
    /// Replace the prior entry payload while retaining its identity and sequence.
    Replace(P),
    /// Remove the prior entry and do not insert the incoming payload.
    Remove,
}

/// Structural context for one adjacent coalescing decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryCoalesceContext<'a> {
    /// Ordinary adjacent records outside an explicit group.
    Adjacent,
    /// Records inside one exact active group.
    Group {
        /// Active group identity.
        group_id: &'a HistoryGroupId,
    },
}

/// Pure consumer policy over one typed product payload.
pub trait HistoryPolicy<P> {
    /// Consumer-owned policy failure.
    type Error;

    /// Produces the inverse payload or rejects unsupported inversion.
    fn inverse(&self, payload: &P) -> Result<P, Self::Error>;

    /// Returns whether a payload has no product effect.
    fn is_noop(&self, payload: &P) -> bool;

    /// Returns the exact encoded payload weight used for retention.
    fn encoded_weight(&self, payload: &P) -> Result<u64, Self::Error>;

    /// Decides the structural result for adjacent compatible entries.
    fn coalesce(
        &self,
        previous: &P,
        incoming: &P,
        context: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<P>, Self::Error>;
}
