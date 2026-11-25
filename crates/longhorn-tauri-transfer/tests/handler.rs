//! Real handler-core behavior over deterministic managed-window readback.

use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, DomainId, DropZoneId, RegionId, ScreenPoint, ScreenRect,
    ScreenSize, SurfaceId, TransferHostBindingId, TransferRequestId, WindowId,
};
use longhorn_tauri_transfer::{
    ManagedTransferSnapshot, ManagedTransferWindow, TransferHandlerAssembly, TransferHandlerError,
    TransferHandlerTeardownStatus, TransferProjectionError, TransferRuntimeError,
    project_client_point, project_client_rect,
};
use longhorn_transfer::{
    ClientDropZone, ClientEpoch, LeaseGeneration, MonotonicClock, TransferAbortSource,
    TransferDuration, TransferErrorCode, TransferInstant, TransferLeaseRequest,
    TransferLeaseResponse, TransferLimits, TransferRevision, TransferTargetBinding,
};
use longhorn_windowing::HostWindowHandle;

#[derive(Clone, Copy)]
struct Clock;

impl MonotonicClock for Clock {
    fn now(&self) -> TransferInstant {
        TransferInstant::new(100)
    }
}

#[test]
fn projection_is_explicit_and_confined_to_current_content() {
    let window = managed_window("main", "window:main");
    let point = project_client_point(&window, ClientPoint::new(4.5, 8.49).unwrap()).unwrap();
    assert_eq!(point, ScreenPoint::new(105, 128));

    let rect = project_client_rect(
        &window,
        ClientRect::new(
            ClientPoint::new(10.25, 20.25).unwrap(),
            ClientSize::new(100.2, 50.2).unwrap(),
        ),
    )
    .unwrap();
    assert_eq!(
        rect,
        ScreenRect::new(ScreenPoint::new(110, 140), ScreenSize::new(101, 51))
    );

    assert_eq!(
        project_client_point(&window, ClientPoint::new(-1.0, 0.0).unwrap()),
        Err(TransferProjectionError::PointOutsideContent)
    );
    assert_eq!(
        project_client_rect(
            &window,
            ClientRect::new(
                ClientPoint::new(-1.0, 0.0).unwrap(),
                ClientSize::new(10.0, 10.0).unwrap(),
            ),
        ),
        Err(TransferProjectionError::RectangleOutsideContent)
    );
}

#[test]
fn mock_and_real_runtimes_share_epoch_lease_and_teardown_core() {
    let caller = handle("main");
    let snapshot = ManagedTransferSnapshot::new(
        &caller,
        [
            managed_window("main", "window:main"),
            managed_window("workspace", "window:workspace"),
        ],
    )
    .unwrap();
    let runtime = move |_caller: &HostWindowHandle| Ok(snapshot.clone());
    let handler = TransferHandlerAssembly::new(runtime, Clock, limits());

    let authority = handler.snapshot(&caller).unwrap();
    assert_eq!(authority.client_epoch(), ClientEpoch::new(1));

    let valid = lease_request(
        "request:lease-1",
        authority.client_id().clone(),
        authority.client_epoch(),
        1,
        ClientRect::new(
            ClientPoint::new(10.0, 10.0).unwrap(),
            ClientSize::new(100.0, 40.0).unwrap(),
        ),
    );
    assert!(matches!(
        handler.publish_lease(&caller, valid).unwrap(),
        TransferLeaseResponse::Published { .. }
    ));

    let invalid = lease_request(
        "request:lease-2-bad",
        authority.client_id().clone(),
        authority.client_epoch(),
        2,
        ClientRect::new(
            ClientPoint::new(-20.0, 10.0).unwrap(),
            ClientSize::new(100.0, 40.0).unwrap(),
        ),
    );
    assert!(matches!(
        handler.publish_lease(&caller, invalid).unwrap(),
        TransferLeaseResponse::Aborted { .. }
    ));

    let valid_second = lease_request(
        "request:lease-2",
        authority.client_id().clone(),
        authority.client_epoch(),
        2,
        ClientRect::new(
            ClientPoint::new(20.0, 20.0).unwrap(),
            ClientSize::new(100.0, 40.0).unwrap(),
        ),
    );
    assert!(matches!(
        handler.publish_lease(&caller, valid_second).unwrap(),
        TransferLeaseResponse::Published { .. }
    ));

    let teardown = handler.teardown().unwrap();
    assert_eq!(teardown.status(), TransferHandlerTeardownStatus::TornDown);
    assert_eq!(teardown.client_windows(), 1);
    assert_eq!(teardown.leases(), 1);
    assert_eq!(
        handler.teardown().unwrap().status(),
        TransferHandlerTeardownStatus::AlreadyTornDown
    );
    assert_eq!(
        handler.snapshot(&caller),
        Err(TransferHandlerError::Inactive)
    );
}

