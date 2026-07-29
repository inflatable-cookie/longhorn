use longhorn_core::{PhysicalRect, ScaleFactor};
use longhorn_display::{DisplayBuiltinStatus, DisplayEvidence, ObservationId};
use serde::{Deserialize, Serialize};

mod window;

pub use window::{
    DesktopObservation, ManagedWebviewWindow, PhysicalDesktopSnapshot,
    PhysicalLiveWindowObservation,
};

/// Raw physical facts for one monitor before observation metadata is attached.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PhysicalMonitorFacts {
    machine_label: Option<String>,
    is_main: bool,
    full_bounds: PhysicalRect,
    work_area: PhysicalRect,
    scale: ScaleFactor,
}

impl PhysicalMonitorFacts {
    /// Constructs raw monitor facts.
    #[must_use]
    pub const fn new(
        machine_label: Option<String>,
        is_main: bool,
        full_bounds: PhysicalRect,
        work_area: PhysicalRect,
        scale: ScaleFactor,
    ) -> Self {
        Self {
            machine_label,
            is_main,
            full_bounds,
            work_area,
            scale,
        }
    }

    /// Returns the machine-provided label.
    #[must_use]
    pub const fn machine_label(&self) -> Option<&String> {
        self.machine_label.as_ref()
    }

    /// Returns whether exact primary-monitor matching selected this monitor.
    #[must_use]
    pub const fn is_main(&self) -> bool {
        self.is_main
    }

    /// Returns full physical bounds.
    #[must_use]
    pub const fn full_bounds(&self) -> PhysicalRect {
        self.full_bounds
    }

    /// Returns the physical work area.
    #[must_use]
    pub const fn work_area(&self) -> PhysicalRect {
        self.work_area
    }

    /// Returns validated scale evidence.
    #[must_use]
    pub const fn scale(&self) -> ScaleFactor {
        self.scale
    }

    pub(crate) fn mark_main(&mut self) {
        self.is_main = true;
    }
}

/// Caller-supplied identity and evidence for one monitor observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DisplayObservationMetadata {
    observation_id: ObservationId,
    builtin_status: DisplayBuiltinStatus,
    evidence: DisplayEvidence,
}

impl DisplayObservationMetadata {
    /// Constructs metadata without allocating canonical display identity.
    #[must_use]
    pub const fn new(
        observation_id: ObservationId,
        builtin_status: DisplayBuiltinStatus,
        evidence: DisplayEvidence,
    ) -> Self {
        Self {
            observation_id,
            builtin_status,
            evidence,
        }
    }

    /// Returns the process-local observation id.
    #[must_use]
    pub const fn observation_id(&self) -> &ObservationId {
        &self.observation_id
    }

    /// Returns supplied built-in status.
    #[must_use]
    pub const fn builtin_status(&self) -> DisplayBuiltinStatus {
        self.builtin_status
    }

    /// Returns supplied correlation evidence.
    #[must_use]
    pub const fn evidence(&self) -> &DisplayEvidence {
        &self.evidence
    }
}

/// Supplies optional platform evidence without allocating canonical ids.
pub trait DisplayMetadataProvider {
    /// Returns metadata for one monitor in the current probe.
    fn metadata(
        &mut self,
        ordinal: usize,
        facts: &PhysicalMonitorFacts,
    ) -> DisplayObservationMetadata;
}

impl<F> DisplayMetadataProvider for F
where
    F: FnMut(usize, &PhysicalMonitorFacts) -> DisplayObservationMetadata,
{
    fn metadata(
        &mut self,
        ordinal: usize,
        facts: &PhysicalMonitorFacts,
    ) -> DisplayObservationMetadata {
        self(ordinal, facts)
    }
}

/// Default metadata: process-local ordinal ids, unknown built-in status, no evidence.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultDisplayMetadata;

impl DisplayMetadataProvider for DefaultDisplayMetadata {
    fn metadata(
        &mut self,
        ordinal: usize,
        _facts: &PhysicalMonitorFacts,
    ) -> DisplayObservationMetadata {
        let observation_id = ObservationId::new(format!("tauri-probe:{ordinal}"))
            .expect("generated observation id is valid");
        DisplayObservationMetadata::new(
            observation_id,
            DisplayBuiltinStatus::Unknown,
            DisplayEvidence::new(),
        )
    }
}

/// Raw physical display observation with caller-supplied evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PhysicalDisplayObservation {
    facts: PhysicalMonitorFacts,
    metadata: DisplayObservationMetadata,
}

impl PhysicalDisplayObservation {
    /// Attaches observation metadata to raw monitor facts.
    #[must_use]
    pub const fn new(facts: PhysicalMonitorFacts, metadata: DisplayObservationMetadata) -> Self {
        Self { facts, metadata }
    }

    /// Returns raw monitor facts.
    #[must_use]
    pub const fn facts(&self) -> &PhysicalMonitorFacts {
        &self.facts
    }

    /// Returns observation metadata.
    #[must_use]
    pub const fn metadata(&self) -> &DisplayObservationMetadata {
        &self.metadata
    }
}
