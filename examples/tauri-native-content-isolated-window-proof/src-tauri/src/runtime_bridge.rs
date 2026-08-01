//! Bridge from the frozen Card 084 process fixture to the production runtime port.

use std::{
    ffi::OsString,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use longhorn_native_content::{
    ApplyPlan, ApplyReceipt, AttachGeneration, ContentSizeDecision, ContentSizeProposalReceipt,
    DesiredState, NativeContentCoordinator, NativeContentFailureCode, NativeContentRequestId,
};
use longhorn_native_content_isolated_window::{
    HelperSnapshot, IsolatedContentRequest, IsolatedContentRequestKind, IsolatedWindowAdapterEvent,
    IsolatedWindowError, IsolatedWindowRuntime, IsolatedWindowRuntimeEvent,
    IsolatedWindowRuntimeEventKind, RuntimeAttachRequest, TeardownOutcome,
};
use longhorn_native_content_isolated_window_prototype as fixture;
use longhorn_native_content_prototype as fixture_kernel;

pub(crate) use fixture::TeardownMode;
pub(crate) use longhorn_native_content_isolated_window::IsolatedWindowSpec;
pub(crate) type ChildRequest = IsolatedContentRequestKind;
pub(crate) type HelperEvent = IsolatedWindowRuntimeEvent;
pub(crate) type HelperEventKind = IsolatedWindowRuntimeEventKind;

/// Same-binary launch inputs retained by the packaged fixture.
pub(crate) struct ProcessRuntimeConfig {
    executable: PathBuf,
    helper_arguments: Vec<OsString>,
    attach_timeout: Duration,
    command_timeout: Duration,
}

impl ProcessRuntimeConfig {
    pub(crate) fn new(
        executable: PathBuf,
        helper_arguments: Vec<OsString>,
        attach_timeout: Duration,
        command_timeout: Duration,
    ) -> Self {
        Self {
            executable,
            helper_arguments,
            attach_timeout,
            command_timeout,
        }
    }
}

/// Production runtime port backed by the frozen packaged process fixture.
#[derive(Clone)]
pub(crate) struct ProcessIsolatedWindowRuntime {
    fixture: fixture::ProcessIsolatedWindowRuntime,
    current: Arc<Mutex<Option<fixture::ProcessHelperHandle>>>,
    next_content_request: Arc<AtomicU64>,
}

impl ProcessIsolatedWindowRuntime {
    pub(crate) fn new(config: ProcessRuntimeConfig) -> Self {
        Self {
            fixture: fixture::ProcessIsolatedWindowRuntime::new(
                fixture::ProcessRuntimeConfig::new(
                    config.executable,
                    config.helper_arguments,
                    config.attach_timeout,
                    config.command_timeout,
                ),
            ),
            current: Arc::new(Mutex::new(None)),
            next_content_request: Arc::new(AtomicU64::new(0)),
        }
    }

    pub(crate) fn set_helper_arguments(
        &self,
        arguments: Vec<OsString>,
    ) -> Result<(), IsolatedWindowError> {
        self.fixture
            .set_helper_arguments(arguments)
            .map_err(|error| fixture_error("attach", error))
    }

    pub(crate) fn set_teardown_mode(&self, mode: TeardownMode) -> Result<(), IsolatedWindowError> {
        self.fixture
            .set_teardown_mode(mode)
            .map_err(|error| fixture_error("teardown", error))
    }

    fn current(&self) -> Result<fixture::ProcessHelperHandle, IsolatedWindowError> {
        self.current
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?
            .clone()
            .ok_or(IsolatedWindowError::NotAttached)
    }

    fn script_request(&self, request: ChildRequest) -> Result<(), IsolatedWindowError> {
        let handle = self.current()?;
        fixture::IsolatedWindowRuntime::script_request(
            &self.fixture,
            &handle,
            to_fixture_request(request),
        )
        .map_err(|error| fixture_error("request", error))
    }

    fn simulate_helper_loss(&self) -> Result<Option<i32>, IsolatedWindowError> {
        let handle = self.current()?;
        fixture::IsolatedWindowRuntime::simulate_helper_loss(&self.fixture, &handle)
            .map_err(|error| fixture_error("teardown", error))
    }
}

impl IsolatedWindowRuntime for ProcessIsolatedWindowRuntime {
    type Handle = fixture::ProcessHelperHandle;

    fn attach(&self, request: RuntimeAttachRequest) -> Result<Self::Handle, IsolatedWindowError> {
        let callback = request.callback;
        let counter = Arc::clone(&self.next_content_request);
        let handle = fixture::IsolatedWindowRuntime::attach(
            &self.fixture,
            fixture::RuntimeAttachRequest {
                island_id: fixture_kernel::NativeContentIslandId::new(
                    request.spec.island_id().as_str(),
                )
                .map_err(|error| fixture_error("attach", error))?,
                generation: fixture_kernel::AttachGeneration::new(request.generation.get()),
                host_window_id: request.spec.host_window_id().clone(),
                callback: Arc::new(move |event| {
                    callback(from_fixture_event(event, &counter));
                }),
            },
        )
        .map_err(|error| fixture_error("attach", error))?;
        *self
            .current
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)? = Some(handle.clone());
        Ok(handle)
    }

    fn set_content_size(
        &self,
        handle: &Self::Handle,
        size: longhorn_core::PhysicalSize,
        _: Duration,
    ) -> Result<(), IsolatedWindowError> {
        fixture::IsolatedWindowRuntime::set_content_size(&self.fixture, handle, size)
            .map_err(|error| fixture_error("size", error))
    }

    fn show(&self, handle: &Self::Handle, _: Duration) -> Result<(), IsolatedWindowError> {
        fixture::IsolatedWindowRuntime::show(&self.fixture, handle)
            .map_err(|error| fixture_error("show", error))
    }

    fn hide(&self, handle: &Self::Handle, _: Duration) -> Result<(), IsolatedWindowError> {
        fixture::IsolatedWindowRuntime::hide(&self.fixture, handle)
            .map_err(|error| fixture_error("hide", error))
    }

    fn focus(&self, handle: &Self::Handle, _: Duration) -> Result<(), IsolatedWindowError> {
        fixture::IsolatedWindowRuntime::focus(&self.fixture, handle)
            .map_err(|error| fixture_error("focus", error))
    }

    fn release_focus(&self, handle: &Self::Handle, _: Duration) -> Result<(), IsolatedWindowError> {
        fixture::IsolatedWindowRuntime::release_focus(&self.fixture, handle)
            .map_err(|error| fixture_error("release_focus", error))
    }

    fn set_resizable(
        &self,
        handle: &Self::Handle,
        resizable: bool,
        _: Duration,
    ) -> Result<(), IsolatedWindowError> {
        fixture::IsolatedWindowRuntime::set_resizable(&self.fixture, handle, resizable)
            .map_err(|error| fixture_error("resize_hint", error))
    }

    fn observe(
        &self,
        handle: &Self::Handle,
        _: Duration,
    ) -> Result<HelperSnapshot, IsolatedWindowError> {
        fixture::IsolatedWindowRuntime::observe(&self.fixture, handle)
            .map(from_fixture_snapshot)
            .map_err(|error| fixture_error("observe", error))
    }

    fn teardown(
        &self,
        handle: &Self::Handle,
        timeout: Duration,
    ) -> Result<TeardownOutcome, IsolatedWindowError> {
        let outcome = fixture::IsolatedWindowRuntime::teardown(&self.fixture, handle, timeout)
            .map_err(|error| fixture_error("teardown", error))?;
        if !matches!(outcome, fixture::TeardownOutcome::TimedOut { .. }) {
            *self
                .current
                .lock()
                .map_err(|_| IsolatedWindowError::Poisoned)? = None;
        }
        Ok(match outcome {
            fixture::TeardownOutcome::Completed { exit_status } => {
                TeardownOutcome::Completed { exit_status }
            }
            fixture::TeardownOutcome::TimedOut { timeout_millis } => {
                TeardownOutcome::TimedOut { timeout_millis }
            }
            fixture::TeardownOutcome::OwnerProcessTerminated { exit_status } => {
                TeardownOutcome::OwnerProcessTerminated { exit_status }
            }
        })
    }
}

