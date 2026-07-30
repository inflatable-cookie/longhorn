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

/// Pure consumer policy over one typed product payload.
pub trait HistoryPolicy<P> {
    /// Consumer-owned policy failure.
    type Error;

    /// Produces the inverse payload or rejects unsupported inversion.
    fn inverse(&self, payload: &P) -> Result<P, Self::Error>;

    /// Returns whether a payload has no product effect.
    fn is_noop(&self, payload: &P) -> bool;

    /// Decides the structural result for adjacent compatible entries.
    fn coalesce(&self, previous: &P, incoming: &P) -> Result<HistoryCoalesce<P>, Self::Error>;
}
