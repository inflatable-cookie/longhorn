use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    sync::Arc,
};

use longhorn_core::WindowId;
use longhorn_windowing::{
    ApplyGeneration, HostWindowHandle, HostWindowHandleError, WindowOperation,
};
use tauri::{Runtime, WebviewWindow};

use crate::{ManagedWebviewWindow, ProgrammaticApplyEvidence, ProgrammaticApplyObserver};

pub(crate) trait ManagedWindowRegistration<R: Runtime>: Send + Sync {
    fn register_window(
        &self,
        window_id: &WindowId,
        window: &WebviewWindow<R>,
    ) -> Result<(), String>;

    fn retag_window(
        &self,
        transport_handle: &HostWindowHandle,
        window_id: &WindowId,
    ) -> Result<(), String>;
}

/// Managed native windows, stable-id bookkeeping, and current apply evidence.
pub struct ManagedWindowRegistry<R: Runtime> {
    windows: BTreeMap<HostWindowHandle, ManagedWebviewWindow<R>>,
    protected_primary: Option<HostWindowHandle>,
    generation: Option<ApplyGeneration>,
    evidence: Vec<ProgrammaticApplyEvidence>,
    apply_observer: Option<Arc<dyn ProgrammaticApplyObserver>>,
    window_registration: Option<Arc<dyn ManagedWindowRegistration<R>>>,
}

impl<R: Runtime> ManagedWindowRegistry<R> {
    /// Validates initial managed identity and optional protected-primary handle.
    pub fn new(
        windows: impl IntoIterator<Item = ManagedWebviewWindow<R>>,
        protected_primary: Option<HostWindowHandle>,
    ) -> Result<Self, ManagedWindowRegistryError> {
        let mut by_handle = BTreeMap::new();
        let mut stable_ids = BTreeSet::new();
        for managed in windows {
            let label = managed.webview_window().label();
            let handle = HostWindowHandle::new(label).map_err(|source| {
                ManagedWindowRegistryError::InvalidTransportHandle {
                    label: label.to_string(),
                    source,
                }
            })?;
            if by_handle.contains_key(&handle) {
                return Err(ManagedWindowRegistryError::DuplicateTransportHandle(handle));
            }
            if let Some(window_id) = managed.window_id() {
                if !stable_ids.insert(window_id.clone()) {
                    return Err(ManagedWindowRegistryError::DuplicateWindowId(
                        window_id.clone(),
                    ));
                }
            }
            by_handle.insert(handle, managed);
        }
        if let Some(handle) = protected_primary.as_ref() {
            if !by_handle.contains_key(handle) {
                return Err(ManagedWindowRegistryError::ProtectedPrimaryMissing(
                    handle.clone(),
                ));
            }
        }
        Ok(Self {
            windows: by_handle,
            protected_primary,
            generation: None,
            evidence: Vec::new(),
            apply_observer: None,
            window_registration: None,
        })
    }

    /// Installs a lifecycle observer invoked before every native apply action.
    #[must_use]
    pub fn with_apply_observer(
        mut self,
        apply_observer: Arc<dyn ProgrammaticApplyObserver>,
    ) -> Self {
        self.apply_observer = Some(apply_observer);
        self
    }

    pub(crate) fn with_window_registration(
        mut self,
        window_registration: Arc<dyn ManagedWindowRegistration<R>>,
    ) -> Self {
        self.window_registration = Some(window_registration);
        self
    }

    /// Starts one generation before any native mutation.
    pub fn begin_generation(
        &mut self,
        generation: ApplyGeneration,
    ) -> Result<(), ManagedWindowRegistryError> {
        if let Some(current) = self.generation {
            if generation < current {
                return Err(ManagedWindowRegistryError::StaleGeneration {
                    current,
                    attempted: generation,
                });
            }
            if generation > current {
                self.evidence.clear();
            }
        }
        self.generation = Some(generation);
        Ok(())
    }

    /// Returns the current generation.
    #[must_use]
    pub const fn generation(&self) -> Option<ApplyGeneration> {
        self.generation
    }

    /// Returns current-generation mutation evidence.
    #[must_use]
    pub fn evidence(&self) -> &[ProgrammaticApplyEvidence] {
        &self.evidence
    }