#[test]
fn destroyed_windows_release_client_state_without_reusing_epochs() {
    let caller = handle("main");
    let main_window = WindowId::new("window:main").unwrap();
    let runtime = |caller: &HostWindowHandle| {
        let window_id = format!("window:{caller}");
        ManagedTransferSnapshot::new(caller, [managed_window(caller.as_str(), &window_id)])
    };
    let handler = TransferHandlerAssembly::new(runtime, Clock, limits());

    let first = handler.snapshot(&caller).unwrap();
    handler.destroy_window(&main_window).unwrap();
    let mut previous_epoch = first.client_epoch();
    for index in 0..32 {
        let short_lived = handle(&format!("short-lived-{index}"));
        let snapshot = handler.snapshot(&short_lived).unwrap();
        assert!(snapshot.client_epoch() > previous_epoch);
        previous_epoch = snapshot.client_epoch();
        handler
            .destroy_window(&WindowId::new(format!("window:{short_lived}")).unwrap())
            .unwrap();
    }

    let second = handler.snapshot(&caller).unwrap();
    assert!(second.client_epoch() > previous_epoch);
    assert_ne!(first.client_id(), second.client_id());

    let bounds = ClientRect::new(
        ClientPoint::new(10.0, 10.0).unwrap(),
        ClientSize::new(20.0, 20.0).unwrap(),
    );
    for client_id in [first.client_id(), second.client_id()] {
        let stale = lease_request(
            "request:stale",
            client_id.clone(),
            first.client_epoch(),
            1,
            bounds,
        );
        let TransferLeaseResponse::Aborted { abort } =
            handler.publish_lease(&caller, stale).unwrap()
        else {
            panic!("a destroyed renderer epoch published a lease");
        };
        assert_eq!(
            abort.source(),
            TransferAbortSource::Transfer(TransferErrorCode::StaleClientEpoch)
        );
    }
    let current = lease_request(
        "request:current",
        second.client_id().clone(),
        second.client_epoch(),
        1,
        bounds,
    );
    assert!(matches!(
        handler.publish_lease(&caller, current).unwrap(),
        TransferLeaseResponse::Published { .. }
    ));
    handler.destroy_window(&main_window).unwrap();
    let discarded = handler.teardown().unwrap();
    assert_eq!(discarded.client_windows(), 0);
    assert_eq!(discarded.leases(), 0);
}

