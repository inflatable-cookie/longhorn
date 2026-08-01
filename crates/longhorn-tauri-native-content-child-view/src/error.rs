use std::{error::Error, fmt};

use longhorn_native_content::{AttachGeneration, NativeContentIslandId, ReceiptError};

/// Failure from child-view validation, runtime execution, or receipt admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChildViewError {
    /// A Tauri label is empty, oversized, or outside the bounded grammar.
    InvalidLabel,
    /// Host and child transport labels must differ.
    DuplicateTransportLabel,
    /// The initial child source is not a remote HTTP or HTTPS URL.
    InvalidContentSource,
    /// The plan belongs to another island.
    ForeignIsland {
        /// Adapter island.
        expected: NativeContentIslandId,
        /// Supplied plan island.
        supplied: NativeContentIslandId,
    },
    /// The plan contains another mechanism's operation.
    WrongMechanism,
    /// Evidence or work names an older generation.
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
    /// A new generation was requested while the current child remained live.
    CurrentGenerationAttached(AttachGeneration),
    /// A successfully closed or invalidated generation cannot attach again.
    GenerationRetired(AttachGeneration),
    /// No child exists for an operation that requires one.
    NotAttached,
    /// The child is reserved but native construction has not completed.
    AttachInProgress,
    /// The plan host differs from the injected label mapping.
    HostBindingMismatch,
    /// Only native-direct or disabled input is supported.
    UnsupportedInputMode,
    /// Portable owned-focus release is unavailable.
    UnsupportedFocusRelease,
    /// Child views require reversible detach.
    UnsupportedDetachPolicy,
    /// macOS data-store identity was requested on another target.
    UnsupportedDataStorePolicy,
    /// A native runtime call failed.
    Native {
        /// Stable operation category.
        operation: &'static str,
        /// Adapter-local diagnostic detail.
        detail: String,
    },
    /// Current coordinator authority rejected the plan or execution evidence.
    Receipt(ReceiptError),
    /// Internal adapter state was poisoned.
    Poisoned,
}

impl ChildViewError {
    /// Returns a bounded failure code suitable for `StepExecution` evidence.
    #[must_use]
    pub fn failure_code(&self) -> &'static str {
        match self {
            Self::InvalidLabel => "child:invalid-label",
            Self::DuplicateTransportLabel => "child:duplicate-label",
            Self::InvalidContentSource => "child:invalid-source",
            Self::ForeignIsland { .. } => "child:foreign-island",
            Self::WrongMechanism => "child:wrong-mechanism",
            Self::StaleGeneration { .. } => "child:stale-generation",
            Self::FutureGeneration { .. } => "child:future-generation",
            Self::CurrentGenerationAttached(_) => "child:generation-attached",
            Self::GenerationRetired(_) => "child:generation-retired",
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
                _ => "child:native-failed",
            },
            Self::Receipt(_) => "child:receipt-rejected",
            Self::Poisoned => "child:poisoned",
        }
    }
}

impl fmt::Display for ChildViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLabel => formatter.write_str("invalid bounded child-view label"),
            Self::DuplicateTransportLabel => {
                formatter.write_str("host and child transport labels must differ")
            }
            Self::InvalidContentSource => {
                formatter.write_str("child content source must be remote HTTP or HTTPS")
            }
            Self::ForeignIsland { expected, supplied } => {
                write!(
                    formatter,
                    "plan island {supplied} does not match {expected}"
                )
            }
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
            Self::NotAttached => formatter.write_str("child view is not attached"),
            Self::AttachInProgress => formatter.write_str("child view attach is in progress"),
            Self::HostBindingMismatch => formatter.write_str("host binding does not match mapping"),
            Self::UnsupportedInputMode => {
                formatter.write_str("child views support native-direct or disabled input")
            }
            Self::UnsupportedFocusRelease => {
                formatter.write_str("owned focus release is not supported")
            }
            Self::UnsupportedDetachPolicy => {
                formatter.write_str("child views require reversible detach")
            }
            Self::UnsupportedDataStorePolicy => {
                formatter.write_str("custom data-store identity is unsupported on this target")
            }
            Self::Native { operation, detail } => {
                write!(formatter, "native {operation} failed: {detail}")
            }
            Self::Receipt(error) => write!(formatter, "apply receipt rejected: {error}"),
            Self::Poisoned => formatter.write_str("child-view adapter state is poisoned"),
        }
    }
}

impl Error for ChildViewError {}

impl From<ReceiptError> for ChildViewError {
    fn from(value: ReceiptError) -> Self {
        Self::Receipt(value)
    }
}
