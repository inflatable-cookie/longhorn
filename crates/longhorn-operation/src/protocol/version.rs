//! Operation protocol version.

use serde::{Deserialize, Serialize};

/// Current exact operation protocol line.
pub const OPERATION_PROTOCOL_VERSION: u32 = 1;

/// Exact operation protocol version.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct OperationProtocolVersion(u32);

impl OperationProtocolVersion {
    /// Current exact protocol version.
    pub const CURRENT: Self = Self(OPERATION_PROTOCOL_VERSION);

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