#[test]
fn snapshot_command_orders_window_hints_without_waiting_for_another_probe() {
    use std::{
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
            mpsc,
        },
        thread,
        time::Duration,
    };

    use longhorn_tauri_transfer::{
        AssembledTransferCommands, PanelTransferAdapter, TRANSFER_CLIENT_CHANGED_EVENT,
        TauriTransferState, TransferCallerAuthority, TransferSessionRegistrar,
        longhorn_transfer_snapshot,
    };
    use longhorn_transfer::{
        PanelSessionStartRequest, PanelTransferCommand, PanelTransferResponse,
        TerminalTransferAttempt, TransferClientSnapshot, TransferSessionResponse,
    };
    use tauri::{Listener, Manager, WebviewWindowBuilder};

    struct SnapshotOnly;

    impl PanelTransferAdapter for SnapshotOnly {
        fn start_panel(
            &mut self,
            _: TransferCallerAuthority,
            _: PanelSessionStartRequest,
            _: &mut TransferSessionRegistrar<'_>,
        ) -> TransferSessionResponse {
            unreachable!("this test only invokes the snapshot command");
        }

        fn commit_panel(
            &mut self,
            _: TransferCallerAuthority,
            _: PanelTransferCommand,
            _: TerminalTransferAttempt,
        ) -> PanelTransferResponse {
            unreachable!("this test only invokes the snapshot command");
        }
    }

    let timeout = Duration::from_secs(5);
    let (blocked, reached_recheck) = mpsc::channel();
    let (release_recheck, resume) = mpsc::channel();
    let resume = Mutex::new(resume);
    let main_probes = AtomicUsize::new(0);
    let windows = [
        managed_window("main", "window:main"),
        managed_window("other", "window:other"),
    ];
    let runtime = move |caller: &HostWindowHandle| {
        if caller.as_str() == "main" && main_probes.fetch_add(1, Ordering::SeqCst) == 1 {
            blocked.send(()).unwrap();
            resume.lock().unwrap().recv().unwrap();
        }
        ManagedTransferSnapshot::new(caller, windows.clone())
    };
    let handler = TransferHandlerAssembly::new(runtime, Clock, limits());
    let commands = Arc::new(AssembledTransferCommands::new(handler, SnapshotOnly));
    let app = tauri::test::mock_builder()
        .manage(TauriTransferState::new(commands))
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let main = WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .unwrap();
    let other = WebviewWindowBuilder::new(&app, "other", tauri::WebviewUrl::default())
        .build()
        .unwrap();

    let (events, received_events) = mpsc::channel();
    for window in [&main, &other] {
        let events = events.clone();
        let label = window.label().to_owned();
        window.listen(TRANSFER_CLIENT_CHANGED_EVENT, move |event| {
            let snapshot: TransferClientSnapshot = serde_json::from_str(event.payload()).unwrap();
            events
                .send((label.clone(), snapshot.client_epoch()))
                .unwrap();
        });
    }
    let snapshot = |window: tauri::WebviewWindow<tauri::test::MockRuntime>| {
        tauri::async_runtime::block_on(longhorn_transfer_snapshot(
            window.clone(),
            window.state::<TauriTransferState>(),
        ))
        .unwrap()
    };
    let first_window = main.clone();
    let first = thread::spawn(move || snapshot(first_window));
    reached_recheck.recv_timeout(timeout).unwrap();

    let (completed, progress) = mpsc::channel();
    let concurrent = thread::spawn(move || {
        completed.send(snapshot(other)).unwrap();
        completed.send(snapshot(main)).unwrap();
    });
    let other_before_release = progress.recv_timeout(timeout);
    let main_before_release = if other_before_release.is_ok() {
        progress.recv_timeout(timeout)
    } else {
        Err(mpsc::RecvTimeoutError::Timeout)
    };
    release_recheck.send(()).unwrap();
    let first = first.join().unwrap();
    concurrent.join().unwrap();
    let other = other_before_release.expect("another window must finish while main is blocked");
    let main = main_before_release.expect("a newer snapshot must overtake the stalled probe");

    assert!(first.client_epoch() < main.client_epoch());
    assert_eq!(
        received_events.try_iter().collect::<Vec<_>>(),
        [
            ("other".to_owned(), other.client_epoch()),
            ("main".to_owned(), main.client_epoch()),
        ]
    );
}

#[test]
fn snapshot_validation_rejects_spoofed_or_ambiguous_callers() {
    let main = managed_window("main", "window:main");
    assert_eq!(
        ManagedTransferSnapshot::new(&handle("other"), [main.clone()]),
        Err(TransferRuntimeError::UnmanagedCaller(handle("other")))
    );
    assert_eq!(
        ManagedTransferSnapshot::new(&handle("main"), [main.clone(), main]),
        Err(TransferRuntimeError::DuplicateTransportHandle(handle(
            "main"
        )))
    );
}

fn limits() -> TransferLimits {
    TransferLimits::new(
        8,
        4,
        4,
        8,
        100,
        TransferDuration::new(1_000),
        TransferDuration::new(250),
    )
    .unwrap()
}

