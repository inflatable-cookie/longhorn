//! Contract 020's last ceiling: a real drag across two real GPUI windows.
//!
//! Everything before this proved the *decision* — `live_transfer_windows`
//! observes windows and `TransferCoordinator` resolves a point against them.
//! What no backend had done is bind real mouse events to a session and release
//! over another window.
//!
//! # How a cross-window drag reaches Longhorn
//!
//! macOS routes mouse events during a drag to the window that received the
//! press, so the *source* window sees the release even when the cursor is over
//! a different one. That is what makes this work at all: the source learns the
//! global release point, and Longhorn decides which window is under it.
//!
//! `MouseUpEvent::position` is window-relative, so the screen point is the
//! window's own origin plus that. Both come from the host, neither from
//! Longhorn.
//!
//! # Freshness
//!
//! Windows are observed at release, not at press. A snapshot taken when the
//! drag began resolves against where windows *were*, and a window moved
//! mid-drag is exactly when a stale answer is wrong.

use std::{cell::RefCell, rc::Rc};

use gpui::{AnyWindowHandle, App, Global, Pixels, Point, Window};
use longhorn_core::{DomainId, SurfaceId, RegionId, ScreenPoint, ScreenRect, WindowId};
use longhorn_gpui_windowing::{GpuiWindowBackend, GpuiWindowKey, live_transfer_windows};
use longhorn_transfer::{ClientEpoch, DragSessionId, DragSessionIdAllocationError, DragSessionIdAllocator, DropZone, DropZoneId, LeaseGeneration, LeasePublication, MonotonicClock, TargetSelector, TerminalTransferResolution, TransferCapability, TransferClientId, TransferCoordinator, TransferDuration, TransferHostBindingId, TransferInstant, TransferLimits, TransferRevision, TransferSessionRequest, TransferSourceAuthority, TransferSubjectId, TransferTargetBinding};

/// One managed window, as this example knows it.
#[derive(Clone)]
pub struct ManagedWindow {
    pub window_id: WindowId,
    pub client_id: TransferClientId,
    pub handle: AnyWindowHandle,
}

/// Shared transfer state, reachable from either window's event handlers.
///
/// A gpui global rather than a channel: both windows run on the one main
/// thread, and the coordinator is not `Sync`. Nothing here is shared across
/// threads because nothing here can be.
pub struct TransferState {
    inner: Rc<RefCell<Inner>>,
}

impl Global for TransferState {}

struct Inner {
    coordinator: TransferCoordinator,
    windows: Vec<ManagedWindow>,
    session: Option<DragSessionId>,
    outcome: String,
    persistence: String,
    allocations: u64,
}

/// A monotonic clock that never advances.
///
/// Sessions in this example live for one gesture and the coordinator's expiry
/// bound is generous, so a real clock would only add a source of flakiness to
/// a proof about geometry. A product supplies a real one.
struct HeldClock;

impl MonotonicClock for HeldClock {
    fn now(&self) -> TransferInstant {
        TransferInstant::new(0)
    }
}

struct CountingAllocator(u64);

impl DragSessionIdAllocator for CountingAllocator {
    fn allocate(&mut self) -> Result<[u8; 16], DragSessionIdAllocationError> {
        let mut bytes = [0_u8; 16];
        bytes[..8].copy_from_slice(&self.0.to_le_bytes());
        Ok(bytes)
    }
}

