use std::collections::BTreeSet;

use longhorn_core::{PhysicalPoint, PhysicalRect, PhysicalSize};
use longhorn_windowing::HostWindowHandle;
use tauri::{AppHandle, Monitor, Runtime};

use crate::{
    DesktopCoordinateMapper, DesktopObservation, DisplayMetadataProvider, HostProbeOperation,
    ManagedWebviewWindow, PhysicalDesktopSnapshot, PhysicalDisplayObservation,
    PhysicalLiveWindowObservation, PhysicalMonitorFacts, ProbeTarget, TauriObservationError,
    TauriProbeError, project_desktop, scale_factor_from_tauri,
};

/// Probes all monitors and explicitly managed windows into one physical snapshot.
pub fn probe_tauri_desktop<R: Runtime>(
    app: &AppHandle<R>,
    managed_windows: &[ManagedWebviewWindow<R>],
    metadata_provider: &mut impl DisplayMetadataProvider,
) -> Result<PhysicalDesktopSnapshot, TauriProbeError> {
    let available = host_result(
        app.available_monitors(),
        HostProbeOperation::AvailableMonitors,
        None,
    )?;
    let primary = host_result(
        app.primary_monitor(),
        HostProbeOperation::PrimaryMonitor,
        None,
    )?;
    let primary_index = primary
        .as_ref()
        .map(|primary| {
            monitor_facts(primary, ProbeTarget::PrimaryDisplay)?;
            exact_primary_index(&available, primary)
        })
        .transpose()?;

    let mut monitors = available
        .iter()
        .enumerate()
        .map(|(ordinal, monitor)| monitor_facts(monitor, ProbeTarget::Display(ordinal)))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(index) = primary_index {
        monitors[index].mark_main();
    }

    let mut observation_ids = BTreeSet::new();
    let mut displays = Vec::with_capacity(monitors.len());
    for (ordinal, facts) in monitors.into_iter().enumerate() {
        let metadata = metadata_provider.metadata(ordinal, &facts);
        if !observation_ids.insert(metadata.observation_id().clone()) {
            return Err(TauriProbeError::DuplicateObservationId(
                metadata.observation_id().clone(),
            ));
        }
        displays.push(PhysicalDisplayObservation::new(facts, metadata));
    }

    let windows = probe_managed_windows(managed_windows)?;

    Ok(PhysicalDesktopSnapshot::new(displays, windows))
}

fn exact_primary_index(monitors: &[Monitor], primary: &Monitor) -> Result<usize, TauriProbeError> {
    let primary = MonitorMatchKey::from(primary);
    let monitors = monitors
        .iter()
        .map(MonitorMatchKey::from)
        .collect::<Vec<_>>();
    exact_primary_key_index(&monitors, &primary)
}

