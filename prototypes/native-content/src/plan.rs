use longhorn_core::{PhysicalRect, PhysicalSize, WindowId};
use serde::{Deserialize, Serialize};

use crate::{
    AttachGeneration, AttachmentLifecycle, CoordinationError, DesiredPresence, DesiredState,
    DesiredVisibility, DetachPolicy, EffectiveFocus, EffectiveVisibility, FocusIntent,
    InputRoutingMode, NativeContentIslandId, NativeContentMechanism, NativeContentRevision,
    ObservedGeometry, ObservedState, PlanStepId, VisibilityReasonId, viewport_to_physical,
};

/// One pure native-content host instruction.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum NativeContentOperation {
    /// Attach selected native content to a host window.
    Attach {
        /// Stable outer-window binding.
        host_window_id: WindowId,
        /// Independently selected host mechanism.
        mechanism: NativeContentMechanism,
    },
    /// Apply complete physical bounds to a child view.
    SetChildBounds {
        /// Target physical child bounds.
        bounds: PhysicalRect,
    },
    /// Apply physical content size without owning outer placement.
    SetIsolatedContentSize {
        /// Target physical content size.
        size: PhysicalSize,
    },
    /// Apply presentation and interaction clipping to a backing surface.
    SetBackingViewport {
        /// Target physical clip. Native storage may remain full-host.
        clip: PhysicalRect,
    },
    /// Show native content.
    Show,
    /// Hide native content for an explicit consumer reason.
    Hide {
        /// Consumer-computed visibility inhibitor.
        reason: VisibilityReasonId,
    },
    /// Change only the declared input route.
    SetInputRouting {
        /// Target routing mode. No input payload crosses this operation.
        mode: InputRoutingMode,
    },
    /// Request native focus.
    RequestFocus,
    /// Release focus only when owned by this content.
    ReleaseFocusIfOwned,
    /// Detach under the mechanism's declared policy.
    Detach {
        /// Honest teardown behavior.
        policy: DetachPolicy,
    },
}

/// One ordered operation and its immediate dependency.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlannedOperation {
    step: PlanStepId,
    depends_on: Option<PlanStepId>,
    operation: NativeContentOperation,
}

impl PlannedOperation {
    /// Returns plan-local step identity.
    #[must_use]
    pub const fn step(&self) -> PlanStepId {
        self.step
    }

    /// Returns the preceding operation that must apply first.
    #[must_use]
    pub const fn depends_on(&self) -> Option<PlanStepId> {
        self.depends_on
    }

    /// Returns the pure host operation.
    #[must_use]
    pub const fn operation(&self) -> &NativeContentOperation {
        &self.operation
    }
}

/// Immutable apply plan bound to exact desired and observed evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplyPlan {
    island_id: NativeContentIslandId,
    desired_revision: NativeContentRevision,
    observed_revision: NativeContentRevision,
    generation: AttachGeneration,
    operations: Vec<PlannedOperation>,
}

impl ApplyPlan {
    /// Returns island identity.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns desired revision used to plan.
    #[must_use]
    pub const fn desired_revision(&self) -> NativeContentRevision {
        self.desired_revision
    }

    /// Returns observed revision used to plan.
    #[must_use]
    pub const fn observed_revision(&self) -> NativeContentRevision {
        self.observed_revision
    }

    /// Returns attach generation used to plan.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }

    /// Returns ordered native operations.
    #[must_use]
    pub fn operations(&self) -> &[PlannedOperation] {
        &self.operations
    }

