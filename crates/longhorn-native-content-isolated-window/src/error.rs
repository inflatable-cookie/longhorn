use std::{error::Error, fmt};

use longhorn_native_content::{
    AttachGeneration, CoordinationError, NativeContentIslandId, ReceiptError,
};

/// Failure from isolated-window validation, execution, or evidence admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IsolatedWindowError {
    /// The plan belongs to another island.
    ForeignIsland {
        /// Adapter island.
        expected: NativeContentIslandId,
        /// Supplied plan island.
        supplied: NativeContentIslandId,
    },
    /// The plan contains another native-content mechanism.
    WrongMechanism,
    /// Work or evidence names an older attach generation.
    StaleGeneration {
        /// Current adapter generation.
        current: AttachGeneration,
        /// Supplied older generation.
        supplied: AttachGeneration,
    },
    /// Work names a generation beyond the next legal attachment.
    FutureGeneration {
        /// Current adapter generation.
        current: AttachGeneration,
        /// Supplied future generation.
        supplied: AttachGeneration,
    },
    /// A new generation was requested while the current owner remained live.
    CurrentGenerationAttached(AttachGeneration),
    /// A completed or invalidated generation cannot attach again.
    GenerationRetired(AttachGeneration),
    /// No owner exists for an operation that requires one.
    NotAttached,
    /// Owner launch is reserved but has not returned a usable handle.
    AttachInProgress,
    /// The mapped host differs from the plan host.
    HostBindingMismatch,
    /// Isolated native content requires native-direct input.
    UnsupportedInputMode,
    /// Isolated native content requires owner-process teardown.
    UnsupportedDetachPolicy,
    /// The current owner generation failed terminally.
    FailedGeneration,
    /// A helper repeated a correlation identity in one generation.
    DuplicateCorrelation,
    /// The bounded pending content-request queue is full.
    RequestCapacity,
    /// A resize decision was attempted for another request category.
    NotResizeRequest,
    /// An injected runtime operation failed.
    Runtime {
        /// Stable operation category.
        operation: &'static str,
        /// Adapter-local diagnostic detail.
        detail: String,
    },
    /// Pure coordination authority rejected a proposal.
    Coordination(CoordinationError),
    /// Current coordinator authority rejected plan or execution evidence.
    Receipt(ReceiptError),
    /// Internal adapter state was poisoned.
    Poisoned,
}

impl IsolatedWindowError {
    /// Returns a bounded failure code suitable for apply evidence.
    #[must_use]
    pub fn failure_code(&self) -> &'static str {
        match self {
            Self::ForeignIsland { .. } => "isolated:foreign-island",
            Self::WrongMechanism => "isolated:wrong-mechanism",
            Self::StaleGeneration { .. } => "isolated:stale-generation",
            Self::FutureGeneration { .. } => "isolated:future-generation",
            Self::CurrentGenerationAttached(_) => "isolated:generation-attached",
            Self::GenerationRetired(_) => "isolated:generation-retired",
            Self::NotAttached => "isolated:not-attached",
            Self::AttachInProgress => "isolated:attach-in-progress",
            Self::HostBindingMismatch => "isolated:host-binding",
            Self::UnsupportedInputMode => "isolated:input-mode",
            Self::UnsupportedDetachPolicy => "isolated:detach-policy",
            Self::FailedGeneration => "isolated:helper-lost",
            Self::DuplicateCorrelation => "isolated:duplicate-correlation",
            Self::RequestCapacity => "isolated:request-capacity",
            Self::NotResizeRequest => "isolated:not-resize-request",
            Self::Runtime { operation, .. } => match *operation {
                "attach" => "isolated:attach-failed",
                "size" => "isolated:size-failed",
                "show" => "isolated:show-failed",
                "hide" => "isolated:hide-failed",
                "focus" => "isolated:focus-failed",
                "release_focus" => "isolated:release-focus-failed",
                "resize_hint" => "isolated:resize-hint-failed",
                "observe" => "isolated:observe-failed",
                "teardown" => "isolated:teardown-failed",
                _ => "isolated:runtime-failed",
            },
            Self::Coordination(_) => "isolated:coordination-rejected",
            Self::Receipt(_) => "isolated:receipt-rejected",
            Self::Poisoned => "isolated:poisoned",
        }
    }
}

impl fmt::Display for IsolatedWindowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignIsland { expected, supplied } => {
                write!(
                    formatter,
                    "plan island {supplied} does not match {expected}"
                )
            }
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
            Self::GenerationRetired(generation) => {
                write!(formatter, "generation {} is retired", generation.get())
            }
            Self::NotAttached => formatter.write_str("isolated content is not attached"),
            Self::AttachInProgress => formatter.write_str("isolated owner launch is in progress"),
            Self::HostBindingMismatch => formatter.write_str("host binding does not match mapping"),
            Self::UnsupportedInputMode => {
                formatter.write_str("isolated content requires native-direct input")
            }
            Self::UnsupportedDetachPolicy => {
                formatter.write_str("isolated content requires owner-process teardown")
            }
            Self::FailedGeneration => formatter.write_str("isolated owner generation failed"),
            Self::DuplicateCorrelation => {
                formatter.write_str("helper repeated a request correlation identity")
            }
            Self::RequestCapacity => {
                formatter.write_str("pending content-request capacity reached")
            }
            Self::NotResizeRequest => formatter.write_str("content request is not a resize"),
            Self::Runtime { operation, detail } => {
                write!(formatter, "isolated runtime {operation} failed: {detail}")
            }
            Self::Coordination(error) => write!(formatter, "coordination rejected: {error}"),
            Self::Receipt(error) => write!(formatter, "apply receipt rejected: {error}"),
            Self::Poisoned => formatter.write_str("isolated-window adapter state is poisoned"),
        }
    }
}

impl Error for IsolatedWindowError {}

impl From<CoordinationError> for IsolatedWindowError {
    fn from(value: CoordinationError) -> Self {
        Self::Coordination(value)
    }
}

impl From<ReceiptError> for IsolatedWindowError {
    fn from(value: ReceiptError) -> Self {
        Self::Receipt(value)
    }
}
