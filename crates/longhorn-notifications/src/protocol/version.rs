//! Notification protocol version.

use serde::{Deserialize, Serialize};

/// Current exact notification protocol line.
pub const NOTIFICATION_PROTOCOL_VERSION: u32 = 1;
/// Default record count returned by mutation results and subscriptions.
pub const NOTIFICATION_DEFAULT_PAGE_SIZE: u64 = 100;

/// Exact notification protocol version.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct NotificationProtocolVersion(u32);

impl NotificationProtocolVersion {
    /// Current exact protocol version.
    pub const CURRENT: Self = Self(NOTIFICATION_PROTOCOL_VERSION);

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
