use longhorn_core::WindowId;
use longhorn_windowing::{ApplyGeneration, HostWindowHandle, WindowOperation, WindowOperationKind};
use tauri::{AppHandle, Runtime};

use crate::{
    ManagedWindowRegistry, ManagedWindowRegistryError, NativeWindowCall, TauriWindowFactory,
    WindowApplyAttempt, WindowApplyFailure, WindowApplyFailureKind, WindowApplyOutcome,
    WindowFactoryError, WindowMutationBackend,
};

pub(super) fn execute_operation<R, F, B>(
    app: &AppHandle<R>,
    registry: &mut ManagedWindowRegistry<R>,
    factory: &mut F,
    backend: &mut B,
    generation: ApplyGeneration,
    operation: &WindowOperation,
) -> WindowApplyAttempt
where
    R: Runtime,
    F: TauriWindowFactory<R>,
    B: WindowMutationBackend<R>,
{
    let window_id = operation.window_id().clone();
    let kind = operation.kind();
    match operation {
        WindowOperation::Retag {
            transport_handle, ..
        } => execute_retag(
            registry,
            generation,
            window_id,
            kind,
            transport_handle,
            operation,
        ),
        WindowOperation::Create { .. } => execute_create(
            app, registry, factory, generation, window_id, kind, operation,
        ),
        _ => execute_existing(registry, backend, generation, operation),
    }
}

fn execute_retag<R: Runtime>(
    registry: &mut ManagedWindowRegistry<R>,
    generation: ApplyGeneration,
    window_id: WindowId,
    kind: WindowOperationKind,
    transport_handle: &HostWindowHandle,
    operation: &WindowOperation,
) -> WindowApplyAttempt {
    if !registry.contains_handle(transport_handle) {
        return failed_attempt(
            generation,
            window_id,
            Some(transport_handle.clone()),
            kind,
            Vec::new(),
            registry_failure(
                NativeWindowCall::ResolveManagedWindow,
                ManagedWindowRegistryError::UnknownTransportHandle(transport_handle.clone()),
            ),
        );
    }
    if let Err(error) = registry.record_evidence(Some(transport_handle.clone()), operation.clone())
    {
        return failed_attempt(
            generation,
            window_id,
            Some(transport_handle.clone()),
            kind,
            Vec::new(),
            registry_failure(NativeWindowCall::RegistryRetag, error),
        );
    }
    match registry.retag(transport_handle, window_id.clone()) {
        Ok(()) => succeeded_attempt(
            generation,
            window_id,
            Some(transport_handle.clone()),
            kind,
            vec![NativeWindowCall::RegistryRetag],
        ),
        Err(error) => failed_attempt(
            generation,
            window_id,
            Some(transport_handle.clone()),
            kind,
            Vec::new(),
            registry_failure(NativeWindowCall::RegistryRetag, error),
        ),
    }
}

fn execute_create<R, F>(
    app: &AppHandle<R>,
    registry: &mut ManagedWindowRegistry<R>,
    factory: &mut F,
    generation: ApplyGeneration,
    window_id: WindowId,
    kind: WindowOperationKind,
    operation: &WindowOperation,
) -> WindowApplyAttempt
where
    R: Runtime,
    F: TauriWindowFactory<R>,
{
    if let Err(error) = registry.record_evidence(None, operation.clone()) {
        return failed_attempt(
            generation,
            window_id,
            None,
            kind,
            Vec::new(),
            registry_failure(NativeWindowCall::FactoryCreate, error),
        );
    }
    let window = match factory.create(app, &window_id) {
        Ok(window) => window,
        Err(error) => {
            return failed_attempt(
                generation,
                window_id,
                None,
                kind,
                Vec::new(),
                factory_failure(NativeWindowCall::FactoryCreate, error),
            );
        }
    };
    let mut completed = vec![NativeWindowCall::FactoryCreate];
    match factory.validate_neutral(&window) {
        Ok(()) => {
            completed.push(NativeWindowCall::ValidateHidden);
            completed.push(NativeWindowCall::ValidateUnmaximized);
        }
        Err(error) => {
            let call = match &error {
                WindowFactoryError::Maximized
                | WindowFactoryError::MaximizedInspectionFailed { .. } => {
                    completed.push(NativeWindowCall::ValidateHidden);
                    NativeWindowCall::ValidateUnmaximized
                }
                _ => NativeWindowCall::ValidateHidden,
            };
            return failed_attempt(
                generation,
                window_id,
                None,
                kind,
                completed,
                factory_failure(call, error),
            );
        }
    }

    match registry.insert_created(window_id.clone(), window) {
        Ok(handle) => {
            completed.push(NativeWindowCall::RegistryInsert);
            if registry.registers_created_windows() {
                completed.push(NativeWindowCall::InstallLifecycleListener);
            }
            succeeded_attempt(generation, window_id, Some(handle), kind, completed)
        }
        Err(error) => failed_attempt(
            generation,
            window_id,
            match &error {
                ManagedWindowRegistryError::DynamicRegistrationFailed {
                    transport_handle, ..
                } => Some(transport_handle.clone()),
                _ => None,
            },
            kind,
            {
                if matches!(
                    error,
                    ManagedWindowRegistryError::DynamicRegistrationFailed { .. }
                ) {
                    completed.push(NativeWindowCall::RegistryInsert);
                }
                completed
            },
            registry_failure(
                if matches!(
                    error,
                    ManagedWindowRegistryError::DynamicRegistrationFailed { .. }
                ) {
                    NativeWindowCall::InstallLifecycleListener
                } else {
                    NativeWindowCall::RegistryInsert
                },
                error,
            ),
        ),
    }
}

