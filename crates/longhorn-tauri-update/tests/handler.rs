//! Direct, serialized, and Tauri mock-runtime handler conformance.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use longhorn_tauri_update::{
    TauriUpdateState, UpdateHandlerAssembly, UpdateHostAuthority, UpdateHostError,
    UpdateHostService, longhorn_update_apply, longhorn_update_cancel, longhorn_update_check,
    longhorn_update_defer, longhorn_update_prepare, longhorn_update_select_channel,
    longhorn_update_snapshot, update_changed_event,
};
use longhorn_update::{
    ArtifactFetch, Channel, DeferralCause, EndpointUrl, FetchError, FetchProgress,
    PreparedTransfer, SourceRequest, UpdateApplyCommand, UpdateAvailabilityProjection,
    UpdateCancelCommand, UpdateChangedKind, UpdateCheckCommand, UpdateDeferCommand,
    UpdateOutcomeProjection, UpdatePrepareCommand, UpdatePrepareStart, UpdateProgressEvent,
    UpdateProgressProjection, UpdateProtocolVersion, UpdateRejectionCode,
    UpdateSelectChannelCommand, UpdateSnapshot,
};
use semver::Version;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

struct Authority {
    calls: Arc<Mutex<Vec<String>>>,
}

impl Authority {
    fn new(calls: Arc<Mutex<Vec<String>>>) -> Self {
        Self { calls }
    }

    fn record(&self, what: &str, caller: &str) {
        self.calls.lock().unwrap().push(format!("{what}:{caller}"));
    }
}

impl UpdateHostAuthority for Authority {
    fn snapshot(&mut self, caller: &str) -> Result<UpdateSnapshot, UpdateHostError> {
        self.record("snapshot", caller);
        Ok(snapshot())
    }

    fn check(
        &mut self,
        caller: &str,
        _: UpdateCheckCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.record("check", caller);
        Ok(committed())
    }

    fn select_channel(
        &mut self,
        caller: &str,
        _: UpdateSelectChannelCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.record("select_channel", caller);
        Ok(committed())
    }

    fn defer(
        &mut self,
        caller: &str,
        _: UpdateDeferCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.record("defer", caller);
        Ok(committed())
    }

    fn begin_prepare(
        &mut self,
        caller: &str,
        command: UpdatePrepareCommand,
    ) -> Result<UpdatePrepareStart, UpdateHostError> {
        self.record("prepare", caller);
        Ok(UpdatePrepareStart::Transfer(PreparedTransfer::new(
            command.authority_epoch,
            Version::parse(&command.version).unwrap(),
            SourceRequest::new(EndpointUrl::new("https://example.test/app.tar.gz").unwrap()),
        )))
    }

    fn complete_prepare(
        &mut self,
        caller: &str,
        _: PreparedTransfer,
        _: Result<Vec<u8>, FetchError>,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.record("complete_prepare", caller);
        Ok(committed())
    }

    fn apply(
        &mut self,
        caller: &str,
        _: UpdateApplyCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.record("apply", caller);
        Ok(rejected())
    }

    fn cancel(
        &mut self,
        caller: &str,
        _: UpdateCancelCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.record("cancel", caller);
        Ok(committed())
    }
}

/// Transfers nothing, reports what it was told to report.
struct Fetch {
    bytes: Vec<u8>,
    reports: Vec<FetchProgress>,
    calls: AtomicU32,
}

impl Fetch {
    fn reporting(reports: Vec<FetchProgress>) -> Self {
        Self {
            bytes: b"an artifact".to_vec(),
            reports,
            calls: AtomicU32::new(0),
        }
    }
}

impl ArtifactFetch for Fetch {
    fn fetch(
        &self,
        _request: &SourceRequest,
        _limit: u64,
        report: &mut dyn FnMut(FetchProgress),
    ) -> Result<Vec<u8>, FetchError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        for progress in &self.reports {
            report(*progress);
        }
        Ok(self.bytes.clone())
    }
}

/// A signal a test thread can wait on and another can set.
struct Signal {
    set: Mutex<bool>,
    condvar: Condvar,
}

