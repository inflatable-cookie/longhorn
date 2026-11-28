use std::fs;

use longhorn_config::DomainLocation;
use longhorn_core::{ScreenPoint, SurfaceRequestId, SurfaceRevision, TransferRequestId};
use longhorn_surface_transfer::{
    EmptyDisplayProvisionPolicy, ProvisionCleanupOutcome, SurfaceProvisionFailureEvidence,
    SurfaceTransferAbort, SurfaceTransferAbortSource, SurfaceTransferCommitRequest,
    SurfaceTransferErrorCode, SurfaceTransferPolicy, SurfaceWindowProvisionStage,
    commit_surface_transfer,
};
use longhorn_surfaces::{
    EmptyWindowPolicy, SurfaceMutationCommand, SurfaceMutationRejectionCode, SurfaceMutationRequest,
};
use longhorn_surfaces_config::publish_surface_mutation;
use longhorn_transfer::{DropZoneId, TargetSelector, TransferErrorCode};

use super::support::{
    Fixture, InterveningProvisioner, MockMode, MockProvisioner, RuntimeFixture, domain,
    load_surface, options, policy, policy_with_provision, surface_id, window_id,
};

#[test]
fn target_loss_and_current_policy_rejection_consume_without_publication() {
    let mut fixture = Fixture::new();
    let domain = domain();
    fixture.store.register(&domain).unwrap();
    let baseline = load_surface(&fixture.store, &domain);

    let mut lost_runtime = RuntimeFixture::new();
    let lost_session = lost_runtime.admit(&fixture.store, &domain).unwrap();
    let mut provisioner = MockProvisioner::new(MockMode::Success);
    let lost = commit_surface_transfer(
        &fixture.store,
        &domain,
        &mut lost_runtime.coordinator,
        &lost_runtime.clock,
        &lost_runtime.bindings,
        &policy(),
        &mut provisioner,
        SurfaceTransferCommitRequest::new(
            lost_session,
            TargetSelector::ExplicitZone(DropZoneId::new("zone:surface-target").unwrap()),
            [],
            options(),
        ),
    )
    .unwrap_err();
    assert_eq!(lost.code(), SurfaceTransferErrorCode::TransferRejected);
    assert_eq!(
        lost.transfer_code(),
        Some(TransferErrorCode::TargetWindowMissing)
    );
    assert!(lost.session_consumed());
    let lost_abort = SurfaceTransferAbort::from_domain(
        longhorn_core::TransferRequestId::new("request:surface-lost").unwrap(),
        &lost,
    );
    assert_eq!(
        lost_abort.source(),
        SurfaceTransferAbortSource::Transfer(TransferErrorCode::TargetWindowMissing)
    );
    assert_eq!(lost_abort.surface_code(), None);
    assert!(!lost_abort.retryable());
    assert!(lost_abort.session_consumed());
    assert!(!lost_abort.reconciliation_required());

    let mut denied_runtime = RuntimeFixture::new();
    let denied_session = denied_runtime.admit(&fixture.store, &domain).unwrap();
    let denied_live = denied_runtime.live_target();
    let denied_policy = SurfaceTransferPolicy::provisioning_disabled([], EmptyWindowPolicy::Reject);
    let denied = commit_surface_transfer(
        &fixture.store,
        &domain,
        &mut denied_runtime.coordinator,
        &denied_runtime.clock,
        &denied_runtime.bindings,
        &denied_policy,
        &mut provisioner,
        SurfaceTransferCommitRequest::new(
            denied_session,
            TargetSelector::ScreenPoint(ScreenPoint::new(150, 150)),
            [denied_live],
            options(),
        ),
    )
    .unwrap_err();
    assert_eq!(denied.code(), SurfaceTransferErrorCode::IneligibleTarget);
    assert_eq!(load_surface(&fixture.store, &domain), baseline);
    assert!(provisioner.calls.is_empty());
}

