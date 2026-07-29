use longhorn_core::{DisplayId, ScaleFactor};
use longhorn_display::{
    ArrangementSignature, DisplayAvailability, DisplayBuiltinStatus, DisplayEvidence, DisplayFacts,
    DisplayLabel, KnownDisplayRegistry, ObservedDisplay, ReconcileError, reconcile_displays,
};

use super::support::*;

#[test]
fn missing_and_reappearing_display_preserves_identity_labels_and_evidence() {
    let old_adapter = adapter("tauri", "monitor-old");
    let mut retained = known(
        "display:desk",
        facts("DELL U2723QE", 0, 2560, 1440, 2000, false),
        DisplayEvidence::new().with_adapter_key(old_adapter.clone()),
    );
    retained.set_user_label(Some(DisplayLabel::new("Editing desk").unwrap()));
    let original = registry([retained]);
    let mut missing_allocator = unavailable_allocator();

    let missing = reconcile_displays(&original, [], &mut missing_allocator).unwrap();
    assert!(matches!(
        missing.inventory().displays()[0].availability(),
        DisplayAvailability::Missing
    ));
    assert_eq!(
        missing.registry().iter().next().unwrap().effective_label(),
        Some(&DisplayLabel::new("Editing desk").unwrap())
    );

    let reappeared = observed(
        "probe:desk",
        facts("DELL U2723QE (USB-C)", 1920, 2560, 1440, 2000, false),
        DisplayEvidence::new()
            .with_adapter_key(old_adapter.clone())
            .with_adapter_key(adapter("tauri", "monitor-new")),
    );
    let mut reappear_allocator = unavailable_allocator();
    let available =
        reconcile_displays(missing.registry(), [reappeared], &mut reappear_allocator).unwrap();
    let display = available.registry().iter().next().unwrap();

    assert_eq!(display.id().as_str(), "display:desk");
    assert_eq!(
        display.effective_label(),
        Some(&DisplayLabel::new("Editing desk").unwrap())
    );
    assert_eq!(
        display.facts().machine_label(),
        Some(&DisplayLabel::new("DELL U2723QE (USB-C)").unwrap())
    );
    assert!(display.evidence().adapter_keys().contains(&old_adapter));
    assert_eq!(display.evidence().adapter_keys().len(), 2);
}

#[test]
fn explicit_forget_removes_only_the_named_display() {
    let mut registry = registry([
        known(
            "display:a",
            facts("A", 0, 1920, 1080, 1000, true),
            DisplayEvidence::new(),
        ),
        known(
            "display:b",
            facts("B", 1920, 1920, 1080, 1000, false),
            DisplayEvidence::new(),
        ),
    ]);

    assert!(
        registry
            .forget(&DisplayId::new("display:a").unwrap())
            .is_some()
    );
    assert!(
        registry
            .get(&DisplayId::new("display:a").unwrap())
            .is_none()
    );
    assert!(
        registry
            .get(&DisplayId::new("display:b").unwrap())
            .is_some()
    );
}

