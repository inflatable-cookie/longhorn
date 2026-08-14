//! Packaged host for Card 159's last two update claims.
//!
//! Every other claim on that card was reachable by a binary that inspects a
//! filesystem, and `packaged-update-proof` makes them. These two need an
//! application that runs:
//!
//! 1. **Relaunch, and tauri#11392 under Longhorn's close handling.** A host
//!    that never relaunches cannot say whether `prevent_close` interferes.
//! 2. **The restart interlock against a genuinely open transfer session.**
//!    `packaged-update-proof` proves the *ordering* — gate after the transfer,
//!    before the install — against a `BusyProbe`. What it cannot prove is that
//!    a real session reports itself.
//!
//! Claim 2 is what this file is built around, and the whole point is that no
//! double appears anywhere in the path. The gate reads
//! [`TransferCoordinator::session_count`] through
//! [`transfer_session_probe`](longhorn_update::transfer_session_probe), and the
//! session it counts is one the coordinator accepted through its own
//! validation — bound client epoch, live window, drop zone, lease lifetime. A
//! session that satisfies the type system and not the coordinator would make
//! the proof green while proving nothing, which is the failure this claim
//! exists to close.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    cell::Cell,
    fs,
    path::PathBuf,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use longhorn_core::{
    DomainId, RegionId, ScreenPoint, ScreenRect, ScreenSize, SurfaceId, TransferSubjectId, WindowId,
};
use longhorn_transfer::{
    ClientEpoch, DragSessionIdAllocationError, DragSessionIdAllocator, DropZone, DropZoneId,
    InsertionPosition, LeaseGeneration, LeasePublication, MonotonicClock, TransferCapability,
    TransferClientId, TransferCoordinator, TransferDuration, TransferHostBindingId,
    TransferInstant, TransferLimits, TransferRevision, TransferSessionRequest,
    TransferSourceAuthority, TransferTargetBinding,
};
use longhorn_update::{InstallAuthorization, UpdateGate, transfer_session_probe};
use serde_json::{Value, json};
use tauri::{Manager, State, WindowEvent};

/// Wall clock in the coordinator's tick space.
///
/// The coordinator needs monotonic ticks and the tests use a fake. A packaged
/// host is where the real one belongs: a session that expires because time
/// genuinely passed is part of what "genuinely open" means.
struct HostClock {
    origin: Cell<Option<u64>>,
}

impl HostClock {
    const fn new() -> Self {
        Self {
            origin: Cell::new(None),
        }
    }
}

impl MonotonicClock for HostClock {
    fn now(&self) -> TransferInstant {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        // Rebased to the first observation so ticks start small, which keeps
        // lease arithmetic away from the top of the range.
        let origin = self.origin.get().unwrap_or_else(|| {
            self.origin.set(Some(seconds));
            seconds
        });
        TransferInstant::new(seconds.saturating_sub(origin))
    }
}

/// Allocates drag-session identifiers.
///
/// A counter rather than a random source: the proof records what it did, and a
/// reproducible id is easier to read back than an opaque one. Uniqueness
/// within a run is all the coordinator asks for.
struct CountingAllocator(u8);

impl DragSessionIdAllocator for CountingAllocator {
    fn allocate(&mut self) -> Result<[u8; 16], DragSessionIdAllocationError> {
        self.0 = self.0.wrapping_add(1);
        let mut bytes = [0_u8; 16];
        bytes[15] = self.0;
        Ok(bytes)
    }
}

/// Whether a close request is refused, as Longhorn's windowing host refuses
/// one when its lifecycle receipt reports a user close.
///
/// tauri#11392 names that refusal as a contributing factor in relaunch failing
/// on macOS, and Longhorn owns close handling — so this is ours to answer
/// rather than an upstream curiosity. A process-wide flag because the window
/// event handler cannot reach managed state before the app is built.
static PREVENT_CLOSE: AtomicBool = AtomicBool::new(true);

/// A marker written immediately before a relaunch is requested.
///
/// The only way to answer "did it come back" is to record the intent, die, and
/// look for the record on the next start. An in-memory flag cannot survive the
/// thing it is measuring.
const RELAUNCH_MARKER: &str = "relaunch-requested.json";

const WINDOW: &str = "main";
const CLIENT: &str = "client:update-proof";

struct Proof {
    coordinator: TransferCoordinator,
    clock: HostClock,
    allocator: CountingAllocator,
    generation: u64,
}

impl Proof {
    fn new() -> Result<Self, String> {
        let limits = TransferLimits::new(
            8,
            8,
            8,
            8,
            20,
            TransferDuration::new(100),
            TransferDuration::new(50),
        )
        .map_err(|error| format!("{error:?}"))?;
        Ok(Self {
            coordinator: TransferCoordinator::new(limits),
            clock: HostClock::new(),
            allocator: CountingAllocator(0),
            generation: 0,
        })
    }

