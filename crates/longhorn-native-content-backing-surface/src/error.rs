use std::{error::Error, fmt};

use longhorn_native_content::{
    AttachGeneration, GenerationRejection, NativeContentIslandId, ReceiptError,
};

/// Failure from backing-surface validation, execution, or evidence admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackingSurfaceError {
    /// The plan or event belongs to another island.
    ForeignIsland {
        /// Adapter island.
        expected: NativeContentIslandId,
        /// Supplied island.
        supplied: NativeContentIslandId,
    },
    /// The plan contains another mechanism's operation.
    WrongMechanism,
    /// Work or evidence names an older generation.
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
    /// A new generation was requested while current storage remained live.
    CurrentGenerationAttached(AttachGeneration),
    /// A detached generation cannot attach again.
    GenerationRetired(AttachGeneration),
    /// Host destruction already invalidated this generation.
    GenerationInvalidated(AttachGeneration),
    /// A runtime callback sequence did not advance current evidence.
    StaleEventSequence {
        /// Last admitted runtime event sequence.
        current: u64,
        /// Supplied duplicate or older sequence.
        supplied: u64,
    },
    /// A runtime result carried older renderer evidence.
    StaleFrameSequence {
        /// Current renderer frame sequence.
        current: u64,
        /// Supplied older frame sequence.
        supplied: u64,
    },
    /// No backing storage exists for the requested operation.
    NotAttached,
    /// Native storage attachment is reserved but incomplete.
    AttachInProgress,
    /// The supplied host does not match the injected mapping.
    HostBindingMismatch,
    /// Only renderer-forwarded or disabled input is supported.
    UnsupportedInputMode,
    /// Native focus operations are not backing-surface authority.
    UnsupportedFocusOperation,
    /// This production adapter requires reversible detach.
    UnsupportedDetachPolicy,
    /// An injected storage or renderer operation failed.
    Runtime {
        /// Stable operation category.
        operation: &'static str,
        /// Adapter-local diagnostic detail.
        detail: String,
    },
    /// Current coordinator authority rejected plan or execution evidence.
    Receipt(ReceiptError),
    /// Internal adapter state was poisoned.
    Poisoned,
}

impl BackingSurfaceError {
    /// Returns a bounded failure code suitable for apply evidence.
    #[must_use]
    pub fn failure_code(&self) -> &'static str {
        match self {
            Self::ForeignIsland { .. } => "backing:foreign-island",
            Self::WrongMechanism => "backing:wrong-mechanism",
            Self::StaleGeneration { .. } => "backing:stale-generation",
            Self::FutureGeneration { .. } => "backing:future-generation",
            Self::CurrentGenerationAttached(_) => "backing:generation-attached",
            Self::GenerationRetired(_) => "backing:generation-retired",
            Self::GenerationInvalidated(_) => "backing:generation-invalidated",
            Self::StaleEventSequence { .. } => "backing:stale-event",
            Self::StaleFrameSequence { .. } => "backing:stale-frame",
            Self::NotAttached => "backing:not-attached",
            Self::AttachInProgress => "backing:attach-in-progress",
            Self::HostBindingMismatch => "backing:host-binding",
            Self::UnsupportedInputMode => "backing:input-mode",
            Self::UnsupportedFocusOperation => "backing:focus-operation",
            Self::UnsupportedDetachPolicy => "backing:detach-policy",
            Self::Runtime { operation, .. } => match *operation {
                "attach" => "backing:attach-failed",
                "clip" => "backing:clip-failed",
                "presentation" => "backing:presentation-failed",
                "input" => "backing:input-failed",
                "observe" => "backing:observe-failed",
                "detach" => "backing:detach-failed",
                _ => "backing:runtime-failed",
            },
            Self::Receipt(_) => "backing:receipt-rejected",
            Self::Poisoned => "backing:poisoned",
        }
    }
}

impl fmt::Display for BackingSurfaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignIsland { expected, supplied } => {
                write!(formatter, "island {supplied} does not match {expected}")
            }
            Self::WrongMechanism => formatter.write_str("plan is not backing-surface-only"),
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
            Self::GenerationInvalidated(generation) => write!(
                formatter,
                "generation {} was invalidated by host destruction",
                generation.get()
            ),
            Self::StaleEventSequence { current, supplied } => write!(
                formatter,
                "stale runtime event sequence {supplied}; current is {current}"
            ),
            Self::StaleFrameSequence { current, supplied } => write!(
                formatter,
                "stale renderer frame sequence {supplied}; current is {current}"
            ),
            Self::NotAttached => formatter.write_str("backing storage is not attached"),
            Self::AttachInProgress => formatter.write_str("backing storage attach is in progress"),
            Self::HostBindingMismatch => formatter.write_str("host binding does not match mapping"),
            Self::UnsupportedInputMode => {
                formatter.write_str("backing surfaces support renderer-forwarded or disabled input")
            }
            Self::UnsupportedFocusOperation => {
                formatter.write_str("backing-surface focus remains consumer gate evidence")
            }
            Self::UnsupportedDetachPolicy => {
                formatter.write_str("backing surfaces require reversible detach")
            }
            Self::Runtime { operation, detail } => {
                write!(formatter, "backing runtime {operation} failed: {detail}")
            }
            Self::Receipt(error) => write!(formatter, "apply receipt rejected: {error}"),
            Self::Poisoned => formatter.write_str("backing-surface adapter state is poisoned"),
        }
    }
}

impl Error for BackingSurfaceError {}

impl From<ReceiptError> for BackingSurfaceError {
    fn from(value: ReceiptError) -> Self {
        Self::Receipt(value)
    }
}

impl From<GenerationRejection> for BackingSurfaceError {
    fn from(rejection: GenerationRejection) -> Self {
        match rejection {
            GenerationRejection::Stale { current, supplied } => {
                Self::StaleGeneration { current, supplied }
            }
            GenerationRejection::Future { current, supplied } => {
                Self::FutureGeneration { current, supplied }
            }
            GenerationRejection::Attached(current) => Self::CurrentGenerationAttached(current),
            GenerationRejection::Retired(generation) => Self::GenerationRetired(generation),
            GenerationRejection::Absent => Self::NotAttached,
            GenerationRejection::Attaching => Self::AttachInProgress,
        }
    }
}