fn lease_request(
    request_id: &str,
    client_id: longhorn_core::TransferClientId,
    epoch: ClientEpoch,
    generation: u64,
    bounds: ClientRect,
) -> TransferLeaseRequest {
    TransferLeaseRequest::new(
        TransferRequestId::new(request_id).unwrap(),
        client_id,
        epoch,
        LeaseGeneration::new(generation),
        [ClientDropZone::new(
            DropZoneId::new("zone:primary").unwrap(),
            bounds,
            None,
            TransferTargetBinding::PanelRegion {
                host_binding_id: TransferHostBindingId::new("binding:main").unwrap(),
                document_id: DomainId::new("layout.main").unwrap(),
                revision: TransferRevision::new(1),
                surface_id: SurfaceId::new("surface:main").unwrap(),
                region_id: RegionId::new("region:main").unwrap(),
            },
        )],
    )
}

fn managed_window(handle_value: &str, window_id: &str) -> ManagedTransferWindow {
    ManagedTransferWindow::new(
        WindowId::new(window_id).unwrap(),
        handle(handle_value),
        ScreenRect::new(ScreenPoint::new(90, 90), ScreenSize::new(1_000, 800)),
        ScreenRect::new(ScreenPoint::new(100, 120), ScreenSize::new(800, 600)),
    )
}

fn handle(value: &str) -> HostWindowHandle {
    HostWindowHandle::new(value).unwrap()
}

#[test]
fn snapshot_destroy_race_never_leaks_client_capacity() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    let caller = handle("main");
    let calls = AtomicUsize::new(0);
    let racing = AtomicBool::new(true);
    let runtime = |caller: &HostWindowHandle| {
        let call = calls.fetch_add(1, Ordering::SeqCst);
        let mut window_id = format!("window:{caller}");
        // On the post-bind probe, simulate either a missing label or a label
        // already bound to another window.
        if racing.load(Ordering::SeqCst) && call % 2 == 1 {
            if call % 4 == 1 {
                return Err(TransferRuntimeError::UnmanagedCaller(caller.clone()));
            }
            window_id = format!("replacement:{caller}");
        }
        ManagedTransferSnapshot::new(caller, [managed_window(caller.as_str(), &window_id)])
    };
    let handler = TransferHandlerAssembly::new(runtime, Clock, limits());

    // Far more raced cycles than the client-window capacity: each must fail
    // with the runtime error, never with a capacity exhaustion from leaked
    // bindings.
    for index in 0..32 {
        let racing_caller = handle(&format!("racing-{index}"));
        let error = handler.snapshot(&racing_caller).unwrap_err();
        assert!(
            matches!(error, TransferHandlerError::Runtime(_)),
            "raced snapshot leaked capacity: {error:?}"
        );
    }

    racing.store(false, Ordering::SeqCst);
    let authority = handler.snapshot(&caller).unwrap();
    assert!(authority.client_epoch().get() > 32);
}