#[test]
fn empty_display_disabled_and_provision_failure_leave_source_exact() {
    let mut fixture = Fixture::new();
    let domain = domain();
    fixture.store.register(&domain).unwrap();
    let baseline = load_surface(&fixture.store, &domain);

    let mut disabled_runtime = RuntimeFixture::new();
    let disabled_session = disabled_runtime.admit(&fixture.store, &domain).unwrap();
    let disabled_live = disabled_runtime.live_target();
    let mut provisioner = MockProvisioner::new(MockMode::Success);
    let disabled = commit_surface_transfer(
        &fixture.store,
        &domain,
        &mut disabled_runtime.coordinator,
        &disabled_runtime.clock,
        &disabled_runtime.bindings,
        &SurfaceTransferPolicy::new(
            [],
            EmptyWindowPolicy::Reject,
            EmptyDisplayProvisionPolicy::Disabled,
        )
        .unwrap(),
        &mut provisioner,
        SurfaceTransferCommitRequest::new(
            disabled_session,
            TargetSelector::ScreenPoint(ScreenPoint::new(1200, 200)),
            [disabled_live],
            options(),
        ),
    )
    .unwrap_err();
    assert_eq!(
        disabled.code(),
        SurfaceTransferErrorCode::EmptyDisplayDisabled
    );
    assert!(provisioner.calls.is_empty());

    let mut failed_runtime = RuntimeFixture::new();
    let failed_session = failed_runtime.admit(&fixture.store, &domain).unwrap();
    let failed_live = failed_runtime.live_target();
    let mut failed_provisioner = MockProvisioner::new(MockMode::ProvisionFail);
    let failed = commit_surface_transfer(
        &fixture.store,
        &domain,
        &mut failed_runtime.coordinator,
        &failed_runtime.clock,
        &failed_runtime.bindings,
        &policy_with_provision(),
        &mut failed_provisioner,
        SurfaceTransferCommitRequest::new(
            failed_session,
            TargetSelector::ScreenPoint(ScreenPoint::new(1200, 200)),
            [failed_live],
            options(),
        ),
    )
    .unwrap_err();
    assert_eq!(failed.code(), SurfaceTransferErrorCode::ProvisionFailed);
    assert_eq!(
        failed_provisioner.calls,
        ["create_hidden", "place", "ready"]
    );
    assert_eq!(load_surface(&fixture.store, &domain), baseline);
}

#[test]
fn mismatched_provision_receipt_reports_cleanup_without_publishing() {
    for (mode, cleanup_failed) in [(MockMode::Success, false), (MockMode::CleanupFail, true)] {
        let mut storage = Fixture::new();
        let domain = domain();
        storage.store.register(&domain).unwrap();
        let baseline = load_surface(&storage.store, &domain);
        let mut runtime = RuntimeFixture::new();
        let session = runtime.admit(&storage.store, &domain).unwrap();
        let live_target = runtime.live_target();
        let unexpected_window = window_id("window:unexpected");
        let mut provisioner =
            MockProvisioner::new(mode).with_provisioned_window(unexpected_window.clone());

        let error = commit_surface_transfer(
            &storage.store,
            &domain,
            &mut runtime.coordinator,
            &runtime.clock,
            &runtime.bindings,
            &policy_with_provision(),
            &mut provisioner,
            SurfaceTransferCommitRequest::new(
                session,
                TargetSelector::ScreenPoint(ScreenPoint::new(1200, 200)),
                [live_target],
                options(),
            ),
        )
        .unwrap_err();
        assert_eq!(
            provisioner.calls,
            ["create_hidden", "place", "ready", "cleanup"]
        );
        assert_eq!(
            error.code(),
            SurfaceTransferErrorCode::ProvisionReceiptMismatch
        );
        let Some(SurfaceProvisionFailureEvidence::PreparedTargetRejected { provision, cleanup }) =
            error.provisioning()
        else {
            panic!("receipt mismatch should retain the rejected target and cleanup evidence");
        };
        assert_eq!(provision.window_id(), &unexpected_window);
        match (cleanup_failed, cleanup) {
            (false, ProvisionCleanupOutcome::Succeeded(receipt)) => {
                assert_eq!(receipt.window_id(), &unexpected_window);
            }
            (true, ProvisionCleanupOutcome::Failed(failure)) => {
                assert_eq!(failure.stage(), SurfaceWindowProvisionStage::Cleanup);
                assert_eq!(failure.detail(), "close failed");
            }
            _ => panic!("unexpected cleanup outcome for {mode:?}: {cleanup:?}"),
        }
        assert_eq!(error.reconciliation_required(), cleanup_failed);
        let abort = SurfaceTransferAbort::from_domain(
            TransferRequestId::new("request:surface-receipt-mismatch").unwrap(),
            &error,
        );
        assert_eq!(
            abort.source(),
            SurfaceTransferAbortSource::SurfaceTransfer(
                SurfaceTransferErrorCode::ProvisionReceiptMismatch,
            )
        );
        assert!(
            abort.message().contains("another admitted target"),
            "{}",
            abort.message()
        );
        assert_eq!(abort.message().contains("close failed"), cleanup_failed);
        assert_eq!(abort.reconciliation_required(), cleanup_failed);
        assert!(abort.session_consumed());
        assert_eq!(load_surface(&storage.store, &domain), baseline);
    }
}

