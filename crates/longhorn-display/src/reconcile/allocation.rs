use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::{DisplayId, ScreenRect};

use crate::{
    AdapterDisplayKey, AssociationKind, CorrelationMatch, DisplayAvailability, DisplayIdAllocator,
    KnownDisplay, KnownDisplayRegistry, ObservationId, ObservedDisplay, ReconcileError,
    StrongDisplayKey, UnresolvedObservation, UnresolvedReason, WeakDisplayKey,
};

#[derive(Eq, Ord, PartialEq, PartialOrd)]
struct AllocationKey {
    strong: Vec<StrongDisplayKey>,
    adapter: Vec<AdapterDisplayKey>,
    weak: Vec<WeakDisplayKey>,
    machine_label: Option<String>,
    is_main: bool,
    builtin_status: crate::DisplayBuiltinStatus,
    full: (i32, i32, u32, u32),
    work: (i32, i32, u32, u32),
    scale: u32,
}

impl AllocationKey {
    fn from_observation(observation: &ObservedDisplay) -> Self {
        Self {
            strong: observation
                .evidence()
                .strong_keys()
                .iter()
                .cloned()
                .collect(),
            adapter: observation
                .evidence()
                .adapter_keys()
                .iter()
                .cloned()
                .collect(),
            weak: observation.evidence().weak_keys().iter().cloned().collect(),
            machine_label: observation
                .facts()
                .machine_label()
                .map(|label| label.as_str().to_string()),
            is_main: observation.facts().is_main(),
            builtin_status: observation.facts().builtin_status(),
            full: rect_key(observation.facts().full_bounds()),
            work: rect_key(observation.facts().work_area()),
            scale: observation.facts().scale().thousandths(),
        }
    }
}

fn rect_key(rect: ScreenRect) -> (i32, i32, u32, u32) {
    (
        rect.origin().x().get(),
        rect.origin().y().get(),
        rect.size().width(),
        rect.size().height(),
    )
}

pub(super) fn allocate_new_displays<A: DisplayIdAllocator>(
    registry: &mut KnownDisplayRegistry,
    observations: &BTreeMap<ObservationId, ObservedDisplay>,
    remaining: &BTreeSet<ObservationId>,
    allocator: &mut A,
    availability: &mut BTreeMap<DisplayId, DisplayAvailability>,
    matches: &mut Vec<CorrelationMatch>,
    unresolved: &mut BTreeMap<ObservationId, UnresolvedObservation>,
) -> Result<(), ReconcileError<A::Error>> {
    let mut groups = BTreeMap::<AllocationKey, Vec<ObservationId>>::new();
    for observation_id in remaining {
        let observation = observations
            .get(observation_id)
            .expect("remaining observation must exist");
        groups
            .entry(AllocationKey::from_observation(observation))
            .or_default()
            .push(observation_id.clone());
    }

    for observation_ids in groups.into_values() {
        if observation_ids.len() > 1 {
            for observation_id in observation_ids {
                unresolved.insert(
                    observation_id.clone(),
                    UnresolvedObservation::new(
                        observations
                            .get(&observation_id)
                            .expect("indistinguishable observation must exist")
                            .clone(),
                        Vec::new(),
                        UnresolvedReason::IndistinguishableNewObservations,
                    ),
                );
            }
            continue;
        }

        let observation_id = observation_ids
            .into_iter()
            .next()
            .expect("allocation group cannot be empty");
        let observation = observations
            .get(&observation_id)
            .expect("allocatable observation must exist");
        let display_id = allocator
            .allocate(observation)
            .map_err(ReconcileError::Allocation)?;
        if registry.get(&display_id).is_some() {
            return Err(ReconcileError::DuplicateAllocatedDisplayId(display_id));
        }
        registry.insert(KnownDisplay::new(
            display_id.clone(),
            observation.facts().clone(),
            observation.evidence().clone(),
        ));
        availability.insert(
            display_id.clone(),
            DisplayAvailability::Available {
                observation_id: observation_id.clone(),
                association: AssociationKind::Allocated,
            },
        );
        matches.push(CorrelationMatch::new(
            display_id,
            observation_id,
            AssociationKind::Allocated,
        ));
    }
    Ok(())
}
