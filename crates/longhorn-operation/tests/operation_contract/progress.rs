use longhorn_core::OperationPhaseId;
use longhorn_operation::{
    OperationCatalogueError, OperationNormalizedProgress, OperationOverallProgress,
    OperationPhaseLabel, OperationPhaseProgress, OperationProgressSequence,
    OperationProgressUpdate, OperationProgressValueError, OperationState, OperationUnitProgress,
};

use super::support::*;

fn normalized(value: f64) -> OperationOverallProgress {
    OperationOverallProgress::Normalized(OperationNormalizedProgress::new(value).unwrap())
}

fn update(
    catalogue: &longhorn_operation::OperationCatalogue,
    overall: OperationOverallProgress,
    phase: Option<OperationPhaseProgress>,
) -> OperationProgressUpdate {
    let record = catalogue
        .operation(&operation_id("operation:progress"))
        .unwrap();
    OperationProgressUpdate::new(
        catalogue.authority().clone(),
        record.operation_id().clone(),
        record.revision(),
        overall,
        phase,
    )
}

fn phase(id: &str, completed: f64, total: f64) -> OperationPhaseProgress {
    OperationPhaseProgress::new(
        OperationPhaseId::new(id).unwrap(),
        OperationPhaseLabel::new(id).unwrap(),
        OperationUnitProgress::new(completed, total).unwrap(),
    )
}

#[test]
fn progress_values_reject_non_finite_invalid_and_overflowing_input() {
    assert_eq!(
        OperationUnitProgress::new(f64::NAN, 1.0),
        Err(OperationProgressValueError::NonFinite)
    );
    assert_eq!(
        OperationUnitProgress::new(-1.0, 1.0),
        Err(OperationProgressValueError::NegativeCompleted)
    );
    assert_eq!(
        OperationUnitProgress::new(0.0, 0.0),
        Err(OperationProgressValueError::NonPositiveTotal)
    );
    assert_eq!(
        OperationUnitProgress::new(2.0, 1.0),
        Err(OperationProgressValueError::CompletedExceedsTotal)
    );
    assert_eq!(
        OperationNormalizedProgress::new(f64::INFINITY),
        Err(OperationProgressValueError::NonFinite)
    );
    assert_eq!(
        OperationNormalizedProgress::new(1.1),
        Err(OperationProgressValueError::NormalizedOutOfRange)
    );
    assert!(
        OperationProgressSequence::new(u64::MAX)
            .checked_next()
            .is_err()
    );
}

#[test]
fn overall_and_same_phase_progress_cannot_regress() {
    let mut catalogue = catalogue("authority:progress", 1);
    catalogue
        .register(registration(
            &catalogue,
            "operation:progress",
            "scan",
            "Scan",
            OperationState::Running,
        ))
        .unwrap();

    let first = catalogue
        .update_progress(update(
            &catalogue,
            normalized(0.4),
            Some(phase("phase:scan", 4.0, 10.0)),
        ))
        .unwrap();
    assert_eq!(first.previous_sequence().get(), 0);
    assert_eq!(first.committed_progress().sequence().get(), 1);

    for rejected in [
        update(&catalogue, normalized(0.39), None),
        update(&catalogue, OperationOverallProgress::Indeterminate, None),
        update(
            &catalogue,
            normalized(0.4),
            Some(phase("phase:scan", 3.0, 10.0)),
        ),
    ] {
        let before = catalogue.clone();
        assert!(matches!(
            catalogue.update_progress(rejected),
            Err(OperationCatalogueError::OverallProgressRegression
                | OperationCatalogueError::PhaseProgressRegression { .. })
        ));
        assert_eq!(catalogue, before);
    }

    let reset = catalogue
        .update_progress(update(
            &catalogue,
            normalized(0.5),
            Some(phase("phase:index", 0.0, 8.0)),
        ))
        .unwrap();
    assert_eq!(
        reset
            .committed_progress()
            .phase()
            .unwrap()
            .phase_id()
            .as_str(),
        "phase:index"
    );
    let preserved = catalogue
        .update_progress(update(&catalogue, normalized(0.6), None))
        .unwrap();
    assert_eq!(
        preserved
            .committed_progress()
            .phase()
            .unwrap()
            .phase_id()
            .as_str(),
        "phase:index"
    );
}

#[test]
fn changed_unit_totals_preserve_fraction_and_late_progress_is_rejected() {
    let mut catalogue = catalogue("authority:progress-late", 1);
    catalogue
        .register(registration(
            &catalogue,
            "operation:progress",
            "scan",
            "Scan",
            OperationState::Running,
        ))
        .unwrap();
    catalogue
        .update_progress(update(
            &catalogue,
            OperationOverallProgress::Units(OperationUnitProgress::new(5.0, 10.0).unwrap()),
            None,
        ))
        .unwrap();
    catalogue
        .update_progress(update(
            &catalogue,
            OperationOverallProgress::Units(OperationUnitProgress::new(10.0, 20.0).unwrap()),
            None,
        ))
        .unwrap();
    let before = catalogue.clone();
    let regressing = update(
        &catalogue,
        OperationOverallProgress::Units(OperationUnitProgress::new(9.0, 20.0).unwrap()),
        None,
    );
    assert_eq!(
        catalogue.update_progress(regressing),
        Err(OperationCatalogueError::OverallProgressRegression)
    );
    assert_eq!(catalogue, before);

    catalogue
        .transition(transition(
            &catalogue,
            "operation:progress",
            OperationState::Succeeded,
        ))
        .unwrap();
    let before = catalogue.clone();
    let late = update(&catalogue, normalized(1.0), None);
    assert_eq!(
        catalogue.update_progress(late),
        Err(OperationCatalogueError::ProgressNotReportable {
            state: OperationState::Succeeded
        })
    );
    assert_eq!(catalogue, before);
}
