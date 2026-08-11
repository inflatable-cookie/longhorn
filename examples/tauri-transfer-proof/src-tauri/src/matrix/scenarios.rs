//! Deterministic terminal failure cases against isolated registered domains.

use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
};

use longhorn_config::DomainLocation;
use longhorn_core::{
    DropZoneId, PanelInstanceId, ScreenPoint, ScreenRect, ScreenSize, TransferHostBindingId,
    WindowId,
};
use longhorn_transfer::{
    ClientEpoch, DropZone, LeaseGeneration, LeasePublication, LiveTransferWindow, MonotonicClock,
    PanelSessionAdmission, PanelTransferCommitRequest, PanelTransferError, PanelTransferErrorCode,
    PanelTransferOperation, TargetSelector, TransferCapability, TransferClientId,
    TransferCoordinator, TransferDuration, TransferErrorCode, TransferInstant, TransferLimits,
    TransferRevision, TransferTargetBinding, admit_panel_session, commit_panel_transfer,
};
use serde_json::{Value, json};

use crate::domain::{
    LAYOUT_DOMAIN_ID, MAIN_REGION_ID, ProofDomains, ProofSessionAllocator, SOURCE_BINDING_ID,
    SOURCE_PANEL_ID, SOURCE_WINDOW_ID, TARGET_BINDING_ID, TARGET_SURFACE_ID, TARGET_WINDOW_ID,
    binding_kind, mutation_options,
};

pub(super) fn run(root: &Path) -> Result<Value, String> {
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let cases = vec![
        cancelled(root)?,
        expired(root)?,
        overlap(root)?,
        target_loss(root)?,
        stale_geometry(root)?,
        stale_revision(root)?,
        replay(root)?,
    ];
    let passed = cases
        .iter()
        .all(|case| case.get("passed").and_then(Value::as_bool).unwrap_or(false));
    Ok(json!({
        "passed": passed,
        "cases": cases,
        "source_invariance": "exact serialized authority bytes before and after each failed attempt",
    }))
}

fn cancelled(root: &Path) -> Result<Value, String> {
    let mut scenario = Scenario::new(root, "cancelled")?;
    scenario.publish(7)?;
    scenario
        .coordinator
        .cancel_session(&scenario.clock, scenario.session_id)
        .map_err(|error| error.to_string())?;
    scenario.expect(
        "cancellation",
        TargetSelector::ExplicitZone(zone_id()),
        normal_windows(),
        Some(TransferErrorCode::SessionCancelled),
        None,
    )
}

fn expired(root: &Path) -> Result<Value, String> {
    let mut scenario = Scenario::new(root, "expired")?;
    scenario.publish(7)?;
    scenario.clock.set(100);
    scenario.expect(
        "expiry",
        TargetSelector::ExplicitZone(zone_id()),
        normal_windows(),
        Some(TransferErrorCode::SessionExpired),
        None,
    )
}

fn overlap(root: &Path) -> Result<Value, String> {
    let mut scenario = Scenario::new(root, "overlap")?;
    scenario.publish(7)?;
    scenario.expect(
        "overlap",
        TargetSelector::ScreenPoint(ScreenPoint::new(850, 50)),
        [
            live(TARGET_WINDOW_ID, target_bounds()),
            live("overlap", rect(825, 25, 200, 200)),
        ],
        Some(TransferErrorCode::AmbiguousWindow),
        None,
    )
}

fn target_loss(root: &Path) -> Result<Value, String> {
    let mut scenario = Scenario::new(root, "target-loss")?;
    scenario.publish(7)?;
    scenario.expect(
        "target_loss",
        TargetSelector::ExplicitZone(zone_id()),
        [live(SOURCE_WINDOW_ID, source_bounds())],
        Some(TransferErrorCode::TargetWindowMissing),
        None,
    )
}

fn stale_geometry(root: &Path) -> Result<Value, String> {
    let mut scenario = Scenario::new(root, "stale-geometry")?;
    scenario.publish(7)?;
    scenario.expect(
        "stale_geometry",
        TargetSelector::ExplicitZone(zone_id()),
        [
            live(SOURCE_WINDOW_ID, source_bounds()),
            live(TARGET_WINDOW_ID, rect(801, 0, 800, 600)),
        ],
        Some(TransferErrorCode::StaleWindowGeometry),
        None,
    )
}

