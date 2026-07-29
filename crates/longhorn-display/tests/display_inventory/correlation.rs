use longhorn_core::DisplayId;
use longhorn_display::{
    AssociationEvidence, AssociationKind, CorrelationConfidence, DisplayAvailability,
    DisplayEvidence, KnownDisplayRegistry, ObservationId, UnresolvedReason, reconcile_displays,
};

use super::support::*;

#[test]
fn macos_strong_key_wins_over_adapter_geometry_and_weak_evidence() {
    let mac_uuid = strong(
        "macos.core-graphics",
        "2FA47A6D-3D08-4E40-9D9D-528C93A26596",
    );
    let strong_target = known(
        "display:strong",
        facts("Old built-in", 0, 1728, 1117, 2000, true),
        DisplayEvidence::new()
            .with_strong_key(mac_uuid.clone())
            .with_weak_key(weak("model", "built-in")),
    );
    let lower_tier_target = known(
        "display:lower",
        facts("External", 1728, 1920, 1080, 1000, false),
        DisplayEvidence::new()
            .with_adapter_key(adapter("tauri", "monitor-7"))
            .with_weak_key(weak("model", "external")),
    );
    let observation = observed(
        "probe:external",
        lower_tier_target.facts().clone(),
        DisplayEvidence::new()
            .with_strong_key(mac_uuid)
            .with_adapter_key(adapter("tauri", "monitor-7"))
            .with_weak_key(weak("model", "external")),
    );
    let mut allocator = unavailable_allocator();

    let result = reconcile_displays(
        &registry([strong_target, lower_tier_target]),
        [observation],
        &mut allocator,
    )
    .unwrap();

    assert_eq!(result.matches()[0].display_id().as_str(), "display:strong");
    assert_eq!(
        result.matches()[0].association(),
        &AssociationKind::Correlated {
            confidence: CorrelationConfidence::Strong,
            evidence: AssociationEvidence::StrongKeys(vec![strong(
                "macos.core-graphics",
                "2FA47A6D-3D08-4E40-9D9D-528C93A26596",
            )]),
        }
    );
    assert!(allocator.calls.is_empty());
}

#[test]
fn remembered_adapter_key_wins_over_exact_geometry() {
    let remembered = known(
        "display:remembered",
        facts("Desk", 0, 2560, 1440, 2000, false),
        DisplayEvidence::new().with_adapter_key(adapter("tauri", "monitor-42")),
    );
    let geometry_target = known(
        "display:geometry",
        facts("Projector", 2560, 1920, 1080, 1000, false),
        DisplayEvidence::new(),
    );
    let observation = observed(
        "probe:1",
        geometry_target.facts().clone(),
        DisplayEvidence::new().with_adapter_key(adapter("tauri", "monitor-42")),
    );
    let mut allocator = unavailable_allocator();

    let result = reconcile_displays(
        &registry([remembered, geometry_target]),
        [observation],
        &mut allocator,
    )
    .unwrap();

    assert_eq!(
        result.matches()[0].display_id().as_str(),
        "display:remembered"
    );
    assert!(matches!(
        result.matches()[0].association(),
        AssociationKind::Correlated {
            confidence: CorrelationConfidence::RememberedAdapter,
            ..
        }
    ));
}

#[test]
fn loophole_geometry_and_rearranged_weak_cases_report_their_tiers() {
    let geometry = known(
        "display:geometry",
        facts("Studio", -1920, 1920, 1080, 1500, false),
        DisplayEvidence::new(),
    );
    let rearranged = known(
        "display:rearranged",
        facts("Side", 0, 2560, 1440, 2000, false),
        DisplayEvidence::new().with_weak_key(weak("panel-model", "u2723qe")),
    );
    let geometry_observation = observed(
        "probe:geometry",
        geometry.facts().clone(),
        DisplayEvidence::new(),
    );
    let rearranged_observation = observed(
        "probe:rearranged",
        facts("Side", 2560, 2560, 1440, 2000, false),
        DisplayEvidence::new().with_weak_key(weak("panel-model", "u2723qe")),
    );
    let mut allocator = unavailable_allocator();

    let result = reconcile_displays(
        &registry([geometry, rearranged]),
        [rearranged_observation, geometry_observation],
        &mut allocator,
    )
    .unwrap();

    assert!(result.matches().iter().any(|association| {
        association.display_id().as_str() == "display:geometry"
            && matches!(
                association.association(),
                AssociationKind::Correlated {
                    confidence: CorrelationConfidence::ExactGeometryAndScale,
                    ..
                }
            )
    }));
    assert!(result.matches().iter().any(|association| {
        association.display_id().as_str() == "display:rearranged"
            && matches!(
                association.association(),
                AssociationKind::Correlated {
                    confidence: CorrelationConfidence::Weak,
                    ..
                }
            )
    }));
}

