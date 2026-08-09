use std::collections::BTreeSet;

use longhorn_core::{ScaleFactor, ScreenPoint, ScreenRect, ScreenSize};
use longhorn_display::{
    AdapterDisplayKey, DisplayBuiltinStatus, DisplayEvidence, DisplayFacts, ObservationId,
    ObservedDisplay, StrongDisplayKey,
};
use longhorn_windowing::{HostWindowHandle, LiveWindow};

use crate::{
    GpuiDisplayError, GpuiDisplayFacts, GpuiObservationError, GpuiWindowBackend, GpuiWindowFacts,
    GpuiWindowKey, GpuiWindowRegistry, UnobtainableDisplayFact,
};

/// The GPUI evidence namespace for display correlation keys.
pub const GPUI_DISPLAY_NAMESPACE: &str = "gpui";

/// Supplies the display facts GPUI will not report.
///
/// GPUI's `PlatformDisplay` reports an id, a persistable UUID, and logical
/// bounds. Contract 020 requires displays "with scale factors", and Longhorn's
/// own [`DisplayFacts`] additionally requires a work area — neither is
/// optional in the pure model. A GPUI application must therefore obtain both
/// from somewhere else, and this trait is where it says where.
///
/// Returning `None` is a valid answer. It produces a
/// [`GpuiDisplayObservation::Unobtainable`], which is what "recorded as
/// unproven" looks like in code.
pub trait GpuiDisplayFactsSource {
    /// Returns the scale factor for one display, if the caller knows it.
    ///
    /// The practical source is a live window on that display:
    /// `Window::scale_factor` is GPUI's only scale query. A display with no
    /// window on it has no scale a GPUI application can read.
    fn scale_factor(&mut self, facts: &GpuiDisplayFacts) -> Option<ScaleFactor>;

    /// Returns the usable work area for one display, if the caller knows it.
    ///
    /// GPUI reports full bounds only. A product that wants windows placed
    /// clear of system chrome supplies the inset here, from the platform or
    /// from its own configuration.
    fn work_area(&mut self, facts: &GpuiDisplayFacts) -> Option<ScreenRect>;

    /// Returns where the display sits in the global plane, if the caller
    /// knows.
    ///
    /// GPUI's macOS backend zeroes every display origin — it reads
    /// `CGDisplayBounds`, which is documented as global, and keeps only the
    /// size. So two attached displays both report `(0, 0)`, and a caller that
    /// took that at face value would place every window on the primary and
    /// would collide two displays in any arrangement signature.
    fn position(&mut self, _facts: &GpuiDisplayFacts) -> Option<ScreenPoint> {
        None
    }

    /// Returns built-in status, when the caller has platform evidence for it.
    fn builtin_status(&mut self, _facts: &GpuiDisplayFacts) -> DisplayBuiltinStatus {
        DisplayBuiltinStatus::Unknown
    }
}

/// One display observation, or a typed record of what GPUI could not report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiDisplayObservation {
    /// Every fact contract 020 requires was obtained.
    Resolved(Box<ObservedDisplay>),
    /// At least one required fact was unavailable, so no facts were invented.
    Unobtainable {
        /// Ordinal of the display in the probe.
        ordinal: usize,
        /// Ephemeral identity, which is obtainable even when facts are not.
        observation_id: ObservationId,
        /// Correlation evidence, which GPUI reports well.
        evidence: DisplayEvidence,
        /// Full logical extent — the geometric fact GPUI does report.
        ///
        /// Deliberately a size and not a rectangle. GPUI discards the display
        /// origin, so there is no honest rectangle to offer here.
        full_size: ScreenSize,
        /// Every fact the caller could not supply.
        missing: Vec<UnobtainableDisplayFact>,
    },
}

impl GpuiDisplayObservation {
    /// Returns the complete observation when one was obtainable.
    #[must_use]
    pub fn resolved(&self) -> Option<&ObservedDisplay> {
        match self {
            Self::Resolved(display) => Some(display),
            Self::Unobtainable { .. } => None,
        }
    }

