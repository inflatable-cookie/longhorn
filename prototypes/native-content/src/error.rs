use std::{error::Error, fmt};

use crate::{
    AttachGeneration, AttachmentLifecycle, NativeContentMechanism, NativeContentRevision,
    PlanStepId,
};

/// Failure to validate or coordinate native-content state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoordinationError {
    /// A request named an old desired or observed revision.
    StaleRevision {
        /// Current authoritative revision.
        current: NativeContentRevision,
        /// Supplied expected revision.
        supplied: NativeContentRevision,
    },
    /// Evidence belongs to an older attach generation.
    StaleGeneration {
        /// Current desired attach generation.
        current: AttachGeneration,
        /// Supplied attach generation.
        supplied: AttachGeneration,
    },
    /// Evidence claims an attach generation not yet requested.
    FutureGeneration {
        /// Current desired attach generation.
        current: AttachGeneration,
        /// Supplied attach generation.
        supplied: AttachGeneration,
    },
    /// Desired state attempted to skip more than one attach generation.
    GenerationGap {
        /// Current desired attach generation.
        current: AttachGeneration,
        /// Supplied next attach generation.
        supplied: AttachGeneration,
    },
    /// A new generation was requested while the current native instance lived.
    GenerationStillAttached(AttachmentLifecycle),
    /// One observed lifecycle transition is not legal.
    IllegalLifecycleTransition {
        /// Current lifecycle.
        current: AttachmentLifecycle,
        /// Proposed lifecycle.
        proposed: AttachmentLifecycle,
    },
    /// A terminal failed generation must be replaced before attaching again.
    TerminalGeneration(AttachGeneration),
    /// The native host is still attaching or detaching.
    LifecycleBusy(AttachmentLifecycle),
    /// Observed geometry does not match the declared mechanism.
    GeometryMechanismMismatch {
        /// Declared mechanism.
        mechanism: NativeContentMechanism,
    },
    /// An adapter reported visibility despite declaring it unobservable.
    UnsupportedVisibilityObservation,
    /// An adapter reported focus despite declaring it unobservable.
    UnsupportedFocusObservation,
    /// Readiness was reported outside an attached lifecycle.
    ReadinessWithoutAttachment,
    /// An absent lifecycle retained native-only evidence.
    AbsentWithNativeEvidence,
    /// Content-size proposals are disabled for this mechanism instance.
    ContentSizeRequestsUnsupported,
    /// The desired viewport could not convert to physical geometry.
    ViewportConversion(ViewportConversionError),
    /// A revision or generation could not advance.
    CounterOverflow,
}

impl fmt::Display for CoordinationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleRevision { current, supplied } => write!(
                formatter,
                "stale revision {}; current is {}",
                supplied.get(),
                current.get()
            ),
            Self::StaleGeneration { current, supplied } => write!(
                formatter,
                "stale attach generation {}; current desired is {}",
                supplied.get(),
                current.get()
            ),
            Self::FutureGeneration { current, supplied } => write!(
                formatter,
                "future attach generation {}; current desired is {}",
                supplied.get(),
                current.get()
            ),
            Self::GenerationGap { current, supplied } => write!(
                formatter,
                "attach generation cannot jump from {} to {}",
                current.get(),
                supplied.get()
            ),
            Self::GenerationStillAttached(lifecycle) => {
                write!(formatter, "cannot advance generation while {lifecycle:?}")
            }
            Self::IllegalLifecycleTransition { current, proposed } => {
                write!(
                    formatter,
                    "illegal lifecycle transition {current:?} -> {proposed:?}"
                )
            }
            Self::TerminalGeneration(generation) => write!(
                formatter,
                "failed attach generation {} must be replaced",
                generation.get()
            ),
            Self::LifecycleBusy(lifecycle) => {
                write!(formatter, "native-content lifecycle is busy: {lifecycle:?}")
            }
            Self::GeometryMechanismMismatch { mechanism } => {
                write!(formatter, "observed geometry does not match {mechanism:?}")
            }
            Self::UnsupportedVisibilityObservation => {
                formatter.write_str("adapter declared visibility unobservable but reported a value")
            }
            Self::UnsupportedFocusObservation => {
                formatter.write_str("adapter declared focus unobservable but reported a value")
            }
            Self::ReadinessWithoutAttachment => {
                formatter.write_str("readiness can be ready only while attached")
            }
            Self::AbsentWithNativeEvidence => formatter
                .write_str("absent lifecycle cannot retain native geometry or input evidence"),
            Self::ContentSizeRequestsUnsupported => {
                formatter.write_str("content-size proposals are unsupported")
            }
            Self::ViewportConversion(error) => error.fmt(formatter),
            Self::CounterOverflow => formatter.write_str("native-content counter overflow"),
        }
    }
}

impl Error for CoordinationError {}

impl From<ViewportConversionError> for CoordinationError {
    fn from(value: ViewportConversionError) -> Self {
        Self::ViewportConversion(value)
    }
}

/// Failure to convert a finite client viewport to integral physical pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewportConversionError {
    /// A scaled coordinate exceeded signed physical-pixel range.
    CoordinateOverflow,
    /// A scaled extent exceeded unsigned physical-pixel range.
    ExtentOverflow,
}

impl fmt::Display for ViewportConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoordinateOverflow => {
                formatter.write_str("scaled client coordinate overflowed physical pixels")
            }
            Self::ExtentOverflow => {
                formatter.write_str("scaled client extent overflowed physical pixels")
            }
        }
    }
}

impl Error for ViewportConversionError {}

/// Failure to reconcile reported executions with an immutable apply plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceiptError {
    /// A report names a step absent from the plan.
    UnknownStep(PlanStepId),
    /// More than one execution report names the same step.
    DuplicateStep(PlanStepId),
    /// A reported step ran after one of its dependencies failed or skipped.
    ExecutedAfterBlockedDependency {
        /// Reported step.
        step: PlanStepId,
        /// Dependency that did not apply.
        blocked_by: PlanStepId,
    },
}

impl fmt::Display for ReceiptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownStep(step) => write!(formatter, "unknown plan step {}", step.get()),
            Self::DuplicateStep(step) => write!(formatter, "duplicate plan step {}", step.get()),
            Self::ExecutedAfterBlockedDependency { step, blocked_by } => write!(
                formatter,
                "step {} executed after blocked dependency {}",
                step.get(),
                blocked_by.get()
            ),
        }
    }
}

impl Error for ReceiptError {}
