use longhorn_native_content_prototype::{
    ApplyReceipt, NativeContentFailureCode, NativeContentMechanism, OperationOutcome, PlanStepId,
    ReceiptError, StepExecution,
};

use super::support::coordinator;

#[test]
fn partial_apply_names_attempted_failed_and_dependency_skipped_steps() {
    let coordinator = coordinator(NativeContentMechanism::ChildView);
    let plan = coordinator.plan().unwrap();
    assert!(plan.operations().len() >= 4);
    let receipt = ApplyReceipt::build(
        &plan,
        [
            StepExecution::Applied {
                step: PlanStepId::new(1),
            },
            StepExecution::Failed {
                step: PlanStepId::new(2),
                code: NativeContentFailureCode::new("native:bounds-rejected").unwrap(),
            },
        ],
    )
    .unwrap();

    assert_eq!(receipt.steps()[0].outcome(), &OperationOutcome::Applied);
    assert!(matches!(
        receipt.steps()[1].outcome(),
        OperationOutcome::Failed { code } if code.as_str() == "native:bounds-rejected"
    ));
    for step in &receipt.steps()[2..] {
        assert_eq!(
            step.outcome(),
            &OperationOutcome::DependencySkipped {
                blocked_by: step
                    .step()
                    .get()
                    .checked_sub(1)
                    .map(PlanStepId::new)
                    .unwrap()
            }
        );
    }
}

#[test]
fn eligible_but_missing_report_is_not_fabricated_as_success() {
    let coordinator = coordinator(NativeContentMechanism::ChildView);
    let plan = coordinator.plan().unwrap();
    let receipt = ApplyReceipt::build(
        &plan,
        [StepExecution::Applied {
            step: PlanStepId::new(1),
        }],
    )
    .unwrap();
    assert_eq!(
        receipt.steps()[1].outcome(),
        &OperationOutcome::NotAttempted
    );
    assert_eq!(
        receipt.steps()[2].outcome(),
        &OperationOutcome::DependencySkipped {
            blocked_by: PlanStepId::new(2)
        }
    );
}

#[test]
fn malformed_execution_reports_fail_closed() {
    let coordinator = coordinator(NativeContentMechanism::ChildView);
    let plan = coordinator.plan().unwrap();
    assert_eq!(
        ApplyReceipt::build(
            &plan,
            [StepExecution::Applied {
                step: PlanStepId::new(99)
            }]
        ),
        Err(ReceiptError::UnknownStep(PlanStepId::new(99)))
    );
    assert_eq!(
        ApplyReceipt::build(
            &plan,
            [
                StepExecution::Applied {
                    step: PlanStepId::new(1)
                },
                StepExecution::Applied {
                    step: PlanStepId::new(1)
                }
            ]
        ),
        Err(ReceiptError::DuplicateStep(PlanStepId::new(1)))
    );

    assert_eq!(
        ApplyReceipt::build(
            &plan,
            [
                StepExecution::Failed {
                    step: PlanStepId::new(1),
                    code: NativeContentFailureCode::new("native:attach-failed").unwrap(),
                },
                StepExecution::Applied {
                    step: PlanStepId::new(2)
                }
            ]
        ),
        Err(ReceiptError::ExecutedAfterBlockedDependency {
            step: PlanStepId::new(2),
            blocked_by: PlanStepId::new(1),
        })
    );
}