/// Thin proof harness preserving the frozen Card 084 matrix over production coordination.
pub(crate) struct IsolatedWindowAdapter {
    inner: longhorn_native_content_isolated_window::IsolatedWindowAdapter<
        ProcessIsolatedWindowRuntime,
    >,
    runtime: ProcessIsolatedWindowRuntime,
    authority: Mutex<Option<NativeContentCoordinator>>,
    last_resize: Mutex<Option<IsolatedContentRequest>>,
}

impl IsolatedWindowAdapter {
    pub(crate) fn new(
        runtime: ProcessIsolatedWindowRuntime,
        spec: IsolatedWindowSpec,
        observer: Arc<dyn Fn(IsolatedWindowAdapterEvent) + Send + Sync>,
    ) -> Self {
        Self {
            inner: longhorn_native_content_isolated_window::IsolatedWindowAdapter::new(
                runtime.clone(),
                spec,
                observer,
            ),
            runtime,
            authority: Mutex::new(None),
            last_resize: Mutex::new(None),
        }
    }

    pub(crate) fn set_authority(
        &self,
        authority: &NativeContentCoordinator,
    ) -> Result<(), IsolatedWindowError> {
        *self
            .authority
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)? = Some(authority.clone());
        Ok(())
    }

    pub(crate) fn apply(&self, plan: &ApplyPlan) -> Result<ApplyReceipt, IsolatedWindowError> {
        let authority = self
            .authority
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?
            .clone()
            .ok_or(IsolatedWindowError::NotAttached)?;
        self.inner.apply(&authority, plan)
    }

    pub(crate) fn observe(
        &self,
        generation: AttachGeneration,
    ) -> Result<longhorn_native_content::ObservationUpdate, IsolatedWindowError> {
        self.inner.observe(generation)
    }

    pub(crate) fn take_requests(
        &self,
        generation: AttachGeneration,
    ) -> Result<Vec<ChildRequest>, IsolatedWindowError> {
        let requests = self.inner.take_requests(generation)?;
        if let Some(request) = requests
            .iter()
            .rev()
            .find(|request| matches!(request.request, IsolatedContentRequestKind::Resize { .. }))
        {
            *self
                .last_resize
                .lock()
                .map_err(|_| IsolatedWindowError::Poisoned)? = Some(request.clone());
        }
        Ok(requests
            .into_iter()
            .map(|request| request.request)
            .collect())
    }

    pub(crate) fn decide_resize(
        &self,
        desired: &DesiredState,
        size: longhorn_core::PhysicalSize,
        decision: ContentSizeDecision,
    ) -> Result<ContentSizeProposalReceipt, IsolatedWindowError> {
        let authority = self
            .authority
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?
            .clone()
            .ok_or(IsolatedWindowError::NotAttached)?;
        let request = self
            .last_resize
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?
            .clone()
            .filter(|request| request.request == (IsolatedContentRequestKind::Resize { size }))
            .ok_or(IsolatedWindowError::NotResizeRequest)?;
        self.inner
            .decide_resize(&authority, desired.generation(), &request, decision)
    }

    pub(crate) fn set_resizable(
        &self,
        generation: AttachGeneration,
        resizable: bool,
    ) -> Result<(), IsolatedWindowError> {
        self.inner.set_resizable(generation, resizable)
    }

    pub(crate) fn script_request(
        &self,
        _: AttachGeneration,
        request: ChildRequest,
    ) -> Result<(), IsolatedWindowError> {
        self.runtime.script_request(request)
    }

    pub(crate) fn simulate_helper_loss(
        &self,
        _: AttachGeneration,
    ) -> Result<Option<i32>, IsolatedWindowError> {
        self.runtime.simulate_helper_loss()
    }

    pub(crate) fn teardown_reports(
        &self,
    ) -> Result<Vec<(AttachGeneration, TeardownOutcome)>, IsolatedWindowError> {
        self.inner.teardown_reports()
    }

    pub(crate) fn admit_runtime_event(
        &self,
        event: HelperEvent,
    ) -> Result<(), IsolatedWindowError> {
        self.inner.admit_runtime_event(event)
    }

    pub(crate) fn spec(&self) -> &IsolatedWindowSpec {
        self.inner.spec()
    }
}

