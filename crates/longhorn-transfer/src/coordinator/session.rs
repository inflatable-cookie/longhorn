use longhorn_core::{TransferClientId, WindowId};

use crate::{
    ClientEpoch, DragSessionId, DragSessionIdAllocator, MonotonicClock, SessionCancellationReceipt,
    SessionCancellationStatus, SessionCreationReceipt, TransferError, TransferErrorCode,
    TransferPayload, TransferSessionRequest,
};

use super::{
    ClientBinding, ClientEpochBindingStatus, SessionRecord, SessionStatus, TransferCoordinator,
    WindowInvalidationReceipt,
};

impl TransferCoordinator {
    /// Installs or advances current renderer authority for one managed window.
    pub fn bind_client_epoch(
        &mut self,
        clock: &impl MonotonicClock,
        window_id: WindowId,
        client_id: TransferClientId,
        epoch: ClientEpoch,
    ) -> Result<ClientEpochBindingStatus, TransferError> {
        self.observe_now(clock)?;
        let status = match self.clients.get(&window_id) {
            Some(current) if current.client_id == client_id && current.epoch == epoch => {
                return Ok(ClientEpochBindingStatus::Unchanged);
            }
            Some(current) if epoch <= current.epoch => {
                return Err(TransferError::new(
                    TransferErrorCode::StaleClientEpoch,
                    format!(
                        "client epoch {} does not advance current epoch {} for {window_id}",
                        epoch.get(),
                        current.epoch.get()
                    ),
                ));
            }
            Some(_) => ClientEpochBindingStatus::Advanced,
            None if self.clients.len() >= self.limits.maximum_client_windows() => {
                return Err(TransferError::new(
                    TransferErrorCode::ClientWindowCapacity,
                    "current client-window registry is full",
                ));
            }
            None => ClientEpochBindingStatus::Installed,
        };

        self.clients.insert(
            window_id.clone(),
            ClientBinding {
                client_id: client_id.clone(),
                epoch,
            },
        );
        self.leases.remove(&window_id);
        for session in self.sessions.values_mut() {
            if session.status == SessionStatus::Active
                && session.source.source_window_id() == &window_id
                && (session.source.client_id() != &client_id
                    || session.source.client_epoch() != epoch)
            {
                session.status = SessionStatus::SourceClientChanged;
            }
        }
        Ok(status)
    }

    /// Creates one finite session after all validation and capacity checks.
    pub fn create_session(
        &mut self,
        clock: &impl MonotonicClock,
        allocator: &mut impl DragSessionIdAllocator,
        request: TransferSessionRequest,
    ) -> Result<SessionCreationReceipt, TransferError> {
        let now = self.observe_now(clock)?;
        let expires_at = self.checked_expiry(
            now,
            request.lifetime(),
            self.limits.maximum_session_lifetime(),
            "session",
        )?;
        self.require_current_source(request.source())?;
        self.reclaim_expired_sessions(now);
        if self.sessions.len() >= self.limits.maximum_sessions() {
            return Err(TransferError::new(
                TransferErrorCode::SessionCapacity,
                "transfer session store is full",
            ));
        }

        let entropy = allocator.allocate().map_err(|_| {
            TransferError::new(
                TransferErrorCode::SessionIdAllocation,
                "drag session entropy allocation failed",
            )
        })?;
        let session_id = DragSessionId::from_entropy(entropy);
        if self.sessions.contains_key(&session_id) {
            return Err(TransferError::new(
                TransferErrorCode::SessionIdCollision,
                format!("allocator returned current drag session {session_id}"),
            ));
        }
        self.session_order.push_back(session_id);
        self.sessions.insert(
            session_id,
            SessionRecord {
                source: request.source().clone(),
                expires_at,
                status: SessionStatus::Active,
            },
        );
        Ok(SessionCreationReceipt::new(
            TransferPayload::new(session_id),
            expires_at,
        ))
    }

    /// Cancels an active session; repeated cancellation is idempotent.
    pub fn cancel_session(
        &mut self,
        clock: &impl MonotonicClock,
        session_id: DragSessionId,
    ) -> Result<SessionCancellationReceipt, TransferError> {
        let now = self.observe_now(clock)?;
        let session = self.session_mut(session_id)?;
        if now >= session.expires_at {
            return Err(session_state_error(
                TransferErrorCode::SessionExpired,
                session_id,
                "expired",
            ));
        }
        match session.status {
            SessionStatus::Active => {
                session.status = SessionStatus::Cancelled;
                Ok(SessionCancellationReceipt::new(
                    session_id,
                    SessionCancellationStatus::Cancelled,
                ))
            }
            SessionStatus::Cancelled => Ok(SessionCancellationReceipt::new(
                session_id,
                SessionCancellationStatus::AlreadyCancelled,
            )),
            status => Err(status_error(session_id, status)),
        }
    }

    /// Invalidates current renderer, lease, and source-session authority.
    pub fn destroy_window(&mut self, window_id: &WindowId) -> WindowInvalidationReceipt {
        let removed_client_binding = self.clients.remove(window_id).is_some();
        let removed_lease = self.leases.remove(window_id).is_some();
        let mut invalidated_source_sessions = 0;
        for session in self.sessions.values_mut() {
            if session.status == SessionStatus::Active
                && session.source.source_window_id() == window_id
                && self.last_now.is_none_or(|now| now < session.expires_at)
            {
                session.status = SessionStatus::SourceWindowDestroyed;
                invalidated_source_sessions += 1;
            }
        }
        WindowInvalidationReceipt {
            removed_client_binding,
            removed_lease,
            invalidated_source_sessions,
        }
    }
}

pub(super) fn status_error(session_id: DragSessionId, status: SessionStatus) -> TransferError {
    let (code, label) = match status {
        SessionStatus::Active => unreachable!("active session is not an error"),
        SessionStatus::Cancelled => (TransferErrorCode::SessionCancelled, "cancelled"),
        SessionStatus::Attempted => (TransferErrorCode::SessionReplayed, "already attempted"),
        SessionStatus::SourceWindowDestroyed => (
            TransferErrorCode::SourceWindowDestroyed,
            "bound to a destroyed source window",
        ),
        SessionStatus::SourceClientChanged => (
            TransferErrorCode::SourceClientChanged,
            "bound to a superseded source client",
        ),
    };
    session_state_error(code, session_id, label)
}

fn session_state_error(
    code: TransferErrorCode,
    session_id: DragSessionId,
    label: &str,
) -> TransferError {
    TransferError::new(code, format!("drag session {session_id} is {label}"))
}