    /// Opens a session the coordinator itself accepted.
    ///
    /// Every step is the coordinator's own validation rather than a shortcut
    /// into its state: the client epoch is bound, and the lease is published
    /// with a real drop zone and lifetime. If any
    /// of that is wrong the coordinator refuses, and `session_count` stays at
    /// zero — which is the honest outcome and is what makes the claim worth
    /// making.
    fn open_session(&mut self) -> Result<usize, String> {
        let window_id = WindowId::new(WINDOW).map_err(|error| format!("{error:?}"))?;
        let client_id = TransferClientId::new(CLIENT).map_err(|error| format!("{error:?}"))?;
        let (window, client) = (window_id.clone(), client_id.clone());
        let bounds = ScreenRect::new(ScreenPoint::new(0, 0), ScreenSize::new(900, 650));

        self.coordinator
            .bind_client_epoch(
                &self.clock,
                window.clone(),
                client.clone(),
                ClientEpoch::new(1),
            )
            .map_err(|error| format!("bind client epoch: {error:?}"))?;

        self.generation += 1;
        let zone = DropZone::new(
            DropZoneId::new("zone:update-proof").map_err(|error| format!("{error:?}"))?,
            ScreenRect::new(ScreenPoint::new(0, 0), ScreenSize::new(400, 300)),
            Some(InsertionPosition::new(0)),
            TransferCapability::MovePanel,
            TransferTargetBinding::PanelRegion {
                host_binding_id: TransferHostBindingId::new("host:update-proof")
                    .map_err(|error| format!("{error:?}"))?,
                document_id: DomainId::new("layout.workspace")
                    .map_err(|error| format!("{error:?}"))?,
                revision: TransferRevision::new(1),
                surface_id: SurfaceId::new("surface:update-proof")
                    .map_err(|error| format!("{error:?}"))?,
                region_id: RegionId::new("region:update-proof")
                    .map_err(|error| format!("{error:?}"))?,
            },
        );
        self.coordinator
            .publish_lease(
                &self.clock,
                LeasePublication::new(
                    window,
                    client,
                    ClientEpoch::new(1),
                    LeaseGeneration::new(self.generation),
                    TransferDuration::new(30),
                    bounds,
                    vec![zone],
                ),
            )
            .map_err(|error| format!("publish lease: {error:?}"))?;

        // A lease is not a session, which is the distinction this proof was
        // built on the wrong side of at first: `publish_lease` succeeded and
        // `session_count` stayed at zero. A lease advertises where a transfer
        // *could* land; a session is a transfer actually in flight, and the
        // interlock is about work in flight.
        self.coordinator
            .create_session(
                &self.clock,
                &mut self.allocator,
                TransferSessionRequest::new(
                    TransferSourceAuthority::Panel {
                        client_id: client_id.clone(),
                        client_epoch: ClientEpoch::new(1),
                        source_window_id: window_id.clone(),
                        subject_id: TransferSubjectId::new("panel:update-proof")
                            .map_err(|error| format!("{error:?}"))?,
                        host_binding_id: TransferHostBindingId::new("host:update-proof")
                            .map_err(|error| format!("{error:?}"))?,
                        document_id: DomainId::new("layout.workspace")
                            .map_err(|error| format!("{error:?}"))?,
                        revision: TransferRevision::new(1),
                        surface_id: SurfaceId::new("surface:update-proof")
                            .map_err(|error| format!("{error:?}"))?,
                        region_id: RegionId::new("region:update-proof")
                            .map_err(|error| format!("{error:?}"))?,
                    },
                    TransferDuration::new(40),
                ),
            )
            .map_err(|error| format!("create session: {error:?}"))?;

        Ok(self.coordinator.session_count())
    }

    /// Asks the gate whether an install may proceed, right now.
    ///
    /// The probe reads the live coordinator. Nothing between the gate and the
    /// session is a double.
    fn authorization(&self) -> (usize, bool, Option<String>) {
        let open = self.coordinator.session_count();
        let probe = transfer_session_probe(|| open);
        let gate = UpdateGate::new(vec![&probe]);
        let version = semver::Version::new(1, 0, 0);
        match gate.authorize(&version) {
            InstallAuthorization::Approved => (open, true, None),
            InstallAuthorization::Deferred(deferral) => {
                (open, false, Some(format!("{:?}", deferral.cause)))
            }
        }
    }
}

type Shared = Mutex<Proof>;

fn hold<'a>(state: &'a State<'_, Shared>) -> Result<MutexGuard<'a, Proof>, String> {
    state
        .lock()
        .map_err(|_| "proof state is poisoned".to_owned())
}