#[test]
fn failed_publication_cleans_prepared_target_and_reports_cleanup_failure() {
    for (mode, cleanup_failed) in [(MockMode::Success, false), (MockMode::CleanupFail, true)] {
        let mut storage = Fixture::new();
        let domain = domain();
        storage.store.register(&domain).unwrap();
        let mut runtime = RuntimeFixture::new();
        let session = runtime.admit(&storage.store, &domain).unwrap();
        let live_target = runtime.live_target();
        let mut provisioner = InterveningProvisioner::new(mode, || {
            publish_surface_mutation(
                &storage.store,
                &domain,
                options(),
                EmptyWindowPolicy::Allow,
                &SurfaceMutationRequest::new(
                    SurfaceRequestId::new("request:intervening").unwrap(),
                    SurfaceRevision::new(7),
                    SurfaceMutationCommand::RenameSurface {
                        surface_id: surface_id("surface:a"),
                        label: Some("Intervening".to_owned()),
                    },
                ),
            )
            .unwrap();
        });

        let error = commit_surface_transfer(
            &storage.store,
            &domain,
            &mut runtime.coordinator,
            &runtime.clock,
            &runtime.bindings,
            &policy_with_provision(),
            &mut provisioner,
            SurfaceTransferCommitRequest::new(
                session,
                TargetSelector::ScreenPoint(ScreenPoint::new(1200, 200)),
                [live_target],
                options(),
            ),
        )
        .unwrap_err();
        assert_eq!(
            provisioner.inner.calls,
            ["create_hidden", "place", "ready", "cleanup"]
        );
        let Some(SurfaceProvisionFailureEvidence::PublicationFailed { cleanup, .. }) =
            error.provisioning()
        else {
            panic!("publication failure should retain cleanup evidence");
        };
        assert_eq!(
            matches!(cleanup, ProvisionCleanupOutcome::Failed(_)),
            cleanup_failed
        );
        assert_eq!(error.code(), SurfaceTransferErrorCode::StaleSurfaceRevision);
        assert_eq!(
            error.surface_code(),
            Some(SurfaceMutationRejectionCode::StaleRevision)
        );
        assert!(error.detail().contains("revision"), "{}", error.detail());
        if cleanup_failed {
            assert!(
                error.detail().contains("close failed"),
                "{}",
                error.detail()
            );
        }
        let abort = SurfaceTransferAbort::from_domain(
            TransferRequestId::new("request:surface-publication").unwrap(),
            &error,
        );
        assert_eq!(
            abort.source(),
            SurfaceTransferAbortSource::SurfaceTransfer(
                SurfaceTransferErrorCode::StaleSurfaceRevision
            )
        );
        assert_eq!(abort.surface_code(), error.surface_code());
        assert_eq!(abort.message(), error.detail());
        assert_eq!(abort.reconciliation_required(), cleanup_failed);
        assert!(abort.session_consumed());
        let current = load_surface(&storage.store, &domain);
        let source = current.surface(&surface_id("surface:a")).unwrap();
        assert_eq!(current.revision().get(), 8);
        assert_eq!(source.label(), Some("Intervening"));
        assert_eq!(
            source.host_preferences().first().unwrap().window_id(),
            &window_id("window:main")
        );
    }
}

