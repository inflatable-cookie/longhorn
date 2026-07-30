//! Connection, bounded retry, authority invalidation, and optional supervision contract.

#[path = "bridge_lifecycle/connection.rs"]
mod connection;
#[path = "bridge_lifecycle/retry.rs"]
mod retry;
#[cfg(feature = "supervision")]
#[path = "bridge_lifecycle/supervision.rs"]
mod supervision;
#[path = "bridge_lifecycle/support.rs"]
mod support;
