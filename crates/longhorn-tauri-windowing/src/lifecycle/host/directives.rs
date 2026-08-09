use std::sync::mpsc::RecvTimeoutError;

use longhorn_core::WindowId;
use longhorn_windowing::{CaptureGeneration, WindowLifecycleDirective, WindowLifecycleEvent};
use tauri::Runtime;

use super::{FlushDisposition, PendingFlush, TauriWindowLifecycleHost, coordination_error};
use crate::lifecycle::{
    ScheduledWindowLifecycleWake, TauriWindowLifecycleAction, TauriWindowLifecycleError,
    WindowFlushOutcome, WindowFlushRequest, WindowFlushScope, WindowFlushTarget,
    WindowPlacementFlushCompletion,
};

impl<R: Runtime> TauriWindowLifecycleHost<R> {
    pub(super) fn execute_directives(
        &self,
        directives: Vec<WindowLifecycleDirective>,
        actions: &mut Vec<TauriWindowLifecycleAction>,
        flushes: &mut FlushDisposition,
    ) -> Result<(), TauriWindowLifecycleError> {
        for directive in directives {
            match directive {
                WindowLifecycleDirective::Ignore { reason, .. } => {
                    actions.push(TauriWindowLifecycleAction::Ignored { reason });
                }
                WindowLifecycleDirective::ScheduleCapture {
                    window_id,
                    generation,
                    due_at,
                } => self.schedule(
                    ScheduledWindowLifecycleWake::new(
                        due_at,
                        WindowLifecycleEvent::CaptureDeadline {
                            window_id,
                            generation,
                        },
                    ),
                    actions,
                ),
                WindowLifecycleDirective::CaptureNow {
                    window_id,
                    generation,
                    reason,
                } => {
                    let (window, retained_normal) = {
                        let windows = self.lock_windows()?;
                        let installed = windows.get(&window_id).ok_or_else(|| {
                            TauriWindowLifecycleError::UnknownWindow {
                                window_id: window_id.clone(),
                            }
                        })?;
                        (installed.window.clone(), installed.retained_normal())
                    };
                    match self
                        .services
                        .capture
                        .capture(&window_id, &window, retained_normal)
                    {
                        Ok(placement) => {
                            if !placement.is_maximized() {
                                let mut windows = self.lock_windows()?;
                                if let Some(installed) = windows.get_mut(&window_id) {
                                    // Only write back over the value observed
                                    // before the unlocked capture; a newer
                                    // retained normal installed concurrently
                                    // by register_apply wins.
                                    if installed.retained_normal() == retained_normal {
                                        installed.set_retained_normal(Some(
                                            placement.normal_placement(),
                                        ));
                                    }
                                }
                            }
                            if let Err(detail) = self.services.sink.stage(&placement) {
                                actions.push(TauriWindowLifecycleAction::PersistenceFailed {
                                    generation,
                                    detail,
                                });
                                self.reset_failed_capture(&window_id, generation)?;
                                continue;
                            }
                            actions.push(TauriWindowLifecycleAction::PlacementStaged {
                                generation,
                                reason,
                                placement,
                            });
                            let nested = {
                                let mut coordinator = self.lock_coordinator()?;
                                coordinator
                                    .handle(
                                        self.services.clock.now(),
                                        WindowLifecycleEvent::CaptureCompleted {
                                            window_id,
                                            generation,
                                        },
                                    )
                                    .map_err(coordination_error)?
                            };
                            self.execute_directives(nested, actions, flushes)?;
                        }
                        Err(detail) => {
                            actions.push(TauriWindowLifecycleAction::CaptureFailed {
                                generation,
                                detail,
                            });
                            self.reset_failed_capture(&window_id, generation)?;
                        }
                    }
                }
                WindowLifecycleDirective::ScheduleFlush {
                    window_id,
                    generation,
                    due_at,
                } => self.schedule(
                    ScheduledWindowLifecycleWake::new(
                        due_at,
                        WindowLifecycleEvent::FlushDeadline {
                            window_id,
                            generation,
                        },
                    ),
                    actions,
                ),
                WindowLifecycleDirective::Flush {
                    window_id,
                    generation,
                    timeout,
                    reason,
                } => {
                    let pending = PendingFlush {
                        target: WindowFlushTarget::new(window_id, generation),
                        timeout,
                        scope: WindowFlushScope::Window { reason },
                    };
                    match flushes {
                        FlushDisposition::Inline => {
                            let request = WindowFlushRequest::new(
                                vec![pending.target],
                                pending.timeout,
                                pending.scope,
                            );
                            let outcome = self.flush(&request);
                            actions.push(TauriWindowLifecycleAction::Flushed { request, outcome });
                        }
                        FlushDisposition::Collect(collected) => collected.push(pending),
                    }
                }
                WindowLifecycleDirective::UserClose { window_id } => {
                    match self.services.user_close.user_close(&window_id) {
                        Ok(()) => actions.push(TauriWindowLifecycleAction::UserCloseReported),
                        Err(detail) => {
                            actions.push(TauriWindowLifecycleAction::UserCloseFailed { detail });
                        }
                    }
                }
                WindowLifecycleDirective::Forget { window_id } => {
                    self.lock_windows()?.remove(&window_id);
                    actions.push(TauriWindowLifecycleAction::Forgotten);
                }
            }
        }
        Ok(())
    }

    fn schedule(
        &self,
        wake: ScheduledWindowLifecycleWake,
        actions: &mut Vec<TauriWindowLifecycleAction>,
    ) {
        match self.services.scheduler.schedule(wake.clone()) {
            Ok(()) => actions.push(TauriWindowLifecycleAction::Scheduled { wake }),
            Err(detail) => {
                actions.push(TauriWindowLifecycleAction::ScheduleFailed { wake, detail });
            }
        }
    }

    fn reset_failed_capture(
        &self,
        window_id: &WindowId,
        generation: CaptureGeneration,
    ) -> Result<(), TauriWindowLifecycleError> {
        self.lock_coordinator()?
            .handle(
                self.services.clock.now(),
                WindowLifecycleEvent::CaptureFailed {
                    window_id: window_id.clone(),
                    generation,
                },
            )
            .map_err(coordination_error)?;
        Ok(())
    }

    pub(super) fn flush(&self, request: &WindowFlushRequest) -> WindowFlushOutcome {
        let ticket = match self.services.sink.request_flush(request) {
            Ok(ticket) => ticket,
            Err(detail) => return WindowFlushOutcome::RequestFailed { detail },
        };
        match ticket.wait(request.timeout().as_millis()) {
            Ok(WindowPlacementFlushCompletion::Succeeded) => WindowFlushOutcome::Succeeded,
            Ok(WindowPlacementFlushCompletion::Failed(detail)) => {
                WindowFlushOutcome::SinkFailed { detail }
            }
            Err(RecvTimeoutError::Timeout) => WindowFlushOutcome::TimedOut,
            Err(RecvTimeoutError::Disconnected) => WindowFlushOutcome::Disconnected,
        }
    }
}