#[tauri::command]
fn proof_state(state: State<'_, Shared>) -> Result<Value, String> {
    let proof = hold(&state)?;
    let (open, approved, cause) = proof.authorization();
    Ok(json!({
        "schema": "longhorn.tauri-update-proof.v1",
        "openTransferSessions": open,
        "installWouldBeAuthorized": approved,
        "deferralCause": cause,
    }))
}

#[tauri::command]
fn open_transfer_session(state: State<'_, Shared>) -> Result<Value, String> {
    let mut proof = hold(&state)?;
    let opened = proof.open_session()?;
    let (open, approved, cause) = proof.authorization();
    Ok(json!({
        "schema": "longhorn.tauri-update-proof.v1",
        "openedThroughTheCoordinator": opened,
        "openTransferSessions": open,
        "installWouldBeAuthorized": approved,
        "deferralCause": cause,
        // The claim, stated where the evidence is: a real session, counted by
        // the coordinator that accepted it, refuses the install.
        "aGenuinelyOpenSessionRefusesTheInstall": open > 0 && !approved,
    }))
}

#[tauri::command]
fn close_transfer_sessions(state: State<'_, Shared>) -> Result<Value, String> {
    let mut proof = hold(&state)?;
    let receipt = proof.coordinator.discard_all();
    let (open, approved, cause) = proof.authorization();
    Ok(json!({
        "schema": "longhorn.tauri-update-proof.v1",
        "discarded": format!("{receipt:?}"),
        "openTransferSessions": open,
        "installWouldBeAuthorized": approved,
        "deferralCause": cause,
        "aQuiescentHostAuthorizesTheInstall": open == 0 && approved,
    }))
}

#[tauri::command]
fn attempt_install(state: State<'_, Shared>) -> Result<Value, String> {
    let proof = hold(&state)?;
    let (open, approved, cause) = proof.authorization();
    Ok(json!({
        "schema": "longhorn.tauri-update-proof.v1",
        "openTransferSessions": open,
        "authorized": approved,
        "deferralCause": cause,
        // Relaunch is not claimed here yet. `packaged-update-proof` performs
        // the replacement; what is still owed is a host that quits and comes
        // back under Longhorn's close handling, which is claim 1.
        "relaunchClaim": "not yet exercised - see README",
    }))
}

fn marker_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("app data dir: {error}"))?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join(RELAUNCH_MARKER))
}

/// Records the intent to relaunch, then relaunches.
///
/// `request_restart` rather than `restart`: the first triggers
/// `ExitRequested` and `Exit` reliably, which is the path a close handler
/// could interfere with. `restart` skips them when called on the main thread,
/// which would answer an easier question than the one asked.
#[tauri::command]
fn request_relaunch(app: tauri::AppHandle) -> Result<Value, String> {
    let path = marker_path(&app)?;
    let requested_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    fs::write(
        &path,
        json!({
            "requestedAt": requested_at,
            "preventCloseInstalled": PREVENT_CLOSE.load(Ordering::Relaxed),
        })
        .to_string(),
    )
    .map_err(|error| error.to_string())?;
    app.request_restart();
    // Unreachable in practice; returned so the command has a type and so a
    // relaunch that silently does nothing is visible as a returned value.
    Ok(json!({ "requestedRestart": true, "marker": path.to_string_lossy() }))
}

/// Whether a relaunch was requested last run, and whether it came back.
///
/// Reaching this at all is the evidence: the marker was written by a process
/// that then asked to die, and this one is reading it.
fn relaunch_evidence(app: &tauri::AppHandle) -> Value {
    let Ok(path) = marker_path(app) else {
        return json!({ "relaunchClaim": "app data dir unavailable" });
    };
    match fs::read_to_string(&path) {
        Ok(recorded) => {
            drop(fs::remove_file(&path));
            let requested: Value = serde_json::from_str(&recorded).unwrap_or(Value::Null);
            json!({
                "relaunchClaim": "met - the process came back after request_restart",
                "requested": requested,
            })
        }
        Err(_) => json!({
            "relaunchClaim": "not exercised this run - use the relaunch control",
        }),
    }
}

#[tauri::command]
fn relaunch_state(app: tauri::AppHandle) -> Value {
    relaunch_evidence(&app)
}