fn exact_primary_key_index(
    monitors: &[MonitorMatchKey],
    primary: &MonitorMatchKey,
) -> Result<usize, TauriProbeError> {
    let matches = monitors
        .iter()
        .enumerate()
        .filter_map(|(index, monitor)| (monitor == primary).then_some(index))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Err(TauriProbeError::PrimaryMonitorNotFound),
        [index] => Ok(*index),
        _ => Err(TauriProbeError::AmbiguousPrimaryMonitor {
            matches: matches.len(),
        }),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MonitorMatchKey {
    name: Option<String>,
    position: (i32, i32),
    size: (u32, u32),
    work_position: (i32, i32),
    work_size: (u32, u32),
    scale_bits: u64,
}

impl From<&Monitor> for MonitorMatchKey {
    fn from(monitor: &Monitor) -> Self {
        let position = monitor.position();
        let size = monitor.size();
        let work = monitor.work_area();
        Self {
            name: monitor.name().cloned(),
            position: (position.x, position.y),
            size: (size.width, size.height),
            work_position: (work.position.x, work.position.y),
            work_size: (work.size.width, work.size.height),
            scale_bits: monitor.scale_factor().to_bits(),
        }
    }
}

/// Probes exactly the supplied managed webview windows or fails without a snapshot.
pub fn probe_managed_windows<R: Runtime>(
    managed_windows: &[ManagedWebviewWindow<R>],
) -> Result<Vec<PhysicalLiveWindowObservation>, TauriProbeError> {
    let handles = validate_managed_windows(managed_windows)?;
    managed_windows
        .iter()
        .zip(handles)
        .map(|(managed, handle)| probe_window(managed, handle))
        .collect()
}

/// Probes and maps one complete Tauri desktop observation.
pub fn observe_tauri_desktop<R: Runtime>(
    app: &AppHandle<R>,
    managed_windows: &[ManagedWebviewWindow<R>],
    metadata_provider: &mut impl DisplayMetadataProvider,
    mapper: &impl DesktopCoordinateMapper,
) -> Result<DesktopObservation, TauriObservationError> {
    let snapshot = probe_tauri_desktop(app, managed_windows, metadata_provider)?;
    Ok(project_desktop(&snapshot, mapper)?)
}

fn monitor_facts(
    monitor: &Monitor,
    target: ProbeTarget,
) -> Result<PhysicalMonitorFacts, TauriProbeError> {
    let position = monitor.position();
    let size = monitor.size();
    let work_area = monitor.work_area();
    let scale = scale_factor_from_tauri(monitor.scale_factor())
        .map_err(|source| TauriProbeError::InvalidScale { target, source })?;

    Ok(PhysicalMonitorFacts::new(
        monitor.name().cloned(),
        false,
        PhysicalRect::new(
            PhysicalPoint::new(position.x, position.y),
            PhysicalSize::new(size.width, size.height),
        ),
        PhysicalRect::new(
            PhysicalPoint::new(work_area.position.x, work_area.position.y),
            PhysicalSize::new(work_area.size.width, work_area.size.height),
        ),
        scale,
    ))
}

fn validate_managed_windows<R: Runtime>(
    managed_windows: &[ManagedWebviewWindow<R>],
) -> Result<Vec<HostWindowHandle>, TauriProbeError> {
    let mut handles = BTreeSet::new();
    let mut stable_ids = BTreeSet::new();
    let mut ordered_handles = Vec::with_capacity(managed_windows.len());

    for managed in managed_windows {
        let label = managed.window().label();
        let handle = HostWindowHandle::new(label).map_err(|source| {
            TauriProbeError::InvalidTransportHandle {
                label: label.to_string(),
                source,
            }
        })?;
        if !handles.insert(handle.clone()) {
            return Err(TauriProbeError::DuplicateTransportHandle(handle));
        }
        if let Some(id) = managed.window_id() {
            if !stable_ids.insert(id.clone()) {
                return Err(TauriProbeError::DuplicateWindowId(id.clone()));
            }
        }
        ordered_handles.push(handle);
    }

    Ok(ordered_handles)
}

fn probe_window<R: Runtime>(
    managed: &ManagedWebviewWindow<R>,
    handle: HostWindowHandle,
) -> Result<PhysicalLiveWindowObservation, TauriProbeError> {
    let window = managed.window();
    let scale = host_result(
        window.scale_factor(),
        HostProbeOperation::WindowScale,
        Some(&handle),
    )
    .and_then(|value| {
        scale_factor_from_tauri(value).map_err(|source| TauriProbeError::InvalidScale {
            target: ProbeTarget::Window(handle.clone()),
            source,
        })
    })?;
    let outer_position = host_result(
        window.outer_position(),
        HostProbeOperation::OuterPosition,
        Some(&handle),
    )?;
    let outer_size = host_result(
        window.outer_size(),
        HostProbeOperation::OuterSize,
        Some(&handle),
    )?;
    let inner_size = host_result(
        window.inner_size(),
        HostProbeOperation::InnerSize,
        Some(&handle),
    )?;
    let maximized = host_result(
        window.is_maximized(),
        HostProbeOperation::Maximized,
        Some(&handle),
    )?;
    let visible = host_result(
        window.is_visible(),
        HostProbeOperation::Visible,
        Some(&handle),
    )?;
    let focused = host_result(
        window.is_focused(),
        HostProbeOperation::Focused,
        Some(&handle),
    )?;

    Ok(PhysicalLiveWindowObservation::new(
        managed.window_id().cloned(),
        handle,
        PhysicalRect::new(
            PhysicalPoint::new(outer_position.x, outer_position.y),
            PhysicalSize::new(outer_size.width, outer_size.height),
        ),
        PhysicalSize::new(inner_size.width, inner_size.height),
        scale,
        maximized,
        visible,
        focused,
    ))
}

fn host_result<T>(
    result: tauri::Result<T>,
    operation: HostProbeOperation,
    handle: Option<&HostWindowHandle>,
) -> Result<T, TauriProbeError> {
    result.map_err(|source| TauriProbeError::Host {
        operation,
        handle: handle.cloned(),
        detail: source.to_string(),
    })
}

#[cfg(test)]
mod tests;
