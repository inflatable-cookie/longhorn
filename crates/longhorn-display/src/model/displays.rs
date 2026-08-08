use longhorn_core::DisplayId;
use serde::{Deserialize, Serialize};

use super::{
    evidence::DisplayEvidence,
    facts::DisplayFacts,
    ids::{DisplayLabel, ObservationId},
};

/// A display retained across observation cycles.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KnownDisplay {
    id: DisplayId,
    facts: DisplayFacts,
    user_label: Option<DisplayLabel>,
    evidence: DisplayEvidence,
}

impl KnownDisplay {
    /// Constructs a known display from allocated identity and observed facts.
    #[must_use]
    pub const fn new(id: DisplayId, facts: DisplayFacts, evidence: DisplayEvidence) -> Self {
        Self {
            id,
            facts,
            user_label: None,
            evidence,
        }
    }

    /// Returns canonical machine-local identity.
    #[must_use]
    pub const fn id(&self) -> &DisplayId {
        &self.id
    }

    /// Returns last-observed facts.
    #[must_use]
    pub const fn facts(&self) -> &DisplayFacts {
        &self.facts
    }

    /// Returns retained correlation evidence.
    #[must_use]
    pub const fn evidence(&self) -> &DisplayEvidence {
        &self.evidence
    }

    /// Returns the explicit user label.
    #[must_use]
    pub const fn user_label(&self) -> Option<&DisplayLabel> {
        self.user_label.as_ref()
    }

    /// Returns the user label when present, otherwise the machine label.
    #[must_use]
    pub fn effective_label(&self) -> Option<&DisplayLabel> {
        self.user_label
            .as_ref()
            .or_else(|| self.facts.machine_label())
    }

    /// Sets or clears the user label without erasing the machine label.
    pub fn set_user_label(&mut self, label: Option<DisplayLabel>) {
        self.user_label = label;
    }

    pub(crate) fn observe(&mut self, observation: &ObservedDisplay) {
        self.facts = observation.facts().clone();
        self.evidence.merge(observation.evidence());
    }
}

/// One current host display observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedDisplay {
    observation_id: ObservationId,
    facts: DisplayFacts,
    evidence: DisplayEvidence,
}

impl ObservedDisplay {
    /// Constructs a host observation without assigning canonical identity.
    #[must_use]
    pub const fn new(
        observation_id: ObservationId,
        facts: DisplayFacts,
        evidence: DisplayEvidence,
    ) -> Self {
        Self {
            observation_id,
            facts,
            evidence,
        }
    }

    /// Returns ephemeral observation identity.
    #[must_use]
    pub const fn observation_id(&self) -> &ObservationId {
        &self.observation_id
    }

    /// Returns observed facts.
    #[must_use]
    pub const fn facts(&self) -> &DisplayFacts {
        &self.facts
    }

    /// Returns observed correlation evidence.
    #[must_use]
    pub const fn evidence(&self) -> &DisplayEvidence {
        &self.evidence
    }
}
