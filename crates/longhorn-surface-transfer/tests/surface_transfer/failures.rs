use longhorn_core::ScreenPoint;
use longhorn_surface_transfer::{
    EmptyDisplayProvisionPolicy, ProvisionCleanupOutcome, SurfaceProvisionFailureEvidence,
    SurfaceTransferCommitRequest, SurfaceTransferErrorCode, SurfaceTransferPolicy,
    commit_surface_transfer,
};
use longhorn_surfaces::EmptyWindowPolicy;
use longhorn_transfer::{DropZoneId, TargetSelector, TransferErrorCode};

use super::support::{
    Fixture, MockMode, MockProvisioner, RuntimeFixture, StalingProvisioner, domain,
    layout_document, load_surface, options, policy, policy_with_provision, surface_id, window_id,
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
        &layout_document(),
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

    let mut denied_runtime = RuntimeFixture::new();
    let denied_session = denied_runtime.admit(&fixture.store, &domain).unwrap();
    let denied_live = denied_runtime.live_target();
    let denied_policy = SurfaceTransferPolicy::provisioning_disabled([], EmptyWindowPolicy::Reject);
    let denied = commit_surface_transfer(
        &fixture.store,
        &domain,
        &layout_document(),
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
        &layout_document(),
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
        &layout_document(),
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
fn failed_publication_cleans_prepared_target_and_reports_cleanup_failure() {
    for (mode, cleanup_failed) in [(MockMode::Success, false), (MockMode::CleanupFail, true)] {
        let mut fixture = Fixture::new();
        let domain = domain();
        fixture.store.register(&domain).unwrap();
        let mut runtime = RuntimeFixture::new();
        let session = runtime.admit(&fixture.store, &domain).unwrap();
        let live_target = runtime.live_target();
        let mut provisioner = StalingProvisioner::new(mode, &fixture.store, &domain);

        let error = commit_surface_transfer(
            &fixture.store,
            &domain,
            &layout_document(),
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
        assert_eq!(
            error.code() == SurfaceTransferErrorCode::HostReconciliationRequired,
            cleanup_failed
        );
        let current = load_surface(&fixture.store, &domain);
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
        &layout_document(),
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
