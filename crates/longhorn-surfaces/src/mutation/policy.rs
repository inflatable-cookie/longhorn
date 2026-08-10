use serde::{Deserialize, Serialize};

/// Consumer policy for participating windows left without hosted Surfaces.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum EmptyWindowPolicy {
    /// Permit an empty window and clear its active Surface.
    Allow,
    /// Reject a mutation that would leave a window empty.
    Reject,
}