fn stale_revision(root: &Path) -> Result<Value, String> {
    let mut scenario = Scenario::new(root, "stale-revision")?;
    scenario.publish(8)?;
    scenario.expect(
        "stale_revision",
        TargetSelector::ExplicitZone(zone_id()),
        normal_windows(),
        None,
        Some(PanelTransferErrorCode::StaleSurfaceRevision),
    )
}

fn replay(root: &Path) -> Result<Value, String> {
    let mut scenario = Scenario::new(root, "replay")?;
    scenario.publish(7)?;
    let request = scenario.request(TargetSelector::ExplicitZone(zone_id()), normal_windows());
    commit_panel_transfer(
        scenario.domains.store(),
        scenario.domains.layout(),
        &mut scenario.coordinator,
        &scenario.clock,
        scenario.domains.panel_bindings(),
        mutation_options(),
        request.clone(),
    )
    .map_err(|error| error.to_string())?;
    scenario.expect_request(
        "replay",
        request,
        Some(TransferErrorCode::SessionReplayed),
        None,
    )
}

struct Scenario {
    domains: ProofDomains,
    coordinator: TransferCoordinator,
    clock: MatrixClock,
    session_id: longhorn_transfer::DragSessionId,
    authority_path: PathBuf,
}

impl Scenario {
    fn new(root: &Path, name: &str) -> Result<Self, String> {
        let domains = ProofDomains::new(&root.join(name), binding_kind())?;
        let clock = MatrixClock::new(10);
        let mut coordinator = TransferCoordinator::new(limits());
        for (window, client) in [
            (SOURCE_WINDOW_ID, "client:source"),
            (TARGET_WINDOW_ID, "client:target"),
        ] {
            coordinator
                .bind_client_epoch(
                    &clock,
                    window_id(window),
                    TransferClientId::new(client).expect("proof client id is valid"),
                    ClientEpoch::new(1),
                )
                .map_err(|error| error.to_string())?;
        }
        let mut allocator = ProofSessionAllocator::new();
        let receipt = admit_panel_session(
            domains.store(),
            domains.layout(),
            &mut coordinator,
            &clock,
            &mut allocator,
            domains.panel_bindings(),
            PanelSessionAdmission::new(
                window_id(SOURCE_WINDOW_ID),
                TransferClientId::new("client:source").expect("proof client id is valid"),
                ClientEpoch::new(1),
                PanelInstanceId::new(SOURCE_PANEL_ID).expect("proof panel id is valid"),
                TransferHostBindingId::new(SOURCE_BINDING_ID).expect("proof binding id is valid"),
                TransferDuration::new(40),
            ),
        )
        .map_err(|error| error.to_string())?;
        let authority_path = match domains
            .store()
            .location(domains.layout())
            .map_err(|error| error.to_string())?
        {
            DomainLocation::File(file) => file.full_path().to_path_buf(),
            other => return Err(format!("proof layout must resolve to a file: {other:?}")),
        };
        Ok(Self {
            domains,
            coordinator,
            clock,
            session_id: receipt.payload().session_id(),
            authority_path,
        })
    }

