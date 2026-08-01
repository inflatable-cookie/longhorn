use longhorn_operation::OperationState;

use super::support::*;

#[test]
fn soundcheck_scan_registers_running_and_preserves_cancellation_race_truth() {
    let mut catalogue = catalogue("authority:soundcheck", 7);
    let registered = catalogue
        .register(scoped_registration(
            &catalogue,
            "scan:plugins-42",
            "soundcheck.plugin-scan",
            "library:default",
            "Scan plug-ins",
            OperationState::Running,
        ))
        .unwrap();
    assert_eq!(registered.operation().revision().get(), 0);
    assert_eq!(registered.operation().sequence().get(), 1);
    assert_eq!(
        registered.operation().scope_id().unwrap().as_str(),
        "library:default"
    );

    catalogue
        .transition(transition(
            &catalogue,
            "scan:plugins-42",
            OperationState::Cancelling,
        ))
        .unwrap();
    let completed = catalogue
        .transition(transition(
            &catalogue,
            "scan:plugins-42",
            OperationState::Succeeded,
        ))
        .unwrap();
    assert_eq!(completed.previous_state(), OperationState::Cancelling);
    assert_eq!(completed.committed_state(), OperationState::Succeeded);

    let projection = catalogue.project();
    assert!(projection.active().is_empty());
    assert_eq!(projection.recent().len(), 1);
    assert_eq!(
        projection.recent()[0].operation_id().as_str(),
        "scan:plugins-42"
    );
}

#[test]
fn renderer_remount_reads_current_scan_without_changing_host_state() {
    let mut catalogue = catalogue("authority:soundcheck", 8);
    catalogue
        .register(registration(
            &catalogue,
            "scan:plugins-43",
            "soundcheck.plugin-scan",
            "Scan plug-ins",
            OperationState::Running,
        ))
        .unwrap();

    let first_mount = catalogue.project();
    let second_mount = catalogue.project();
    assert_eq!(first_mount, second_mount);
    assert_eq!(catalogue.revision().get(), 1);
    assert_eq!(second_mount.active()[0].state(), OperationState::Running);
}