impl Signal {
    fn new() -> Self {
        Self {
            set: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }

    fn wait(&self) {
        let mut set = self.set.lock().unwrap();
        while !*set {
            set = self.condvar.wait(set).unwrap();
        }
    }

    fn raise(&self) {
        *self.set.lock().unwrap() = true;
        self.condvar.notify_all();
    }
}

/// Blocks inside the transfer until the test releases it, so the lock state
/// during the transfer can be observed.
struct BlockingFetch {
    entered: Signal,
    release: Signal,
    reports: Vec<FetchProgress>,
}

impl ArtifactFetch for BlockingFetch {
    fn fetch(
        &self,
        _request: &SourceRequest,
        _limit: u64,
        report: &mut dyn FnMut(FetchProgress),
    ) -> Result<Vec<u8>, FetchError> {
        self.entered.raise();
        self.release.wait();
        for progress in &self.reports {
            report(*progress);
        }
        Ok(b"an artifact".to_vec())
    }
}

fn snapshot() -> UpdateSnapshot {
    UpdateSnapshot {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 4,
        channel: Channel::Production,
        installed_version: "1.3.0".to_owned(),
        availability: UpdateAvailabilityProjection::UpToDate,
        deferral: None,
        staged: None,
        progress: UpdateProgressProjection::Idle,
    }
}

fn committed() -> UpdateOutcomeProjection {
    UpdateOutcomeProjection::Committed {
        snapshot: snapshot(),
    }
}

fn rejected() -> UpdateOutcomeProjection {
    UpdateOutcomeProjection::Rejected {
        code: UpdateRejectionCode::NotWritable,
        snapshot: snapshot(),
    }
}

fn check_command() -> UpdateCheckCommand {
    UpdateCheckCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 4,
    }
}

fn prepare_command() -> UpdatePrepareCommand {
    UpdatePrepareCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 4,
        version: "1.4.0".to_owned(),
    }
}

