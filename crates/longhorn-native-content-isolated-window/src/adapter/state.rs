//! Adapter handle and attachment state.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use longhorn_core::{NativeContentRequestId, PhysicalSize};
use longhorn_native_content::{AttachGeneration, NativeContentIslandId, NativeContentRevision};

use crate::{
    IsolatedContentRequest, IsolatedWindowAdapterEvent, IsolatedWindowRuntime, IsolatedWindowSpec,
    TeardownOutcome,
};

pub(crate) const MAX_PENDING_CONTENT_REQUESTS: usize = 128;
pub(crate) const MAX_RECENT_CONTENT_REQUEST_IDS: usize = 256;
pub(crate) const MAX_TEARDOWN_REPORTS: usize = 128;

/// A helper request bound to its island, generation, and desired revision at admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedIsolatedContentRequest {
    pub(super) island_id: NativeContentIslandId,
    pub(super) generation: AttachGeneration,
    pub(super) desired_revision: NativeContentRevision,
    pub(super) request: IsolatedContentRequest,
}

impl AdmittedIsolatedContentRequest {
    /// Returns the helper's correlation identity and request payload.
    #[must_use]
    pub const fn request(&self) -> &IsolatedContentRequest {
        &self.request
    }
}

pub(crate) struct Attachment<H> {
    pub(crate) generation: AttachGeneration,
    pub(crate) desired_revision: NativeContentRevision,
    pub(crate) handle: Option<H>,
    pub(crate) ready: bool,
    pub(crate) detaching: bool,
    pub(crate) failed: bool,
    pub(crate) last_host_size: Option<PhysicalSize>,
    pub(crate) requests: Vec<AdmittedIsolatedContentRequest>,
    pub(crate) recent_request_ids: VecDeque<NativeContentRequestId>,
}

pub(crate) struct AdapterState<H> {
    pub(crate) latest_generation: Option<AttachGeneration>,
    pub(crate) retired_generation: Option<AttachGeneration>,
    pub(crate) attachment: Option<Attachment<H>>,
    pub(crate) teardown_reports: VecDeque<(AttachGeneration, TeardownOutcome)>,
}

impl<H> Default for AdapterState<H> {
    fn default() -> Self {
        Self {
            latest_generation: None,
            retired_generation: None,
            attachment: None,
            teardown_reports: VecDeque::new(),
        }
    }
}

/// Generation-checked isolated-window executor over one injected owner runtime.
pub struct IsolatedWindowAdapter<R: IsolatedWindowRuntime> {
    pub(crate) runtime: R,
    pub(crate) spec: IsolatedWindowSpec,
    pub(crate) state: Arc<Mutex<AdapterState<R::Handle>>>,
    pub(crate) observer: Arc<dyn Fn(IsolatedWindowAdapterEvent) + Send + Sync>,
}

impl<R: IsolatedWindowRuntime> Clone for IsolatedWindowAdapter<R> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            spec: self.spec.clone(),
            state: Arc::clone(&self.state),
            observer: Arc::clone(&self.observer),
        }
    }
}

impl<R: IsolatedWindowRuntime> IsolatedWindowAdapter<R> {
    /// Creates one adapter from explicit mapping, timeout policy, and observer.
    #[must_use]
    pub fn new(
        runtime: R,
        spec: IsolatedWindowSpec,
        observer: Arc<dyn Fn(IsolatedWindowAdapterEvent) + Send + Sync>,
    ) -> Self {
        Self {
            runtime,
            spec,
            state: Arc::new(Mutex::new(AdapterState::default())),
            observer,
        }
    }

    /// Returns immutable mapping and timeout policy.
    #[must_use]
    pub const fn spec(&self) -> &IsolatedWindowSpec {
        &self.spec
    }
}
