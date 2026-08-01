use std::{error::Error, fmt};

use longhorn_core::{NativeContentIslandId, NativeContentRevision, WindowId};

use crate::{
    AttachGeneration, AttachmentLifecycle, InputRoutingMode, NativeContentMechanism, PlanStepId,
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
    /// Desired state attempted to skip an attach generation.
    GenerationGap {
        /// Current attach generation.
        current: AttachGeneration,
        /// Supplied generation.
        supplied: AttachGeneration,
    },
    /// A new generation was requested while the current native instance lived.
    GenerationStillAttached(AttachmentLifecycle),
    /// Host binding changed without a new attach generation.
    HostChangeRequiresGeneration,
    /// A host lifecycle event named a window other than the current binding.
    HostBindingMismatch {
        /// Current host binding.
        current: WindowId,
        /// Supplied host binding.
        supplied: WindowId,
    },
    /// One observed lifecycle transition is illegal.
    IllegalLifecycleTransition {
        /// Current lifecycle.
        current: AttachmentLifecycle,
        /// Proposed lifecycle.
        proposed: AttachmentLifecycle,
    },
    /// A terminal failed generation must be replaced before attaching again.
    TerminalGeneration(AttachGeneration),
    /// Host destruction invalidated this generation.
    InvalidatedGeneration(AttachGeneration),
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
    /// Desired or observed input routing exceeds the mechanism descriptor.
    UnsupportedInputRouting {
        /// Active route declared by the mechanism.
        supported: InputRoutingMode,
        /// Supplied route.
        supplied: InputRoutingMode,
    },
    /// Readiness was reported outside an attached lifecycle.
    ReadinessWithoutAttachment,
    /// An absent lifecycle retained native-only evidence.
    AbsentWithNativeEvidence,
    /// Content-size proposals are disabled for this mechanism instance.
    ContentSizeRequestsUnsupported,
    /// The desired viewport could not convert to physical geometry.
    ViewportConversion(ViewportConversionError),
    /// A revision could not advance.
    RevisionOverflow,
    /// An attach generation could not advance.
    GenerationOverflow,
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
                "stale attach generation {}; current is {}",
                supplied.get(),
                current.get()
            ),
            Self::FutureGeneration { current, supplied } => write!(
                formatter,
                "future attach generation {}; current is {}",
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
            Self::HostChangeRequiresGeneration => {
                formatter.write_str("host binding changes require a new attach generation")
            }
            Self::HostBindingMismatch { current, supplied } => write!(
                formatter,
                "host binding {supplied} does not match current host {current}"
            ),
            Self::IllegalLifecycleTransition { current, proposed } => write!(
                formatter,
                "illegal lifecycle transition {current:?} -> {proposed:?}"
            ),
            Self::TerminalGeneration(generation) => write!(
                formatter,
                "failed attach generation {} must be replaced",
                generation.get()
            ),
            Self::InvalidatedGeneration(generation) => write!(
                formatter,
                "attach generation {} was invalidated by host destruction",
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
            Self::UnsupportedInputRouting {
                supported,
                supplied,
            } => write!(
                formatter,
                "input route {supplied:?} is unsupported; active route is {supported:?}"
            ),
            Self::ReadinessWithoutAttachment => {
                formatter.write_str("readiness can be ready only while attached")
            }
            Self::AbsentWithNativeEvidence => formatter
                .write_str("absent lifecycle cannot retain native geometry or input evidence"),
            Self::ContentSizeRequestsUnsupported => {
                formatter.write_str("content-size proposals are unsupported")
            }
            Self::ViewportConversion(error) => error.fmt(formatter),
            Self::RevisionOverflow => formatter.write_str("native-content revision overflow"),
            Self::GenerationOverflow => formatter.write_str("attach generation overflow"),
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

/// Failure to reconcile reported executions with a current immutable plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceiptError {
    /// The plan belongs to another island.
    IslandMismatch {
        /// Current island.
        current: NativeContentIslandId,
        /// Plan island.
        supplied: NativeContentIslandId,
    },
    /// The plan uses an old desired revision.
    StaleDesiredPlan {
        /// Current desired revision.
        current: NativeContentRevision,
        /// Planned desired revision.
        supplied: NativeContentRevision,
    },
    /// The plan uses an old observed revision.
    StaleObservedPlan {
        /// Current observed revision.
        current: NativeContentRevision,
        /// Planned observed revision.
        supplied: NativeContentRevision,
    },
    /// The plan belongs to another or invalidated generation.
    InvalidGeneration {
        /// Current desired generation.
        current: AttachGeneration,
        /// Planned generation.
        supplied: AttachGeneration,
    },
    /// A report names a step absent from the plan.
    UnknownStep(PlanStepId),
    /// More than one report names the same step.
    DuplicateStep(PlanStepId),
    /// A reported step ran after a dependency failed or skipped.
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
            Self::IslandMismatch { current, supplied } => {
                write!(formatter, "plan island {supplied} does not match {current}")
            }
            Self::StaleDesiredPlan { current, supplied } => write!(
                formatter,
                "planned desired revision {} is stale; current is {}",
                supplied.get(),
                current.get()
            ),
            Self::StaleObservedPlan { current, supplied } => write!(
                formatter,
                "planned observed revision {} is stale; current is {}",
                supplied.get(),
                current.get()
            ),
            Self::InvalidGeneration { current, supplied } => write!(
                formatter,
                "planned generation {} is not current generation {}",
                supplied.get(),
                current.get()
            ),
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