/// Card 159's last claim: an account sign-in through the real system browser.
///
/// Everything below the click is proved headlessly; this exercises the click.
/// A stub authorization server runs on loopback, `longhorn-browser` launches
/// the operator's actual browser at its approve page, the approve link is the
/// authorization redirect, `LoopbackRedirect` receives it, and the flow
/// accepts the callback in constant time. The stub stands where a real
/// identity provider would; every other piece is the production code.
#[tauri::command]
fn attempt_sign_in() -> Result<Value, String> {
    use longhorn_browser::{BrowserUrl, LoopbackRedirect, NativeSystemBrowser, SystemBrowser};
    use longhorn_licence::{AccountFlow, CodeVerifier};

    // Uniqueness is what the proof needs from these, not secrecy: the state
    // binds the callback to this flow, and a proof run is its own audience.
    let stamp = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis())
    );
    let state = format!("proof-state-{stamp}");
    let verifier = CodeVerifier::new(format!(
        "proof-verifier-{stamp}-padding-to-forty-three-chars"
    ))
    .map_err(|error| error.to_string())?;

    let listener = LoopbackRedirect::bind().map_err(|error| error.to_string())?;
    let flow = AccountFlow::begin(verifier, state.clone(), listener.port())
        .map_err(|error| error.to_string())?;

    // The approve page: one human-sized decision, whose link is the
    // authorization redirect a real server would answer with.
    let approve = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Stub authorization</title>\
         </head><body style=\"font-family:sans-serif;margin:4rem auto;max-width:30rem\">\
         <h1>Stub authorization server</h1>\
         <p>This page stands where an identity provider would. Approving sends \
         the authorization redirect to the application's loopback listener.</p>\
         <p><a href=\"{}?state={state}&code=proof-authorization-code\">Approve sign-in</a></p>\
         </body></html>",
        flow.redirect_uri()
    );
    let stub_port = serve_stub_authorization(approve).map_err(|error| error.to_string())?;

    NativeSystemBrowser
        .open(
            &BrowserUrl::new(format!("http://127.0.0.1:{stub_port}/authorize"))
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("browser launch: {error}"))?;

    // Blocking is correct here: a sync Tauri command runs off the main
    // thread, and the operator is in the browser for the duration.
    let callback = listener
        .receive(std::time::Duration::from_secs(120))
        .map_err(|error| format!("no redirect arrived: {error}"))?;
    let authorization = flow
        .accept_callback(&callback)
        .map_err(|error| format!("callback refused: {error}"))?;
    drop(authorization);

    Ok(json!({
        "schema": "longhorn.tauri-update-proof.v1",
        "rfc8252SignIn": "met - the system browser carried the flow: launch, approve, loopback redirect, constant-time acceptance",
        "state": state,
    }))
}

fn main() {
    let proof = Proof::new().expect("transfer limits are valid");
    tauri::Builder::default()
        .manage(Mutex::new(proof))
        .on_window_event(|_window, event| {
            // The tauri#11392 contributing factor, reproduced rather than
            // imported: Longhorn's windowing host refuses a user close the
            // same way, and the question is whether a relaunch survives it.
            if PREVENT_CLOSE.load(Ordering::Relaxed)
                && let WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            proof_state,
            open_transfer_session,
            close_transfer_sessions,
            attempt_install,
            attempt_sign_in,
            request_relaunch,
            relaunch_state
        ])
        .run(tauri::generate_context!())
        .expect("packaged update proof failed to run");
}

/// Serves one authorization page on loopback until the process ends.
///
/// A stub, not a server: one route, one verb, dropped when the proof exits.
/// The thread is deliberately leaked -- the page must stay reachable while
/// the operator's browser tab is open, and the process's end is its cleanup.
fn serve_stub_authorization(page: String) -> std::io::Result<u16> {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = [0_u8; 2048];
            drop(stream.read(&mut request));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
                page.len()
            );
            drop(stream.write_all(response.as_bytes()));
        }
    });
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The claim, without the window: the coordinator must actually accept the
    /// lease, and the gate must read that acceptance.
    ///
    /// Here rather than only in the packaged run because a session the
    /// coordinator refuses leaves `session_count` at zero, the gate approves,
    /// and the operator sees a green proof of nothing. This is the assertion
    /// that stops that.
    #[test]
    fn a_genuinely_open_session_refuses_the_install() {
        let mut proof = Proof::new().expect("limits");
        assert_eq!(proof.authorization(), (0, true, None));

        let opened = proof.open_session().expect("the coordinator accepts it");

        assert_eq!(opened, 1, "the coordinator counted the session it accepted");
        let (open, approved, cause) = proof.authorization();
        assert_eq!(open, 1);
        assert!(!approved, "an open session must refuse the install");
        assert!(cause.is_some(), "a refusal carries its reason");
    }

    /// The other half. Once the session is gone the install proceeds, so the
    /// refusal above is the session's doing and not a gate that always says no.
    #[test]
    fn a_quiescent_host_authorizes_the_install() {
        let mut proof = Proof::new().expect("limits");
        proof.open_session().expect("open");
        proof.coordinator.discard_all();

        assert_eq!(proof.authorization(), (0, true, None));
    }
}
