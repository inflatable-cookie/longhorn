use std::{error::Error, fmt};

use longhorn_native_content_prototype::{AttachGeneration, NativeContentIslandId};

/// Failure from the private child-webview mechanism adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChildWebviewError {
    /// A label is empty, oversized, or outside the bounded transport grammar.
    InvalidLabel,
    /// A source URL is not HTTP or HTTPS.
    InvalidContentSource,
    /// The apply plan belongs to another island.
    ForeignIsland {
        /// Adapter-owned island.
        expected: NativeContentIslandId,
        /// Plan-owned island.
        supplied: NativeContentIslandId,
    },
    /// The plan contains a non-child mechanism or operation.
    WrongMechanism,
    /// Evidence or work names an older attach generation.
    StaleGeneration {
        /// Current generation.
        current: AttachGeneration,
        /// Supplied older generation.
        supplied: AttachGeneration,
    },
    /// Work names a generation that has not been attached.
    FutureGeneration {
        /// Current generation.
        current: AttachGeneration,
        /// Supplied future generation.
        supplied: AttachGeneration,
    },
    /// A new generation was requested while the current child remained live.
    CurrentGenerationAttached(AttachGeneration),
    /// No child exists for an operation that requires one.
    NotAttached,
    /// The child is reserved but its native handle is not ready.
    AttachInProgress,
    /// The plan's host binding differs from the explicit transport mapping.
    HostBindingMismatch,
    /// Only native-direct input is valid for this mechanism.
    UnsupportedInputMode,
    /// Tauri exposes focus request but no owned-focus release operation.
    UnsupportedFocusRelease,
    /// Detach must use the reversible child-view policy.
    UnsupportedDetachPolicy,
    /// macOS data-store identity was requested on another target.
    UnsupportedDataStorePolicy,
    /// A native host call failed.
    Native {
        /// Stable operation category.
        operation: &'static str,
        /// Adapter-local diagnostic detail.
        detail: String,
    },
    /// The shared receipt rejected internally inconsistent execution evidence.
    InvalidReceipt(String),
    /// Internal adapter state was poisoned.
    Poisoned,
}

impl ChildWebviewError {
    /// Returns a stable receipt code without exposing native diagnostics.
    #[must_use]
    pub fn failure_code(&self) -> &'static str {
        match self {
            Self::InvalidLabel => "child:invalid-label",
            Self::InvalidContentSource => "child:invalid-source",
            Self::ForeignIsland { .. } => "child:foreign-island",
            Self::WrongMechanism => "child:wrong-mechanism",
            Self::StaleGeneration { .. } => "child:stale-generation",
            Self::FutureGeneration { .. } => "child:future-generation",
            Self::CurrentGenerationAttached(_) => "child:generation-attached",
            Self::NotAttached => "child:not-attached",
            Self::AttachInProgress => "child:attach-in-progress",
            Self::HostBindingMismatch => "child:host-binding",
            Self::UnsupportedInputMode => "child:input-mode",
            Self::UnsupportedFocusRelease => "child:focus-release",
            Self::UnsupportedDetachPolicy => "child:detach-policy",
            Self::UnsupportedDataStorePolicy => "child:data-store-policy",
            Self::Native { operation, .. } => match *operation {
                "attach" => "child:attach-failed",
                "bounds" => "child:bounds-failed",
                "show" => "child:show-failed",
                "hide" => "child:hide-failed",
                "focus" => "child:focus-failed",
                "close" => "child:close-failed",
                "observe" => "child:observe-failed",
                "evaluate" => "child:evaluate-failed",
                _ => "child:native-failed",
            },
            Self::InvalidReceipt(_) => "child:invalid-receipt",
            Self::Poisoned => "child:poisoned",
        }
    }
}

impl fmt::Display for ChildWebviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLabel => formatter.write_str("invalid bounded child-webview label"),
            Self::InvalidContentSource => {
                formatter.write_str("child content source must be HTTP or HTTPS")
            }
            Self::ForeignIsland { expected, supplied } => write!(
                formatter,
                "plan island {} does not match {}",
                supplied.as_str(),
                expected.as_str()
            ),
            Self::WrongMechanism => formatter.write_str("plan is not child-view-only"),
            Self::StaleGeneration { current, supplied } => write!(
                formatter,
                "stale generation {}; current is {}",
                supplied.get(),
                current.get()
            ),
            Self::FutureGeneration { current, supplied } => write!(
                formatter,
                "future generation {}; current is {}",
                supplied.get(),
                current.get()
            ),
            Self::CurrentGenerationAttached(generation) => write!(
                formatter,
                "generation {} remains attached",
                generation.get()
            ),
            Self::NotAttached => formatter.write_str("child webview is not attached"),
            Self::AttachInProgress => formatter.write_str("child webview attach is in progress"),
            Self::HostBindingMismatch => formatter.write_str("host binding does not match mapping"),
            Self::UnsupportedInputMode => {
                formatter.write_str("child webviews require native-direct input")
            }
            Self::UnsupportedFocusRelease => {
                formatter.write_str("owned focus release is not supported")
            }
            Self::UnsupportedDetachPolicy => {
                formatter.write_str("child webviews require reversible detach")
            }
            Self::UnsupportedDataStorePolicy => {
                formatter.write_str("custom data-store identity is unsupported on this target")
            }
            Self::Native { operation, detail } => {
                write!(formatter, "native {operation} failed: {detail}")
            }
            Self::InvalidReceipt(detail) => write!(formatter, "invalid apply receipt: {detail}"),
            Self::Poisoned => formatter.write_str("child-webview adapter state is poisoned"),
        }
    }
}

impl Error for ChildWebviewError {}