#[test]
fn real_panel_commit_through_transfer_handler_moves_panel_without_holding_lock() {
    use std::{fs, sync::Arc, time::Duration};

    use longhorn_config::{
        ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath,
        DurabilityRequirement, MutationOptions, StorageClass,
    };
    use longhorn_core::{
        LayoutSchemaId, PanelDefinitionId, PanelInstanceId, RegionFamilyId, SchemaVersion,
        SurfaceRevision,
    };
    use longhorn_storage_layout::StorageRoots;
    use longhorn_surfaces::{
        EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutSchemaDefinition, PanelDefinition,
        PanelInstance, PanelInstancePolicy, ParticipatingWindow, PlacementSelector,
        RegionDefinition, RegionState, SurfaceDocument, SurfaceHostPreference, SurfaceLimits,
        SurfaceRecord, normalize_document,
    };
    use longhorn_surfaces_config::{
        NoSurfaceMigration, RegisteredSurfaceDomain, SurfaceBackupPolicy,
    };
    use longhorn_tauri_transfer::{
        AssembledTransferCommands, PanelTransferAdapter, TransferCallerAuthority,
        TransferSessionRegistrar,
    };
    use longhorn_transfer::{
        DragSessionId, DragSessionIdAllocationError, DragSessionIdAllocator, PanelHostBinding,
        PanelHostBindings, PanelSessionAdmission, PanelSessionStartRequest, PanelTransferCommand,
        PanelTransferCompletion, PanelTransferOperation, PanelTransferResponse,
        TerminalTransferAttempt, TransferAbort, TransferCommitSelector, TransferSessionRequest,
        TransferSessionResponse, TransferSessionStarted, commit_resolved_panel_transfer,
        prepare_panel_session_source,
    };
    use tempfile::TempDir;

    struct OneShotAllocator(Option<[u8; 16]>);

    impl DragSessionIdAllocator for OneShotAllocator {
        fn allocate(&mut self) -> Result<[u8; 16], DragSessionIdAllocationError> {
            self.0.take().ok_or(DragSessionIdAllocationError)
        }
    }

    struct RealPanelAdapter<R, C> {
        store: ConfigStore,
        domain: RegisteredSurfaceDomain<NoSurfaceMigration>,
        bindings: PanelHostBindings,
        source_binding: TransferHostBindingId,
        allocator: OneShotAllocator,
        // Held only to prove the coordinator lock is not held during commit_panel.
        handler: Arc<TransferHandlerAssembly<R, C>>,
        probe_caller: HostWindowHandle,
    }

    impl<R, C> PanelTransferAdapter for RealPanelAdapter<R, C>
    where
        R: longhorn_tauri_transfer::ManagedTransferRuntime,
        C: MonotonicClock,
    {
        fn start_panel(
            &mut self,
            caller: TransferCallerAuthority,
            request: PanelSessionStartRequest,
            sessions: &mut TransferSessionRegistrar<'_>,
        ) -> TransferSessionResponse {
            let admission = PanelSessionAdmission::new(
                caller.window_id().clone(),
                caller.client_id().clone(),
                caller.client_epoch(),
                request.panel_instance_id().clone(),
                self.source_binding.clone(),
                caller.session_lifetime(),
            );
            let source = match prepare_panel_session_source(
                &self.store,
                &self.domain,
                &self.bindings,
                &admission,
            ) {
                Ok(source) => source,
                Err(error) => {
                    return TransferSessionResponse::Aborted {
                        abort: TransferAbort::from_panel(request.request_id().clone(), &error),
                    };
                }
            };
            match sessions.create_session(
                &mut self.allocator,
                TransferSessionRequest::new(source, caller.session_lifetime()),
            ) {
                Ok(Ok(receipt)) => TransferSessionResponse::Started {
                    session: TransferSessionStarted::from_domain(
                        request.request_id().clone(),
                        receipt,
                    ),
                },
                Ok(Err(error)) => TransferSessionResponse::Aborted {
                    abort: TransferAbort::from_transfer(request.request_id().clone(), &error),
                },
                Err(error) => TransferSessionResponse::Aborted {
                    abort: TransferAbort::host_transfer(
                        request.request_id().clone(),
                        longhorn_transfer::TransferErrorCode::UnknownClientEpoch,
                        error.to_string(),
                    ),
                },
            }
        }

        fn commit_panel(
            &mut self,
            _caller: TransferCallerAuthority,
            request: PanelTransferCommand,
            attempt: TerminalTransferAttempt,
        ) -> PanelTransferResponse {
            // Re-entering handler.snapshot() here would deadlock if commit_panel
            // held the coordinator mutex across consumer config I/O.
            let _ = self.handler.snapshot(&self.probe_caller).unwrap();

            let options =
                MutationOptions::new(Duration::from_secs(1), DurabilityRequirement::Atomic);
            match commit_resolved_panel_transfer(
                &self.store,
                &self.domain,
                &self.bindings,
                options,
                PanelTransferOperation::Move,
                attempt,
            ) {
                Ok(receipt) => PanelTransferResponse::Committed {
                    completion: Box::new(PanelTransferCompletion::from_domain(
                        request.request_id().clone(),
                        &receipt,
                    )),
                },
                Err(error) => PanelTransferResponse::Aborted {
                    abort: TransferAbort::from_panel(request.request_id().clone(), &error),
                },
            }
        }
    }

    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let data = root.join("data");
    for path in [
        root.join("config"),
        data.clone(),
        root.join("state"),
        root.join("cache"),
        root.join("runtime"),
        root.join("log"),
        root.join("backups"),
    ] {
        fs::create_dir_all(path).unwrap();
    }
    let roots = StorageRoots::new(
        root.join("config"),
        &data,
        root.join("state"),
        root.join("cache"),
        root.join("runtime"),
        root.join("log"),
        root.join("backups"),
    )
    .unwrap();
    let coordination = CoordinationAuthority::new(data).unwrap();
    let mut store = ConfigStore::new(roots.clone(), coordination.clone());

    let schema_id = LayoutSchemaId::new("schema:main").unwrap();
    let region_id = RegionId::new("region:main").unwrap();
    let def_id = PanelDefinitionId::new("panel-def:tool").unwrap();
    let panel_id = PanelInstanceId::new("panel:tool-1").unwrap();
    let source_surface = SurfaceId::new("surface:source").unwrap();
    let target_surface = SurfaceId::new("surface:target").unwrap();
    let source_window = WindowId::new("window:main").unwrap();
    let target_window = WindowId::new("window:workspace").unwrap();
    let domain_id = DomainId::new("layout.main").unwrap();
    let source_binding = TransferHostBindingId::new("binding:source").unwrap();
    let target_binding = TransferHostBindingId::new("binding:target").unwrap();

    let registry = LayoutDefinitionRegistry::new(
        SurfaceLimits::builder()
            .maximum_schemas(4)
            .maximum_regions_per_schema(4)
            .maximum_sizing_slots_per_schema(4)
            .maximum_panel_definitions(4)
            .maximum_surfaces(4)
            .maximum_windows(4)
            .maximum_host_preferences_per_surface(4)
            .maximum_label_bytes(64)
            .maximum_panel_instances(16)
            .maximum_panel_instances_per_region(8)
            .build()
            .unwrap(),
        [LayoutSchemaDefinition::new(
            schema_id.clone(),
            [RegionDefinition::new(
                region_id.clone(),
                RegionFamilyId::new("family:content").unwrap(),
                10,
                EmptyRegionPolicy::KeepVisible,
                false,
            )],
            [],
        )],
        [PanelDefinition::new(
            def_id.clone(),
            [PlacementSelector::Region(region_id.clone())],
            [PlacementSelector::Region(region_id.clone())],
            PanelInstancePolicy::Multiple,
            true,
            true,
        )],
    )
    .unwrap();

    let initial_document = SurfaceDocument::new(
        SurfaceRevision::new(1),
        [
            SurfaceRecord::new(
                source_surface.clone(),
                schema_id.clone(),
                None,
                [RegionState::new(
                    region_id.clone(),
                    [panel_id.clone()],
                    Some(panel_id.clone()),
                    None,
                )],
                [],
                [SurfaceHostPreference::new(source_window.clone(), 0)],
            ),
            SurfaceRecord::new(
                target_surface.clone(),
                schema_id,
                None,
                [RegionState::new(region_id.clone(), [], None, None)],
                [],
                [SurfaceHostPreference::new(target_window.clone(), 0)],
            ),
        ],
        [PanelInstance::new(panel_id.clone(), def_id)],
        [
            ParticipatingWindow::new(source_window.clone(), Some(source_surface.clone())),
            ParticipatingWindow::new(target_window.clone(), Some(target_surface.clone())),
        ],
    );

    let domain = RegisteredSurfaceDomain::new(
        DomainDescriptor::new(
            domain_id.clone(),
            SchemaVersion::new(1).unwrap(),
            StorageClass::MachineState,
            Some(DomainFilePath::new("workspace/layout.json").unwrap()),
        )
        .unwrap(),
        normalize_document(&registry, &initial_document).unwrap(),
        registry,
        NoSurfaceMigration,
        SurfaceBackupPolicy::Include,
    )
    .unwrap();
    store.register(&domain).unwrap();

    let bindings = PanelHostBindings::new([
        PanelHostBinding::surface_container(
            source_binding.clone(),
            source_window.clone(),
            domain_id.clone(),
            source_surface.clone(),
        ),
        PanelHostBinding::surface_container(
            target_binding.clone(),
            target_window.clone(),
            domain_id.clone(),
            target_surface.clone(),
        ),
    ])
    .unwrap();

    let source_handle = handle("main");
    let target_handle = handle("workspace");
    let windows = [
        ManagedTransferWindow::new(
            source_window,
            source_handle.clone(),
            ScreenRect::new(ScreenPoint::new(0, 0), ScreenSize::new(500, 500)),
            ScreenRect::new(ScreenPoint::new(10, 30), ScreenSize::new(480, 460)),
        ),
        ManagedTransferWindow::new(
            target_window,
            target_handle.clone(),
            ScreenRect::new(ScreenPoint::new(600, 0), ScreenSize::new(500, 500)),
            ScreenRect::new(ScreenPoint::new(610, 30), ScreenSize::new(480, 460)),
        ),
    ];
    let runtime =
        move |caller: &HostWindowHandle| ManagedTransferSnapshot::new(caller, windows.clone());
    let handler = Arc::new(TransferHandlerAssembly::new(runtime, Clock, limits()));
    let mut adapter_store = ConfigStore::new(roots, coordination);
    adapter_store.register(&domain).unwrap();
    let adapter = RealPanelAdapter {
        store: adapter_store,
        domain: domain.clone(),
        bindings,
        source_binding,
        allocator: OneShotAllocator(Some([42; 16])),
        handler: Arc::clone(&handler),
        probe_caller: target_handle.clone(),
    };
    let commands = AssembledTransferCommands::from_shared(handler, adapter);

    let _source_authority = commands.snapshot(&source_handle).unwrap();
    let target_authority = commands.snapshot(&target_handle).unwrap();

    let lease_response = commands
        .publish_lease(
            &target_handle,
            TransferLeaseRequest::new(
                TransferRequestId::new("request:lease-target").unwrap(),
                target_authority.client_id().clone(),
                target_authority.client_epoch(),
                LeaseGeneration::new(1),
                [ClientDropZone::new(
                    DropZoneId::new("zone:target").unwrap(),
                    ClientRect::new(
                        ClientPoint::new(10.0, 10.0).unwrap(),
                        ClientSize::new(200.0, 200.0).unwrap(),
                    ),
                    None,
                    TransferTargetBinding::PanelRegion {
                        host_binding_id: target_binding,
                        document_id: domain_id,
                        revision: TransferRevision::new(1),
                        surface_id: target_surface.clone(),
                        region_id: region_id.clone(),
                    },
                )],
            ),
        )
        .unwrap();
    assert!(matches!(
        lease_response,
        TransferLeaseResponse::Published { .. }
    ));

    let started = commands
        .start_panel(
            &source_handle,
            PanelSessionStartRequest::new(
                TransferRequestId::new("request:start-panel").unwrap(),
                panel_id.clone(),
            ),
        )
        .unwrap();
    assert!(
        matches!(started, TransferSessionResponse::Started { .. }),
        "expected started panel session, got {started:?}"
    );
    let session_id = DragSessionId::from_entropy([42; 16]);

    let committed = commands
        .commit_panel(
            &source_handle,
            PanelTransferCommand::new(
                TransferRequestId::new("request:commit-panel").unwrap(),
                session_id,
                TransferCommitSelector::ExplicitZone {
                    drop_zone_id: DropZoneId::new("zone:target").unwrap(),
                },
            ),
        )
        .unwrap();
    assert!(
        matches!(committed, PanelTransferResponse::Committed { .. }),
        "expected committed panel transfer, got {committed:?}"
    );

    let longhorn_config::LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
        panic!("persisted surface document should load cleanly");
    };
    let persisted = loaded.value;
    assert_eq!(persisted.revision(), SurfaceRevision::new(2));
    assert!(
        persisted
            .surface(&source_surface)
            .unwrap()
            .region(&region_id)
            .unwrap()
            .panel_instance_ids()
            .is_empty()
    );
    assert_eq!(
        persisted
            .surface(&target_surface)
            .unwrap()
            .region(&region_id)
            .unwrap()
            .panel_instance_ids(),
        &[panel_id]
    );
}
