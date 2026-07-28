mod flush_set;
mod lane;
mod policy;
mod types;

pub use flush_set::DebounceFlushSet;
pub use lane::DebouncedMutation;
pub use policy::{
    DebounceClock, DebouncePolicy, DebouncePolicyError, DebounceStrategy, SystemClock,
};
pub use types::{
    DebounceSnapshot, DebounceTerminal, FlushOutcome, FlushSetError, PendingSnapshot,
    RetryDisposition, StageDisposition, StageError, StageReceipt,
};

#[cfg(test)]
mod tests;