    /// Returns whether fresh observation already converged.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

/// Plans desired native-content state against fresh observed evidence.
pub fn plan_transition(
    desired: &DesiredState,
    observed: &ObservedState,
) -> Result<ApplyPlan, CoordinationError> {
    if observed.generation() > desired.generation() {
        return Err(CoordinationError::FutureGeneration {
            current: desired.generation(),
            supplied: observed.generation(),
        });
    }

    let mut builder = PlanBuilder::new(desired, observed);
    if desired.presence() == DesiredPresence::Absent {
        if observed.lifecycle() != AttachmentLifecycle::Absent {
            builder.push(NativeContentOperation::Detach {
                policy: desired.capabilities().detach_policy(),
            });
        }
        return Ok(builder.finish());
    }

    let old_generation = observed.generation() < desired.generation();
    if old_generation
        && !matches!(
            observed.lifecycle(),
            AttachmentLifecycle::Absent | AttachmentLifecycle::Failed
        )
    {
        return Err(CoordinationError::GenerationStillAttached(
            observed.lifecycle(),
        ));
    }

    let needs_attach = old_generation || observed.lifecycle() == AttachmentLifecycle::Absent;
    if !old_generation {
        match observed.lifecycle() {
            AttachmentLifecycle::Failed => {
                return Err(CoordinationError::TerminalGeneration(desired.generation()));
            }
            AttachmentLifecycle::Attaching | AttachmentLifecycle::Detaching => {
                return Err(CoordinationError::LifecycleBusy(observed.lifecycle()));
            }
            AttachmentLifecycle::Absent | AttachmentLifecycle::Attached => {}
        }
    }

    if needs_attach {
        builder.push(NativeContentOperation::Attach {
            host_window_id: desired.host_window_id().clone(),
            mechanism: desired.capabilities().mechanism(),
        });
    }

    let target = viewport_to_physical(desired.viewport(), desired.scale(), desired.rounding())?;
    if needs_attach || !geometry_matches(desired.capabilities().mechanism(), observed, &target)? {
        builder.push(geometry_operation(
            desired.capabilities().mechanism(),
            target,
        ));
    }

    match desired.visibility() {
        DesiredVisibility::Visible
            if needs_attach || observed.visibility() != EffectiveVisibility::Visible =>
        {
            builder.push(NativeContentOperation::Show);
        }
        DesiredVisibility::Hidden { reason }
            if needs_attach || observed.visibility() != EffectiveVisibility::Hidden =>
        {
            builder.push(NativeContentOperation::Hide {
                reason: reason.clone(),
            });
        }
        DesiredVisibility::Visible | DesiredVisibility::Hidden { .. } => {}
    }

    if needs_attach || observed.input_routing() != Some(desired.input_routing()) {
        builder.push(NativeContentOperation::SetInputRouting {
            mode: desired.input_routing(),
        });
    }

    match desired.focus() {
        FocusIntent::Unchanged => {}
        FocusIntent::Request if needs_attach || observed.focus() != EffectiveFocus::Focused => {
            builder.push(NativeContentOperation::RequestFocus);
        }
        FocusIntent::ReleaseIfOwned
            if needs_attach || observed.focus() != EffectiveFocus::Unfocused =>
        {
            builder.push(NativeContentOperation::ReleaseFocusIfOwned);
        }
        FocusIntent::Request | FocusIntent::ReleaseIfOwned => {}
    }

    Ok(builder.finish())
}

fn geometry_operation(
    mechanism: NativeContentMechanism,
    target: PhysicalRect,
) -> NativeContentOperation {
    match mechanism {
        NativeContentMechanism::ChildView => {
            NativeContentOperation::SetChildBounds { bounds: target }
        }
        NativeContentMechanism::IsolatedWindow => NativeContentOperation::SetIsolatedContentSize {
            size: target.size(),
        },
        NativeContentMechanism::BackingSurface => {
            NativeContentOperation::SetBackingViewport { clip: target }
        }
    }
}

fn geometry_matches(
    mechanism: NativeContentMechanism,
    observed: &ObservedState,
    target: &PhysicalRect,
) -> Result<bool, CoordinationError> {
    match (mechanism, observed.geometry()) {
        (_, ObservedGeometry::Unknown) => Ok(false),
        (NativeContentMechanism::ChildView, ObservedGeometry::ChildBounds { bounds }) => {
            Ok(bounds == target)
        }
        (NativeContentMechanism::IsolatedWindow, ObservedGeometry::IsolatedContent { size }) => {
            Ok(*size == target.size())
        }
        (NativeContentMechanism::BackingSurface, ObservedGeometry::BackingSurface { clip, .. }) => {
            Ok(clip == target)
        }
        (mechanism, _) => Err(CoordinationError::GeometryMechanismMismatch { mechanism }),
    }
}

struct PlanBuilder {
    island_id: NativeContentIslandId,
    desired_revision: NativeContentRevision,
    observed_revision: NativeContentRevision,
    generation: AttachGeneration,
    operations: Vec<PlannedOperation>,
}

impl PlanBuilder {
    fn new(desired: &DesiredState, observed: &ObservedState) -> Self {
        Self {
            island_id: desired.island_id().clone(),
            desired_revision: desired.revision(),
            observed_revision: observed.revision(),
            generation: desired.generation(),
            operations: Vec::new(),
        }
    }

    fn push(&mut self, operation: NativeContentOperation) {
        let step = PlanStepId::new(
            u32::try_from(self.operations.len() + 1).expect("bounded in-memory plan length"),
        );
        let depends_on = self.operations.last().map(PlannedOperation::step);
        self.operations.push(PlannedOperation {
            step,
            depends_on,
            operation,
        });
    }

    fn finish(self) -> ApplyPlan {
        ApplyPlan {
            island_id: self.island_id,
            desired_revision: self.desired_revision,
            observed_revision: self.observed_revision,
            generation: self.generation,
            operations: self.operations,
        }
    }
}
