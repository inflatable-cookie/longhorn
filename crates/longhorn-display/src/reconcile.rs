use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::DisplayId;

mod allocation;

use crate::{
    AssociationEvidence, AssociationKind, CorrelationAmbiguity, CorrelationConfidence,
    CorrelationMatch, DisplayAvailability, DisplayEvidence, DisplayIdAllocator, DisplayInventory,
    InventoryDisplay, KnownDisplay, KnownDisplayRegistry, ObservationId, ObservedDisplay,
    ReconcileError, Reconciliation, UnresolvedObservation, UnresolvedReason,
    arrangement::build_signature,
};
use allocation::allocate_new_displays;

/// Reconciles persistent known displays with one current observation set.
///
/// Matching is one-to-one and order-independent. New identity comes only from
/// `allocator`, after every correlation tier has failed.
pub fn reconcile_displays<A: DisplayIdAllocator>(
    registry: &KnownDisplayRegistry,
    observations: impl IntoIterator<Item = ObservedDisplay>,
    allocator: &mut A,
) -> Result<Reconciliation, ReconcileError<A::Error>> {
    let observations = collect_observations(observations)?;
    let mut updated_registry = registry.clone();
    let mut remaining_known = registry
        .iter()
        .map(|display| display.id().clone())
        .collect::<BTreeSet<_>>();
    let mut remaining_observations = observations.keys().cloned().collect::<BTreeSet<_>>();
    let mut availability = BTreeMap::new();
    let mut matches = Vec::new();
    let mut ambiguities = Vec::new();
    let mut unresolved = BTreeMap::new();
    let mut unresolved_known =
        BTreeMap::<DisplayId, (CorrelationConfidence, BTreeSet<ObservationId>)>::new();

    for tier in Tier::ALL {
        let result = resolve_tier(
            tier,
            registry,
            &observations,
            &remaining_known,
            &remaining_observations,
        );

        for plan in result.matches {
            remaining_known.remove(&plan.display_id);
            remaining_observations.remove(&plan.observation_id);
            let observation = observations
                .get(&plan.observation_id)
                .expect("planned observation must exist");
            updated_registry
                .get_mut(&plan.display_id)
                .expect("planned known display must exist")
                .observe(observation);
            availability.insert(
                plan.display_id.clone(),
                DisplayAvailability::Available {
                    observation_id: plan.observation_id.clone(),
                    association: plan.association.clone(),
                },
            );
            matches.push(CorrelationMatch::new(
                plan.display_id,
                plan.observation_id,
                plan.association,
            ));
        }

        for ambiguity in result.ambiguities {
            for observation_id in &ambiguity.observation_ids {
                remaining_observations.remove(observation_id);
                let observation = observations
                    .get(observation_id)
                    .expect("ambiguous observation must exist")
                    .clone();
                unresolved.insert(
                    observation_id.clone(),
                    UnresolvedObservation::new(
                        observation,
                        ambiguity.candidate_display_ids.clone(),
                        UnresolvedReason::AmbiguousCorrelation {
                            confidence: ambiguity.confidence,
                            evidence: ambiguity.evidence.clone(),
                        },
                    ),
                );
            }
            for display_id in &ambiguity.candidate_display_ids {
                remaining_known.remove(display_id);
                let entry = unresolved_known
                    .entry(display_id.clone())
                    .or_insert_with(|| (ambiguity.confidence, BTreeSet::new()));
                entry.1.extend(ambiguity.observation_ids.iter().cloned());
            }
            ambiguities.push(CorrelationAmbiguity::new(
                ambiguity.confidence,
                ambiguity.evidence,
                ambiguity.observation_ids,
                ambiguity.candidate_display_ids,
            ));
        }
    }

    allocate_new_displays(
        &mut updated_registry,
        &observations,
        &remaining_observations,
        allocator,
        &mut availability,
        &mut matches,
        &mut unresolved,
    )?;

    for (display_id, (confidence, observation_ids)) in unresolved_known {
        availability.insert(
            display_id,
            DisplayAvailability::Unresolved {
                observation_ids: observation_ids.into_iter().collect(),
                confidence,
            },
        );
    }

    let displays = updated_registry
        .iter()
        .map(|display| {
            let state = availability
                .remove(display.id())
                .unwrap_or(DisplayAvailability::Missing);
            InventoryDisplay::new(display.clone(), state)
        })
        .collect::<Vec<_>>();
    let signature = build_signature(displays.iter().filter_map(|display| {
        if matches!(
            display.availability(),
            DisplayAvailability::Available { .. }
        ) {
            Some((display.display().id(), display.display().facts()))
        } else {
            None
        }
    }));
    matches.sort_by(|left, right| {
        left.display_id()
            .cmp(right.display_id())
            .then_with(|| left.observation_id().cmp(right.observation_id()))
    });
    let inventory = DisplayInventory::new(displays, unresolved.into_values().collect(), signature);

    Ok(Reconciliation::new(
        updated_registry,
        inventory,
        matches,
        ambiguities,
    ))
}

