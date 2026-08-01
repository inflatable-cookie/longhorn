use std::{error::Error, fmt};

use longhorn_core::{OperationCatalogueRevision, OperationId, OperationPhaseId, OperationRevision};

use crate::{OperationAuthorityCursor, OperationPhaseLabel, OperationProgressSequence};

/// Finite non-negative completed and total units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OperationUnitProgress {
    completed: f64,
    total: f64,
}

impl Eq for OperationUnitProgress {}

impl OperationUnitProgress {
    /// Validates and constructs unit progress.
    pub fn new(completed: f64, total: f64) -> Result<Self, OperationProgressValueError> {
        if !completed.is_finite() || !total.is_finite() {
            return Err(OperationProgressValueError::NonFinite);
        }
        if completed < 0.0 {
            return Err(OperationProgressValueError::NegativeCompleted);
        }
        if total <= 0.0 {
            return Err(OperationProgressValueError::NonPositiveTotal);
        }
        if completed > total {
            return Err(OperationProgressValueError::CompletedExceedsTotal);
        }
        Ok(Self {
            completed: normalize_zero(completed),
            total,
        })
    }

    /// Returns completed units.
    #[must_use]
    pub const fn completed(self) -> f64 {
        self.completed
    }

    /// Returns declared total units.
    #[must_use]
    pub const fn total(self) -> f64 {
        self.total
    }

    pub(crate) fn fraction(self) -> f64 {
        self.completed / self.total
    }
}

/// Finite normalized progress in the inclusive range zero through one.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct OperationNormalizedProgress(f64);

impl Eq for OperationNormalizedProgress {}

impl OperationNormalizedProgress {
    /// Validates and constructs normalized progress.
    pub fn new(value: f64) -> Result<Self, OperationProgressValueError> {
        if !value.is_finite() {
            return Err(OperationProgressValueError::NonFinite);
        }
        if !(0.0..=1.0).contains(&value) {
            return Err(OperationProgressValueError::NormalizedOutOfRange);
        }
        Ok(Self(normalize_zero(value)))
    }

    /// Returns the normalized value.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Product-neutral overall progress projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationOverallProgress {
    /// Work is active but has no determinate overall measurement.
    Indeterminate,
    /// Completed and total consumer-owned units.
    Units(OperationUnitProgress),
    /// Normalized overall completion.
    Normalized(OperationNormalizedProgress),
}

impl OperationOverallProgress {
    pub(crate) fn fraction(self) -> Option<f64> {
        match self {
            Self::Indeterminate => None,
            Self::Units(progress) => Some(progress.fraction()),
            Self::Normalized(progress) => Some(progress.get()),
        }
    }
}

/// Current phase-local progress. A changed phase id admits a local reset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationPhaseProgress {
    phase_id: OperationPhaseId,
    label: OperationPhaseLabel,
    units: OperationUnitProgress,
}

impl OperationPhaseProgress {
    /// Constructs validated phase progress.
    #[must_use]
    pub const fn new(
        phase_id: OperationPhaseId,
        label: OperationPhaseLabel,
        units: OperationUnitProgress,
    ) -> Self {
        Self {
            phase_id,
            label,
            units,
        }
    }

    /// Returns the stable phase identity.
    #[must_use]
    pub const fn phase_id(&self) -> &OperationPhaseId {
        &self.phase_id
    }

    /// Returns the bounded phase label.
    #[must_use]
    pub const fn label(&self) -> &OperationPhaseLabel {
        &self.label
    }

    /// Returns phase-local units.
    #[must_use]
    pub const fn units(&self) -> OperationUnitProgress {
        self.units
    }
}

/// Authoritative current progress for one operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationProgress {
    sequence: OperationProgressSequence,
    overall: OperationOverallProgress,
    phase: Option<OperationPhaseProgress>,
}

impl OperationProgress {
    pub(crate) const INITIAL: Self = Self {
        sequence: OperationProgressSequence::INITIAL,
        overall: OperationOverallProgress::Indeterminate,
        phase: None,
    };

    /// Returns the monotonic progress sequence.
    #[must_use]
    pub const fn sequence(&self) -> OperationProgressSequence {
        self.sequence
    }