fn from_fixture_event(
    event: fixture::HelperEvent,
    counter: &AtomicU64,
) -> IsolatedWindowRuntimeEvent {
    let kind = match event.kind {
        fixture::HelperEventKind::Progress { phase } => {
            IsolatedWindowRuntimeEventKind::Progress { phase }
        }
        fixture::HelperEventKind::Ready {
            content_size,
            process_id,
            native_child_attached,
        } => IsolatedWindowRuntimeEventKind::Ready {
            snapshot: HelperSnapshot {
                content_size,
                visible: false,
                focused: false,
            },
            owner_process_id: process_id,
            native_content_attached: native_child_attached,
        },
        fixture::HelperEventKind::ChildRequest { request } => {
            let value = counter.fetch_add(1, Ordering::Relaxed) + 1;
            IsolatedWindowRuntimeEventKind::ContentRequest {
                request: IsolatedContentRequest {
                    request_id: NativeContentRequestId::new(format!("fixture:content-{value}"))
                        .expect("fixture request ids are bounded"),
                    request: from_fixture_request(request),
                },
            }
        }
        fixture::HelperEventKind::FocusChanged { focused } => {
            IsolatedWindowRuntimeEventKind::FocusChanged { focused }
        }
        fixture::HelperEventKind::VisibilityChanged { visible } => {
            IsolatedWindowRuntimeEventKind::VisibilityChanged { visible }
        }
        fixture::HelperEventKind::HelperLost { code, exit_status } => {
            IsolatedWindowRuntimeEventKind::HelperLost {
                code: NativeContentFailureCode::new(code.as_str())
                    .expect("fixture failure code is bounded"),
                exit_status,
            }
        }
    };
    IsolatedWindowRuntimeEvent {
        island_id: longhorn_native_content::NativeContentIslandId::new(event.island_id.as_str())
            .expect("fixture island id is bounded"),
        generation: AttachGeneration::new(event.generation.get())
            .expect("fixture generation is positive"),
        kind,
    }
}