#[test]
fn mock_runtime_uses_one_injected_caller_aware_assembly() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let fetch = Arc::new(Fetch::reporting(vec![FetchProgress::of(7, 14)]));
    let service = Arc::new(UpdateHandlerAssembly::new(
        Authority::new(calls.clone()),
        fetch.clone(),
    ));
    let app = tauri::test::mock_builder()
        .manage(TauriUpdateState::new(service))
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let window = WebviewWindowBuilder::new(&app, "settings", WebviewUrl::default())
        .build()
        .unwrap();

    assert_eq!(
        longhorn_update_snapshot(window.clone(), app.state()).unwrap(),
        snapshot()
    );
    assert_eq!(
        longhorn_update_check(window.clone(), app.state(), check_command()).unwrap(),
        committed()
    );
    assert_eq!(
        longhorn_update_select_channel(
            window.clone(),
            app.state(),
            UpdateSelectChannelCommand {
                protocol_version: UpdateProtocolVersion::CURRENT,
                authority_epoch: 4,
                channel: Channel::Beta,
            }
        )
        .unwrap(),
        committed()
    );
    assert_eq!(
        longhorn_update_defer(
            window.clone(),
            app.state(),
            UpdateDeferCommand {
                protocol_version: UpdateProtocolVersion::CURRENT,
                authority_epoch: 4,
                version: "1.4.0".to_owned(),
                cause: DeferralCause::UserPostponed,
            }
        )
        .unwrap(),
        committed()
    );
    // Preparing transfers and retains; it does not replace.
    assert_eq!(
        tauri::async_runtime::block_on(longhorn_update_prepare(
            window.clone(),
            app.state(),
            prepare_command(),
        ))
        .unwrap(),
        committed()
    );
    // A refused apply still answers. The caller gets the reason and the state
    // as it remains, not an adapter error.
    assert_eq!(
        longhorn_update_apply(
            window.clone(),
            app.state(),
            UpdateApplyCommand {
                protocol_version: UpdateProtocolVersion::CURRENT,
                authority_epoch: 4,
                version: "1.4.0".to_owned(),
            }
        )
        .unwrap(),
        rejected()
    );
    assert_eq!(
        longhorn_update_cancel(
            window,
            app.state(),
            UpdateCancelCommand {
                protocol_version: UpdateProtocolVersion::CURRENT,
                authority_epoch: 4,
            }
        )
        .unwrap(),
        committed()
    );

    assert_eq!(
        *calls.lock().unwrap(),
        [
            "snapshot:settings",
            "check:settings",
            "select_channel:settings",
            "defer:settings",
            "prepare:settings",
            "complete_prepare:settings",
            "apply:settings",
            "cancel:settings",
        ]
    );
    assert_eq!(fetch.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn serialized_service_and_committed_event_remain_payload_free() {
    let fetch = Arc::new(Fetch::reporting(Vec::new()));
    let service =
        UpdateHandlerAssembly::new(Authority::new(Arc::new(Mutex::new(Vec::new()))), fetch);

    let outcome: UpdateOutcomeProjection = serde_json::from_slice(
        &serde_json::to_vec(&service.check("settings", check_command()).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(outcome, committed());

    let event = update_changed_event(&committed(), UpdateChangedKind::Checked).unwrap();
    assert_eq!(event.authority_epoch, 4);
    assert_eq!(event.kind, UpdateChangedKind::Checked);
}

/// A rejection leaves the state as it was, so there is nothing to invalidate
/// and a consumer that refetched on one would be refetching for nothing.
#[test]
fn a_rejected_outcome_publishes_no_invalidation_hint() {
    assert!(update_changed_event(&rejected(), UpdateChangedKind::Progressed).is_none());
}

/// The adapter never widens a caller. Every command reads the window label
/// and passes it through, so a window's own identity is what authorizes it.
#[test]
fn every_command_passes_the_caller_through_unchanged() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let fetch = Arc::new(Fetch::reporting(Vec::new()));
    let service = UpdateHandlerAssembly::new(Authority::new(calls.clone()), fetch);

    drop(service.snapshot("main"));
    drop(service.prepare("main", prepare_command(), &mut |_| {}));
    drop(service.apply(
        "main",
        UpdateApplyCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: 4,
            version: "1.4.0".to_owned(),
        },
    ));

    assert_eq!(
        *calls.lock().unwrap(),
        [
            "snapshot:main",
            "prepare:main",
            "complete_prepare:main",
            "apply:main",
        ]
    );
}

/// Progress leaves through the assembly while the transfer runs, and carries
/// the epoch the transfer was started under.
#[test]
fn progress_is_reported_out_of_band_with_its_epoch() {
    let fetch = Arc::new(Fetch::reporting(vec![
        FetchProgress::of(5, 20),
        FetchProgress::of(20, 20),
    ]));
    let service =
        UpdateHandlerAssembly::new(Authority::new(Arc::new(Mutex::new(Vec::new()))), fetch);

    let mut observed: Vec<UpdateProgressEvent> = Vec::new();
    let outcome = service
        .prepare("settings", prepare_command(), &mut |event| {
            observed.push(event)
        })
        .unwrap();

    assert_eq!(observed.len(), 2);
    assert!(observed.iter().all(|event| event.authority_epoch == 4));
    assert_eq!(
        observed.last().unwrap().progress,
        UpdateProgressProjection::Downloading {
            received: 20,
            expected: Some(20),
            fraction: Some(1.0),
        }
    );
    assert!(matches!(outcome, UpdateOutcomeProjection::Committed { .. }));
}

/// The point of splitting prepare: the authority lock is released for the
/// transfer. Reading the snapshot while the transfer is blocked would deadlock
/// if the assembly still held it, so the read gets a watchdog.
#[test]
fn the_authority_lock_is_not_held_across_the_transfer() {
    let fetch = Arc::new(BlockingFetch {
        entered: Signal::new(),
        release: Signal::new(),
        reports: vec![FetchProgress::of(1, 2)],
    });
    let service = UpdateHandlerAssembly::new(
        Authority::new(Arc::new(Mutex::new(Vec::new()))),
        fetch.clone(),
    );

    std::thread::scope(|scope| {
        let service = &service;
        let preparing =
            scope.spawn(move || service.prepare("settings", prepare_command(), &mut |_| {}));
        fetch.entered.wait();

        // The transfer is on the stack. If the lock were held across it, this
        // read would block until the release below rather than answer now.
        let (sender, receiver) = std::sync::mpsc::channel();
        scope.spawn(move || {
            let _ = sender.send(service.snapshot("other").is_ok());
        });
        let readable = receiver.recv_timeout(std::time::Duration::from_secs(5));

        // Release first so a failed read still tears the scope down cleanly.
        fetch.release.raise();
        let outcome = preparing.join().unwrap().unwrap();
        assert!(matches!(outcome, UpdateOutcomeProjection::Committed { .. }));
        assert!(
            readable.expect("the authority lock is released during the transfer"),
            "the state is readable while the transfer runs"
        );
    });
}