    /// Returns managed windows in stable transport-handle order.
    #[must_use]
    pub fn managed_windows(&self) -> Vec<ManagedWebviewWindow<R>> {
        self.windows.values().cloned().collect()
    }

    /// Returns whether a handle is the protected primary.
    #[must_use]
    pub fn is_protected_primary(&self, handle: &HostWindowHandle) -> bool {
        self.protected_primary.as_ref() == Some(handle)
    }

    pub(crate) fn contains_handle(&self, handle: &HostWindowHandle) -> bool {
        self.windows.contains_key(handle)
    }

    pub(crate) fn remove_closed(&mut self, handle: &HostWindowHandle) {
        self.windows.remove(handle);
    }

    pub(crate) fn registers_created_windows(&self) -> bool {
        self.window_registration.is_some()
    }

    pub(crate) fn record_evidence(
        &mut self,
        transport_handle: Option<HostWindowHandle>,
        operation: WindowOperation,
    ) -> Result<(), ManagedWindowRegistryError> {
        let generation = self
            .generation
            .expect("generation must be registered before operation evidence");
        if let Some(observer) = self.apply_observer.as_ref() {
            observer
                .register_apply(generation, &operation)
                .map_err(|detail| ManagedWindowRegistryError::ApplyEvidenceRejected {
                    generation,
                    window_id: operation.window_id().clone(),
                    detail,
                })?;
        }
        self.evidence.push(ProgrammaticApplyEvidence::new(
            generation,
            transport_handle,
            operation,
        ));
        Ok(())
    }

    pub(crate) fn resolve(
        &self,
        window_id: &WindowId,
        handle: Option<&HostWindowHandle>,
    ) -> Result<(HostWindowHandle, WebviewWindow<R>), ManagedWindowRegistryError> {
        if let Some(handle) = handle {
            let managed = self.windows.get(handle).ok_or_else(|| {
                ManagedWindowRegistryError::UnknownTransportHandle(handle.clone())
            })?;
            if managed.window_id() != Some(window_id) {
                return Err(ManagedWindowRegistryError::IdentityMismatch {
                    window_id: window_id.clone(),
                    transport_handle: handle.clone(),
                });
            }
            return Ok((handle.clone(), managed.webview_window().clone()));
        }

        self.windows
            .iter()
            .find(|(_, managed)| managed.window_id() == Some(window_id))
            .map(|(handle, managed)| (handle.clone(), managed.webview_window().clone()))
            .ok_or_else(|| ManagedWindowRegistryError::UnknownWindowId(window_id.clone()))
    }

    pub(crate) fn retag(
        &mut self,
        handle: &HostWindowHandle,
        window_id: WindowId,
    ) -> Result<(), ManagedWindowRegistryError> {
        if self.windows.iter().any(|(other_handle, managed)| {
            other_handle != handle && managed.window_id() == Some(&window_id)
        }) {
            return Err(ManagedWindowRegistryError::DuplicateWindowId(window_id));
        }
        let managed = self
            .windows
            .get_mut(handle)
            .ok_or_else(|| ManagedWindowRegistryError::UnknownTransportHandle(handle.clone()))?;
        if let Some(registration) = self.window_registration.as_ref() {
            registration
                .retag_window(handle, &window_id)
                .map_err(
                    |detail| ManagedWindowRegistryError::RetagRegistrationFailed {
                        window_id: window_id.clone(),
                        transport_handle: handle.clone(),
                        detail,
                    },
                )?;
        }
        managed.set_window_id(window_id);
        Ok(())
    }

    pub(crate) fn insert_created(
        &mut self,
        window_id: WindowId,
        window: WebviewWindow<R>,
    ) -> Result<HostWindowHandle, ManagedWindowRegistryError> {
        let label = window.label();
        let handle = HostWindowHandle::new(label).map_err(|source| {
            ManagedWindowRegistryError::InvalidTransportHandle {
                label: label.to_string(),
                source,
            }
        })?;
        if self.windows.contains_key(&handle) {
            return Err(ManagedWindowRegistryError::DuplicateTransportHandle(handle));
        }
        if self
            .windows
            .values()
            .any(|managed| managed.window_id() == Some(&window_id))
        {
            return Err(ManagedWindowRegistryError::DuplicateWindowId(window_id));
        }
        self.windows.insert(
            handle.clone(),
            ManagedWebviewWindow::new(Some(window_id.clone()), window.clone()),
        );
        if let Some(registration) = self.window_registration.as_ref() {
            if let Err(detail) = registration.register_window(&window_id, &window) {
                self.windows.remove(&handle);
                return Err(ManagedWindowRegistryError::DynamicRegistrationFailed {
                    window_id,
                    transport_handle: handle,
                    detail,
                });
            }
        }
        Ok(handle)
    }
}

