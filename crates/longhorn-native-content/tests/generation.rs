//! Contract evidence for the shared attach-generation admission rule.

use longhorn_native_content::{
    AttachGeneration, AttachmentGate, GenerationRejection, check_attach_reservation,
    compare_attached_generation, compare_generation, compare_generation_allow_next, gate_attached,
    gate_detach, validate_plan_generation,
};

fn generation(value: u64) -> AttachGeneration {
    AttachGeneration::new(value).unwrap()
}

#[test]
fn compare_generation_requires_exact_latest() {
    assert_eq!(compare_generation(None, generation(1)), Ok(()));
    assert_eq!(
        compare_generation(Some(generation(2)), generation(2)),
        Ok(())
    );
    assert_eq!(
        compare_generation(Some(generation(2)), generation(1)),
        Err(GenerationRejection::Stale {
            current: generation(2),
            supplied: generation(1),
        })
    );
    assert_eq!(
        compare_generation(Some(generation(2)), generation(3)),
        Err(GenerationRejection::Future {
            current: generation(2),
            supplied: generation(3),
        })
    );
}

#[test]
fn compare_generation_allow_next_admits_latest_and_next_only() {
    assert_eq!(compare_generation_allow_next(None, generation(1)), Ok(()));
    assert_eq!(
        compare_generation_allow_next(Some(generation(2)), generation(2)),
        Ok(())
    );
    assert_eq!(
        compare_generation_allow_next(Some(generation(2)), generation(3)),
        Ok(())
    );
    assert_eq!(
        compare_generation_allow_next(Some(generation(2)), generation(1)),
        Err(GenerationRejection::Stale {
            current: generation(2),
            supplied: generation(1),
        })
    );
    assert_eq!(
        compare_generation_allow_next(Some(generation(2)), generation(4)),
        Err(GenerationRejection::Future {
            current: generation(2),
            supplied: generation(4),
        })
    );
    assert_eq!(
        compare_generation_allow_next(
            Some(AttachGeneration::new(u64::MAX).unwrap()),
            generation(1)
        ),
        Err(GenerationRejection::Stale {
            current: AttachGeneration::new(u64::MAX).unwrap(),
            supplied: generation(1),
        })
    );
}

#[test]
fn compare_attached_generation_accepts_equal_and_classifies_mismatches() {
    assert_eq!(
        compare_attached_generation(generation(2), generation(2)),
        Ok(())
    );
    assert_eq!(
        compare_attached_generation(generation(2), generation(1)),
        Err(GenerationRejection::Stale {
            current: generation(2),
            supplied: generation(1),
        })
    );
    assert_eq!(
        compare_attached_generation(generation(2), generation(3)),
        Err(GenerationRejection::Future {
            current: generation(2),
            supplied: generation(3),
        })
    );
}

#[test]
fn validate_plan_generation_requires_the_attached_generation() {
    assert_eq!(
        validate_plan_generation(
            Some(generation(2)),
            None,
            Some(generation(2)),
            generation(2),
            false,
        ),
        Ok(())
    );
    assert_eq!(
        validate_plan_generation(
            Some(generation(2)),
            None,
            Some(generation(2)),
            generation(1),
            false,
        ),
        Err(GenerationRejection::Stale {
            current: generation(2),
            supplied: generation(1),
        })
    );
    assert_eq!(
        validate_plan_generation(
            Some(generation(2)),
            None,
            Some(generation(2)),
            generation(3),
            true,
        ),
        Err(GenerationRejection::Attached(generation(2)))
    );
}

#[test]
fn validate_plan_generation_without_attachment_gates_next_and_retired() {
    assert_eq!(
        validate_plan_generation(Some(generation(1)), None, None, generation(2), true),
        Ok(())
    );
    assert_eq!(
        validate_plan_generation(Some(generation(1)), None, None, generation(3), true),
        Err(GenerationRejection::Future {
            current: generation(1),
            supplied: generation(3),
        })
    );
    assert_eq!(
        validate_plan_generation(
            Some(generation(1)),
            Some(generation(1)),
            None,
            generation(1),
            true,
        ),
        Err(GenerationRejection::Retired(generation(1)))
    );
    assert_eq!(
        validate_plan_generation(
            Some(generation(1)),
            Some(generation(1)),
            None,
            generation(1),
            false,
        ),
        Ok(())
    );
}

#[test]
fn check_attach_reservation_admits_replay_and_rejects_live_attachment() {
    let complete = AttachmentGate::new(generation(2), true);
    assert_eq!(
        check_attach_reservation(Some(generation(2)), None, Some(complete), generation(2)),
        Ok(true)
    );
    assert_eq!(
        check_attach_reservation(Some(generation(2)), None, Some(complete), generation(3)),
        Err(GenerationRejection::Attached(generation(2)))
    );
    let reserved = AttachmentGate::new(generation(2), false);
    assert_eq!(
        check_attach_reservation(Some(generation(2)), None, Some(reserved), generation(2)),
        Err(GenerationRejection::Attached(generation(2)))
    );
}

#[test]
fn check_attach_reservation_gates_next_and_retired() {
    assert_eq!(
        check_attach_reservation(Some(generation(1)), None, None, generation(2)),
        Ok(false)
    );
    assert_eq!(
        check_attach_reservation(Some(generation(1)), None, None, generation(3)),
        Err(GenerationRejection::Future {
            current: generation(1),
            supplied: generation(3),
        })
    );
    assert_eq!(
        check_attach_reservation(
            Some(generation(1)),
            Some(generation(1)),
            None,
            generation(1),
        ),
        Err(GenerationRejection::Retired(generation(1)))
    );
}

#[test]
fn gate_attached_returns_the_attachment_and_rejects_retired_absent_and_mismatched() {
    let mut attachment = (generation(2), false);
    let admitted = gate_attached(
        Some(generation(2)),
        None,
        Some(&mut attachment),
        generation(2),
        |value| value.0,
    )
    .unwrap();
    admitted.1 = true;
    assert!(attachment.1);

    let gate =
        |retired, attached| gate_attached(None, retired, attached, generation(2), |value| *value);
    assert_eq!(
        gate(Some(generation(2)), None),
        Err(GenerationRejection::Retired(generation(2)))
    );
    assert_eq!(gate(None, None), Err(GenerationRejection::Absent));
    assert_eq!(
        gate(None, Some(generation(3))),
        Err(GenerationRejection::Stale {
            current: generation(3),
            supplied: generation(2),
        })
    );
}

#[test]
fn gate_detach_is_idempotent_and_requires_a_complete_attachment() {
    let gate = |retired, attached| gate_detach(retired, attached, generation(2), |value| *value);
    let complete = AttachmentGate::new(generation(2), true);
    assert_eq!(gate(Some(generation(2)), None), Ok(None));
    assert_eq!(gate(None, None), Err(GenerationRejection::Absent));
    assert_eq!(gate(None, Some(complete)), Ok(Some(complete)));
    assert_eq!(
        gate(None, Some(AttachmentGate::new(generation(2), false))),
        Err(GenerationRejection::Attaching)
    );
    assert_eq!(
        gate(None, Some(AttachmentGate::new(generation(3), true))),
        Err(GenerationRejection::Stale {
            current: generation(3),
            supplied: generation(2),
        })
    );
}