fn collect_observations<A>(
    observations: impl IntoIterator<Item = ObservedDisplay>,
) -> Result<BTreeMap<ObservationId, ObservedDisplay>, ReconcileError<A>> {
    let mut collected = BTreeMap::new();
    for observation in observations {
        let id = observation.observation_id().clone();
        if collected.insert(id.clone(), observation).is_some() {
            return Err(ReconcileError::DuplicateObservationId(id));
        }
    }
    Ok(collected)
}

#[derive(Clone, Copy)]
enum Tier {
    Strong,
    RememberedAdapter,
    ExactGeometryAndScale,
    Weak,
}

impl Tier {
    const ALL: [Self; 4] = [
        Self::Strong,
        Self::RememberedAdapter,
        Self::ExactGeometryAndScale,
        Self::Weak,
    ];

    const fn confidence(self) -> CorrelationConfidence {
        match self {
            Self::Strong => CorrelationConfidence::Strong,
            Self::RememberedAdapter => CorrelationConfidence::RememberedAdapter,
            Self::ExactGeometryAndScale => CorrelationConfidence::ExactGeometryAndScale,
            Self::Weak => CorrelationConfidence::Weak,
        }
    }

    fn evidence(self, known: &DisplayEvidence, observed: &DisplayEvidence) -> AssociationEvidence {
        match self {
            Self::Strong => {
                AssociationEvidence::StrongKeys(common(known.strong_keys(), observed.strong_keys()))
            }
            Self::RememberedAdapter => AssociationEvidence::AdapterKeys(common(
                known.adapter_keys(),
                observed.adapter_keys(),
            )),
            Self::ExactGeometryAndScale => AssociationEvidence::ExactGeometryAndScale,
            Self::Weak => {
                AssociationEvidence::WeakKeys(common(known.weak_keys(), observed.weak_keys()))
            }
        }
    }
}

fn common<T: Clone + Ord>(left: &BTreeSet<T>, right: &BTreeSet<T>) -> Vec<T> {
    left.intersection(right).cloned().collect()
}

struct TierResult {
    matches: Vec<MatchPlan>,
    ambiguities: Vec<AmbiguityPlan>,
}

struct MatchPlan {
    display_id: DisplayId,
    observation_id: ObservationId,
    association: AssociationKind,
}

struct AmbiguityPlan {
    confidence: CorrelationConfidence,
    evidence: AssociationEvidence,
    observation_ids: Vec<ObservationId>,
    candidate_display_ids: Vec<DisplayId>,
}

