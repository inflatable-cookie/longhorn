use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use longhorn_core::WindowId;
use longhorn_windowing::{ApplyGeneration, HostWindowHandle, WindowOperation};

use crate::GpuiWindowKey;

/// One GPUI window slot with its current logical identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedGpuiWindow {
    key: GpuiWindowKey,
    window_id: Option<WindowId>,
}

impl ManagedGpuiWindow {
    /// Records one managed slot, optionally already tagged.
    #[must_use]
    pub const fn new(key: GpuiWindowKey, window_id: Option<WindowId>) -> Self {
        Self { key, window_id }
    }

    /// Returns the GPUI slot.
    #[must_use]
    pub const fn key(&self) -> GpuiWindowKey {
        self.key
    }

    /// Returns stable domain identity when the host has assigned one.
    #[must_use]
    pub const fn window_id(&self) -> Option<&WindowId> {
        self.window_id.as_ref()
    }

    fn set_window_id(&mut self, window_id: WindowId) {
        self.window_id = Some(window_id);
    }
}

/// One generation marker registered before a native mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuiApplyEvidence {
    generation: ApplyGeneration,
    window_id: WindowId,
    transport_handle: Option<HostWindowHandle>,
    operation: WindowOperation,
}

impl GpuiApplyEvidence {
    pub(crate) fn new(
        generation: ApplyGeneration,
        transport_handle: Option<HostWindowHandle>,
        operation: WindowOperation,
    ) -> Self {
        Self {
            generation,
            window_id: operation.window_id().clone(),
            transport_handle,
            operation,
        }
    }

    /// Returns the owning apply generation.
    #[must_use]
    pub const fn generation(&self) -> ApplyGeneration {
        self.generation
    }

    /// Returns stable logical target identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the known native handle.
    #[must_use]
    pub const fn transport_handle(&self) -> Option<&HostWindowHandle> {
        self.transport_handle.as_ref()
    }

    /// Returns the complete expected operation registered before mutation.
    #[must_use]
    pub const fn operation(&self) -> &WindowOperation {
        &self.operation
    }
}

/// Managed GPUI slots, stable-id bookkeeping, and current apply evidence.
#[derive(Clone, Debug, Default)]
pub struct GpuiWindowRegistry {
    windows: BTreeMap<HostWindowHandle, ManagedGpuiWindow>,
    protected_primary: Option<HostWindowHandle>,
    generation: Option<ApplyGeneration>,
    evidence: Vec<GpuiApplyEvidence>,
}

impl GpuiWindowRegistry {
    /// Validates initial managed identity and optional protected-primary slot.
    pub fn new(
        windows: impl IntoIterator<Item = ManagedGpuiWindow>,
        protected_primary: Option<HostWindowHandle>,
    ) -> Result<Self, GpuiWindowRegistryError> {
        let mut by_handle = BTreeMap::new();
        let mut stable_ids = BTreeSet::new();
        for managed in windows {
            let handle = managed.key().transport_handle();
            if by_handle.contains_key(&handle) {
                return Err(GpuiWindowRegistryError::DuplicateTransportHandle(handle));
            }
            if let Some(window_id) = managed.window_id()
                && !stable_ids.insert(window_id.clone())
            {
                return Err(GpuiWindowRegistryError::DuplicateWindowId(
                    window_id.clone(),
                ));
            }
            by_handle.insert(handle, managed);
        }
        if let Some(handle) = protected_primary.as_ref()
            && !by_handle.contains_key(handle)
        {
            return Err(GpuiWindowRegistryError::ProtectedPrimaryMissing(
                handle.clone(),
            ));
        }
        Ok(Self {
            windows: by_handle,
            protected_primary,
            generation: None,
            evidence: Vec::new(),
        })
    }

    /// Starts one generation before any native mutation.
    pub fn begin_generation(
        &mut self,
        generation: ApplyGeneration,
    ) -> Result<(), GpuiWindowRegistryError> {
        if let Some(current) = self.generation {
            if generation < current {
                return Err(GpuiWindowRegistryError::StaleGeneration {
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
    pub fn evidence(&self) -> &[GpuiApplyEvidence] {
        &self.evidence
    }

    /// Returns managed slots in stable transport-handle order.
    #[must_use]
    pub fn managed_windows(&self) -> Vec<ManagedGpuiWindow> {
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

    pub(crate) fn record_evidence(
        &mut self,
        transport_handle: Option<HostWindowHandle>,
        operation: WindowOperation,
    ) {
        let generation = self
            .generation
            .expect("generation must be registered before operation evidence");
        self.evidence.push(GpuiApplyEvidence::new(
            generation,
            transport_handle,
            operation,
        ));
    }

    pub(crate) fn resolve(
        &self,
        window_id: &WindowId,
        handle: Option<&HostWindowHandle>,
    ) -> Result<(HostWindowHandle, GpuiWindowKey), GpuiWindowRegistryError> {
        if let Some(handle) = handle {
            let managed = self
                .windows
                .get(handle)
                .ok_or_else(|| GpuiWindowRegistryError::UnknownTransportHandle(handle.clone()))?;
            if managed.window_id() != Some(window_id) {
                return Err(GpuiWindowRegistryError::IdentityMismatch {
                    window_id: window_id.clone(),
                    transport_handle: handle.clone(),
                });
            }
            return Ok((handle.clone(), managed.key()));
        }

        self.windows
            .iter()
            .find(|(_, managed)| managed.window_id() == Some(window_id))
            .map(|(handle, managed)| (handle.clone(), managed.key()))
            .ok_or_else(|| GpuiWindowRegistryError::UnknownWindowId(window_id.clone()))
    }

    pub(crate) fn retag(
        &mut self,
        handle: &HostWindowHandle,
        window_id: WindowId,
    ) -> Result<(), GpuiWindowRegistryError> {
        if self.windows.iter().any(|(other_handle, managed)| {
            other_handle != handle && managed.window_id() == Some(&window_id)
        }) {
            return Err(GpuiWindowRegistryError::DuplicateWindowId(window_id));
        }
        let managed = self
            .windows
            .get_mut(handle)
            .ok_or_else(|| GpuiWindowRegistryError::UnknownTransportHandle(handle.clone()))?;
        managed.set_window_id(window_id);
        Ok(())
    }

    pub(crate) fn insert_created(
        &mut self,
        window_id: WindowId,
        key: GpuiWindowKey,
    ) -> Result<HostWindowHandle, GpuiWindowRegistryError> {
        let handle = key.transport_handle();
        if self.windows.contains_key(&handle) {
            return Err(GpuiWindowRegistryError::DuplicateTransportHandle(handle));
        }
        if self
            .windows
            .values()
            .any(|managed| managed.window_id() == Some(&window_id))
        {
            return Err(GpuiWindowRegistryError::DuplicateWindowId(window_id));
        }
        self.windows
            .insert(handle.clone(), ManagedGpuiWindow::new(key, Some(window_id)));
        Ok(handle)
    }
}

/// Managed registry invariant failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiWindowRegistryError {
    /// More than one slot used a handle.
    DuplicateTransportHandle(HostWindowHandle),
    /// More than one slot used a stable id.
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
}

impl fmt::Display for GpuiWindowRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
        }
    }
}

impl Error for GpuiWindowRegistryError {}