    /// Returns the facts GPUI and the caller together could not supply.
    #[must_use]
    pub fn missing(&self) -> &[UnobtainableDisplayFact] {
        match self {
            Self::Resolved(_) => &[],
            Self::Unobtainable { missing, .. } => missing,
        }
    }
}

/// Complete GPUI desktop observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuiDesktopObservation {
    displays: Vec<GpuiDisplayObservation>,
    windows: Vec<LiveWindow>,
}

impl GpuiDesktopObservation {
    /// Records one complete observation.
    #[must_use]
    pub const fn new(displays: Vec<GpuiDisplayObservation>, windows: Vec<LiveWindow>) -> Self {
        Self { displays, windows }
    }

    /// Returns observed displays in probe order.
    #[must_use]
    pub fn displays(&self) -> &[GpuiDisplayObservation] {
        &self.displays
    }

    /// Returns observed managed windows in stable handle order.
    #[must_use]
    pub fn windows(&self) -> &[LiveWindow] {
        &self.windows
    }
}

/// Projects one GPUI display into Longhorn's display vocabulary.
pub fn project_gpui_display(
    ordinal: usize,
    facts: &GpuiDisplayFacts,
    source: &mut impl GpuiDisplayFactsSource,
) -> Result<GpuiDisplayObservation, GpuiDisplayError> {
    let observation_id = ObservationId::new(format!("gpui-probe:{ordinal}")).map_err(|error| {
        GpuiDisplayError::Host {
            detail: error.to_string(),
        }
    })?;
    let full_size =
        facts
            .bounds()
            .to_screen_size()
            .map_err(|error| GpuiDisplayError::InvalidBounds {
                ordinal,
                detail: error.to_string(),
            })?;

    let mut evidence = DisplayEvidence::new().with_adapter_key(
        AdapterDisplayKey::new(GPUI_DISPLAY_NAMESPACE, format!("id:{}", facts.display_id()))
            .map_err(|error| GpuiDisplayError::Host {
                detail: error.to_string(),
            })?,
    );
    // GPUI's UUID is documented as stable across system restarts. Tauri has no
    // equivalent, which is why its adapter correlates monitors by name,
    // position and size and carries an ambiguity error for the cases where
    // that fails. Recording it as a strong key is the one place the GPUI host
    // supplies better evidence than the Tauri host.
    if let Some(uuid) = facts.stable_uuid() {
        evidence = evidence.with_strong_key(
            StrongDisplayKey::new(GPUI_DISPLAY_NAMESPACE, format!("uuid:{uuid}")).map_err(
                |error| GpuiDisplayError::Host {
                    detail: error.to_string(),
                },
            )?,
        );
    }

    let scale = source.scale_factor(facts);
    let work_area = source.work_area(facts);
    let position = source.position(facts);
    let mut missing = Vec::new();
    if position.is_none() {
        missing.push(UnobtainableDisplayFact::Position);
    }
    if scale.is_none() {
        missing.push(UnobtainableDisplayFact::ScaleFactor);
    }
    if work_area.is_none() {
        missing.push(UnobtainableDisplayFact::WorkArea);
    }
    let (Some(position), Some(scale), Some(work_area)) = (position, scale, work_area) else {
        return Ok(GpuiDisplayObservation::Unobtainable {
            ordinal,
            observation_id,
            evidence,
            full_size,
            missing,
        });
    };
    let full_bounds = ScreenRect::new(position, full_size);

    Ok(GpuiDisplayObservation::Resolved(Box::new(
        ObservedDisplay::new(
            observation_id,
            DisplayFacts::new(
                // GPUI reports no machine label. Its UUID is the identity, and
                // it is evidence rather than a display name.
                None,
                facts.is_primary(),
                source.builtin_status(facts),
                full_bounds,
                work_area,
                scale,
            ),
            evidence,
        ),
    )))
}

