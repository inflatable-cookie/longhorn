use std::collections::BTreeSet;

use longhorn_core::LayoutContainerId;
use serde::{Deserialize, Serialize};

/// Read-only evidence of layout containers known to another authority.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LayoutContainerInventory {
    ids: BTreeSet<LayoutContainerId>,
}

impl LayoutContainerInventory {
    /// Captures the exact caller-supplied set of currently existing containers.
    #[must_use]
    pub fn new(ids: impl IntoIterator<Item = LayoutContainerId>) -> Self {
        Self {
            ids: ids.into_iter().collect(),
        }
    }

    /// Returns whether the external authority reported one container.
    #[must_use]
    pub fn contains(&self, id: &LayoutContainerId) -> bool {
        self.ids.contains(id)
    }
}

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

/// Explicit external cleanup work produced by Surface close.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct LayoutContainerCleanupIntent {
    layout_container_id: LayoutContainerId,
}

impl LayoutContainerCleanupIntent {
    pub(super) const fn new(layout_container_id: LayoutContainerId) -> Self {
        Self {
            layout_container_id,
        }
    }

    /// Returns the now-unbound layout container.
    #[must_use]
    pub const fn layout_container_id(&self) -> &LayoutContainerId {
        &self.layout_container_id
    }
}
