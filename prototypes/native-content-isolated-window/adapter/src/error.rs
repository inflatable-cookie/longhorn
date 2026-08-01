use std::{error::Error, fmt};

use longhorn_native_content_prototype::{AttachGeneration, NativeContentIslandId};

/// Failure from the private isolated native-window mechanism adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IsolatedWindowError {
    /// The apply plan belongs to another island.
    ForeignIsland {
        /// Adapter-owned island.
        expected: NativeContentIslandId,
        /// Plan-owned island.
        supplied: NativeContentIslandId,
    },
    /// The plan contains another native-content mechanism.
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
    /// A new generation was requested while the current helper remained live.
    CurrentGenerationAttached(AttachGeneration),
    /// No helper exists for an operation that requires one.
    NotAttached,
    /// The helper is reserved but has not returned a usable handle.
    AttachInProgress,
    /// The plan's host binding differs from the explicit island mapping.
    HostBindingMismatch,
    /// Isolated native content requires native-direct input.
    UnsupportedInputMode,
    /// Detach must use disposable owner-process policy.
    UnsupportedDetachPolicy,
    /// The current generation has already failed terminally.
    FailedGeneration,
    /// The selected runtime failed an operation.
    Runtime {
        /// Stable operation category.
        operation: &'static str,
        /// Adapter-local diagnostic detail.
        detail: String,
    },
    /// The shared receipt rejected inconsistent execution evidence.
    InvalidReceipt(String),
    /// Internal adapter state was poisoned.
    Poisoned,
}

impl IsolatedWindowError {
    /// Returns a stable receipt code without exposing runtime diagnostics.
    #[must_use]
    pub fn failure_code(&self) -> &'static str {
        match self {
            Self::ForeignIsland { .. } => "isolated:foreign-island",
            Self::WrongMechanism => "isolated:wrong-mechanism",
            Self::StaleGeneration { .. } => "isolated:stale-generation",
            Self::FutureGeneration { .. } => "isolated:future-generation",
            Self::CurrentGenerationAttached(_) => "isolated:generation-attached",
            Self::NotAttached => "isolated:not-attached",
            Self::AttachInProgress => "isolated:attach-in-progress",
            Self::HostBindingMismatch => "isolated:host-binding",
            Self::UnsupportedInputMode => "isolated:input-mode",
            Self::UnsupportedDetachPolicy => "isolated:detach-policy",
            Self::FailedGeneration => "isolated:helper-lost",
            Self::Runtime { operation, .. } => match *operation {
                "attach" => "isolated:attach-failed",
                "size" => "isolated:size-failed",
                "show" => "isolated:show-failed",
                "hide" => "isolated:hide-failed",
                "focus" => "isolated:focus-failed",
                "release_focus" => "isolated:release-focus-failed",
                "resize_hint" => "isolated:resize-hint-failed",
                "teardown" => "isolated:teardown-failed",
                "observe" => "isolated:observe-failed",
                "script" => "isolated:script-failed",
                _ => "isolated:runtime-failed",
            },
            Self::InvalidReceipt(_) => "isolated:invalid-receipt",
            Self::Poisoned => "isolated:poisoned",
        }
    }
}

impl fmt::Display for IsolatedWindowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignIsland { expected, supplied } => write!(
                formatter,
                "plan island {} does not match {}",
                supplied.as_str(),
                expected.as_str()
            ),
            Self::WrongMechanism => formatter.write_str("plan is not isolated-window-only"),
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
            Self::CurrentGenerationAttached(generation) => {
                write!(
                    formatter,
                    "generation {} remains attached",
                    generation.get()
                )
            }
            Self::NotAttached => formatter.write_str("isolated window is not attached"),
            Self::AttachInProgress => formatter.write_str("isolated window attach is in progress"),
            Self::HostBindingMismatch => formatter.write_str("host binding does not match mapping"),
            Self::UnsupportedInputMode => {
                formatter.write_str("isolated native windows require native-direct input")
            }
            Self::UnsupportedDetachPolicy => {
                formatter.write_str("isolated window requires owner-process termination policy")
            }
            Self::FailedGeneration => formatter.write_str("isolated helper generation failed"),
            Self::Runtime { operation, detail } => {
                write!(formatter, "isolated runtime {operation} failed: {detail}")
            }
            Self::InvalidReceipt(detail) => write!(formatter, "invalid apply receipt: {detail}"),
            Self::Poisoned => formatter.write_str("isolated-window adapter state is poisoned"),
        }
    }
}

impl Error for IsolatedWindowError {}