fn from_fixture_snapshot(value: fixture::RuntimeSnapshot) -> HelperSnapshot {
    HelperSnapshot {
        content_size: value.content_size,
        visible: value.visible,
        focused: value.focused,
    }
}

fn from_fixture_request(value: fixture::ChildRequest) -> ChildRequest {
    match value {
        fixture::ChildRequest::Resize { size } => ChildRequest::Resize { size },
        fixture::ChildRequest::Show => ChildRequest::Show,
        fixture::ChildRequest::Hide => ChildRequest::Hide,
        fixture::ChildRequest::Close => ChildRequest::Close,
        fixture::ChildRequest::ResizeHint { resizable } => ChildRequest::ResizeHint { resizable },
    }
}

fn to_fixture_request(value: ChildRequest) -> fixture::ChildRequest {
    match value {
        ChildRequest::Resize { size } => fixture::ChildRequest::Resize { size },
        ChildRequest::Show => fixture::ChildRequest::Show,
        ChildRequest::Hide => fixture::ChildRequest::Hide,
        ChildRequest::Close => fixture::ChildRequest::Close,
        ChildRequest::ResizeHint { resizable } => fixture::ChildRequest::ResizeHint { resizable },
    }
}

fn fixture_error(operation: &'static str, error: impl std::fmt::Display) -> IsolatedWindowError {
    IsolatedWindowError::Runtime {
        operation,
        detail: error.to_string(),
    }
}
