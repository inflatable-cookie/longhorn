//! Adapter handle and attachment state.

use std::sync::{Arc, Mutex};

use longhorn_native_content::AttachGeneration;

use crate::{
    BackingSurfaceAdapterEvent, BackingSurfaceRuntime, BackingSurfaceSnapshot, BackingSurfaceSpec,
};

pub(crate) struct Attachment<H> {
    pub(crate) generation: AttachGeneration,
    pub(crate) handle: Option<H>,
    pub(crate) snapshot: Option<BackingSurfaceSnapshot>,
    pub(crate) host_focused: bool,
    pub(crate) detaching: bool,
    pub(crate) last_event_sequence: u64,
}

pub(crate) struct AdapterState<H> {
    pub(crate) latest_generation: Option<AttachGeneration>,
    pub(crate) retired_generation: Option<AttachGeneration>,
    pub(crate) invalidated_generation: Option<AttachGeneration>,
    pub(crate) attachment: Option<Attachment<H>>,
}

impl<H> Default for AdapterState<H> {
    fn default() -> Self {
        Self {
            latest_generation: None,
            retired_generation: None,
            invalidated_generation: None,
            attachment: None,
        }
    }
}

/// Generation-checked backing-surface executor over one consumer runtime port.
pub struct BackingSurfaceAdapter<R: BackingSurfaceRuntime> {
    pub(crate) runtime: R,
    pub(crate) spec: BackingSurfaceSpec,
    pub(crate) state: Arc<Mutex<AdapterState<R::Handle>>>,
    pub(crate) observer: Arc<dyn Fn(BackingSurfaceAdapterEvent) + Send + Sync>,
}

impl<R: BackingSurfaceRuntime> Clone for BackingSurfaceAdapter<R> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            spec: self.spec.clone(),
            state: Arc::clone(&self.state),
            observer: Arc::clone(&self.observer),
        }
    }
}

impl<R: BackingSurfaceRuntime> BackingSurfaceAdapter<R> {
    /// Creates one adapter from an explicit mapping and lifecycle observer.
    #[must_use]
    pub fn new(
        runtime: R,
        spec: BackingSurfaceSpec,
        observer: Arc<dyn Fn(BackingSurfaceAdapterEvent) + Send + Sync>,
    ) -> Self {
        Self {
            runtime,
            spec,
            state: Arc::new(Mutex::new(AdapterState::default())),
            observer,
        }
    }

    /// Returns immutable island and host mapping.
    #[must_use]
    pub const fn spec(&self) -> &BackingSurfaceSpec {
        &self.spec
    }
}