#[test]
fn cleanup_failure_preserves_the_config_store_failure() {
    let mut storage = Fixture::new();
    let domain = domain();
    storage.store.register(&domain).unwrap();
    let DomainLocation::File(location) = storage.store.location(&domain).unwrap() else {
        panic!("Surface transfer uses a file-backed domain");
    };
    let mut runtime = RuntimeFixture::new();
    let session = runtime.admit(&storage.store, &domain).unwrap();
    let live_target = runtime.live_target();
    let invalid_document = b"{ invalid";
    let mut provisioner = InterveningProvisioner::new(MockMode::CleanupFail, || {
        let path = location.full_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, invalid_document).unwrap();
    });

    let error = commit_surface_transfer(
        &storage.store,
        &domain,
        &mut runtime.coordinator,
        &runtime.clock,
        &runtime.bindings,
        &policy_with_provision(),
        &mut provisioner,
        SurfaceTransferCommitRequest::new(
            session,
            TargetSelector::ScreenPoint(ScreenPoint::new(1200, 200)),
            [live_target],
            options(),
        ),
    )
    .unwrap_err();
    let abort = SurfaceTransferAbort::from_domain(
        TransferRequestId::new("request:surface-store-failure").unwrap(),
        &error,
    );
    assert_eq!(error.code(), SurfaceTransferErrorCode::PublicationFailed);
    assert_eq!(
        abort.source(),
        SurfaceTransferAbortSource::SurfaceTransfer(error.code())
    );
    assert_eq!(abort.surface_code(), None);
    assert!(abort.reconciliation_required());
    assert!(abort.session_consumed());
    assert!(
        abort.message().contains("mutation refused: Recovery"),
        "{}",
        abort.message()
    );
    assert!(
        abort.message().contains("close failed"),
        "{}",
        abort.message()
    );
    assert_eq!(fs::read(location.full_path()).unwrap(), invalid_document);
}

#[test]
fn host_commit_failure_returns_authoritative_reconciliation_evidence() {
    let mut fixture = Fixture::new();
    let domain = domain();
    fixture.store.register(&domain).unwrap();
    let mut runtime = RuntimeFixture::new();
    let session = runtime.admit(&fixture.store, &domain).unwrap();
    let live_target = runtime.live_target();
    let mut provisioner = MockProvisioner::new(MockMode::CommitFail);

    let error = commit_surface_transfer(
        &fixture.store,
        &domain,
        &mut runtime.coordinator,
        &runtime.clock,
        &runtime.bindings,
        &policy_with_provision(),
        &mut provisioner,
        SurfaceTransferCommitRequest::new(
            session,
            TargetSelector::ScreenPoint(ScreenPoint::new(1200, 200)),
            [live_target],
            options(),
        ),
    )
    .unwrap_err();
    assert_eq!(
        error.code(),
        SurfaceTransferErrorCode::HostReconciliationRequired
    );
    let abort = SurfaceTransferAbort::from_domain(
        TransferRequestId::new("request:surface-host-commit").unwrap(),
        &error,
    );
    assert!(abort.reconciliation_required());
    let Some(SurfaceProvisionFailureEvidence::ReconciliationRequired { publication, .. }) =
        error.provisioning()
    else {
        panic!("partial commit should include authoritative Surface publication");
    };
    assert_eq!(publication.surface().committed_revision().get(), 8);
    assert_eq!(
        provisioner.calls,
        ["create_hidden", "place", "ready", "commit"]
    );
    assert_eq!(load_surface(&fixture.store, &domain).revision().get(), 8);
}

#[test]
fn consumed_binding_failure_reports_consumption_and_replay_is_terminal() {
    let mut fixture = Fixture::new();
    let domain = domain();
    fixture.store.register(&domain).unwrap();
    let baseline = load_surface(&fixture.store, &domain);

    let mut runtime = RuntimeFixture::new();
    let session = runtime.admit(&fixture.store, &domain).unwrap();
    let stale_bindings = longhorn_surface_transfer::SurfaceHostBindings::new([]).unwrap();
    let live = runtime.live_target();
    let mut provisioner = MockProvisioner::new(MockMode::Success);

    let error = commit_surface_transfer(
        &fixture.store,
        &domain,
        &mut runtime.coordinator,
        &runtime.clock,
        &stale_bindings,
        &policy(),
        &mut provisioner,
        SurfaceTransferCommitRequest::new(
            session,
            TargetSelector::ExplicitZone(DropZoneId::new("zone:surface-target").unwrap()),
            [live.clone()],
            options(),
        ),
    )
    .unwrap_err();
    assert_eq!(error.code(), SurfaceTransferErrorCode::UnknownHostBinding);
    assert!(
        error.session_consumed(),
        "post-consumption binding failure must report the consumed session"
    );
    assert_eq!(load_surface(&fixture.store, &domain), baseline);

    let replay = commit_surface_transfer(
        &fixture.store,
        &domain,
        &mut runtime.coordinator,
        &runtime.clock,
        &runtime.bindings,
        &policy(),
        &mut provisioner,
        SurfaceTransferCommitRequest::new(
            session,
            TargetSelector::ExplicitZone(DropZoneId::new("zone:surface-target").unwrap()),
            [live],
            options(),
        ),
    )
    .unwrap_err();
    assert_eq!(
        replay.transfer_code(),
        Some(TransferErrorCode::SessionReplayed)
    );
}
