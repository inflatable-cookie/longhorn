use serde::{Deserialize, Serialize};

use crate::{CoordinationError, ReceiptError};

/// Stable reason a renderer protocol request was rejected.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum NativeContentRejectionCode {
    /// Request used an unsupported exact protocol line.
    UnsupportedProtocolVersion,
    /// Request named another island authority.
    IslandMismatch,
    /// Request belongs to an older renderer session.
    StaleClientEpoch,
    /// Request claims a renderer session not issued by this host.
    FutureClientEpoch,
    /// The client-epoch counter cannot advance.
    ClientEpochExhausted,
    /// Expected desired or observed revision is stale.
    StaleRevision,
    /// Attach generation is stale.
    StaleAttachGeneration,
    /// Attach generation is from the future.
    FutureAttachGeneration,
    /// Attach generation skipped a required value.
    AttachGenerationGap,
    /// The current generation remains attached.
    GenerationStillAttached,
    /// Host changes require a new attach generation.
    HostChangeRequiresGeneration,
    /// The supplied host does not own the island.
    HostBindingMismatch,
    /// Observed lifecycle transition is illegal.
    IllegalLifecycleTransition,
    /// The generation failed terminally.
    TerminalGeneration,
    /// Host destruction invalidated the generation.
    InvalidatedGeneration,
    /// Native lifecycle is currently busy.
    LifecycleBusy,
    /// Observation geometry contradicts mechanism capabilities.
    GeometryMechanismMismatch,
    /// Visibility was reported without declared observation support.
    UnsupportedVisibilityObservation,
    /// Focus was reported without declared observation support.
    UnsupportedFocusObservation,
    /// Input routing exceeds declared mechanism capabilities.
    UnsupportedInputRouting,
    /// Readiness was reported without attachment.
    ReadinessWithoutAttachment,
    /// An absent observation retained native-only evidence.
    AbsentWithNativeEvidence,
    /// Content-size proposals are disabled.
    ContentSizeRequestsUnsupported,
    /// Viewport geometry could not be converted safely.
    ViewportConversion,
    /// A native-content revision is exhausted.
    RevisionExhausted,
    /// An attach generation is exhausted.
    AttachGenerationExhausted,
    /// Apply receipt names another island.
    ReceiptIslandMismatch,
    /// Apply receipt names stale desired evidence.
    ReceiptStaleDesired,
    /// Apply receipt names stale observed evidence.
    ReceiptStaleObserved,
    /// Apply receipt names a non-current generation.
    ReceiptInvalidGeneration,
    /// Apply receipt names an unknown step.
    ReceiptUnknownStep,
    /// Apply receipt repeats a step.
    ReceiptDuplicateStep,
    /// Apply receipt reports work after a failed dependency.
    ReceiptBlockedDependency,
}

/// Phase in which a protocol rejection occurred.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum NativeContentFailurePhase {
    /// Exact wire compatibility or envelope validation.
    Compatibility,
    /// Island or renderer-session authority admission.
    Admission,
    /// Pure native-content coordination.
    Coordination,
}

/// Whether retry requires new authoritative state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum NativeContentRetryClass {
    /// Repeating the same request cannot succeed.
    Never,
    /// Reconnect or load a fresh snapshot before rebuilding the request.
    Refresh,
}

/// Stable typed rejection without product or mechanism payloads.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct NativeContentProtocolRejection {
    /// Stable rejection category.
    pub code: NativeContentRejectionCode,
    /// Human-readable diagnostic derived from bounded protocol state.
    pub message: String,
    /// Failure phase.
    pub phase: NativeContentFailurePhase,
    /// Retry classification.
    pub retry: NativeContentRetryClass,
}

impl NativeContentProtocolRejection {
    pub(super) fn compatibility(message: impl Into<String>) -> Self {
        Self {
            code: NativeContentRejectionCode::UnsupportedProtocolVersion,
            message: message.into(),
            phase: NativeContentFailurePhase::Compatibility,
            retry: NativeContentRetryClass::Never,
        }
    }

    pub(super) fn admission(code: NativeContentRejectionCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            phase: NativeContentFailurePhase::Admission,
            retry: NativeContentRetryClass::Refresh,
        }
    }
}