    /// Returns overall progress.
    #[must_use]
    pub const fn overall(&self) -> OperationOverallProgress {
        self.overall
    }

    /// Returns current phase-local progress when reported.
    #[must_use]
    pub const fn phase(&self) -> Option<&OperationPhaseProgress> {
        self.phase.as_ref()
    }

    pub(crate) fn commit(
        &mut self,
        sequence: OperationProgressSequence,
        overall: OperationOverallProgress,
        phase: Option<OperationPhaseProgress>,
    ) {
        self.sequence = sequence;
        self.overall = overall;
        if let Some(phase) = phase {
            self.phase = Some(phase);
        }
    }
}

/// Revision-bound progress update supplied by the consumer executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationProgressUpdate {
    pub(crate) authority: OperationAuthorityCursor,
    pub(crate) operation_id: OperationId,
    pub(crate) expected_operation_revision: OperationRevision,
    pub(crate) overall: OperationOverallProgress,
    pub(crate) phase: Option<OperationPhaseProgress>,
}

impl OperationProgressUpdate {
    /// Constructs a progress update. Omitted phase evidence preserves the current phase.
    #[must_use]
    pub const fn new(
        authority: OperationAuthorityCursor,
        operation_id: OperationId,
        expected_operation_revision: OperationRevision,
        overall: OperationOverallProgress,
        phase: Option<OperationPhaseProgress>,
    ) -> Self {
        Self {
            authority,
            operation_id,
            expected_operation_revision,
            overall,
            phase,
        }
    }
}

/// Receipt for one committed progress update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationProgressReceipt {
    pub(crate) operation_id: OperationId,
    pub(crate) previous_operation_revision: OperationRevision,
    pub(crate) committed_operation_revision: OperationRevision,
    pub(crate) previous_sequence: OperationProgressSequence,
    pub(crate) committed_progress: OperationProgress,
    pub(crate) previous_catalogue_revision: OperationCatalogueRevision,
    pub(crate) committed_catalogue_revision: OperationCatalogueRevision,
}

impl OperationProgressReceipt {
    /// Returns the changed operation identity.
    #[must_use]
    pub const fn operation_id(&self) -> &OperationId {
        &self.operation_id
    }

    /// Returns the operation revision before the update.
    #[must_use]
    pub const fn previous_operation_revision(&self) -> OperationRevision {
        self.previous_operation_revision
    }

    /// Returns the committed operation revision.
    #[must_use]
    pub const fn committed_operation_revision(&self) -> OperationRevision {
        self.committed_operation_revision
    }

    /// Returns the progress sequence before the update.
    #[must_use]
    pub const fn previous_sequence(&self) -> OperationProgressSequence {
        self.previous_sequence
    }

    /// Returns committed progress.
    #[must_use]
    pub const fn committed_progress(&self) -> &OperationProgress {
        &self.committed_progress
    }

    /// Returns the catalogue revision before the update.
    #[must_use]
    pub const fn previous_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.previous_catalogue_revision
    }

    /// Returns the committed catalogue revision.
    #[must_use]
    pub const fn committed_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.committed_catalogue_revision
    }
}

/// Invalid finite progress value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationProgressValueError {
    /// One supplied value was NaN or infinite.
    NonFinite,
    /// Completed units were negative.
    NegativeCompleted,
    /// Total units were zero or negative.
    NonPositiveTotal,
    /// Completed units exceeded the declared total.
    CompletedExceedsTotal,
    /// Normalized progress was outside zero through one.
    NormalizedOutOfRange,
}

impl fmt::Display for OperationProgressValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => formatter.write_str("operation progress must be finite"),
            Self::NegativeCompleted => {
                formatter.write_str("operation completed units cannot be negative")
            }
            Self::NonPositiveTotal => formatter.write_str("operation total units must be positive"),
            Self::CompletedExceedsTotal => {
                formatter.write_str("operation completed units cannot exceed total units")
            }
            Self::NormalizedOutOfRange => {
                formatter.write_str("normalized operation progress must be between zero and one")
            }
        }
    }
}

impl Error for OperationProgressValueError {}

const fn normalize_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}