/// Managed registry invariant failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManagedWindowRegistryError {
    /// A Tauri label violated opaque handle syntax.
    InvalidTransportHandle {
        /// Invalid native label.
        label: String,
        /// Validation failure.
        source: HostWindowHandleError,
    },
    /// More than one window used a handle.
    DuplicateTransportHandle(HostWindowHandle),
    /// More than one window used a stable id.
    DuplicateWindowId(WindowId),
    /// The protected primary was not managed.
    ProtectedPrimaryMissing(HostWindowHandle),
    /// An operation named no managed handle.
    UnknownTransportHandle(HostWindowHandle),
    /// An operation named no managed stable id.
    UnknownWindowId(WindowId),
    /// Stable and transport identity did not name the same window.
    IdentityMismatch {
        /// Requested stable id.
        window_id: WindowId,
        /// Requested handle.
        transport_handle: HostWindowHandle,
    },
    /// A generation older than the registry's current generation was rejected.
    StaleGeneration {
        /// Current generation.
        current: ApplyGeneration,
        /// Rejected generation.
        attempted: ApplyGeneration,
    },
    /// Lifecycle attribution refused evidence before native mutation.
    ApplyEvidenceRejected {
        /// Apply generation.
        generation: ApplyGeneration,
        /// Logical target.
        window_id: WindowId,
        /// Observer diagnostic.
        detail: String,
    },
    /// A created slot could not install its lifecycle listener.
    DynamicRegistrationFailed {
        /// Logical target.
        window_id: WindowId,
        /// Created native handle.
        transport_handle: HostWindowHandle,
        /// Listener diagnostic.
        detail: String,
    },
    /// A protected-slot retag could not move lifecycle identity with it.
    RetagRegistrationFailed {
        /// New logical target.
        window_id: WindowId,
        /// Existing native handle.
        transport_handle: HostWindowHandle,
        /// Lifecycle diagnostic.
        detail: String,
    },
}

impl fmt::Display for ManagedWindowRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransportHandle { label, .. } => {
                write!(formatter, "invalid Tauri window label {label:?}")
            }
            Self::DuplicateTransportHandle(handle) => {
                write!(formatter, "duplicate managed handle {handle}")
            }
            Self::DuplicateWindowId(window_id) => {
                write!(formatter, "duplicate managed window id {window_id}")
            }
            Self::ProtectedPrimaryMissing(handle) => {
                write!(
                    formatter,
                    "protected primary handle {handle} is not managed"
                )
            }
            Self::UnknownTransportHandle(handle) => {
                write!(formatter, "unknown managed handle {handle}")
            }
            Self::UnknownWindowId(window_id) => {
                write!(formatter, "unknown managed window id {window_id}")
            }
            Self::IdentityMismatch {
                window_id,
                transport_handle,
            } => write!(
                formatter,
                "window id {window_id} does not match managed handle {transport_handle}"
            ),
            Self::StaleGeneration { current, attempted } => write!(
                formatter,
                "apply generation {} is older than current generation {}",
                attempted.get(),
                current.get()
            ),
            Self::ApplyEvidenceRejected {
                generation,
                window_id,
                detail,
            } => write!(
                formatter,
                "apply evidence for window {window_id} generation {} was rejected: {detail}",
                generation.get()
            ),
            Self::DynamicRegistrationFailed {
                window_id,
                transport_handle,
                detail,
            } => write!(
                formatter,
                "lifecycle registration for window {window_id} handle {transport_handle} failed: {detail}"
            ),
            Self::RetagRegistrationFailed {
                window_id,
                transport_handle,
                detail,
            } => write!(
                formatter,
                "lifecycle retag for window {window_id} handle {transport_handle} failed: {detail}"
            ),
        }
    }
}

impl Error for ManagedWindowRegistryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidTransportHandle { source, .. } => Some(source),
            _ => None,
        }
    }
}
