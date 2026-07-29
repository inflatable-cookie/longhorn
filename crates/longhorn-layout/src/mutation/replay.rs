use std::{
    collections::{BTreeMap, VecDeque},
    error::Error,
    fmt,
};

use longhorn_core::LayoutRequestId;

use super::{LayoutMutationReceipt, LayoutMutationRequest};

const HARD_MAXIMUM_REPLAY_ENTRIES: usize = 4_096;

/// Invalid bounded replay-store capacity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutReplayStoreError {
    /// Replay capacity was zero.
    ZeroCapacity,
    /// Replay capacity exceeded the library hard ceiling.
    ExceedsHardMaximum {
        /// Library hard ceiling.
        maximum: usize,
        /// Rejected capacity.
        actual: usize,
    },
}

impl fmt::Display for LayoutReplayStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCapacity => formatter.write_str("layout replay capacity must be nonzero"),
            Self::ExceedsHardMaximum { maximum, actual } => write!(
                formatter,
                "layout replay capacity is {actual}; library hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for LayoutReplayStoreError {}

#[derive(Clone, Debug)]
struct ReplayEntry {
    request: LayoutMutationRequest,
    receipt: LayoutMutationReceipt,
}

/// Explicit process-local bounded successful-request replay authority.
#[derive(Clone, Debug)]
pub struct BoundedLayoutReplayStore {
    maximum_entries: usize,
    insertion_order: VecDeque<LayoutRequestId>,
    entries: BTreeMap<LayoutRequestId, ReplayEntry>,
}

impl BoundedLayoutReplayStore {
    /// Constructs an empty finite replay store.
    pub fn new(maximum_entries: usize) -> Result<Self, LayoutReplayStoreError> {
        if maximum_entries == 0 {
            return Err(LayoutReplayStoreError::ZeroCapacity);
        }
        if maximum_entries > HARD_MAXIMUM_REPLAY_ENTRIES {
            return Err(LayoutReplayStoreError::ExceedsHardMaximum {
                maximum: HARD_MAXIMUM_REPLAY_ENTRIES,
                actual: maximum_entries,
            });
        }
        Ok(Self {
            maximum_entries,
            insertion_order: VecDeque::new(),
            entries: BTreeMap::new(),
        })
    }

    /// Returns configured entry capacity.
    #[must_use]
    pub const fn maximum_entries(&self) -> usize {
        self.maximum_entries
    }

    /// Returns retained successful-request count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether no successful request is retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(super) fn lookup(
        &self,
        request: &LayoutMutationRequest,
    ) -> Option<Result<LayoutMutationReceipt, ()>> {
        self.entries.get(request.request_id()).map(|entry| {
            if entry.request == *request {
                Ok(entry.receipt.clone())
            } else {
                Err(())
            }
        })
    }

    pub(super) fn record(
        &mut self,
        request: &LayoutMutationRequest,
        receipt: &LayoutMutationReceipt,
    ) {
        if self.entries.contains_key(request.request_id()) {
            return;
        }
        while self.entries.len() >= self.maximum_entries {
            let oldest = self
                .insertion_order
                .pop_front()
                .expect("nonempty entries have insertion order");
            self.entries.remove(&oldest);
        }
        self.insertion_order.push_back(request.request_id().clone());
        self.entries.insert(
            request.request_id().clone(),
            ReplayEntry {
                request: request.clone(),
                receipt: receipt.clone(),
            },
        );
    }
}
