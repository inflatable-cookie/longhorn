use std::{error::Error, fmt};

use longhorn_core::DisplayId;
use serde::{Deserialize, Serialize};

use crate::{
    AdapterDisplayKey, ArrangementSignature, KnownDisplay, KnownDisplayRegistry, ObservationId,
    ObservedDisplay, StrongDisplayKey, WeakDisplayKey,
};

/// Strength assigned to a successful or ambiguous correlation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrelationConfidence {
    /// Shared platform or hardware evidence.
    Strong,
    /// Shared adapter evidence remembered from an earlier correlation.
    RememberedAdapter,
    /// Exact full bounds and scale.
    ExactGeometryAndScale,
    /// Unique shared weak fingerprint.
    Weak,
}

/// Exact evidence behind an association.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "keys")]
pub enum AssociationEvidence {
    /// Shared strong evidence.
    StrongKeys(Vec<StrongDisplayKey>),
    /// Shared remembered adapter evidence.
    AdapterKeys(Vec<AdapterDisplayKey>),
    /// Exact full bounds and scale.
    ExactGeometryAndScale,
    /// Shared unique weak evidence.
    WeakKeys(Vec<WeakDisplayKey>),
}

/// How an available observation obtained canonical identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AssociationKind {
    /// Correlated with an existing known display.
    Correlated {
        /// Explicit correlation strength.
        confidence: CorrelationConfidence,
        /// Exact evidence used by the winning tier.
        evidence: AssociationEvidence,
    },
    /// Received a new id from the injected allocator.
    Allocated,
}

/// One successful known-to-observed association.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CorrelationMatch {
    display_id: DisplayId,
    observation_id: ObservationId,
    association: AssociationKind,
}

impl CorrelationMatch {
    pub(crate) const fn new(
        display_id: DisplayId,
        observation_id: ObservationId,
        association: AssociationKind,
    ) -> Self {
        Self {
            display_id,
            observation_id,
            association,
        }
    }

    /// Returns canonical display identity.
    #[must_use]
    pub const fn display_id(&self) -> &DisplayId {
        &self.display_id
    }

    /// Returns ephemeral observation identity.
    #[must_use]
    pub const fn observation_id(&self) -> &ObservationId {
        &self.observation_id
    }

    /// Returns association evidence.
    #[must_use]
    pub const fn association(&self) -> &AssociationKind {
        &self.association
    }
}

/// Current availability for one known display.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum DisplayAvailability {
    /// Correlated or newly allocated in this observation cycle.
    Available {
        /// Current host observation.
        observation_id: ObservationId,
        /// Identity association source.
        association: AssociationKind,
    },
    /// Not observed and not implicated in ambiguity.
    Missing,
    /// Candidate for one or more unresolved observations.
    Unresolved {
        /// Candidate observations in canonical order.
        observation_ids: Vec<ObservationId>,
        /// Highest evidence tier that could not resolve one-to-one.
        confidence: CorrelationConfidence,
    },
}

/// Known display plus current availability.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InventoryDisplay {
    display: KnownDisplay,
    availability: DisplayAvailability,
}

impl InventoryDisplay {
    pub(crate) const fn new(display: KnownDisplay, availability: DisplayAvailability) -> Self {
        Self {
            display,
            availability,
        }
    }

    /// Returns retained display data.
    #[must_use]
    pub const fn display(&self) -> &KnownDisplay {
        &self.display
    }

    /// Returns current availability.
    #[must_use]
    pub const fn availability(&self) -> &DisplayAvailability {
        &self.availability
    }
}

/// Why a current observation remains unresolved.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum UnresolvedReason {
    /// Multiple one-to-one assignments exist at the named evidence tier.
    AmbiguousCorrelation {
        /// Highest ambiguous evidence strength.
        confidence: CorrelationConfidence,
        /// Exact evidence shared with the candidate set.
        evidence: AssociationEvidence,
    },
    /// Multiple new observations are indistinguishable without enumeration.
    IndistinguishableNewObservations,
}

/// One unresolved current observation and its candidate known displays.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnresolvedObservation {
    observation: ObservedDisplay,
    candidate_display_ids: Vec<DisplayId>,
    reason: UnresolvedReason,
}

impl UnresolvedObservation {
    pub(crate) const fn new(
        observation: ObservedDisplay,
        candidate_display_ids: Vec<DisplayId>,
        reason: UnresolvedReason,
    ) -> Self {
        Self {
            observation,
            candidate_display_ids,
            reason,
        }
    }

    /// Returns the unresolved observation.
    #[must_use]
    pub const fn observation(&self) -> &ObservedDisplay {
        &self.observation
    }

    /// Returns candidate known ids in canonical order.
    #[must_use]
    pub fn candidate_display_ids(&self) -> &[DisplayId] {
        self.candidate_display_ids.as_slice()
    }

    /// Returns the unresolved reason.
    #[must_use]
    pub const fn reason(&self) -> &UnresolvedReason {
        &self.reason
    }
}

