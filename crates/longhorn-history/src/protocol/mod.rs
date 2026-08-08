//! Strict metadata-only renderer and transport protocol.

mod error;
mod event;
mod navigation;
mod snapshot;
mod version;

pub use error::HistoryProtocolProjectionError;
pub(crate) use error::project_count;
pub use event::{HistoryChangedEvent, HistoryChangedKind};
pub use navigation::{
    HistoryNavigationCommand, HistoryNavigationDirectionProjection,
    HistoryNavigationPositionProjection, HistoryNavigationReceiptProjection,
    HistoryNavigationRejectionCode, HistoryNavigationRejectionProjection, HistoryNavigationResult,
    HistoryNavigationTargetProjection,
};
pub use snapshot::{
    HistoryBaselineProjection, HistoryEntryRecord, HistoryPageCommand, HistoryPageSnapshot,
    HistorySnapshot, HistorySummaryProjection,
};
pub use version::{
    HISTORY_PROTOCOL_VERSION, HistoryAuthorityEpoch, HistoryAuthorityEpochError,
    HistoryProjectionPosition, HistoryProtocolMode, HistoryProtocolVersion,
};
