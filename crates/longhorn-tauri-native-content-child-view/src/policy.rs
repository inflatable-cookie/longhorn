use std::{fmt, sync::Arc};

use longhorn_core::WindowId;
use longhorn_native_content::{
    DetachPolicy, InputRoutingMode, MechanismCapabilities, NativeContentIslandId,
    NativeContentMechanism,
};
use tauri::Url;

use crate::ChildViewError;

const MAX_LABEL_BYTES: usize = 128;

/// Honest production capabilities of the Tauri child-view adapter.
pub const CHILD_VIEW_CAPABILITIES: MechanismCapabilities = MechanismCapabilities::new(
    NativeContentMechanism::ChildView,
    InputRoutingMode::NativeDirect,
    false,
    DetachPolicy::Reversible,
    false,
    false,
);

/// Explicit Tauri transport label. It is never native-content identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChildViewLabel(String);

impl ChildViewLabel {
    /// Validates a bounded Tauri label without deriving it from domain identity.
    pub fn new(value: impl Into<String>) -> Result<Self, ChildViewError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_LABEL_BYTES
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-/:_.".contains(&byte))
        {
            return Err(ChildViewError::InvalidLabel);
        }
        Ok(Self(value))
    }

    /// Returns the exact transport label.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ChildViewLabel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Consumer-supplied construction and security inputs for one child island.
#[derive(Clone)]
pub struct ChildViewSpec {
    island_id: NativeContentIslandId,
    host_window_id: WindowId,
    host_window_label: ChildViewLabel,
    child_label: ChildViewLabel,
    source: Url,
    data_store_identifier: Option<[u8; 16]>,
    navigation_policy: Arc<dyn Fn(&Url) -> bool + Send + Sync>,
}

impl ChildViewSpec {
    /// Creates complete injected construction policy for one remote child.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        island_id: NativeContentIslandId,
        host_window_id: WindowId,
        host_window_label: ChildViewLabel,
        child_label: ChildViewLabel,
        source: Url,
        data_store_identifier: Option<[u8; 16]>,
        navigation_policy: Arc<dyn Fn(&Url) -> bool + Send + Sync>,
    ) -> Result<Self, ChildViewError> {
        if !matches!(source.scheme(), "http" | "https") || source.host_str().is_none() {
            return Err(ChildViewError::InvalidContentSource);
        }
        if host_window_label == child_label {
            return Err(ChildViewError::DuplicateTransportLabel);
        }
        Ok(Self {
            island_id,
            host_window_id,
            host_window_label,
            child_label,
            source,
            data_store_identifier,
            navigation_policy,
        })
    }

    /// Returns the shared island identity.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the logical host-window binding.
    #[must_use]
    pub const fn host_window_id(&self) -> &WindowId {
        &self.host_window_id
    }

    /// Returns the explicitly mapped Tauri host-window label.
    #[must_use]
    pub const fn host_window_label(&self) -> &ChildViewLabel {
        &self.host_window_label
    }

    /// Returns the independent Tauri child label.
    #[must_use]
    pub const fn child_label(&self) -> &ChildViewLabel {
        &self.child_label
    }

    /// Returns the consumer-supplied initial remote source.
    #[must_use]
    pub const fn source(&self) -> &Url {
        &self.source
    }

    /// Returns optional consumer-supplied macOS data-store identity.
    #[must_use]
    pub const fn data_store_identifier(&self) -> Option<[u8; 16]> {
        self.data_store_identifier
    }

    /// Evaluates consumer-owned navigation policy.
    #[must_use]
    pub fn allows_navigation(&self, candidate: &Url) -> bool {
        (self.navigation_policy)(candidate)
    }
}
