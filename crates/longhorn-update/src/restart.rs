use core::fmt;

use serde::{Deserialize, Serialize};

use crate::DeferralCause;

/// One kind of Longhorn-owned work that must settle before a restart.
///
/// An update that relaunches mid-commit is data loss, and only Longhorn knows
/// what is outstanding — this is the part of the update path a consuming
/// application could not write for itself.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(export))]
pub enum QuiescenceKind {
    /// Configuration or settings writes that have not reached disk.
    PendingFlush,
    /// A cross-window transfer session that has not committed or aborted.
    OpenTransferSession,
    /// An async operation still running.
    RunningOperation,
}

impl QuiescenceKind {
    /// Every kind, in the order a surface should list them.
    pub const ALL: [Self; 3] = [
        Self::PendingFlush,
        Self::OpenTransferSession,
        Self::RunningOperation,
    ];

    /// Returns the stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PendingFlush => "pending-flush",
            Self::OpenTransferSession => "open-transfer-session",
            Self::RunningOperation => "running-operation",
        }
    }

    /// Returns the display noun for a count.
    #[must_use]
    pub const fn noun(self, count: usize) -> &'static str {
        match (self, count) {
            (Self::PendingFlush, 1) => "pending flush",
            (Self::PendingFlush, _) => "pending flushes",
            (Self::OpenTransferSession, 1) => "open transfer session",
            (Self::OpenTransferSession, _) => "open transfer sessions",
            (Self::RunningOperation, 1) => "running operation",
            (Self::RunningOperation, _) => "running operations",
        }
    }
}

impl fmt::Display for QuiescenceKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.noun(1))
    }
}

/// Outstanding work of one kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutstandingWork {
    /// What is outstanding.
    pub kind: QuiescenceKind,
    /// How many, for display.
    pub count: usize,
}

impl OutstandingWork {
    /// Records outstanding work.
    #[must_use]
    pub const fn new(kind: QuiescenceKind, count: usize) -> Self {
        Self { kind, count }
    }
}

/// Reports outstanding Longhorn-owned work.
///
/// Implemented against the live host. Each probe answers only for its own
/// subsystem; the receipt is the union.
pub trait QuiescenceProbe {
    /// Returns outstanding work, or `None` when this subsystem is settled.
    fn outstanding(&self) -> Option<OutstandingWork>;
}

/// The answer to "is it safe to restart right now".
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuiescenceReceipt {
    /// Everything still outstanding. Empty means quiescent.
    pub outstanding: Vec<OutstandingWork>,
}

impl QuiescenceReceipt {
    /// Collects a receipt from a set of probes.
    #[must_use]
    pub fn collect<'probe, I>(probes: I) -> Self
    where
        I: IntoIterator<Item = &'probe dyn QuiescenceProbe>,
    {
        Self {
            // Every probe runs. Stopping at the first outstanding item would
            // make the deferral reason depend on probe order, and a surface
            // that says "1 pending flush" when there are also two open
            // transfers has told the user something false.
            outstanding: probes
                .into_iter()
                .filter_map(QuiescenceProbe::outstanding)
                .filter(|work| work.count > 0)
                .collect(),
        }
    }

    /// Records a receipt directly.
    #[must_use]
    pub const fn new(outstanding: Vec<OutstandingWork>) -> Self {
        Self { outstanding }
    }

    /// Returns whether nothing is outstanding.
    #[must_use]
    pub fn is_quiescent(&self) -> bool {
        self.outstanding.is_empty()
    }

    /// Renders the outstanding work for display.
    #[must_use]
    pub fn detail(&self) -> String {
        self.outstanding
            .iter()
            .map(|work| format!("{} {}", work.count, work.kind.noun(work.count)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Converts a non-quiescent receipt into a deferral cause.
    ///
    /// Returns `None` when quiescent, so a caller cannot accidentally defer a
    /// restart that was safe to take.
    #[must_use]
    pub fn as_deferral_cause(&self) -> Option<DeferralCause> {
        (!self.is_quiescent()).then(|| DeferralCause::WorkInFlight {
            detail: self.detail(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed(Option<OutstandingWork>);

    impl QuiescenceProbe for Fixed {
        fn outstanding(&self) -> Option<OutstandingWork> {
            self.0
        }
    }

    fn probe(kind: QuiescenceKind, count: usize) -> Fixed {
        Fixed(Some(OutstandingWork::new(kind, count)))
    }

    #[test]
    fn no_probes_reporting_work_is_quiescent() {
        let settled = [Fixed(None), Fixed(None)];
        let probes: Vec<&dyn QuiescenceProbe> =
            settled.iter().map(|p| p as &dyn QuiescenceProbe).collect();

        let receipt = QuiescenceReceipt::collect(probes);

        assert!(receipt.is_quiescent());
        assert_eq!(receipt.as_deferral_cause(), None);
    }

    #[test]
    fn a_zero_count_is_not_outstanding_work() {
        let zero = [probe(QuiescenceKind::PendingFlush, 0)];
        let probes: Vec<&dyn QuiescenceProbe> =
            zero.iter().map(|p| p as &dyn QuiescenceProbe).collect();

        assert!(QuiescenceReceipt::collect(probes).is_quiescent());
    }

    #[test]
    fn every_probe_runs_so_the_reason_is_complete() {
        // A receipt that stopped at the first outstanding item would report a
        // reason that depends on probe order, and would understate what the
        // user is about to interrupt.
        let busy = [
            probe(QuiescenceKind::PendingFlush, 1),
            probe(QuiescenceKind::OpenTransferSession, 2),
        ];
        let probes: Vec<&dyn QuiescenceProbe> =
            busy.iter().map(|p| p as &dyn QuiescenceProbe).collect();

        let receipt = QuiescenceReceipt::collect(probes);

        assert!(!receipt.is_quiescent());
        assert_eq!(receipt.outstanding.len(), 2);
        assert_eq!(
            receipt.detail(),
            "1 pending flush, 2 open transfer sessions"
        );
    }

    #[test]
    fn a_non_quiescent_receipt_becomes_a_deferral_with_its_reason() {
        let busy = [probe(QuiescenceKind::RunningOperation, 3)];
        let probes: Vec<&dyn QuiescenceProbe> =
            busy.iter().map(|p| p as &dyn QuiescenceProbe).collect();

        let receipt = QuiescenceReceipt::collect(probes);

        assert_eq!(
            receipt.as_deferral_cause(),
            Some(DeferralCause::WorkInFlight {
                detail: "3 running operations".to_owned(),
            })
        );
    }
}
