//! Persisted fork-history envelopes, codecs, and checked load/encode.

mod codec;
mod decode;
mod error;
mod outcome;
mod service;
mod wire;

pub(crate) use decode::decode_graph;
pub use codec::{
    CURRENT_FORK_HISTORY_STRUCTURAL_VERSION, FORK_HISTORY_FORMAT_FAMILY, ForkPersistenceLimits,
    ForkPersistenceLimitsError, ForkStructuralMigration, ForkStructuralMigrationStep,
    ForkStructuralMigrationTarget, MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES, NoForkStructuralMigration,
};
pub use error::{ForkEncodeError, ForkLoadError};
pub use outcome::{ForkLoadOutcome, ForkLoadReceipt, ForkLoadResult};
pub use service::ForkPersistence;
