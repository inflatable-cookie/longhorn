use std::time::Duration;

use longhorn_core::DomainId;

use crate::{
    ConfigDomain, ConfigStore, MutationError, StoreError,
    store::mutation::{self, MutationOutcome},
};

use super::{
    DebounceClock, DebouncePolicy, DebounceSnapshot, DebounceStrategy, DebounceTerminal,
    FlushOutcome, PendingSnapshot, StageDisposition, StageError, StageReceipt,
    types::retry_disposition,
};

struct Pending<I> {
    intent: I,
    generation: u64,
    due_at: Duration,
    weight: usize,
    retry_required: bool,
}

/// One bounded typed debounce lane over a registered domain.
pub struct DebouncedMutation<'config, D, S, C>
where
    D: ConfigDomain,
    S: DebounceStrategy<D>,
    C: DebounceClock,
{
    store: &'config ConfigStore,
    domain: &'config D,
    strategy: S,
    clock: C,
    policy: DebouncePolicy,
    last_generation: u64,
    pending: Option<Pending<S::Intent>>,
    last_terminal: Option<DebounceTerminal>,
}

impl<'config, D, S, C> DebouncedMutation<'config, D, S, C>
where
    D: ConfigDomain,
    S: DebounceStrategy<D>,
    C: DebounceClock,
{
    /// Constructs a lane over a domain already registered with the store.
    pub fn new(
        store: &'config ConfigStore,
        domain: &'config D,
        strategy: S,
        clock: C,
        policy: DebouncePolicy,
    ) -> Result<Self, StoreError> {
        store.ensure_registered(domain)?;
        Ok(Self {
            store,
            domain,
            strategy,
            clock,
            policy,
            last_generation: 0,
            pending: None,
            last_terminal: None,
        })
    }

    /// Stages one intent and resets the trailing-edge deadline.
    pub fn stage(&mut self, next: S::Intent) -> Result<StageReceipt, StageError> {
        let domain = self.domain_id();
        let (candidate, disposition) = match &self.pending {
            Some(pending) => (
                self.strategy
                    .coalesce(&pending.intent, next)
                    .map_err(|issue| StageError::Coalescing {
                        domain: domain.clone(),
                        issue,
                    })?,
                StageDisposition::Coalesced {
                    previous_generation: pending.generation,
                },
            ),
            None => (next, StageDisposition::Opened),
        };
        let weight = self.strategy.pending_weight(&candidate);
        if weight > self.policy.max_pending_weight {
            return Err(StageError::PendingWeightExceeded {
                domain,
                attempted: weight,
                maximum: self.policy.max_pending_weight,
            });
        }

        let now = self.clock.now();
        let due_at =
            now.checked_add(self.policy.delay)
                .ok_or_else(|| StageError::DeadlineOverflow {
                    domain: self.domain_id(),
                    now,
                    delay: self.policy.delay,
                })?;
        let generation =
            self.last_generation
                .checked_add(1)
                .ok_or_else(|| StageError::GenerationExhausted {
                    domain: self.domain_id(),
                })?;

        self.pending = Some(Pending {
            intent: candidate,
            generation,
            due_at,
            weight,
            retry_required: false,
        });
        self.last_generation = generation;

        Ok(StageReceipt {
            domain: self.domain_id(),
            generation,
            due_at,
            pending_weight: weight,
            disposition,
        })
    }

    /// Returns the next host wake-up deadline, excluding retry-required work.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Duration> {
        self.pending
            .as_ref()
            .filter(|pending| !pending.retry_required)
            .map(|pending| pending.due_at)
    }

    /// Flushes pending work only after its deadline.
    pub fn flush_due(&mut self) -> FlushOutcome {
        let Some(pending) = &self.pending else {
            return self.no_pending();
        };
        if pending.retry_required {
            return FlushOutcome::RetryRequired {
                domain: self.domain_id(),
                generation: pending.generation,
            };
        }
        if self.clock.now() < pending.due_at {
            return FlushOutcome::NotDue {
                domain: self.domain_id(),
                generation: pending.generation,
                due_at: pending.due_at,
            };
        }
        self.flush_pending()
    }

    /// Flushes pending work immediately, including retry-required work.
    pub fn flush_forced(&mut self) -> FlushOutcome {
        if self.pending.is_none() {
            self.no_pending()
        } else {
            self.flush_pending()
        }
    }

    /// Explicitly discards unpublished pending intent without filesystem I/O.
    pub fn discard(&mut self) -> FlushOutcome {
        let Some(pending) = self.pending.take() else {
            return self.no_pending();
        };
        let terminal = DebounceTerminal::Discarded {
            domain: self.domain_id(),
            generation: pending.generation,
        };
        self.last_terminal = Some(terminal.clone());
        FlushOutcome::Terminal(terminal)
    }

    /// Returns bounded lane state without exposing staged intent.
    #[must_use]
    pub fn snapshot(&self) -> DebounceSnapshot {
        DebounceSnapshot {
            domain: self.domain_id(),
            pending: self.pending.as_ref().map(|pending| PendingSnapshot {
                generation: pending.generation,
                due_at: pending.due_at,
                pending_weight: pending.weight,
                retry_required: pending.retry_required,
            }),
            last_terminal: self.last_terminal.clone(),
        }
    }

    pub(super) fn domain_id(&self) -> DomainId {
        self.domain.descriptor().id().clone()
    }

    pub(super) fn store_identity(&self) -> *const ConfigStore {
        std::ptr::from_ref(self.store)
    }

    fn flush_pending(&mut self) -> FlushOutcome {
        let generation = self
            .pending
            .as_ref()
            .expect("flush_pending requires pending state")
            .generation;
        let result = {
            let intent = &self
                .pending
                .as_ref()
                .expect("flush_pending requires pending state")
                .intent;
            mutation::mutate_if_changed(self.store, self.domain, self.policy.mutation, |value| {
                self.strategy.apply(intent, value)
            })
        };
        self.finish_flush(generation, result)
    }

    pub(super) fn finish_flush(
        &mut self,
        generation: u64,
        result: Result<MutationOutcome, MutationError>,
    ) -> FlushOutcome {
        let terminal = match result {
            Ok(MutationOutcome::Unchanged) => {
                self.pending = None;
                DebounceTerminal::Unchanged {
                    domain: self.domain_id(),
                    generation,
                }
            }
            Ok(MutationOutcome::Published(receipt)) => {
                self.pending = None;
                DebounceTerminal::Published {
                    domain: self.domain_id(),
                    generation,
                    receipt,
                }
            }
            Err(MutationError::Publication(failure)) if failure.published => {
                self.pending = None;
                DebounceTerminal::PublishedWithDurabilityFailure {
                    domain: self.domain_id(),
                    generation,
                    failure,
                }
            }
            Err(error) => {
                self.pending
                    .as_mut()
                    .expect("failed flush retains pending state")
                    .retry_required = true;
                DebounceTerminal::Failed {
                    domain: self.domain_id(),
                    generation,
                    retry: retry_disposition(&error),
                    error,
                }
            }
        };
        self.last_terminal = Some(terminal.clone());
        FlushOutcome::Terminal(terminal)
    }

    fn no_pending(&self) -> FlushOutcome {
        FlushOutcome::NoPending {
            domain: self.domain_id(),
        }
    }
}
