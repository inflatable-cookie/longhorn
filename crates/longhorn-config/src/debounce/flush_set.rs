use std::collections::BTreeMap;

use longhorn_core::DomainId;

use crate::{ConfigDomain, ConfigStore};

use super::{DebounceClock, DebounceStrategy, DebouncedMutation, FlushOutcome, FlushSetError};

trait ErasedFlushLane {
    fn flush_forced(&mut self) -> FlushOutcome;
}

impl<D, S, C> ErasedFlushLane for DebouncedMutation<'_, D, S, C>
where
    D: ConfigDomain,
    S: DebounceStrategy<D>,
    C: DebounceClock,
{
    fn flush_forced(&mut self) -> FlushOutcome {
        self.flush_forced()
    }
}

/// Ephemeral stable-order aggregate over heterogeneous debounce lanes.
pub struct DebounceFlushSet<'store, 'lanes> {
    store: &'store ConfigStore,
    lanes: BTreeMap<DomainId, &'lanes mut dyn ErasedFlushLane>,
}

impl<'store, 'lanes> DebounceFlushSet<'store, 'lanes> {
    /// Constructs an empty set for one configuration store.
    #[must_use]
    pub fn new(store: &'store ConfigStore) -> Self {
        Self {
            store,
            lanes: BTreeMap::new(),
        }
    }

    /// Adds one lane and rejects another store or duplicate domain.
    pub fn insert<D, S, C>(
        &mut self,
        lane: &'lanes mut DebouncedMutation<'_, D, S, C>,
    ) -> Result<(), FlushSetError>
    where
        D: ConfigDomain,
        S: DebounceStrategy<D>,
        C: DebounceClock,
    {
        let domain = lane.domain_id();
        if lane.store_identity() != std::ptr::from_ref(self.store) {
            return Err(FlushSetError::WrongStore { domain });
        }
        if self.lanes.contains_key(&domain) {
            return Err(FlushSetError::DuplicateDomain { domain });
        }
        self.lanes.insert(domain, lane);
        Ok(())
    }

    /// Forces every inserted lane in stable domain-id order.
    pub fn flush_all(&mut self) -> Vec<FlushOutcome> {
        self.lanes
            .values_mut()
            .map(|lane| lane.flush_forced())
            .collect()
    }

    /// Returns the number of inserted lanes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lanes.len()
    }

    /// Returns whether the set has no lanes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lanes.is_empty()
    }
}