impl From<CoordinationError> for NativeContentProtocolRejection {
    fn from(error: CoordinationError) -> Self {
        let code = match &error {
            CoordinationError::StaleRevision { .. } => NativeContentRejectionCode::StaleRevision,
            CoordinationError::StaleGeneration { .. } => {
                NativeContentRejectionCode::StaleAttachGeneration
            }
            CoordinationError::FutureGeneration { .. } => {
                NativeContentRejectionCode::FutureAttachGeneration
            }
            CoordinationError::GenerationGap { .. } => {
                NativeContentRejectionCode::AttachGenerationGap
            }
            CoordinationError::GenerationStillAttached(_) => {
                NativeContentRejectionCode::GenerationStillAttached
            }
            CoordinationError::HostChangeRequiresGeneration => {
                NativeContentRejectionCode::HostChangeRequiresGeneration
            }
            CoordinationError::HostBindingMismatch { .. } => {
                NativeContentRejectionCode::HostBindingMismatch
            }
            CoordinationError::IllegalLifecycleTransition { .. } => {
                NativeContentRejectionCode::IllegalLifecycleTransition
            }
            CoordinationError::TerminalGeneration(_) => {
                NativeContentRejectionCode::TerminalGeneration
            }
            CoordinationError::InvalidatedGeneration(_) => {
                NativeContentRejectionCode::InvalidatedGeneration
            }
            CoordinationError::LifecycleBusy(_) => NativeContentRejectionCode::LifecycleBusy,
            CoordinationError::GeometryMechanismMismatch { .. } => {
                NativeContentRejectionCode::GeometryMechanismMismatch
            }
            CoordinationError::UnsupportedVisibilityObservation => {
                NativeContentRejectionCode::UnsupportedVisibilityObservation
            }
            CoordinationError::UnsupportedFocusObservation => {
                NativeContentRejectionCode::UnsupportedFocusObservation
            }
            CoordinationError::UnsupportedInputRouting { .. } => {
                NativeContentRejectionCode::UnsupportedInputRouting
            }
            CoordinationError::ReadinessWithoutAttachment => {
                NativeContentRejectionCode::ReadinessWithoutAttachment
            }
            CoordinationError::AbsentWithNativeEvidence => {
                NativeContentRejectionCode::AbsentWithNativeEvidence
            }
            CoordinationError::ContentSizeRequestsUnsupported => {
                NativeContentRejectionCode::ContentSizeRequestsUnsupported
            }
            CoordinationError::ViewportConversion(_) => {
                NativeContentRejectionCode::ViewportConversion
            }
            CoordinationError::RevisionOverflow => NativeContentRejectionCode::RevisionExhausted,
            CoordinationError::GenerationOverflow => {
                NativeContentRejectionCode::AttachGenerationExhausted
            }
        };
        Self {
            code,
            message: error.to_string(),
            phase: NativeContentFailurePhase::Coordination,
            retry: NativeContentRetryClass::Refresh,
        }
    }
}

impl From<ReceiptError> for NativeContentProtocolRejection {
    fn from(error: ReceiptError) -> Self {
        let code = match &error {
            ReceiptError::IslandMismatch { .. } => {
                NativeContentRejectionCode::ReceiptIslandMismatch
            }
            ReceiptError::StaleDesiredPlan { .. } => {
                NativeContentRejectionCode::ReceiptStaleDesired
            }
            ReceiptError::StaleObservedPlan { .. } => {
                NativeContentRejectionCode::ReceiptStaleObserved
            }
            ReceiptError::InvalidGeneration { .. } => {
                NativeContentRejectionCode::ReceiptInvalidGeneration
            }
            ReceiptError::UnknownStep(_) => NativeContentRejectionCode::ReceiptUnknownStep,
            ReceiptError::DuplicateStep(_) => NativeContentRejectionCode::ReceiptDuplicateStep,
            ReceiptError::ExecutedAfterBlockedDependency { .. } => {
                NativeContentRejectionCode::ReceiptBlockedDependency
            }
        };
        Self {
            code,
            message: error.to_string(),
            phase: NativeContentFailurePhase::Coordination,
            retry: NativeContentRetryClass::Refresh,
        }
    }
}
