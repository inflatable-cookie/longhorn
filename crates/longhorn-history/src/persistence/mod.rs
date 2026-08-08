//! Persisted linear-history envelopes, codecs, and checked load/encode.

mod codec;
mod encode;
mod error;
mod load;
mod outcome;
mod service;
mod wire;

pub use codec::{
    CURRENT_HISTORY_STRUCTURAL_VERSION, HISTORY_FORMAT_FAMILY, HistoryPayloadCodec,
    HistoryPayloadCodecFamily, HistoryPayloadCodecFamilyError, HistoryPayloadCodecVersion,
    HistoryPayloadMigrationStep, HistoryPayloadMigrationTarget, HistoryPersistenceLimits,
    HistoryPersistenceLimitsError, HistoryStructuralMigration, HistoryStructuralMigrationStep,
    HistoryStructuralMigrationTarget, MAXIMUM_HISTORY_PAYLOAD_CODEC_FAMILY_BYTES,
    MAXIMUM_HISTORY_PERSISTED_BYTES, NoHistoryStructuralMigration,
};
pub use error::{HistoryEncodeError, HistoryLoadError, HistoryStructuralHeaderError};
pub use outcome::{
    HistoryDiscardRecovery, HistoryDiscardRecoveryReceipt, HistoryLoadAttempt, HistoryLoadOutcome,
    HistoryLoadReceipt, HistoryLoadResult, discard_persisted_history,
};
pub use service::HistoryPersistence;