#[test]
fn nucleus_synthetic_identity_is_weak_and_ambiguous_duplicates_do_not_bind() {
    let nucleus_key = weak("nucleus.synthetic", "dell@0,0:1920x1080");
    let first = known(
        "display:first",
        facts("Dell", 0, 1920, 1080, 1000, false),
        DisplayEvidence::new().with_weak_key(nucleus_key.clone()),
    );
    let second = known(
        "display:second",
        facts("Dell", 1920, 1920, 1080, 1000, false),
        DisplayEvidence::new().with_weak_key(nucleus_key.clone()),
    );
    let observation = observed(
        "probe:duplicate",
        facts("Dell", 4000, 1920, 1080, 1000, false),
        DisplayEvidence::new().with_weak_key(nucleus_key),
    );
    let original = registry([first, second]);
    let mut allocator = unavailable_allocator();

    let result = reconcile_displays(&original, [observation], &mut allocator).unwrap();

    assert!(result.matches().is_empty());
    assert_eq!(result.registry(), &original);
    assert_eq!(result.ambiguities().len(), 1);
    assert_eq!(
        result.ambiguities()[0].confidence(),
        CorrelationConfidence::Weak
    );
    assert_eq!(
        result.ambiguities()[0].evidence(),
        &AssociationEvidence::WeakKeys(vec![weak("nucleus.synthetic", "dell@0,0:1920x1080",)])
    );
    assert_eq!(
        result.inventory().unresolved_observations()[0]
            .candidate_display_ids()
            .iter()
            .map(DisplayId::as_str)
            .collect::<Vec<_>>(),
        vec!["display:first", "display:second"]
    );
    assert!(allocator.calls.is_empty());
}

#[test]
fn duplicate_weak_fingerprints_are_permutation_invariant() {
    let duplicate = weak("panel-model", "identical");
    let known_displays = registry([
        known(
            "display:a",
            facts("A", 0, 1920, 1080, 1000, false),
            DisplayEvidence::new().with_weak_key(duplicate.clone()),
        ),
        known(
            "display:b",
            facts("B", 1920, 1920, 1080, 1000, false),
            DisplayEvidence::new().with_weak_key(duplicate.clone()),
        ),
    ]);
    let a = observed(
        "probe:a",
        facts("A", 5000, 1920, 1080, 1000, false),
        DisplayEvidence::new().with_weak_key(duplicate.clone()),
    );
    let b = observed(
        "probe:b",
        facts("B", 7000, 1920, 1080, 1000, false),
        DisplayEvidence::new().with_weak_key(duplicate),
    );
    let mut first_allocator = unavailable_allocator();
    let mut second_allocator = unavailable_allocator();

    let first = reconcile_displays(
        &known_displays,
        [a.clone(), b.clone()],
        &mut first_allocator,
    )
    .unwrap();
    let second = reconcile_displays(&known_displays, [b, a], &mut second_allocator).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.inventory().unresolved_observations().len(), 2);
    assert_eq!(first.registry(), &known_displays);
}