impl TransferState {
    /// Binds both windows and publishes a drop zone covering each.
    ///
    /// Bounds are observed rather than assumed. The origin a window was asked
    /// for and the origin it got are the same on this platform — the
    /// neighbouring prototype proved that — but a lease published from a
    /// request rather than a reading is a lease that is wrong the first time
    /// a window manager disagrees.
    ///
    /// The zone is the whole window here. A product publishes the regions it
    /// actually accepts; the resolution is the same either way.
    pub fn install(windows: Vec<ManagedWindow>, cx: &mut App) {
        let observed = {
            let mut backend = longhorn_gpui_windowing_prototype::GpuiAppBackend::new(cx);
            for window in &windows {
                backend.adopt(window.handle);
            }
            let keys: Vec<(WindowId, GpuiWindowKey)> = windows
                .iter()
                .map(|window| {
                    (
                        window.window_id.clone(),
                        GpuiWindowKey::new(window.handle.window_id().as_u64()),
                    )
                })
                .collect();
            live_transfer_windows(&mut backend, keys.iter().map(|(id, key)| (id, *key)))
                .expect("both windows observe")
        };
        let bounds: Vec<ScreenRect> = observed
            .iter()
            .map(longhorn_transfer::LiveTransferWindow::outer_bounds)
            .collect();

        let mut coordinator = TransferCoordinator::new(
            TransferLimits::new(
                8,
                8,
                8,
                8,
                16,
                TransferDuration::new(1_000),
                TransferDuration::new(500),
            )
            .expect("transfer limits"),
        );

        for (window, bounds) in windows.iter().zip(&bounds) {
            coordinator
                .bind_client_epoch(
                    &HeldClock,
                    window.window_id.clone(),
                    window.client_id.clone(),
                    ClientEpoch::new(1),
                )
                .expect("client epoch binds");
            coordinator
                .publish_lease(&HeldClock, whole_window_lease(window, *bounds))
                .expect("lease publishes");
        }

        // Card 176's first step, taken here because `install` is the one place
        // that already has both a backend and the observed windows. Every
        // teardown proof before this used a sink that answered synchronously;
        // this is a real coordinated, atomically published write, and the
        // elapsed time is what decides whether "the answer arrives after the
        // window is gone" is a real risk.
        let store = crate::store::sink();
        let mut persistence = String::new();
        {
            let mut backend = longhorn_gpui_windowing_prototype::GpuiAppBackend::new(cx);
            for window in &windows {
                backend.adopt(window.handle);
            }
            for window in &windows {
                // Every failure reported, none skipped. The first version of
                // this loop used `let Ok(..) else { continue }` and wrote an
                // empty document while claiming success — the same silent-skip
                // mistake `live_transfer_windows` exists to refuse.
                let key = GpuiWindowKey::new(window.handle.window_id().as_u64());
                persistence = match backend.observe(key) {
                    Err(error) => format!("observe failed: {error}"),
                    Ok(facts) => {
                        eprintln!(
                            "[facts] {} bounds={:?} content={:?}",
                            window.window_id,
                            facts.bounds(),
                            facts.content_size()
                        );
                        match longhorn_gpui_windowing::capture_from_gpui_facts(
                            &window.window_id,
                            &facts,
                        ) {
                            Err(error) => format!("capture failed: {error}"),
                            Ok(placement) => match crate::store::persist_now(&store, &placement) {
                                Ok(elapsed) => format!("placement written in {elapsed:?}"),
                                Err(error) => format!("write failed: {error}"),
                            },
                        }
                    }
                };
            }
        }

        let restored = crate::store::persisted().placements.len();
        // Also to stderr, so a run that nobody is watching still says what the
        // store did. A window is a poor place to report a durability result.
        eprintln!("[store] {persistence}; {restored} placement(s) read back");

        cx.set_global(Self {
            inner: Rc::new(RefCell::new(Inner {
                coordinator,
                windows,
                session: None,
                outcome: "press in one window, release over the other".to_owned(),
                persistence: format!("{persistence}; {restored} placement(s) read back"),
                allocations: 0,
            })),
        });
    }

    /// Starts a session sourced from `window_id`.
    pub fn press(cx: &mut App, window_id: &WindowId) {
        let Some(state) = cx.try_global::<Self>().map(|state| state.inner.clone()) else {
            return;
        };
        let mut inner = state.borrow_mut();
        let Some(source) = inner
            .windows
            .iter()
            .find(|window| &window.window_id == window_id)
            .cloned()
        else {
            return;
        };

        inner.allocations += 1;
        let mut allocator = CountingAllocator(inner.allocations);
        let request =
            TransferSessionRequest::new(panel_source(&source), TransferDuration::new(500));

        match inner
            .coordinator
            .create_session(&HeldClock, &mut allocator, request)
        {
            Ok(receipt) => {
                inner.session = Some(receipt.payload().session_id());
                inner.outcome = format!("dragging from {window_id}");
            }
            Err(error) => inner.outcome = format!("session refused: {error}"),
        }
    }

    /// Resolves the release point against windows observed *now* — except the
    /// source, which cannot be.
    ///
    /// gpui takes a window out of the application's window map for the
    /// duration of its own event dispatch, so observing the window whose
    /// handler is running fails with "window not found". The handler holds
    /// `&mut Window` for exactly that window, so its bounds come from there
    /// and every other window is observed live.
    ///
    /// Found by dragging. `live_transfer_windows` fails the whole list when
    /// any window fails — correctly, since a short list loses a transfer
    /// silently — so a release handler that observed everything observed
    /// nothing.
    pub fn release(
        cx: &mut App,
        screen_point: ScreenPoint,
        source_id: &WindowId,
        source_bounds: ScreenRect,
    ) {
        let Some(state) = cx.try_global::<Self>().map(|state| state.inner.clone()) else {
            return;
        };
        let mut inner = state.borrow_mut();
        let Some(session) = inner.session.take() else {
            return;
        };

        let managed: Vec<(WindowId, GpuiWindowKey, AnyWindowHandle)> = inner
            .windows
            .iter()
            .filter(|window| &window.window_id != source_id)
            .map(|window| {
                (
                    window.window_id.clone(),
                    GpuiWindowKey::new(window.handle.window_id().as_u64()),
                    window.handle,
                )
            })
            .collect();

        // Observed here, at release. This is the freshness claim, and it is
        // the reason the backend is built inside the handler rather than held:
        // `GpuiAppBackend` borrows `&mut App` and cannot outlive the callback.
        let mut backend = longhorn_gpui_windowing_prototype::GpuiAppBackend::new(cx);
        for (_, _, handle) in &managed {
            backend.adopt(*handle);
        }
        let live = match live_transfer_windows(
            &mut backend,
            managed.iter().map(|(id, key, _)| (id, *key)),
        ) {
            Ok(mut live) => {
                // The source's own bounds, from the handler that has them.
                live.push(longhorn_transfer::LiveTransferWindow::new(
                    source_id.clone(),
                    source_bounds,
                ));
                live
            }
            Err(error) => {
                eprintln!("[transfer] observation failed at release: {error}");
                inner.outcome = format!("observation failed: {error}");
                return;
            }
        };

        inner.outcome = match inner.coordinator.attempt_target_or_empty_display(
            &HeldClock,
            session,
            TargetSelector::ScreenPoint(screen_point),
            &live,
        ) {
            Ok(TerminalTransferResolution::Target(attempt)) => {
                eprintln!(
                    "[transfer] released at {},{} -> {}",
                    screen_point.x().get(),
                    screen_point.y().get(),
                    attempt.target().window_id()
                );
                format!(
                    "released at {},{} -> {}",
                    screen_point.x().get(),
                    screen_point.y().get(),
                    attempt.target().window_id()
                )
            }
            Ok(TerminalTransferResolution::EmptyDisplay(_)) => {
                eprintln!(
                    "[transfer] released at {},{} -> bare desktop",
                    screen_point.x().get(),
                    screen_point.y().get()
                );
                format!(
                    "released at {},{} -> bare desktop",
                    screen_point.x().get(),
                    screen_point.y().get()
                )
            }
            Err(error) => {
                eprintln!("[transfer] no target: {error}");
                format!("no target: {error}")
            }
        };
    }