fn execute_existing<R, B>(
    registry: &mut ManagedWindowRegistry<R>,
    backend: &mut B,
    generation: ApplyGeneration,
    operation: &WindowOperation,
) -> WindowApplyAttempt
where
    R: Runtime,
    B: WindowMutationBackend<R>,
{
    let window_id = operation.window_id().clone();
    let kind = operation.kind();
    let (handle, window) = match registry.resolve(&window_id, operation.transport_handle()) {
        Ok(resolved) => resolved,
        Err(error) => {
            return failed_attempt(
                generation,
                window_id,
                operation.transport_handle().cloned(),
                kind,
                Vec::new(),
                registry_failure(NativeWindowCall::ResolveManagedWindow, error),
            );
        }
    };
    if matches!(operation, WindowOperation::Close { .. }) && registry.is_protected_primary(&handle)
    {
        return failed_attempt(
            generation,
            window_id,
            Some(handle),
            kind,
            Vec::new(),
            WindowApplyFailure::new(
                NativeWindowCall::ProtectPrimary,
                WindowApplyFailureKind::ProtectedPrimary,
                "protected primary cannot be closed",
            ),
        );
    }

    if let Err(error) = registry.record_evidence(Some(handle.clone()), operation.clone()) {
        return failed_attempt(
            generation,
            window_id,
            Some(handle),
            kind,
            Vec::new(),
            registry_failure(NativeWindowCall::ResolveManagedWindow, error),
        );
    }
    let mut completed = Vec::new();
    let result = match operation {
        WindowOperation::Unmaximize { .. } => native_call(
            backend.unmaximize(&window),
            NativeWindowCall::Unmaximize,
            &mut completed,
        ),
        WindowOperation::MoveResize { placement, .. } => native_call(
            backend.set_outer_position(&window, placement.outer_origin()),
            NativeWindowCall::SetOuterPosition,
            &mut completed,
        )
        .and_then(|()| {
            native_call(
                backend.set_inner_size(&window, placement.inner_size()),
                NativeWindowCall::SetInnerSize,
                &mut completed,
            )
        }),
        WindowOperation::Maximize { .. } => native_call(
            backend.maximize(&window),
            NativeWindowCall::Maximize,
            &mut completed,
        ),
        WindowOperation::Show { .. } => native_call(
            backend.show(&window),
            NativeWindowCall::Show,
            &mut completed,
        ),
        WindowOperation::Hide { .. } => native_call(
            backend.hide(&window),
            NativeWindowCall::Hide,
            &mut completed,
        ),
        WindowOperation::Focus { .. } => native_call(
            backend.focus(&window),
            NativeWindowCall::Focus,
            &mut completed,
        ),
        WindowOperation::Close { .. } => native_call(
            backend.close(&window),
            NativeWindowCall::Close,
            &mut completed,
        ),
        WindowOperation::Retag { .. } | WindowOperation::Create { .. } => unreachable!(),
    };

    match result {
        Ok(()) => {
            if matches!(operation, WindowOperation::Close { .. }) {
                registry.remove_closed(&handle);
            }
            succeeded_attempt(generation, window_id, Some(handle), kind, completed)
        }
        Err(failure) => failed_attempt(
            generation,
            window_id,
            Some(handle),
            kind,
            completed,
            failure,
        ),
    }
}

fn native_call(
    result: Result<(), crate::NativeWindowMutationError>,
    call: NativeWindowCall,
    completed: &mut Vec<NativeWindowCall>,
) -> Result<(), WindowApplyFailure> {
    match result {
        Ok(()) => {
            completed.push(call);
            Ok(())
        }
        Err(error) => Err(WindowApplyFailure::new(
            call,
            WindowApplyFailureKind::Native,
            error.detail(),
        )),
    }
}

fn succeeded_attempt(
    generation: ApplyGeneration,
    window_id: WindowId,
    transport_handle: Option<HostWindowHandle>,
    operation: WindowOperationKind,
    completed_calls: Vec<NativeWindowCall>,
) -> WindowApplyAttempt {
    WindowApplyAttempt::new(
        generation,
        window_id,
        transport_handle,
        operation,
        WindowApplyOutcome::Succeeded { completed_calls },
    )
}

fn failed_attempt(
    generation: ApplyGeneration,
    window_id: WindowId,
    transport_handle: Option<HostWindowHandle>,
    operation: WindowOperationKind,
    completed_calls: Vec<NativeWindowCall>,
    failure: WindowApplyFailure,
) -> WindowApplyAttempt {
    WindowApplyAttempt::new(
        generation,
        window_id,
        transport_handle,
        operation,
        WindowApplyOutcome::Failed {
            completed_calls,
            failure,
        },
    )
}

fn registry_failure(
    call: NativeWindowCall,
    error: ManagedWindowRegistryError,
) -> WindowApplyFailure {
    WindowApplyFailure::new(call, WindowApplyFailureKind::Registry, error.to_string())
}

fn factory_failure(call: NativeWindowCall, error: WindowFactoryError) -> WindowApplyFailure {
    WindowApplyFailure::new(call, WindowApplyFailureKind::Factory, error.to_string())
}
