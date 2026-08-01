//! Exact renderer protocol for one native-content island authority.

mod error;
mod host;
mod identity;
mod types;

pub use error::{
    NativeContentFailurePhase, NativeContentProtocolRejection, NativeContentRejectionCode,
    NativeContentRetryClass,
};
pub use host::NativeContentProtocolHost;
pub use identity::{
    NativeContentAuthorityEpoch, NativeContentClientEpoch, NativeContentProtocolCounterError,
    NativeContentProtocolVersion,
};
pub use types::{
    NativeContentChangeProjection, NativeContentChangedEvent, NativeContentConnectRequest,
    NativeContentConnectResult, NativeContentContentSizeDecisionRequest,
    NativeContentContentSizeDecisionResult, NativeContentCursor, NativeContentDesiredUpdateRequest,
    NativeContentDesiredUpdateResult, NativeContentSnapshot, NativeContentSnapshotRequest,
    NativeContentSnapshotResult,
};

/// Current exact native-content protocol line.
pub const NATIVE_CONTENT_PROTOCOL_VERSION: u32 = 1;
