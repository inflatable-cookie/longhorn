use longhorn_core::{
    NativeContentIslandId, NativeContentRevision, PhysicalRect, PhysicalSize, VisibilityReasonId,
    WindowId,
};
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::{
    AttachGeneration, AttachmentLifecycle, CoordinationError, DesiredPresence, DesiredState,
    DesiredVisibility, DetachPolicy, EffectiveFocus, EffectiveVisibility, FocusIntent,
    InputRoutingMode, NativeContentMechanism, ObservedGeometry, ObservedState, PlanStepId,
    viewport_to_physical,
};

/// Maximum operations produced by one transition plan.
pub const MAX_APPLY_PLAN_STEPS: usize = 5;

/// One pure native-content host instruction.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum NativeContentOperation {
    /// Attach selected native content to a host window.
    Attach {
        /// Stable logical host-window binding.
        host_window_id: WindowId,
        /// Independently selected host mechanism.
        mechanism: NativeContentMechanism,
    },
    /// Apply complete physical bounds to a child view.
    SetChildBounds {
        /// Target physical child bounds.
        #[cfg_attr(
            feature = "bindings",
            ts(
                type = "{ origin: { x: number; y: number }; size: { width: number; height: number } }"
            )
        )]
        bounds: PhysicalRect,
    },
    /// Apply physical content size without owning outer placement.
    SetIsolatedContentSize {
        /// Target physical content size.
        #[cfg_attr(feature = "bindings", ts(type = "{ width: number; height: number }"))]
        size: PhysicalSize,
    },
    /// Apply presentation and interaction clipping to a backing surface.
    SetBackingViewport {
        /// Target physical clip. Native storage may remain full-host.
        #[cfg_attr(
            feature = "bindings",
            ts(
                type = "{ origin: { x: number; y: number }; size: { width: number; height: number } }"
            )
        )]
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
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(try_from = "PlannedOperationWire")]
pub struct PlannedOperation {
    step: PlanStepId,
    depends_on: Option<PlanStepId>,
    operation: NativeContentOperation,
}

#[derive(Deserialize)]
struct PlannedOperationWire {
    step: PlanStepId,
    depends_on: Option<PlanStepId>,
    operation: NativeContentOperation,
}

impl TryFrom<PlannedOperationWire> for PlannedOperation {
    type Error = &'static str;

    fn try_from(wire: PlannedOperationWire) -> Result<Self, Self::Error> {
        let step = wire.step.get();
        if step as usize > MAX_APPLY_PLAN_STEPS {
            return Err("native-content plan step exceeds the plan bound");
        }
        if wire.depends_on.map(PlanStepId::get) != (step > 1).then_some(step - 1) {
            return Err("native-content plan step must depend on its immediate predecessor");
        }
        Ok(Self {
            step: wire.step,
            depends_on: wire.depends_on,
            operation: wire.operation,
        })
    }
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
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ApplyPlan {
    island_id: NativeContentIslandId,
    desired_revision: NativeContentRevision,
    observed_revision: NativeContentRevision,
    generation: AttachGeneration,
    #[serde(deserialize_with = "deserialize_operations")]
    operations: Vec<PlannedOperation>,
}

fn deserialize_operations<'de, D>(deserializer: D) -> Result<Vec<PlannedOperation>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Operations;

    impl<'de> de::Visitor<'de> for Operations {
        type Value = Vec<PlannedOperation>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                formatter,
                "at most {MAX_APPLY_PLAN_STEPS} consecutive plan steps"
            )
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut operations = Vec::with_capacity(MAX_APPLY_PLAN_STEPS);
            for index in 0..MAX_APPLY_PLAN_STEPS {
                let Some(operation) = seq.next_element::<PlannedOperation>()? else {
                    return Ok(operations);
                };
                if operation.step() != PlanStepId::from_plan_index(index) {
                    return Err(de::Error::custom(
                        "native-content plan step ids must start at one and be consecutive",
                    ));
                }
                operations.push(operation);
            }
            if seq.next_element::<de::IgnoredAny>()?.is_some() {
                return Err(de::Error::invalid_length(MAX_APPLY_PLAN_STEPS + 1, &self));
            }
            Ok(operations)
        }
    }

    deserializer.deserialize_seq(Operations)
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

pub(crate) fn plan_transition(
    desired: &DesiredState,
    observed: &ObservedState,
    retired_generation: Option<AttachGeneration>,
) -> Result<ApplyPlan, CoordinationError> {
    if observed.generation() > desired.generation() {
        return Err(CoordinationError::FutureGeneration {
            current: desired.generation(),
            supplied: observed.generation(),
        });
    }

    let mut builder = PlanBuilder::new(desired, observed);
    if desired.presence() == DesiredPresence::Absent {
        if needs_detach(desired, observed) {
            builder.push(NativeContentOperation::Detach {
                policy: desired.capabilities().detach_policy(),
            });
        }
        return Ok(builder.finish());
    }

    if retired_generation == Some(desired.generation()) {
        return Err(CoordinationError::TerminalGeneration(desired.generation()));
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
            builder.push(NativeContentOperation::Show)
        }
        DesiredVisibility::Hidden { reason }
            if needs_attach || observed.visibility() != EffectiveVisibility::Hidden =>
        {
            builder.push(NativeContentOperation::Hide {
                reason: reason.clone(),
            })
        }
        DesiredVisibility::Visible | DesiredVisibility::Hidden { .. } => {}
    }

    if needs_attach || observed.input_routing() != Some(desired.input_routing()) {
        builder.push(NativeContentOperation::SetInputRouting {
            mode: desired.input_routing(),
        });
    }

    if desired.capabilities().observes_focus() {
        match desired.focus() {
            FocusIntent::Request if needs_attach || observed.focus() != EffectiveFocus::Focused => {
                builder.push(NativeContentOperation::RequestFocus);
            }
            FocusIntent::ReleaseIfOwned
                if needs_attach || observed.focus() != EffectiveFocus::Unfocused =>
            {
                builder.push(NativeContentOperation::ReleaseFocusIfOwned);
            }
            _ => {}
        }
    }

    Ok(builder.finish())
}

pub(crate) fn needs_detach(desired: &DesiredState, observed: &ObservedState) -> bool {
    desired.presence() == DesiredPresence::Absent
        && !matches!(
            observed.lifecycle(),
            AttachmentLifecycle::Absent | AttachmentLifecycle::Detaching
        )
}

pub(crate) fn plan_generation(
    desired: &DesiredState,
    observed: &ObservedState,
) -> AttachGeneration {
    if needs_detach(desired, observed) {
        observed.generation()
    } else {
        desired.generation()
    }
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
            generation: plan_generation(desired, observed),
            operations: Vec::with_capacity(MAX_APPLY_PLAN_STEPS),
        }
    }

    fn push(&mut self, operation: NativeContentOperation) {
        assert!(
            self.operations.len() < MAX_APPLY_PLAN_STEPS,
            "native-content plan exceeded its contract bound"
        );
        let step = PlanStepId::from_plan_index(self.operations.len());
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
