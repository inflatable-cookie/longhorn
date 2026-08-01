use std::{fmt, sync::Arc};

use longhorn_core::WindowId;
use longhorn_native_content::{
    DetachPolicy, InputRoutingMode, MechanismCapabilities, NativeContentIslandId,
    NativeContentMechanism,
};
use tauri::Url;

use crate::ChildViewError;

const MAX_LABEL_BYTES: usize = 128;
const MAX_INITIALIZATION_SCRIPT_BYTES: usize = 64 * 1024;

/// Consumer-owned browser event retained on the native side.
///
/// These events never enter the native-content renderer protocol. They let an
/// application preserve trusted chrome and construction policy while the
/// adapter owns only child-view mechanics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChildViewPolicyEvent {
    /// A page load began for an admitted URL.
    PageLoadStarted {
        /// Admitted page URL.
        url: Url,
    },
    /// A page load finished for an admitted URL.
    PageLoadFinished {
        /// Admitted page URL.
        url: Url,
    },
    /// Native popup creation was denied closed.
    PopupDenied {
        /// Denied popup URL.
        url: Url,
    },
    /// Native download persistence was denied closed.
    DownloadDenied {
        /// Denied download URL.
        url: Url,
    },
    /// A document title changed. Consumers must treat it as untrusted content.
    DocumentTitleChanged {
        /// Untrusted document title.
        title: String,
    },
}

/// Consumer-owned child construction hooks with no renderer authority.
#[derive(Clone)]
pub struct ChildViewPolicyHooks {
    initialization_script: Option<String>,
    observer: Arc<dyn Fn(ChildViewPolicyEvent) + Send + Sync>,
}

impl ChildViewPolicyHooks {
    /// Validates a bounded optional initialization script and native observer.
    pub fn new(
        initialization_script: Option<String>,
        observer: Arc<dyn Fn(ChildViewPolicyEvent) + Send + Sync>,
    ) -> Result<Self, ChildViewError> {
        if initialization_script.as_ref().is_some_and(|script| {
            script.is_empty()
                || script.len() > MAX_INITIALIZATION_SCRIPT_BYTES
                || script.contains('\0')
        }) {
            return Err(ChildViewError::InvalidInitializationScript);
        }
        Ok(Self {
            initialization_script,
            observer,
        })
    }

    /// Returns the optional consumer initialization script.
    #[must_use]
    pub fn initialization_script(&self) -> Option<&str> {
        self.initialization_script.as_deref()
    }

    /// Delivers one native-only policy event.
    pub fn emit(&self, event: ChildViewPolicyEvent) {
        (self.observer)(event);
    }
}

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
    policy_hooks: ChildViewPolicyHooks,
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
        policy_hooks: ChildViewPolicyHooks,
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
            policy_hooks,
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

    /// Returns consumer-owned construction and trusted-chrome hooks.
    #[must_use]
    pub const fn policy_hooks(&self) -> &ChildViewPolicyHooks {
        &self.policy_hooks
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{ChildViewPolicyEvent, ChildViewPolicyHooks};

    #[test]
    fn construction_hooks_bound_scripts_and_keep_events_native() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = events.clone();
        let hooks = ChildViewPolicyHooks::new(
            Some("globalThis.__fixture = true;".to_owned()),
            Arc::new(move |event| recorded.lock().unwrap().push(event)),
        )
        .unwrap();
        hooks.emit(ChildViewPolicyEvent::PopupDenied {
            url: "https://example.com/popup".parse().unwrap(),
        });

        assert_eq!(
            hooks.initialization_script(),
            Some("globalThis.__fixture = true;")
        );
        assert!(matches!(
            events.lock().unwrap().as_slice(),
            [ChildViewPolicyEvent::PopupDenied { url }] if url.as_str() == "https://example.com/popup"
        ));
        assert!(ChildViewPolicyHooks::new(Some(String::new()), Arc::new(|_| {})).is_err());
        assert!(
            ChildViewPolicyHooks::new(Some("bad\0script".to_owned()), Arc::new(|_| {})).is_err()
        );
        assert!(
            ChildViewPolicyHooks::new(Some("x".repeat(64 * 1024 + 1)), Arc::new(|_| {})).is_err()
        );
    }
}
