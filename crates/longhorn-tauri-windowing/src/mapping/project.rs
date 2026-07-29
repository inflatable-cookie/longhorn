use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::{LiveWindowMetrics, WindowId};
use longhorn_display::{DisplayFacts, DisplayLabel, ObservationId, ObservedDisplay};
use longhorn_windowing::{HostWindowHandle, LiveWindow};

use super::{DesktopCoordinateMapper, MappedDisplayGeometry, MappedWindowGeometry};
use crate::{DesktopObservation, DesktopObservationError, PhysicalDesktopSnapshot};

/// Validates and projects raw facts plus mapped geometry into pure domain observations.
pub fn project_desktop(
    snapshot: &PhysicalDesktopSnapshot,
    mapper: &impl DesktopCoordinateMapper,
) -> Result<DesktopObservation, DesktopObservationError> {
    let raw_displays = index_raw_displays(snapshot)?;
    let raw_windows = index_raw_windows(snapshot)?;
    let mapped = mapper.map_desktop(snapshot)?;
    let mapped_displays = index_mapped_displays(mapped.displays)?;
    let mapped_windows = index_mapped_windows(mapped.windows)?;

    if let Some(id) = mapped_displays
        .keys()
        .find(|id| !raw_displays.contains_key(*id))
    {
        return Err(DesktopObservationError::UnexpectedDisplayMapping(
            (*id).clone(),
        ));
    }
    if let Some(handle) = mapped_windows
        .keys()
        .find(|handle| !raw_windows.contains_key(*handle))
    {
        return Err(DesktopObservationError::UnexpectedWindowMapping(
            (*handle).clone(),
        ));
    }

    let mut displays = Vec::with_capacity(raw_displays.len());
    for (id, raw) in raw_displays {
        let geometry = mapped_displays
            .get(&id)
            .ok_or_else(|| DesktopObservationError::MissingDisplayMapping(id.clone()))?;
        let machine_label = raw
            .facts()
            .machine_label()
            .cloned()
            .map(DisplayLabel::new)
            .transpose()
            .map_err(|source| DesktopObservationError::InvalidMachineLabel {
                observation_id: id.clone(),
                source,
            })?;
        let facts = DisplayFacts::new(
            machine_label,
            raw.facts().is_main(),
            raw.metadata().builtin_status(),
            geometry.full_bounds(),
            geometry.work_area(),
            raw.facts().scale(),
        );
        displays.push(ObservedDisplay::new(
            id,
            facts,
            raw.metadata().evidence().clone(),
        ));
    }

    let mut windows = Vec::with_capacity(raw_windows.len());
    for (handle, raw) in raw_windows {
        let geometry = mapped_windows
            .get(&handle)
            .ok_or_else(|| DesktopObservationError::MissingWindowMapping(handle.clone()))?;
        windows.push(LiveWindow::new(
            raw.window_id().cloned(),
            handle,
            LiveWindowMetrics::new(geometry.outer_bounds(), geometry.inner_size()),
            raw.is_maximized(),
            raw.is_visible(),
            raw.is_focused(),
        ));
    }

    Ok(DesktopObservation::new(displays, windows))
}

fn index_raw_displays(
    snapshot: &PhysicalDesktopSnapshot,
) -> Result<BTreeMap<ObservationId, &crate::PhysicalDisplayObservation>, DesktopObservationError> {
    let mut indexed = BTreeMap::new();
    for display in snapshot.displays() {
        let id = display.metadata().observation_id().clone();
        if indexed.insert(id.clone(), display).is_some() {
            return Err(DesktopObservationError::DuplicateDisplayObservation(id));
        }
    }
    Ok(indexed)
}

fn index_raw_windows(
    snapshot: &PhysicalDesktopSnapshot,
) -> Result<
    BTreeMap<HostWindowHandle, &crate::PhysicalLiveWindowObservation>,
    DesktopObservationError,
> {
    let mut indexed = BTreeMap::new();
    let mut stable_ids = BTreeSet::<WindowId>::new();
    for window in snapshot.windows() {
        let handle = window.transport_handle().clone();
        if indexed.insert(handle.clone(), window).is_some() {
            return Err(DesktopObservationError::DuplicateWindowObservation(handle));
        }
        if let Some(id) = window.window_id() {
            if !stable_ids.insert(id.clone()) {
                return Err(DesktopObservationError::DuplicateStableWindowId(id.clone()));
            }
        }
    }
    Ok(indexed)
}

fn index_mapped_displays(
    displays: Vec<MappedDisplayGeometry>,
) -> Result<BTreeMap<ObservationId, MappedDisplayGeometry>, DesktopObservationError> {
    let mut indexed = BTreeMap::new();
    for display in displays {
        let id = display.observation_id().clone();
        if indexed.insert(id.clone(), display).is_some() {
            return Err(DesktopObservationError::DuplicateMappedDisplay(id));
        }
    }
    Ok(indexed)
}

fn index_mapped_windows(
    windows: Vec<MappedWindowGeometry>,
) -> Result<BTreeMap<HostWindowHandle, MappedWindowGeometry>, DesktopObservationError> {
    let mut indexed = BTreeMap::new();
    for window in windows {
        let handle = window.transport_handle().clone();
        if indexed.insert(handle.clone(), window).is_some() {
            return Err(DesktopObservationError::DuplicateMappedWindow(handle));
        }
    }
    Ok(indexed)
}