/// Ambiguity summary for one evidence tier.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CorrelationAmbiguity {
    confidence: CorrelationConfidence,
    evidence: AssociationEvidence,
    observation_ids: Vec<ObservationId>,
    candidate_display_ids: Vec<DisplayId>,
}

impl CorrelationAmbiguity {
    pub(crate) const fn new(
        confidence: CorrelationConfidence,
        evidence: AssociationEvidence,
        observation_ids: Vec<ObservationId>,
        candidate_display_ids: Vec<DisplayId>,
    ) -> Self {
        Self {
            confidence,
            evidence,
            observation_ids,
            candidate_display_ids,
        }
    }

    /// Returns the unresolved evidence tier.
    #[must_use]
    pub const fn confidence(&self) -> CorrelationConfidence {
        self.confidence
    }

    /// Returns the exact ambiguous evidence.
    #[must_use]
    pub const fn evidence(&self) -> &AssociationEvidence {
        &self.evidence
    }

    /// Returns implicated observations.
    #[must_use]
    pub fn observation_ids(&self) -> &[ObservationId] {
        self.observation_ids.as_slice()
    }

    /// Returns candidate known displays.
    #[must_use]
    pub fn candidate_display_ids(&self) -> &[DisplayId] {
        self.candidate_display_ids.as_slice()
    }
}

/// Current inventory built from persistent known displays and observations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DisplayInventory {
    displays: Vec<InventoryDisplay>,
    unresolved_observations: Vec<UnresolvedObservation>,
    arrangement_signature: ArrangementSignature,
}

impl DisplayInventory {
    pub(crate) const fn new(
        displays: Vec<InventoryDisplay>,
        unresolved_observations: Vec<UnresolvedObservation>,
        arrangement_signature: ArrangementSignature,
    ) -> Self {
        Self {
            displays,
            unresolved_observations,
            arrangement_signature,
        }
    }

    /// Returns known displays in canonical-id order.
    #[must_use]
    pub fn displays(&self) -> &[InventoryDisplay] {
        self.displays.as_slice()
    }

    /// Returns unresolved observations in observation-id order.
    #[must_use]
    pub fn unresolved_observations(&self) -> &[UnresolvedObservation] {
        self.unresolved_observations.as_slice()
    }

    /// Returns the versioned deterministic arrangement signature.
    #[must_use]
    pub const fn arrangement_signature(&self) -> &ArrangementSignature {
        &self.arrangement_signature
    }
}

/// Complete pure reconciliation result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reconciliation {
    registry: KnownDisplayRegistry,
    inventory: DisplayInventory,
    matches: Vec<CorrelationMatch>,
    ambiguities: Vec<CorrelationAmbiguity>,
}

impl Reconciliation {
    pub(crate) const fn new(
        registry: KnownDisplayRegistry,
        inventory: DisplayInventory,
        matches: Vec<CorrelationMatch>,
        ambiguities: Vec<CorrelationAmbiguity>,
    ) -> Self {
        Self {
            registry,
            inventory,
            matches,
            ambiguities,
        }
    }

    /// Returns the updated persistent registry.
    #[must_use]
    pub const fn registry(&self) -> &KnownDisplayRegistry {
        &self.registry
    }

    /// Returns the current inventory.
    #[must_use]
    pub const fn inventory(&self) -> &DisplayInventory {
        &self.inventory
    }

    /// Returns successful associations in canonical-id order.
    #[must_use]
    pub fn matches(&self) -> &[CorrelationMatch] {
        self.matches.as_slice()
    }

    /// Returns ambiguity summaries in evidence-precedence order.
    #[must_use]
    pub fn ambiguities(&self) -> &[CorrelationAmbiguity] {
        self.ambiguities.as_slice()
    }
}

/// Injected authority for new machine-local display identity.
pub trait DisplayIdAllocator {
    /// Allocator-specific failure.
    type Error;

    /// Allocates identity after all correlation tiers have failed.
    fn allocate(&mut self, observation: &ObservedDisplay) -> Result<DisplayId, Self::Error>;
}

/// Pure reconciliation failure.
#[derive(Debug)]
pub enum ReconcileError<AllocatorError> {
    /// Current observations reused an ephemeral observation id.
    DuplicateObservationId(ObservationId),
    /// The allocator returned an id already present in the registry or batch.
    DuplicateAllocatedDisplayId(DisplayId),
    /// Injected allocation failed.
    Allocation(AllocatorError),
}

impl<AllocatorError: fmt::Display> fmt::Display for ReconcileError<AllocatorError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateObservationId(id) => {
                write!(formatter, "duplicate observation id {id}")
            }
            Self::DuplicateAllocatedDisplayId(id) => {
                write!(formatter, "allocator returned duplicate display id {id}")
            }
            Self::Allocation(error) => write!(formatter, "display id allocation failed: {error}"),
        }
    }
}

impl<AllocatorError: Error + 'static> Error for ReconcileError<AllocatorError> {}