    fn publish(&mut self, revision: u64) -> Result<(), String> {
        self.coordinator
            .publish_lease(
                &self.clock,
                LeasePublication::new(
                    window_id(TARGET_WINDOW_ID),
                    TransferClientId::new("client:target").expect("proof client id is valid"),
                    ClientEpoch::new(1),
                    LeaseGeneration::new(1),
                    TransferDuration::new(30),
                    target_bounds(),
                    vec![DropZone::new(
                        zone_id(),
                        target_bounds(),
                        None,
                        TransferCapability::MovePanel,
                        TransferTargetBinding::PanelRegion {
                            host_binding_id: TransferHostBindingId::new(TARGET_BINDING_ID)
                                .expect("proof binding id is valid"),
                            document_id: longhorn_core::DomainId::new(LAYOUT_DOMAIN_ID)
                                .expect("proof domain id is valid"),
                            revision: TransferRevision::new(revision),
                            surface_id: longhorn_core::SurfaceId::new(TARGET_SURFACE_ID)
                                .expect("proof container id is valid"),
                            region_id: longhorn_core::RegionId::new(MAIN_REGION_ID)
                                .expect("proof region id is valid"),
                        },
                    )],
                ),
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn expect(
        &mut self,
        name: &str,
        selector: TargetSelector,
        live_windows: impl IntoIterator<Item = LiveTransferWindow>,
        transfer_code: Option<TransferErrorCode>,
        panel_code: Option<PanelTransferErrorCode>,
    ) -> Result<Value, String> {
        let request = self.request(selector, live_windows);
        self.expect_request(name, request, transfer_code, panel_code)
    }

    fn expect_request(
        &mut self,
        name: &str,
        request: PanelTransferCommitRequest,
        transfer_code: Option<TransferErrorCode>,
        panel_code: Option<PanelTransferErrorCode>,
    ) -> Result<Value, String> {
        let before = fs::read(&self.authority_path).ok();
        let error = commit_panel_transfer(
            self.domains.store(),
            self.domains.layout(),
            &mut self.coordinator,
            &self.clock,
            self.domains.panel_bindings(),
            mutation_options(),
            request,
        )
        .expect_err("failure matrix case unexpectedly committed");
        let after = fs::read(&self.authority_path).ok();
        Ok(case_result(
            name,
            &error,
            transfer_code,
            panel_code,
            before == after,
        ))
    }

    fn request(
        &self,
        selector: TargetSelector,
        live_windows: impl IntoIterator<Item = LiveTransferWindow>,
    ) -> PanelTransferCommitRequest {
        PanelTransferCommitRequest::new(
            self.session_id,
            selector,
            live_windows,
            PanelTransferOperation::Move,
        )
    }
}

fn case_result(
    name: &str,
    error: &PanelTransferError,
    transfer_code: Option<TransferErrorCode>,
    panel_code: Option<PanelTransferErrorCode>,
    unchanged: bool,
) -> Value {
    let expected = error.transfer_code() == transfer_code
        && panel_code.is_none_or(|code| error.code() == code);
    json!({
        "name": name,
        "passed": expected && unchanged,
        "panel_code": error.code(),
        "transfer_code": error.transfer_code(),
        "session_consumed": error.session_consumed(),
        "source_bytes_unchanged": unchanged,
        "detail": error.detail(),
    })
}

struct MatrixClock(Cell<u64>);

impl MatrixClock {
    const fn new(now: u64) -> Self {
        Self(Cell::new(now))
    }

    fn set(&self, now: u64) {
        self.0.set(now);
    }
}

impl MonotonicClock for MatrixClock {
    fn now(&self) -> TransferInstant {
        TransferInstant::new(self.0.get())
    }
}

fn limits() -> TransferLimits {
    TransferLimits::new(
        8,
        8,
        8,
        8,
        32,
        TransferDuration::new(100),
        TransferDuration::new(50),
    )
    .expect("proof matrix limits are valid")
}

fn window_id(value: &str) -> WindowId {
    WindowId::new(value).expect("proof window id is valid")
}

fn zone_id() -> DropZoneId {
    DropZoneId::new("zone:matrix").expect("proof zone id is valid")
}

fn rect(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
    ScreenRect::new(ScreenPoint::new(x, y), ScreenSize::new(width, height))
}

fn source_bounds() -> ScreenRect {
    rect(0, 0, 800, 600)
}

fn target_bounds() -> ScreenRect {
    rect(800, 0, 800, 600)
}

fn live(window: &str, bounds: ScreenRect) -> LiveTransferWindow {
    LiveTransferWindow::new(window_id(window), bounds)
}

fn normal_windows() -> [LiveTransferWindow; 2] {
    [
        live(SOURCE_WINDOW_ID, source_bounds()),
        live(TARGET_WINDOW_ID, target_bounds()),
    ]
}
