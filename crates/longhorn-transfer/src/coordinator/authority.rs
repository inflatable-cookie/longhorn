use longhorn_core::{TransferClientId, WindowId};

use crate::{
    ClientEpoch, DragSessionId, MonotonicClock, TransferDuration, TransferError, TransferErrorCode,
    TransferInstant, TransferSourceAuthority,
};

use super::{SessionRecord, TransferCoordinator};

impl TransferCoordinator {
    pub(super) fn observe_now(
        &mut self,
        clock: &impl MonotonicClock,
    ) -> Result<TransferInstant, TransferError> {
        let now = clock.now();
        if self.last_now.is_some_and(|previous| now < previous) {
            return Err(TransferError::new(
                TransferErrorCode::ClockRegressed,
                "injected transfer clock moved backwards",
            ));
        }
        self.last_now = Some(now);
        Ok(now)
    }

    pub(super) fn checked_expiry(
        &self,
        now: TransferInstant,
        lifetime: TransferDuration,
        maximum: TransferDuration,
        kind: &str,
    ) -> Result<TransferInstant, TransferError> {
        if lifetime.get() == 0 || lifetime > maximum {
            return Err(TransferError::new(
                TransferErrorCode::InvalidLifetime,
                format!(
                    "{kind} lifetime {} is outside 1..={}",
                    lifetime.get(),
                    maximum.get()
                ),
            ));
        }
        now.get()
            .checked_add(lifetime.get())
            .map(TransferInstant::new)
            .ok_or_else(|| {
                TransferError::new(
                    TransferErrorCode::InvalidLifetime,
                    format!("{kind} expiry overflows the monotonic clock"),
                )
            })
    }

    pub(super) fn require_current_source(
        &self,
        source: &TransferSourceAuthority,
    ) -> Result<(), TransferError> {
        self.require_client(
            source.source_window_id(),
            source.client_id(),
            source.client_epoch(),
        )
    }

    pub(super) fn require_client(
        &self,
        window_id: &WindowId,
        client_id: &TransferClientId,
        epoch: ClientEpoch,
    ) -> Result<(), TransferError> {
        let Some(current) = self.clients.get(window_id) else {
            return Err(TransferError::new(
                TransferErrorCode::UnknownClientEpoch,
                format!("window {window_id} has no current renderer epoch"),
            ));
        };
        if current.client_id != *client_id || current.epoch != epoch {
            return Err(TransferError::new(
                TransferErrorCode::StaleClientEpoch,
                format!("renderer authority is stale for window {window_id}"),
            ));
        }
        Ok(())
    }

    pub(super) fn session_mut(
        &mut self,
        session_id: DragSessionId,
    ) -> Result<&mut SessionRecord, TransferError> {
        self.sessions.get_mut(&session_id).ok_or_else(|| {
            TransferError::new(
                TransferErrorCode::UnknownSession,
                format!("drag session {session_id} is unknown"),
            )
        })
    }

    pub(super) fn reclaim_expired_sessions(&mut self, now: TransferInstant) {
        while self.sessions.len() >= self.limits.maximum_sessions() {
            let Some(index) = self.session_order.iter().position(|id| {
                self.sessions
                    .get(id)
                    .is_some_and(|session| now >= session.expires_at)
            }) else {
                break;
            };
            let session_id = self
                .session_order
                .remove(index)
                .expect("located session order entry");
            self.sessions.remove(&session_id);
        }
    }
}
