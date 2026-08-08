//! Adapter handle and attachment state.

use std::sync::{Arc, Mutex};

use longhorn_native_content::{AttachGeneration, InputRoutingMode};

use crate::{ChildViewAdapterEvent, ChildViewRuntime, ChildViewSpec};

pub(crate) struct Attachment<H> {
    pub(crate) generation: AttachGeneration,
    pub(crate) handle: Option<H>,
    pub(crate) ready: bool,
    pub(crate) detaching: bool,
    pub(crate) input_routing: InputRoutingMode,
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

/// Generation-checked child-only executor over one selected runtime port.
pub struct ChildViewAdapter<R: ChildViewRuntime> {
    pub(crate) runtime: R,
    pub(crate) spec: ChildViewSpec,
    pub(crate) state: Arc<Mutex<AdapterState<R::Handle>>>,
    pub(crate) observer: Arc<dyn Fn(ChildViewAdapterEvent) + Send + Sync>,
}

impl<R: ChildViewRuntime> Clone for ChildViewAdapter<R> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            spec: self.spec.clone(),
            state: self.state.clone(),
            observer: self.observer.clone(),
        }
    }
}

impl<R: ChildViewRuntime> ChildViewAdapter<R> {
    /// Creates one adapter from injected construction/security policy and an observer.
    #[must_use]
    pub fn new(
        runtime: R,
        spec: ChildViewSpec,
        observer: Arc<dyn Fn(ChildViewAdapterEvent) + Send + Sync>,
    ) -> Self {
        Self {
            runtime,
            spec,
            state: Arc::new(Mutex::new(AdapterState::default())),
            observer,
        }
    }

    /// Returns immutable construction and security policy.
    #[must_use]
    pub const fn spec(&self) -> &ChildViewSpec {
        &self.spec
    }

}
