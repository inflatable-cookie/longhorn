use std::{
    ffi::OsString,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use longhorn_native_content_prototype::NativeContentFailureCode;

use crate::{
    ChildRequest, HelperEvent, HelperEventKind, IsolatedWindowError, IsolatedWindowRuntime,
    RuntimeAttachRequest, RuntimeSnapshot, TeardownOutcome, WireCommand, WireCommandKind,
    WireEvent, WireEventKind,
};

/// Consumer-supplied process launch configuration. Outer placement is encoded only in arguments.
#[derive(Clone, Debug)]
pub struct ProcessRuntimeConfig {
    executable: PathBuf,
    helper_arguments: Vec<OsString>,
    attach_timeout: Duration,
    command_timeout: Duration,
}

impl ProcessRuntimeConfig {
    /// Constructs one same-binary helper launch configuration.
    #[must_use]
    pub fn new(
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

/// Scripted teardown posture for the next bounded detach attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TeardownMode {
    /// Ask the helper to close and exit cooperatively.
    Cooperative,
    /// Preserve the live helper until the timeout expires.
    WaitOnly,
    /// Terminate the disposable owner process directly.
    TerminateOwner,
}

/// Same-binary disposable helper runtime used by the packaged proof.
#[derive(Clone)]
pub struct ProcessIsolatedWindowRuntime {
    config: Arc<Mutex<ProcessRuntimeConfig>>,
    teardown_mode: Arc<Mutex<TeardownMode>>,
}

impl ProcessIsolatedWindowRuntime {
    /// Creates a runtime from consumer-owned launch configuration.
    #[must_use]
    pub fn new(config: ProcessRuntimeConfig) -> Self {
        Self {
            config: Arc::new(Mutex::new(config)),
            teardown_mode: Arc::new(Mutex::new(TeardownMode::Cooperative)),
        }
    }

    /// Replaces consumer-owned helper arguments before the next generation attaches.
    pub fn set_helper_arguments(
        &self,
        arguments: Vec<OsString>,
    ) -> Result<(), IsolatedWindowError> {
        self.config
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?
            .helper_arguments = arguments;
        Ok(())
    }

    /// Selects the next bounded teardown behavior for controlled evidence.
    pub fn set_teardown_mode(&self, mode: TeardownMode) -> Result<(), IsolatedWindowError> {
        *self
            .teardown_mode
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)? = mode;
        Ok(())
    }

    fn send(
        &self,
        handle: &ProcessHelperHandle,
        command: WireCommandKind,
    ) -> Result<RuntimeSnapshot, IsolatedWindowError> {
        let request_id = handle.session.next_request.fetch_add(1, Ordering::Relaxed) + 1;
        let command = WireCommand {
            request_id,
            generation: handle.session.generation,
            command,
        };
        write_command(&handle.session, &command)?;
        let timeout = self
            .config
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?
            .command_timeout;
        let event = handle.session.wait_for(timeout, |event| {
            matches!(
                event.event,
                WireEventKind::Acknowledged {
                    request_id: candidate,
                    ..
                } if candidate == request_id
            )
        })?;
        let WireEventKind::Acknowledged {
            applied,
            detail,
            snapshot,
            ..
        } = event.event
        else {
            unreachable!("wait predicate selects acknowledgements")
        };
        if !applied {
            return Err(runtime_error(
                operation_name(&command.command),
                detail.unwrap_or_else(|| "helper rejected command".to_string()),
            ));
        }
        snapshot
            .or_else(|| handle.session.snapshot.lock().ok().and_then(|value| *value))
            .ok_or_else(|| runtime_error("observe", "helper returned no native snapshot"))
    }

    fn wait_for_exit(
        &self,
        handle: &ProcessHelperHandle,
        timeout: Duration,
    ) -> Result<Option<Option<i32>>, IsolatedWindowError> {
        let deadline = Instant::now() + timeout;
        loop {
            let status = handle
                .session
                .child
                .lock()
                .map_err(|_| IsolatedWindowError::Poisoned)?
                .try_wait()
                .map_err(|error| runtime_error("teardown", error.to_string()))?;
            if let Some(status) = status {
                return Ok(Some(status.code()));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
}

/// Opaque process session retained by the isolated-window adapter.
#[derive(Clone)]
pub struct ProcessHelperHandle {
    session: Arc<ProcessSession>,
}

struct ProcessSession {
    island_id: longhorn_native_content_prototype::NativeContentIslandId,
    generation: longhorn_native_content_prototype::AttachGeneration,
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    events: Mutex<Vec<WireEvent>>,
    changed: Condvar,
    snapshot: Mutex<Option<RuntimeSnapshot>>,
    next_request: AtomicU64,
    expected_exit: AtomicBool,
    callback: Arc<dyn Fn(HelperEvent) + Send + Sync>,
}

impl ProcessSession {
    fn wait_for(
        &self,
        timeout: Duration,
        predicate: impl Fn(&WireEvent) -> bool,
    ) -> Result<WireEvent, IsolatedWindowError> {
        let deadline = Instant::now() + timeout;
        let mut events = self
            .events
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?;
        loop {
            if let Some(event) = events.iter().find(|event| predicate(event)) {
                return Ok(event.clone());
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(runtime_error("observe", "helper evidence timed out"));
            }
            let (next, wait) = self
                .changed
                .wait_timeout(events, deadline.saturating_duration_since(now))
                .map_err(|_| IsolatedWindowError::Poisoned)?;
            events = next;
            if wait.timed_out() && !events.iter().any(&predicate) {
                return Err(runtime_error("observe", "helper evidence timed out"));
            }
        }
    }

    fn record(&self, event: WireEvent) {
        match &event.event {
            WireEventKind::Ready { snapshot, .. }
            | WireEventKind::Acknowledged {
                snapshot: Some(snapshot),
                ..
            } => {
                if let Ok(mut current) = self.snapshot.lock() {
                    *current = Some(*snapshot);
                }
            }
            WireEventKind::FocusChanged { focused } => {
                if let Ok(mut current) = self.snapshot.lock() {
                    if let Some(snapshot) = current.as_mut() {
                        snapshot.focused = *focused;
                    }
                }
            }
            WireEventKind::VisibilityChanged { visible } => {
                if let Ok(mut current) = self.snapshot.lock() {
                    if let Some(snapshot) = current.as_mut() {
                        snapshot.visible = *visible;
                    }
                }
            }
            WireEventKind::ContentResized { size } => {
                if let Ok(mut current) = self.snapshot.lock() {
                    if let Some(snapshot) = current.as_mut() {
                        snapshot.content_size = *size;
                    }
                }
            }
            WireEventKind::Progress { .. }
            | WireEventKind::ChildRequest { .. }
            | WireEventKind::TeardownCompleted => {}
            WireEventKind::Acknowledged { snapshot: None, .. } => {}
        }
        let semantic = match &event.event {
            WireEventKind::Progress { phase } => Some(HelperEventKind::Progress {
                phase: phase.clone(),
            }),
            WireEventKind::Ready {
                snapshot,
                process_id,
                native_child_attached,
            } => Some(HelperEventKind::Ready {
                content_size: snapshot.content_size,
                process_id: *process_id,
                native_child_attached: *native_child_attached,
            }),
            WireEventKind::ChildRequest { request } => Some(HelperEventKind::ChildRequest {
                request: request.clone(),
            }),
            WireEventKind::FocusChanged { focused } => {
                Some(HelperEventKind::FocusChanged { focused: *focused })
            }
            WireEventKind::VisibilityChanged { visible } => {
                Some(HelperEventKind::VisibilityChanged { visible: *visible })
            }
            WireEventKind::Acknowledged { .. }
            | WireEventKind::TeardownCompleted
            | WireEventKind::ContentResized { .. } => None,
        };
        if let Ok(mut events) = self.events.lock() {
            events.push(event);
            self.changed.notify_all();
        }
        if let Some(kind) = semantic {
            (self.callback)(HelperEvent {
                island_id: self.island_id.clone(),
                generation: self.generation,
                kind,
            });
        }
    }
}

impl IsolatedWindowRuntime for ProcessIsolatedWindowRuntime {
    type Handle = ProcessHelperHandle;

    fn attach(&self, request: RuntimeAttachRequest) -> Result<Self::Handle, IsolatedWindowError> {
        let config = self
            .config
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?
            .clone();
        let mut command = Command::new(&config.executable);
        command
            .args(&config.helper_arguments)
            .arg("--longhorn-isolated-helper")
            .arg(request.generation.get().to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|error| runtime_error("attach", error.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| runtime_error("attach", "helper stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| runtime_error("attach", "helper stdout unavailable"))?;
        let stderr = child.stderr.take();
        let session = Arc::new(ProcessSession {
            island_id: request.island_id,
            generation: request.generation,
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            events: Mutex::new(Vec::new()),
            changed: Condvar::new(),
            snapshot: Mutex::new(None),
            next_request: AtomicU64::new(0),
            expected_exit: AtomicBool::new(false),
            callback: request.callback,
        });
        let reader_session = session.clone();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => match serde_json::from_str::<WireEvent>(&line) {
                        Ok(event) if event.generation == reader_session.generation => {
                            reader_session.record(event);
                        }
                        Ok(_) | Err(_) => {}
                    },
                    Err(_) => break,
                }
            }
            if !reader_session.expected_exit.load(Ordering::Acquire) {
                (reader_session.callback)(HelperEvent {
                    island_id: reader_session.island_id.clone(),
                    generation: reader_session.generation,
                    kind: HelperEventKind::HelperLost {
                        code: NativeContentFailureCode::new("isolated:helper-exited")
                            .expect("static helper failure code is valid"),
                        exit_status: None,
                    },
                });
            }
        });
        if let Some(stderr) = stderr {
            thread::spawn(move || {
                for line in BufReader::new(stderr).lines() {
                    if line.is_err() {
                        break;
                    }
                }
            });
        }
        let handle = ProcessHelperHandle { session };
        handle.session.wait_for(config.attach_timeout, |event| {
            matches!(event.event, WireEventKind::Ready { .. })
        })?;
        Ok(handle)
    }

    fn set_content_size(
        &self,
        handle: &Self::Handle,
        size: longhorn_core::PhysicalSize,
    ) -> Result<(), IsolatedWindowError> {
        self.send(handle, WireCommandKind::SetContentSize { size })?;
        Ok(())
    }

    fn show(&self, handle: &Self::Handle) -> Result<(), IsolatedWindowError> {
        self.send(handle, WireCommandKind::Show)?;
        Ok(())
    }

    fn hide(&self, handle: &Self::Handle) -> Result<(), IsolatedWindowError> {
        self.send(handle, WireCommandKind::Hide)?;
        Ok(())
    }

    fn focus(&self, handle: &Self::Handle) -> Result<(), IsolatedWindowError> {
        self.send(handle, WireCommandKind::Focus)?;
        Ok(())
    }

    fn release_focus(&self, handle: &Self::Handle) -> Result<(), IsolatedWindowError> {
        self.send(handle, WireCommandKind::ReleaseFocus)?;
        Ok(())
    }

    fn set_resizable(
        &self,
        handle: &Self::Handle,
        resizable: bool,
    ) -> Result<(), IsolatedWindowError> {
        self.send(handle, WireCommandKind::SetResizable { resizable })?;
        Ok(())
    }

    fn script_request(
        &self,
        handle: &Self::Handle,
        request: ChildRequest,
    ) -> Result<(), IsolatedWindowError> {
        self.send(handle, WireCommandKind::ScriptRequest { request })?;
        Ok(())
    }

    fn simulate_helper_loss(
        &self,
        handle: &Self::Handle,
    ) -> Result<Option<i32>, IsolatedWindowError> {
        let command = WireCommand {
            request_id: handle.session.next_request.fetch_add(1, Ordering::Relaxed) + 1,
            generation: handle.session.generation,
            command: WireCommandKind::Crash,
        };
        write_command(&handle.session, &command)?;
        Ok(self
            .wait_for_exit(handle, Duration::from_secs(3))?
            .flatten())
    }

    fn observe(&self, handle: &Self::Handle) -> Result<RuntimeSnapshot, IsolatedWindowError> {
        self.send(handle, WireCommandKind::Observe)
    }

    fn teardown(
        &self,
        handle: &Self::Handle,
        timeout: Duration,
    ) -> Result<TeardownOutcome, IsolatedWindowError> {
        let mode = *self
            .teardown_mode
            .lock()
            .map_err(|_| IsolatedWindowError::Poisoned)?;
        match mode {
            TeardownMode::WaitOnly => {
                thread::sleep(timeout);
                Ok(TeardownOutcome::TimedOut {
                    timeout_millis: u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
                })
            }
            TeardownMode::Cooperative => {
                handle.session.expected_exit.store(true, Ordering::Release);
                self.send(handle, WireCommandKind::Shutdown)?;
                handle.session.wait_for(timeout, |event| {
                    matches!(event.event, WireEventKind::TeardownCompleted)
                })?;
                let status = self.wait_for_exit(handle, timeout)?.ok_or_else(|| {
                    runtime_error("teardown", "helper reported completion but did not exit")
                })?;
                Ok(TeardownOutcome::Completed {
                    exit_status: status,
                })
            }
            TeardownMode::TerminateOwner => {
                handle.session.expected_exit.store(true, Ordering::Release);
                let mut child = handle
                    .session
                    .child
                    .lock()
                    .map_err(|_| IsolatedWindowError::Poisoned)?;
                child
                    .kill()
                    .map_err(|error| runtime_error("teardown", error.to_string()))?;
                let status = child
                    .wait()
                    .map_err(|error| runtime_error("teardown", error.to_string()))?;
                Ok(TeardownOutcome::OwnerProcessTerminated {
                    exit_status: status.code(),
                })
            }
        }
    }
}

fn write_command(
    session: &ProcessSession,
    command: &WireCommand,
) -> Result<(), IsolatedWindowError> {
    let mut stdin = session
        .stdin
        .lock()
        .map_err(|_| IsolatedWindowError::Poisoned)?;
    serde_json::to_writer(&mut *stdin, command)
        .map_err(|error| runtime_error(operation_name(&command.command), error.to_string()))?;
    stdin
        .write_all(b"\n")
        .and_then(|()| stdin.flush())
        .map_err(|error| runtime_error(operation_name(&command.command), error.to_string()))
}

const fn operation_name(command: &WireCommandKind) -> &'static str {
    match command {
        WireCommandKind::SetContentSize { .. } => "size",
        WireCommandKind::Show => "show",
        WireCommandKind::Hide => "hide",
        WireCommandKind::Focus => "focus",
        WireCommandKind::ReleaseFocus => "release_focus",
        WireCommandKind::SetResizable { .. } => "resize_hint",
        WireCommandKind::ScriptRequest { .. } => "script",
        WireCommandKind::Observe => "observe",
        WireCommandKind::Shutdown | WireCommandKind::Crash => "teardown",
    }
}

fn runtime_error(operation: &'static str, detail: impl Into<String>) -> IsolatedWindowError {
    IsolatedWindowError::Runtime {
        operation,
        detail: detail.into(),
    }
}
