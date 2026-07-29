use std::collections::BTreeMap;

use longhorn_core::{DropZoneId, ScreenPoint, ScreenRect, WindowId};

use crate::{
    DragSessionId, LiveTransferWindow, MonotonicClock, ResolvedTransferTarget,
    TargetResolutionPath, TargetSelector, TerminalTransferAttempt, TransferCapability,
    TransferError, TransferErrorCode, TransferInstant,
};

use super::{
    LeaseRecord, SessionStatus, TransferCoordinator, lease::lease_expired, session::status_error,
};

impl TransferCoordinator {
    /// Consumes the first active terminal attempt and resolves its current target.
    pub fn attempt_target_resolution(
        &mut self,
        clock: &impl MonotonicClock,
        session_id: DragSessionId,
        selector: TargetSelector,
        live_windows: &[LiveTransferWindow],
    ) -> Result<TerminalTransferAttempt, TransferError> {
        let now = self.observe_now(clock)?;
        let (source, expires_at, status) = self
            .sessions
            .get(&session_id)
            .map(|session| (session.source.clone(), session.expires_at, session.status))
            .ok_or_else(|| {
                TransferError::new(
                    TransferErrorCode::UnknownSession,
                    format!("drag session {session_id} is unknown"),
                )
            })?;
        if now >= expires_at {
            return Err(TransferError::new(
                TransferErrorCode::SessionExpired,
                format!("drag session {session_id} is expired"),
            ));
        }
        if status != SessionStatus::Active {
            return Err(status_error(session_id, status));
        }

        self.sessions
            .get_mut(&session_id)
            .expect("session was resolved above")
            .status = SessionStatus::Attempted;
        self.require_current_source(&source)
            .map_err(TransferError::consumed)?;
        let windows = validate_live_windows(live_windows).map_err(TransferError::consumed)?;
        let target = match selector {
            TargetSelector::ExplicitZone(zone_id) => {
                self.resolve_explicit(now, &windows, source.capability(), &zone_id)
            }
            TargetSelector::ScreenPoint(point) => {
                self.resolve_point(now, &windows, source.capability(), point)
            }
        }
        .map_err(TransferError::consumed)?;
        Ok(TerminalTransferAttempt::new(session_id, source, target))
    }

    fn resolve_explicit(
        &self,
        now: TransferInstant,
        windows: &BTreeMap<WindowId, ScreenRect>,
        capability: TransferCapability,
        zone_id: &DropZoneId,
    ) -> Result<ResolvedTransferTarget, TransferError> {
        let mut candidates = Vec::new();
        let mut saw_expired = None;
        let mut saw_wrong_capability = false;
        let mut saw_missing_window = false;
        let mut saw_stale_geometry = false;

        for (window_id, lease) in &self.leases {
            let matching = lease
                .zones
                .iter()
                .filter(|zone| zone.id() == zone_id)
                .collect::<Vec<_>>();
            if matching.is_empty() {
                continue;
            }
            if now >= lease.expires_at {
                saw_expired = Some(window_id);
                continue;
            }
            for zone in matching {
                if zone.accepted_capability() != capability {
                    saw_wrong_capability = true;
                    continue;
                }
                let Some(bounds) = windows.get(window_id) else {
                    saw_missing_window = true;
                    continue;
                };
                if bounds != &lease.window_outer_bounds {
                    saw_stale_geometry = true;
                    continue;
                }
                if !self.lease_has_current_client(window_id, lease) {
                    continue;
                }
                candidates.push((window_id.clone(), zone.clone()));
            }
        }

        match candidates.as_slice() {
            [(window_id, zone)] => Ok(ResolvedTransferTarget::new(
                TargetResolutionPath::ExplicitZone,
                window_id.clone(),
                zone.clone(),
            )),
            [] => {
                if saw_stale_geometry {
                    Err(stale_geometry())
                } else if saw_missing_window {
                    Err(TransferError::new(
                        TransferErrorCode::TargetWindowMissing,
                        format!("drop zone {zone_id} belongs to a missing window"),
                    ))
                } else if let Some(window_id) = saw_expired {
                    Err(lease_expired(window_id, now))
                } else if saw_wrong_capability {
                    Err(TransferError::new(
                        TransferErrorCode::IneligibleCapability,
                        format!("drop zone {zone_id} does not accept the session capability"),
                    ))
                } else {
                    Err(no_target())
                }
            }
            _ => Err(TransferError::new(
                TransferErrorCode::AmbiguousZone,
                format!("drop zone {zone_id} resolves to multiple current targets"),
            )),
        }
    }

    fn resolve_point(
        &self,
        now: TransferInstant,
        windows: &BTreeMap<WindowId, ScreenRect>,
        capability: TransferCapability,
        point: ScreenPoint,
    ) -> Result<ResolvedTransferTarget, TransferError> {
        let containing = windows
            .iter()
            .filter(|(_, bounds)| bounds.contains_point(&point))
            .collect::<Vec<_>>();
        let [(window_id, current_bounds)] = containing.as_slice() else {
            return if containing.is_empty() {
                Err(no_target())
            } else {
                Err(TransferError::new(
                    TransferErrorCode::AmbiguousWindow,
                    "screen point lies inside multiple current managed windows",
                ))
            };
        };
        let Some(lease) = self.leases.get(*window_id) else {
            return Err(no_target());
        };
        if now >= lease.expires_at {
            return Err(lease_expired(window_id, now));
        }
        if !self.lease_has_current_client(window_id, lease) {
            return Err(no_target());
        }
        if current_bounds != &&lease.window_outer_bounds {
            return Err(stale_geometry());
        }
        let zones = lease
            .zones
            .iter()
            .filter(|zone| {
                zone.accepted_capability() == capability && zone.bounds().contains_point(&point)
            })
            .collect::<Vec<_>>();
        match zones.as_slice() {
            [zone] => Ok(ResolvedTransferTarget::new(
                TargetResolutionPath::ScreenPoint,
                (*window_id).clone(),
                (*zone).clone(),
            )),
            [] => Err(no_target()),
            _ => Err(TransferError::new(
                TransferErrorCode::AmbiguousZone,
                "screen point lies inside multiple eligible drop zones",
            )),
        }
    }

    fn lease_has_current_client(&self, window_id: &WindowId, lease: &LeaseRecord) -> bool {
        self.clients.get(window_id).is_some_and(|current| {
            current.client_id == lease.client_id && current.epoch == lease.client_epoch
        })
    }
}

fn validate_live_windows(
    windows: &[LiveTransferWindow],
) -> Result<BTreeMap<WindowId, ScreenRect>, TransferError> {
    let mut validated = BTreeMap::new();
    for window in windows {
        if validated.contains_key(window.window_id()) {
            return Err(TransferError::new(
                TransferErrorCode::InvalidLiveWindows,
                format!("duplicate live window {}", window.window_id()),
            ));
        }
        super::lease::validate_rect(window.outer_bounds(), "live window outer bounds").map_err(
            |error| TransferError::new(TransferErrorCode::InvalidLiveWindows, error.detail()),
        )?;
        validated.insert(window.window_id().clone(), window.outer_bounds());
    }
    Ok(validated)
}

fn stale_geometry() -> TransferError {
    TransferError::new(
        TransferErrorCode::StaleWindowGeometry,
        "fresh window bounds differ from leased bounds",
    )
}

fn no_target() -> TransferError {
    TransferError::new(
        TransferErrorCode::NoTarget,
        "no current eligible transfer target matched",
    )
}
