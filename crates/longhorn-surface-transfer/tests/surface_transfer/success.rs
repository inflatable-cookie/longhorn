use longhorn_core::ScreenPoint;
use longhorn_surface_transfer::{
    SurfaceTerminalAttempt, SurfaceTransferCommitRequest, commit_surface_transfer,
};
use longhorn_transfer::TargetSelector;

use super::support::{
    Fixture, MockMode, MockProvisioner, RuntimeFixture, domain, layout_document, load_surface,
    options, policy, policy_with_provision, surface_id, window_id,
};

#[test]
fn ordinary_move_changes_only_surface_authority_and_retains_layout_binding() {
    let mut fixture = Fixture::new();
    let domain = domain();
    fixture.store.register(&domain).unwrap();
    let mut runtime = RuntimeFixture::new();
    let session = runtime.admit(&fixture.store, &domain).unwrap();
    let layout = layout_document();
    let exact_layout = layout.clone();
    let mut provisioner = MockProvisioner::new(MockMode::Success);
    let live_target = runtime.live_target();

    let receipt = commit_surface_transfer(
        &fixture.store,
        &domain,
        &layout,
        &mut runtime.coordinator,
        &runtime.clock,
        &runtime.bindings,
        &policy(),
        &mut provisioner,
        SurfaceTransferCommitRequest::new(
            session,
            TargetSelector::ScreenPoint(ScreenPoint::new(150, 150)),
            [live_target],
            options(),
        ),
    )
    .unwrap();

    assert!(matches!(
        receipt.attempt(),
        SurfaceTerminalAttempt::Existing(_)
    ));
    assert_eq!(receipt.publication().surface().previous_revision().get(), 7);
    assert_eq!(
        receipt.publication().surface().committed_revision().get(),
        8
    );
    assert!(receipt.provisioning().is_none());
    assert!(provisioner.calls.is_empty());
    assert_eq!(layout, exact_layout);
    let current = load_surface(&fixture.store, &domain);
    let moved = current.surface(&surface_id("surface:a")).unwrap();
    assert_eq!(
        moved.host_preferences().first().unwrap().window_id(),
        &window_id("window:target")
    );
    assert_eq!(moved.layout_container_id().as_str(), "container:a");
}

#[test]
fn empty_display_provisions_hidden_ready_target_then_commits_host() {
    let mut fixture = Fixture::new();
    let domain = domain();
    fixture.store.register(&domain).unwrap();
    let mut runtime = RuntimeFixture::new();
    let session = runtime.admit(&fixture.store, &domain).unwrap();
    let mut provisioner = MockProvisioner::new(MockMode::Success);
    let live_target = runtime.live_target();

    let receipt = commit_surface_transfer(
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
    .unwrap();

    assert!(matches!(
        receipt.attempt(),
        SurfaceTerminalAttempt::EmptyDisplay(_)
    ));
    assert_eq!(
        provisioner.calls,
        ["create_hidden", "place", "ready", "commit"]
    );
    let provisioning = receipt.provisioning().unwrap();
    assert_eq!(
        provisioning.provision().window_id(),
        &window_id("window:new")
    );
    assert_eq!(provisioning.commit().window_id(), &window_id("window:new"));
    assert_eq!(
        load_surface(&fixture.store, &domain)
            .surface(&surface_id("surface:a"))
            .unwrap()
            .host_preferences()
            .first()
            .unwrap()
            .window_id(),
        &window_id("window:new")
    );
}