#[test]
fn soundcheck_single_display_is_allocated_once_then_retained_by_evidence() {
    let mac_uuid = strong(
        "macos.core-graphics",
        "F7E11B5C-7606-4A4A-8FA0-DA43B859BDE6",
    );
    let observation = observed(
        "probe:new",
        facts("Built-in", 0, 1728, 1117, 2000, true),
        DisplayEvidence::new()
            .with_strong_key(mac_uuid)
            .with_adapter_key(adapter("tauri", "4294967295")),
    );
    let mut first_allocator = QueueAllocator::new(["display:allocated-1"]);

    let first = reconcile_displays(
        &KnownDisplayRegistry::new(),
        [observation.clone()],
        &mut first_allocator,
    )
    .unwrap();

    assert_eq!(
        first_allocator.calls,
        vec![ObservationId::new("probe:new").unwrap()]
    );
    assert_eq!(
        first.registry().iter().next().unwrap().id().as_str(),
        "display:allocated-1"
    );
    assert_ne!(
        first.registry().iter().next().unwrap().id().as_str(),
        observation
            .evidence()
            .strong_keys()
            .first()
            .unwrap()
            .value()
    );

    let mut second_allocator = unavailable_allocator();
    let second =
        reconcile_displays(first.registry(), [observation], &mut second_allocator).unwrap();

    assert!(second_allocator.calls.is_empty());
    assert_eq!(
        second.registry().iter().next().unwrap().id().as_str(),
        "display:allocated-1"
    );
    assert!(matches!(
        second.matches()[0].association(),
        AssociationKind::Correlated {
            confidence: CorrelationConfidence::Strong,
            ..
        }
    ));
}

#[test]
fn loophole_geometry_without_matching_scale_does_not_silently_bind() {
    let known_displays = registry([known(
        "display:known",
        facts("Desk", 0, 2560, 1440, 2000, true),
        DisplayEvidence::new(),
    )]);
    let observation = observed(
        "probe:scale-changed",
        facts("Desk", 0, 2560, 1440, 1500, true),
        DisplayEvidence::new(),
    );
    let mut allocator = QueueAllocator::new(["display:new"]);

    let result = reconcile_displays(&known_displays, [observation], &mut allocator).unwrap();

    assert_eq!(allocator.calls.len(), 1);
    assert_eq!(result.registry().len(), 2);
    assert!(matches!(
        result.inventory().displays()[0].availability(),
        DisplayAvailability::Missing
    ));
    assert!(matches!(
        result.inventory().displays()[1].availability(),
        DisplayAvailability::Available {
            association: AssociationKind::Allocated,
            ..
        }
    ));
}

#[test]
fn allocation_order_uses_observation_content_not_enumeration() {
    let left = observed(
        "probe:left",
        facts("Left", -1920, 1920, 1080, 1000, false),
        DisplayEvidence::new(),
    );
    let right = observed(
        "probe:right",
        facts("Right", 0, 2560, 1440, 2000, true),
        DisplayEvidence::new(),
    );
    let mut first_allocator = QueueAllocator::new(["display:1", "display:2"]);
    let mut second_allocator = QueueAllocator::new(["display:1", "display:2"]);

    let first = reconcile_displays(
        &KnownDisplayRegistry::new(),
        [left.clone(), right.clone()],
        &mut first_allocator,
    )
    .unwrap();
    let second = reconcile_displays(
        &KnownDisplayRegistry::new(),
        [right, left],
        &mut second_allocator,
    )
    .unwrap();

    assert_eq!(first, second);
    assert_eq!(first_allocator.calls, second_allocator.calls);
}

#[test]
fn indistinguishable_new_observations_remain_unresolved_without_allocation() {
    let duplicate_facts = facts("Identical", 0, 1920, 1080, 1000, false);
    let first = observed("probe:1", duplicate_facts.clone(), DisplayEvidence::new());
    let second = observed("probe:2", duplicate_facts, DisplayEvidence::new());
    let mut allocator = unavailable_allocator();

    let result = reconcile_displays(
        &KnownDisplayRegistry::new(),
        [first, second],
        &mut allocator,
    )
    .unwrap();

    assert!(result.registry().is_empty());
    assert!(allocator.calls.is_empty());
    assert!(
        result
            .inventory()
            .unresolved_observations()
            .iter()
            .all(|observation| matches!(
                observation.reason(),
                UnresolvedReason::IndistinguishableNewObservations
            ))
    );
}
