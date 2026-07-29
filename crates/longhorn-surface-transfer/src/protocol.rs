use longhorn_config::MutationOptions;
use longhorn_core::{SurfaceId, TransferClientId, TransferHostBindingId, WindowId};
use longhorn_surfaces_config::SurfaceConfigPublicationReceipt;
use longhorn_transfer::{
    ClientEpoch, DragSessionId, EmptyDisplayTransferAttempt, LiveTransferWindow, TargetSelector,
    TerminalTransferAttempt, TransferDuration, TransferSourceAuthority,
};

use crate::{SurfaceWindowCommitReceipt, SurfaceWindowProvisionReceipt};

/// Fresh host inputs needed to admit one whole-Surface session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceSessionAdmission {
    source_window_id: WindowId,
    client_id: TransferClientId,
    client_epoch: ClientEpoch,
    surface_id: SurfaceId,
    host_binding_id: TransferHostBindingId,
    lifetime: TransferDuration,
}

impl SurfaceSessionAdmission {
    /// Constructs one whole-Surface admission request.
    #[must_use]
    pub const fn new(
        source_window_id: WindowId,
        client_id: TransferClientId,
        client_epoch: ClientEpoch,
        surface_id: SurfaceId,
        host_binding_id: TransferHostBindingId,
        lifetime: TransferDuration,
    ) -> Self {
        Self {
            source_window_id,
            client_id,
            client_epoch,
            surface_id,
            host_binding_id,
            lifetime,
        }
    }

    pub(crate) const fn source_window_id(&self) -> &WindowId {
        &self.source_window_id
    }

    pub(crate) const fn client_id(&self) -> &TransferClientId {
        &self.client_id
    }

    pub(crate) const fn client_epoch(&self) -> ClientEpoch {
        self.client_epoch
    }

    pub(crate) const fn surface_id(&self) -> &SurfaceId {
        &self.surface_id
    }

    pub(crate) const fn host_binding_id(&self) -> &TransferHostBindingId {
        &self.host_binding_id
    }

    pub(crate) const fn lifetime(&self) -> TransferDuration {
        self.lifetime
    }
}

/// Inputs for one terminal whole-Surface transfer attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceTransferCommitRequest {
    session_id: DragSessionId,
    selector: TargetSelector,
    live_windows: Vec<LiveTransferWindow>,
    mutation_options: MutationOptions,
}

impl SurfaceTransferCommitRequest {
    /// Constructs one terminal request from fresh managed-window evidence.
    #[must_use]
    pub fn new(
        session_id: DragSessionId,
        selector: TargetSelector,
        live_windows: impl IntoIterator<Item = LiveTransferWindow>,
        mutation_options: MutationOptions,
    ) -> Self {
        Self {
            session_id,
            selector,
            live_windows: live_windows.into_iter().collect(),
            mutation_options,
        }
    }

    pub(crate) const fn session_id(&self) -> DragSessionId {
        self.session_id
    }

    pub(crate) const fn selector(&self) -> &TargetSelector {
        &self.selector
    }

    pub(crate) fn live_windows(&self) -> &[LiveTransferWindow] {
        self.live_windows.as_slice()
    }

    pub(crate) const fn mutation_options(&self) -> MutationOptions {
        self.mutation_options
    }
}

/// Consumed terminal evidence for an existing or empty-display target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SurfaceTerminalAttempt {
    /// A current leased Surface-window target resolved.
    Existing(TerminalTransferAttempt),
    /// The screen point hit no managed window and policy selected a display.
    EmptyDisplay(EmptyDisplayTransferAttempt),
}

impl SurfaceTerminalAttempt {
    /// Returns the consumed transfer session.
    #[must_use]
    pub const fn session_id(&self) -> DragSessionId {
        match self {
            Self::Existing(attempt) => attempt.session_id(),
            Self::EmptyDisplay(attempt) => attempt.session_id(),
        }
    }

    /// Returns source authority recorded at session creation.
    #[must_use]
    pub const fn source(&self) -> &TransferSourceAuthority {
        match self {
            Self::Existing(attempt) => attempt.source(),
            Self::EmptyDisplay(attempt) => attempt.source(),
        }
    }

    /// Returns existing leased target evidence when used.
    #[must_use]
    pub const fn existing(&self) -> Option<&TerminalTransferAttempt> {
        match self {
            Self::Existing(attempt) => Some(attempt),
            Self::EmptyDisplay(_) => None,
        }
    }
}

/// Completed provision lifecycle attached to successful Surface publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletedSurfaceProvision {
    provision: SurfaceWindowProvisionReceipt,
    commit: SurfaceWindowCommitReceipt,
}

impl CompletedSurfaceProvision {
    pub(crate) const fn new(
        provision: SurfaceWindowProvisionReceipt,
        commit: SurfaceWindowCommitReceipt,
    ) -> Self {
        Self { provision, commit }
    }

    /// Returns hidden creation, placement, and readiness evidence.
    #[must_use]
    pub const fn provision(&self) -> &SurfaceWindowProvisionReceipt {
        &self.provision
    }

    /// Returns host commit evidence after Surface publication.
    #[must_use]
    pub const fn commit(&self) -> &SurfaceWindowCommitReceipt {
        &self.commit
    }
}

/// Successful whole-Surface publication and optional window lifecycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceTransferCommitReceipt {
    attempt: SurfaceTerminalAttempt,
    source_host_binding_id: TransferHostBindingId,
    target_host_binding_id: TransferHostBindingId,
    publication: SurfaceConfigPublicationReceipt,
    provisioning: Option<CompletedSurfaceProvision>,
}

impl SurfaceTransferCommitReceipt {
    pub(crate) const fn new(
        attempt: SurfaceTerminalAttempt,
        source_host_binding_id: TransferHostBindingId,
        target_host_binding_id: TransferHostBindingId,
        publication: SurfaceConfigPublicationReceipt,
        provisioning: Option<CompletedSurfaceProvision>,
    ) -> Self {
        Self {
            attempt,
            source_host_binding_id,
            target_host_binding_id,
            publication,
            provisioning,
        }
    }

    /// Returns consumed session and target-path evidence.
    #[must_use]
    pub const fn attempt(&self) -> &SurfaceTerminalAttempt {
        &self.attempt
    }

    /// Returns the fresh source host binding.
    #[must_use]
    pub const fn source_host_binding_id(&self) -> &TransferHostBindingId {
        &self.source_host_binding_id
    }

    /// Returns the fresh or provisioned target host binding.
    #[must_use]
    pub const fn target_host_binding_id(&self) -> &TransferHostBindingId {
        &self.target_host_binding_id
    }

    /// Returns authoritative Surface and configuration publication.
    #[must_use]
    pub const fn publication(&self) -> &SurfaceConfigPublicationReceipt {
        &self.publication
    }

    /// Returns completed window provisioning when an empty display was used.
    #[must_use]
    pub const fn provisioning(&self) -> Option<&CompletedSurfaceProvision> {
        self.provisioning.as_ref()
    }
}
