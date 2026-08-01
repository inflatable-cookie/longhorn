use std::{fmt, sync::Arc};

use longhorn_core::WindowId;
use longhorn_native_content_prototype::NativeContentIslandId;
use tauri::Url;

use crate::ChildWebviewError;

const MAX_LABEL_BYTES: usize = 128;

/// Explicit Tauri transport label for one child webview or host window.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChildWebviewLabel(String);

impl ChildWebviewLabel {
    /// Validates a bounded label without deriving it from island or product identity.
    pub fn new(value: impl Into<String>) -> Result<Self, ChildWebviewError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_LABEL_BYTES
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-/:_.".contains(&byte))
        {
            return Err(ChildWebviewError::InvalidLabel);
        }
        Ok(Self(value))
    }

    /// Returns the exact transport label.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ChildWebviewLabel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Closed popup policy proved by Card 083.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PopupPolicy {
    /// Reject every child-created window. Consumers may own a richer policy outside this adapter.
    Deny,
}

/// Closed download policy proved by Card 083.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DownloadPolicy {
    /// Reject every download. Consumers may own a richer policy outside this adapter.
    Deny,
}

/// Explicit capability posture for remote child content.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteCapabilityPolicy {
    /// No Tauri capability may match the remote child URL and label.
    NoRemoteCapabilities,
}

/// Consumer-supplied construction and security policy for one child island.
#[derive(Clone)]
pub struct ChildWebviewSpec {
    island_id: NativeContentIslandId,
    host_window_id: WindowId,
    host_window_label: ChildWebviewLabel,
    webview_label: ChildWebviewLabel,
    source: Url,
    data_store_identifier: Option<[u8; 16]>,
    navigation_policy: Arc<dyn Fn(&Url) -> bool + Send + Sync>,
    popup_policy: PopupPolicy,
    download_policy: DownloadPolicy,
    remote_capability_policy: RemoteCapabilityPolicy,
}

impl ChildWebviewSpec {
    /// Creates a complete child construction policy.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        island_id: NativeContentIslandId,
        host_window_id: WindowId,
        host_window_label: ChildWebviewLabel,
        webview_label: ChildWebviewLabel,
        source: Url,
        data_store_identifier: Option<[u8; 16]>,
        navigation_policy: Arc<dyn Fn(&Url) -> bool + Send + Sync>,
        popup_policy: PopupPolicy,
        download_policy: DownloadPolicy,
        remote_capability_policy: RemoteCapabilityPolicy,
    ) -> Result<Self, ChildWebviewError> {
        if !matches!(source.scheme(), "http" | "https") || source.host_str().is_none() {
            return Err(ChildWebviewError::InvalidContentSource);
        }
        Ok(Self {
            island_id,
            host_window_id,
            host_window_label,
            webview_label,
            source,
            data_store_identifier,
            navigation_policy,
            popup_policy,
            download_policy,
            remote_capability_policy,
        })
    }

    /// Returns the shared island identity.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the explicit shared host-window binding.
    #[must_use]
    pub const fn host_window_id(&self) -> &WindowId {
        &self.host_window_id
    }

    /// Returns the mapped Tauri host-window label.
    #[must_use]
    pub const fn host_window_label(&self) -> &ChildWebviewLabel {
        &self.host_window_label
    }

    /// Returns the independent Tauri child-webview label.
    #[must_use]
    pub const fn webview_label(&self) -> &ChildWebviewLabel {
        &self.webview_label
    }

    /// Returns the consumer-supplied external content source.
    #[must_use]
    pub const fn source(&self) -> &Url {
        &self.source
    }

    /// Returns the optional consumer-supplied data-store identity.
    #[must_use]
    pub const fn data_store_identifier(&self) -> Option<[u8; 16]> {
        self.data_store_identifier
    }

    /// Evaluates injected navigation policy.
    #[must_use]
    pub fn allows_navigation(&self, url: &Url) -> bool {
        (self.navigation_policy)(url)
    }

    /// Returns popup policy.
    #[must_use]
    pub const fn popup_policy(&self) -> PopupPolicy {
        self.popup_policy
    }

    /// Returns download policy.
    #[must_use]
    pub const fn download_policy(&self) -> DownloadPolicy {
        self.download_policy
    }

    /// Returns declared remote capability posture.
    #[must_use]
    pub const fn remote_capability_policy(&self) -> RemoteCapabilityPolicy {
        self.remote_capability_policy
    }
}