/// Projects one GPUI window observation into the pure diff vocabulary.
///
/// `visible` is reported as `true` unconditionally. A GPUI window exists on
/// screen from creation until it is removed; there is no hidden state to
/// observe, so reporting anything else would be an invention.
pub fn project_gpui_window(
    handle: &HostWindowHandle,
    window_id: Option<longhorn_core::WindowId>,
    facts: &GpuiWindowFacts,
) -> Result<LiveWindow, GpuiObservationError> {
    let metrics = facts
        .to_live_metrics()
        .map_err(|error| GpuiObservationError::Geometry {
            handle: handle.clone(),
            detail: error.to_string(),
        })?;
    facts
        .scale_factor()
        .map_err(|source| GpuiObservationError::InvalidScale {
            handle: handle.clone(),
            source,
        })?;
    Ok(LiveWindow::new(
        window_id,
        handle.clone(),
        metrics,
        facts.bounds_state().is_maximized(),
        true,
        facts.is_active(),
    ))
}

/// Probes every managed window and every display GPUI knows about.
pub fn observe_gpui_desktop(
    backend: &mut impl GpuiWindowBackend,
    registry: &GpuiWindowRegistry,
    source: &mut impl GpuiDisplayFactsSource,
) -> Result<GpuiDesktopObservation, GpuiObservationError> {
    let windows = observe_gpui_windows(backend, registry)?;
    let displays = observe_gpui_displays(backend, source).map_err(|error| {
        // Display failure is reported through the window-observation error so
        // one readback has one typed failure. The detail keeps the display
        // diagnostic intact.
        GpuiObservationError::Host {
            handle: HostWindowHandle::new("gpui-displays")
                .expect("literal display probe handle is valid"),
            detail: error.to_string(),
        }
    })?;
    Ok(GpuiDesktopObservation::new(displays, windows))
}

/// Probes exactly the managed windows or fails without a snapshot.
pub fn observe_gpui_windows(
    backend: &mut impl GpuiWindowBackend,
    registry: &GpuiWindowRegistry,
) -> Result<Vec<LiveWindow>, GpuiObservationError> {
    let mut handles = BTreeSet::new();
    let mut stable_ids = BTreeSet::new();
    let mut observations = Vec::new();

    for managed in registry.managed_windows() {
        let handle = managed.key().transport_handle();
        if !handles.insert(handle.clone()) {
            return Err(GpuiObservationError::DuplicateTransportHandle(handle));
        }
        if let Some(window_id) = managed.window_id()
            && !stable_ids.insert(window_id.clone())
        {
            return Err(GpuiObservationError::DuplicateWindowId(window_id.clone()));
        }
        let facts = observe_key(backend, managed.key(), &handle)?;
        observations.push(project_gpui_window(
            &handle,
            managed.window_id().cloned(),
            &facts,
        )?);
    }

    Ok(observations)
}

/// Probes every display, projecting each into Longhorn's vocabulary.
pub fn observe_gpui_displays(
    backend: &mut impl GpuiWindowBackend,
    source: &mut impl GpuiDisplayFactsSource,
) -> Result<Vec<GpuiDisplayObservation>, GpuiDisplayError> {
    let facts = backend.displays().map_err(|error| GpuiDisplayError::Host {
        detail: error.detail().to_string(),
    })?;
    let mut observation_ids = BTreeSet::new();
    let mut displays = Vec::with_capacity(facts.len());
    for (ordinal, display) in facts.iter().enumerate() {
        let observation = project_gpui_display(ordinal, display, source)?;
        let observation_id = match &observation {
            GpuiDisplayObservation::Resolved(resolved) => resolved.observation_id().clone(),
            GpuiDisplayObservation::Unobtainable { observation_id, .. } => observation_id.clone(),
        };
        if !observation_ids.insert(observation_id.clone()) {
            return Err(GpuiDisplayError::DuplicateObservationId {
                observation_id: observation_id.to_string(),
            });
        }
        displays.push(observation);
    }
    Ok(displays)
}

fn observe_key(
    backend: &mut impl GpuiWindowBackend,
    key: GpuiWindowKey,
    handle: &HostWindowHandle,
) -> Result<GpuiWindowFacts, GpuiObservationError> {
    backend
        .observe(key)
        .map_err(|error| GpuiObservationError::Host {
            handle: handle.clone(),
            detail: error.detail().to_string(),
        })
}
