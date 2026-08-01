use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::{NativeContentFailureCode, NativeContentIslandId, NativeContentRevision};
use serde::{Deserialize, Serialize};

use crate::{ApplyPlan, AttachGeneration, NativeContentOperation, PlanStepId, ReceiptError};

/// Adapter execution evidence for one attempted plan step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepExecution {
    /// The native call returned success. Fresh observation still decides convergence.
    Applied {
        /// Attempted plan step.
        step: PlanStepId,
    },
    /// The native call failed with a stable mechanism code.
    Failed {
        /// Attempted plan step.
        step: PlanStepId,
        /// Adapter-owned failure category.
        code: NativeContentFailureCode,
    },
}

impl StepExecution {
    pub(crate) const fn step(&self) -> PlanStepId {
        match self {
            Self::Applied { step } | Self::Failed { step, .. } => *step,
        }
    }
}

/// Exact outcome for one planned operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum OperationOutcome {
    /// The adapter attempted the operation and returned success.
    Applied,
    /// The adapter attempted the operation and returned failure.
    Failed {
        /// Stable adapter-owned failure category.
        code: NativeContentFailureCode,
    },
    /// The operation did not run because a dependency did not apply.
    DependencySkipped {
        /// First immediate dependency that did not apply.
        blocked_by: PlanStepId,
    },
    /// The operation was eligible but no execution evidence was supplied.
    NotAttempted,
}

impl OperationOutcome {
    fn is_applied(&self) -> bool {
        matches!(self, Self::Applied)
    }
}

/// One operation paired with exact execution or skip evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct StepReceipt {
    step: PlanStepId,
    operation: NativeContentOperation,
    outcome: OperationOutcome,
}

impl StepReceipt {
    /// Returns plan-local step identity.
    #[must_use]
    pub const fn step(&self) -> PlanStepId {
        self.step
    }
    /// Returns the immutable planned operation.
    #[must_use]
    pub const fn operation(&self) -> &NativeContentOperation {
        &self.operation
    }
    /// Returns exact apply, failure, or skip evidence.
    #[must_use]
    pub const fn outcome(&self) -> &OperationOutcome {
        &self.outcome
    }
}

/// Complete execution receipt for one immutable apply plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ApplyReceipt {
    island_id: NativeContentIslandId,
    desired_revision: NativeContentRevision,
    observed_revision: NativeContentRevision,
    generation: AttachGeneration,
    steps: Vec<StepReceipt>,
}

impl ApplyReceipt {
    pub(crate) fn build(
        plan: &ApplyPlan,
        executions: impl IntoIterator<Item = StepExecution>,
    ) -> Result<Self, ReceiptError> {
        let known: BTreeSet<_> = plan
            .operations()
            .iter()
            .map(crate::PlannedOperation::step)
            .collect();
        let mut reports = BTreeMap::new();
        for execution in executions {
            let step = execution.step();
            if !known.contains(&step) {
                return Err(ReceiptError::UnknownStep(step));
            }
            if reports.insert(step, execution).is_some() {
                return Err(ReceiptError::DuplicateStep(step));
            }
        }

        let mut outcomes = BTreeMap::<PlanStepId, OperationOutcome>::new();
        let mut steps = Vec::with_capacity(plan.operations().len());
        for planned in plan.operations() {
            let blocked_by = planned.depends_on().and_then(|dependency| {
                outcomes
                    .get(&dependency)
                    .filter(|outcome| !outcome.is_applied())
                    .map(|_| dependency)
            });
            let report = reports.remove(&planned.step());
            if let (Some(blocked_by), Some(_)) = (blocked_by, report.as_ref()) {
                return Err(ReceiptError::ExecutedAfterBlockedDependency {
                    step: planned.step(),
                    blocked_by,
                });
            }
            let outcome = match (blocked_by, report) {
                (Some(blocked_by), None) => OperationOutcome::DependencySkipped { blocked_by },
                (None, Some(StepExecution::Applied { .. })) => OperationOutcome::Applied,
                (None, Some(StepExecution::Failed { code, .. })) => {
                    OperationOutcome::Failed { code }
                }
                (None, None) => OperationOutcome::NotAttempted,
                (Some(_), Some(_)) => unreachable!("blocked execution rejected above"),
            };
            outcomes.insert(planned.step(), outcome.clone());
            steps.push(StepReceipt {
                step: planned.step(),
                operation: planned.operation().clone(),
                outcome,
            });
        }

        Ok(Self {
            island_id: plan.island_id().clone(),
            desired_revision: plan.desired_revision(),
            observed_revision: plan.observed_revision(),
            generation: plan.generation(),
            steps,
        })
    }

    /// Returns island identity.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }
    /// Returns desired revision used by the plan.
    #[must_use]
    pub const fn desired_revision(&self) -> NativeContentRevision {
        self.desired_revision
    }
    /// Returns observed revision used by the plan.
    #[must_use]
    pub const fn observed_revision(&self) -> NativeContentRevision {
        self.observed_revision
    }
    /// Returns attach generation used by the plan.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }
    /// Returns one receipt for every planned operation.
    #[must_use]
    pub fn steps(&self) -> &[StepReceipt] {
        &self.steps
    }
}