    /// What the real store did, for both windows to draw.
    ///
    /// Separate from `outcome` because they answer different cards: this one
    /// is teardown durability, that one is cross-window transfer.
    pub fn persistence(cx: &App) -> String {
        cx.try_global::<Self>().map_or_else(String::new, |state| {
            state.inner.borrow().persistence.clone()
        })
    }

    /// The last thing that happened, for both windows to draw.
    ///
    /// `try_global`, because a window paints before `install` runs: the
    /// windows must exist before their bounds can be observed, and their
    /// bounds are what the leases are published from. Asking for the global
    /// unconditionally panicked on the first frame.
    pub fn outcome(cx: &App) -> String {
        cx.try_global::<Self>().map_or_else(
            || "binding windows...".to_owned(),
            |state| state.inner.borrow().outcome.clone(),
        )
    }
}

/// The screen point of a window-relative mouse position.
///
/// `MouseUpEvent::position` is relative to the window that received the press,
/// which during a cross-window drag is the *source* window even when the
/// cursor is elsewhere. Adding the window's own origin is what turns it into
/// something Longhorn can resolve.
#[must_use]
pub fn screen_point_of(window: &Window, position: Point<Pixels>) -> ScreenPoint {
    let origin = window.bounds().origin;
    ScreenPoint::new(
        (f32::from(origin.x) + f32::from(position.x)).round() as i32,
        (f32::from(origin.y) + f32::from(position.y)).round() as i32,
    )
}

fn whole_window_lease(window: &ManagedWindow, bounds: ScreenRect) -> LeasePublication {
    LeasePublication::new(
        window.window_id.clone(),
        window.client_id.clone(),
        ClientEpoch::new(1),
        LeaseGeneration::new(1),
        TransferDuration::new(400),
        bounds,
        vec![DropZone::new(
            DropZoneId::new(format!("{}:whole", window.window_id)).expect("zone id"),
            bounds,
            None,
            TransferCapability::MovePanel,
            TransferTargetBinding::PanelRegion {
                host_binding_id: TransferHostBindingId::new("host:example").expect("binding"),
                document_id: DomainId::new("layout.workspace").expect("document"),
                revision: TransferRevision::new(1),
                surface_id: SurfaceId::new("container:example").expect("container"),
                region_id: RegionId::new("region:main").expect("region"),
            },
        )],
    )
}

fn panel_source(window: &ManagedWindow) -> TransferSourceAuthority {
    TransferSourceAuthority::Panel {
        client_id: window.client_id.clone(),
        client_epoch: ClientEpoch::new(1),
        source_window_id: window.window_id.clone(),
        subject_id: TransferSubjectId::new("panel:example").expect("subject"),
        host_binding_id: TransferHostBindingId::new("host:example").expect("binding"),
        document_id: DomainId::new("layout.workspace").expect("document"),
        revision: TransferRevision::new(1),
        surface_id: SurfaceId::new("container:example").expect("container"),
        region_id: RegionId::new("region:tools").expect("region"),
    }
}

/// A window's screen rect from the `Window` a handler already holds.
///
/// The source window cannot be observed through the backend during its own
/// event dispatch, and this is the geometry that replaces that reading.
#[must_use]
pub fn screen_rect_of(window: &Window) -> ScreenRect {
    let bounds = window.bounds();
    ScreenRect::new(
        ScreenPoint::new(
            f32::from(bounds.origin.x).round() as i32,
            f32::from(bounds.origin.y).round() as i32,
        ),
        longhorn_core::ScreenSize::new(
            f32::from(bounds.size.width).round().max(0.0) as u32,
            f32::from(bounds.size.height).round().max(0.0) as u32,
        ),
    )
}