fn resolve_tier(
    tier: Tier,
    registry: &KnownDisplayRegistry,
    observations: &BTreeMap<ObservationId, ObservedDisplay>,
    remaining_known: &BTreeSet<DisplayId>,
    remaining_observations: &BTreeSet<ObservationId>,
) -> TierResult {
    let mut by_observation = BTreeMap::<ObservationId, BTreeSet<DisplayId>>::new();
    let mut by_known = BTreeMap::<DisplayId, BTreeSet<ObservationId>>::new();

    for observation_id in remaining_observations {
        let observation = observations
            .get(observation_id)
            .expect("remaining observation must exist");
        for display_id in remaining_known {
            let known = registry
                .get(display_id)
                .expect("remaining known display must exist");
            if tier_matches(tier, known, observation) {
                by_observation
                    .entry(observation_id.clone())
                    .or_default()
                    .insert(display_id.clone());
                by_known
                    .entry(display_id.clone())
                    .or_default()
                    .insert(observation_id.clone());
            }
        }
    }

    let mut matched_observations = BTreeSet::new();
    let mut matches = Vec::new();
    for (observation_id, candidates) in &by_observation {
        let Some(display_id) = exactly_one(candidates) else {
            continue;
        };
        if by_known.get(display_id).and_then(exactly_one) != Some(observation_id) {
            continue;
        }
        let known = registry
            .get(display_id)
            .expect("candidate known must exist");
        let observed = observations
            .get(observation_id)
            .expect("candidate observation must exist");
        matches.push(MatchPlan {
            display_id: display_id.clone(),
            observation_id: observation_id.clone(),
            association: AssociationKind::Correlated {
                confidence: tier.confidence(),
                evidence: tier.evidence(known.evidence(), observed.evidence()),
            },
        });
        matched_observations.insert(observation_id.clone());
    }

    let ambiguities = by_observation
        .into_iter()
        .filter(|(observation_id, _)| !matched_observations.contains(observation_id))
        .map(|(observation_id, candidates)| {
            let observed = observations
                .get(&observation_id)
                .expect("ambiguous observation must exist");
            let evidence = ambiguity_evidence(tier, registry, observed, &candidates);
            AmbiguityPlan {
                confidence: tier.confidence(),
                evidence,
                observation_ids: vec![observation_id],
                candidate_display_ids: candidates.into_iter().collect(),
            }
        })
        .collect();

    TierResult {
        matches,
        ambiguities,
    }
}

fn ambiguity_evidence(
    tier: Tier,
    registry: &KnownDisplayRegistry,
    observed: &ObservedDisplay,
    candidates: &BTreeSet<DisplayId>,
) -> AssociationEvidence {
    match tier {
        Tier::Strong => AssociationEvidence::StrongKeys(
            candidates
                .iter()
                .flat_map(|id| {
                    common(
                        registry
                            .get(id)
                            .expect("candidate known must exist")
                            .evidence()
                            .strong_keys(),
                        observed.evidence().strong_keys(),
                    )
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        ),
        Tier::RememberedAdapter => AssociationEvidence::AdapterKeys(
            candidates
                .iter()
                .flat_map(|id| {
                    common(
                        registry
                            .get(id)
                            .expect("candidate known must exist")
                            .evidence()
                            .adapter_keys(),
                        observed.evidence().adapter_keys(),
                    )
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        ),
        Tier::ExactGeometryAndScale => AssociationEvidence::ExactGeometryAndScale,
        Tier::Weak => AssociationEvidence::WeakKeys(
            candidates
                .iter()
                .flat_map(|id| {
                    common(
                        registry
                            .get(id)
                            .expect("candidate known must exist")
                            .evidence()
                            .weak_keys(),
                        observed.evidence().weak_keys(),
                    )
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        ),
    }
}

fn exactly_one<T: Ord>(values: &BTreeSet<T>) -> Option<&T> {
    if values.len() == 1 {
        values.first()
    } else {
        None
    }
}

fn tier_matches(tier: Tier, known: &KnownDisplay, observed: &ObservedDisplay) -> bool {
    match tier {
        Tier::Strong => !known
            .evidence()
            .strong_keys()
            .is_disjoint(observed.evidence().strong_keys()),
        Tier::RememberedAdapter => !known
            .evidence()
            .adapter_keys()
            .is_disjoint(observed.evidence().adapter_keys()),
        Tier::ExactGeometryAndScale => {
            known.facts().full_bounds() == observed.facts().full_bounds()
                && known.facts().scale() == observed.facts().scale()
        }
        Tier::Weak => !known
            .evidence()
            .weak_keys()
            .is_disjoint(observed.evidence().weak_keys()),
    }
}