#[test]
fn arrangement_signature_is_order_independent_and_binds_all_contract_fields() {
    let key = strong("fixture", "one");
    let one_known = registry([known(
        "display:a",
        facts("A", 0, 1920, 1080, 1000, true),
        DisplayEvidence::new().with_strong_key(key.clone()),
    )]);
    let baseline = observed(
        "probe:a",
        facts("A", 0, 1920, 1080, 1000, true),
        DisplayEvidence::new().with_strong_key(key.clone()),
    );

    fn signature(
        known: &KnownDisplayRegistry,
        observation: Option<ObservedDisplay>,
    ) -> ArrangementSignature {
        let mut allocator = unavailable_allocator();
        reconcile_displays(known, observation, &mut allocator)
            .unwrap()
            .inventory()
            .arrangement_signature()
            .clone()
    }

    let original = signature(&one_known, Some(baseline.clone()));
    let changed_full = signature(
        &one_known,
        Some(observed(
            "probe:a",
            facts("A", 10, 1920, 1080, 1000, true),
            DisplayEvidence::new().with_strong_key(key.clone()),
        )),
    );
    let changed_scale = signature(
        &one_known,
        Some(observed(
            "probe:a",
            facts("A", 0, 1920, 1080, 1250, true),
            DisplayEvidence::new().with_strong_key(key.clone()),
        )),
    );
    let changed_work = signature(
        &one_known,
        Some(observed(
            "probe:a",
            DisplayFacts::new(
                Some(DisplayLabel::new("A").unwrap()),
                true,
                DisplayBuiltinStatus::BuiltIn,
                rect(0, 0, 1920, 1080),
                rect(0, 48, 1920, 1032),
                ScaleFactor::from_thousandths(1000).unwrap(),
            ),
            DisplayEvidence::new().with_strong_key(key.clone()),
        )),
    );
    let changed_main = signature(
        &one_known,
        Some(observed(
            "probe:a",
            facts("A", 0, 1920, 1080, 1000, false),
            DisplayEvidence::new().with_strong_key(key.clone()),
        )),
    );
    let unavailable = signature(&one_known, None);

    assert_ne!(original, changed_full);
    assert_ne!(original, changed_scale);
    assert_ne!(original, changed_work);
    assert_ne!(original, changed_main);
    assert_ne!(original, unavailable);
    assert_eq!(unavailable.as_str(), "longhorn-arrangement-v1|empty");

    let second_key = strong("fixture", "two");
    let two_known = registry([
        known(
            "display:a",
            facts("A", 0, 1920, 1080, 1000, true),
            DisplayEvidence::new().with_strong_key(key.clone()),
        ),
        known(
            "display:b",
            facts("B", 1920, 2560, 1440, 2000, false),
            DisplayEvidence::new().with_strong_key(second_key.clone()),
        ),
    ]);
    let a = observed(
        "probe:a",
        facts("A", 0, 1920, 1080, 1000, true),
        DisplayEvidence::new().with_strong_key(key),
    );
    let b = observed(
        "probe:b",
        facts("B", 1920, 2560, 1440, 2000, false),
        DisplayEvidence::new().with_strong_key(second_key),
    );
    let mut first_allocator = unavailable_allocator();
    let mut second_allocator = unavailable_allocator();
    let first =
        reconcile_displays(&two_known, [a.clone(), b.clone()], &mut first_allocator).unwrap();
    let second = reconcile_displays(&two_known, [b, a], &mut second_allocator).unwrap();

    assert_eq!(
        first.inventory().arrangement_signature(),
        second.inventory().arrangement_signature()
    );
}

#[test]
fn registry_and_inventory_serde_are_deterministic_and_strict() {
    let a = known(
        "display:a",
        facts("A", 0, 1920, 1080, 1000, true),
        DisplayEvidence::new(),
    );
    let b = known(
        "display:b",
        facts("B", 1920, 2560, 1440, 2000, false),
        DisplayEvidence::new(),
    );
    let first = registry([a.clone(), b.clone()]);
    let second = registry([b, a]);

    let first_json = serde_json::to_string(&first).unwrap();
    assert_eq!(first_json, serde_json::to_string(&second).unwrap());
    assert_eq!(
        serde_json::from_str::<KnownDisplayRegistry>(&first_json).unwrap(),
        first
    );

    let duplicate_json = format!("[{0},{0}]", &first_json[1..first_json.len() - 1]);
    assert!(serde_json::from_str::<KnownDisplayRegistry>(&duplicate_json).is_err());
    assert!(
        serde_json::from_str::<ArrangementSignature>(r#""longhorn-arrangement-v2|empty""#).is_err()
    );
}

#[test]
fn duplicate_observation_and_allocator_identity_fail_typed() {
    let observation = observed(
        "probe:duplicate",
        facts("A", 0, 1920, 1080, 1000, true),
        DisplayEvidence::new(),
    );
    let mut allocator = QueueAllocator::new(["display:new"]);
    let duplicate = reconcile_displays(
        &KnownDisplayRegistry::new(),
        [observation.clone(), observation],
        &mut allocator,
    );
    assert!(matches!(
        duplicate,
        Err(ReconcileError::DuplicateObservationId(_))
    ));

    let existing = registry([known(
        "display:existing",
        facts("Existing", 5000, 800, 600, 1000, false),
        DisplayEvidence::new(),
    )]);
    let new_observation = observed(
        "probe:new",
        facts("New", 0, 1920, 1080, 1000, true),
        DisplayEvidence::new(),
    );
    let mut duplicate_allocator = QueueAllocator::new(["display:existing"]);
    let duplicate_id = reconcile_displays(&existing, [new_observation], &mut duplicate_allocator);
    assert!(matches!(
        duplicate_id,
        Err(ReconcileError::DuplicateAllocatedDisplayId(_))
    ));
}
