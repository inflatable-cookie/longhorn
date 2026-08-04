use longhorn_core::{DisplayId, ScreenRect, WindowPlacement};
use longhorn_display::{DisplayAvailability, DisplayFacts, DisplayInventory};

use crate::{
    PlacementPolicy, PlacementReason, PlacementResolutionError, ResolvedWindowPlacement,
    UnavailablePlacementReason, UnavailableWindowPlacement, WindowPlacementConfig,
    WindowPlacementResolution, WindowRole,
};

struct AvailableDisplay<'a> {
    id: &'a DisplayId,
    facts: &'a DisplayFacts,
}

/// Resolves one logical window without persistence or host mutation.
pub fn resolve_window_placement(
    config: &WindowPlacementConfig,
    inventory: &DisplayInventory,
    policy: PlacementPolicy,
) -> Result<WindowPlacementResolution, PlacementResolutionError> {
    if !config.is_enabled() {
        return Ok(WindowPlacementResolution::Disabled {
            window_id: config.window_id().clone(),
            role: config.role(),
        });
    }

    let available = available_displays(inventory);
    if available.is_empty() {
        return Ok(unavailable(
            config,
            UnavailablePlacementReason::NoAvailableDisplays,
        ));
    }

    let reference = config.reference_normal_placement();
    let selected = configured_display(config, &available)
        .or_else(|| required_fallback(config, &available, reference, policy));

    let Some((target, reason)) = selected else {
        return Ok(unavailable(
            config,
            UnavailablePlacementReason::NoConfiguredDisplayAvailable,
        ));
    };

    let requested = config.normal_placement_for(target.id).unwrap_or(reference);
    let requested_rect = ScreenRect::new(requested.outer_origin(), requested.inner_size());
    let fitted = requested_rect
        .fit_within(&target.facts.work_area(), &policy.minimum_size())
        .map_err(|source| PlacementResolutionError::Geometry {
            window_id: config.window_id().clone(),
            display_id: target.id.clone(),
            source,
        })?;

    Ok(WindowPlacementResolution::Resolved(
        ResolvedWindowPlacement {
            window_id: config.window_id().clone(),
            role: config.role(),
            configured_home_display_id: config.home_display_id().cloned(),
            target_display_id: target.id.clone(),
            target_work_area: target.facts.work_area(),
            reason,
            normal_placement: WindowPlacement::new(fitted.origin(), fitted.size()),
            maximized: config.is_maximized(),
        },
    ))
}

fn available_displays(inventory: &DisplayInventory) -> Vec<AvailableDisplay<'_>> {
    let mut available: Vec<_> = inventory
        .displays()
        .iter()
        .filter_map(|entry| match entry.availability() {
            DisplayAvailability::Available { .. } => Some(AvailableDisplay {
                id: entry.display().id(),
                facts: entry.display().facts(),
            }),
            DisplayAvailability::Missing | DisplayAvailability::Unresolved { .. } => None,
        })
        .collect();
    available.sort_by(|left, right| left.id.cmp(right.id));
    available
}

fn configured_display<'a>(
    config: &WindowPlacementConfig,
    available: &'a [AvailableDisplay<'a>],
) -> Option<(&'a AvailableDisplay<'a>, PlacementReason)> {
    if let Some(home) = config.home_display_id()
        && let Some(display) = find_available(available, home)
    {
        return Some((display, PlacementReason::ConfiguredHome));
    }

    config.fallback_display_ids().iter().find_map(|id| {
        find_available(available, id).map(|display| (display, PlacementReason::ConfiguredFallback))
    })
}

fn required_fallback<'a>(
    config: &WindowPlacementConfig,
    available: &'a [AvailableDisplay<'a>],
    reference: WindowPlacement,
    policy: PlacementPolicy,
) -> Option<(&'a AvailableDisplay<'a>, PlacementReason)> {
    if config.role() == WindowRole::Optional {
        return None;
    }

    if let Some((display, area)) = largest_useful_intersection(available, reference, policy) {
        return Some((display, PlacementReason::UsefulIntersection { area }));
    }

    if let Some(main) = available.iter().find(|display| display.facts.is_main()) {
        return Some((main, PlacementReason::MainDisplay));
    }

    available
        .first()
        .map(|display| (display, PlacementReason::DeterministicFallback))
}

fn largest_useful_intersection<'a>(
    available: &'a [AvailableDisplay<'a>],
    reference: WindowPlacement,
    policy: PlacementPolicy,
) -> Option<(&'a AvailableDisplay<'a>, u64)> {
    let reference_rect = ScreenRect::new(reference.outer_origin(), reference.inner_size());
    let mut best: Option<(&AvailableDisplay<'_>, u64)> = None;

    for display in available {
        let Some(intersection) = reference_rect.intersection(&display.facts.work_area()) else {
            continue;
        };
        if !is_useful(
            &intersection,
            &reference_rect,
            &display.facts.work_area(),
            policy,
        ) {
            continue;
        }
        let area = intersection.area();
        if best.is_none_or(|(_, best_area)| area > best_area) {
            best = Some((display, area));
        }
    }

    best
}

fn is_useful(
    intersection: &ScreenRect,
    reference: &ScreenRect,
    work_area: &ScreenRect,
    policy: PlacementPolicy,
) -> bool {
    let requested = policy.minimum_visible_extent();
    let required_width = requested
        .width()
        .min(reference.size().width())
        .min(work_area.size().width());
    let required_height = requested
        .height()
        .min(reference.size().height())
        .min(work_area.size().height());

    intersection.size().width() >= required_width && intersection.size().height() >= required_height
}

fn find_available<'a>(
    available: &'a [AvailableDisplay<'a>],
    id: &DisplayId,
) -> Option<&'a AvailableDisplay<'a>> {
    available.iter().find(|display| display.id == id)
}

fn unavailable(
    config: &WindowPlacementConfig,
    reason: UnavailablePlacementReason,
) -> WindowPlacementResolution {
    WindowPlacementResolution::Unavailable(UnavailableWindowPlacement::new(
        config.window_id().clone(),
        config.role(),
        config.home_display_id().cloned(),
        reason,
    ))
}
