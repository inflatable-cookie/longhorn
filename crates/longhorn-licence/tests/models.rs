//! Every purchase model, expressed with only the two windows.
//!
//! The point of this file is what it does *not* contain: no branch on a
//! product type, no enum naming a business model. If a model cannot be
//! written here without a Longhorn change, the abstraction is wrong.

use longhorn_licence::{
    ClockGuard, EntitlementId, Entitlements, GracePolicy, LicencePayload, Limit, Timestamp,
    Usability, VerifiedLicence, usability,
};

const DAY: i64 = 86_400;

fn at(day: i64) -> Timestamp {
    Timestamp::from_unix_seconds(day * DAY)
}

fn id(value: &str) -> EntitlementId {
    EntitlementId::new(value).unwrap()
}

/// Every model below is built the same way and checked the same way. Only
/// the two windows differ.
fn licence(payload: LicencePayload) -> VerifiedLicence {
    VerifiedLicence::from_remote_assertion(payload, at(0))
}

fn state(licence: &VerifiedLicence, now: Timestamp) -> Usability {
    usability(licence, now, ClockGuard::new(at(0)), GracePolicy::default())
}

#[test]
fn subscription() {
    // Use and update end together, and the lease tracks the billing period.
    let subscription = licence(
        LicencePayload::new("example")
            .with_entitlements(Entitlements::new().granting(id("all")))
            .with_use_until(at(30))
            .with_update_until(at(30))
            .with_lease_until(at(30)),
    );

    assert_eq!(state(&subscription, at(15)), Usability::Active);
    assert!(subscription.may_take_updates(at(15)));

    assert_eq!(
        state(&subscription, at(45)),
        Usability::UseWindowExpired { at: at(30) }
    );
    assert!(!subscription.may_take_updates(at(45)));
}

#[test]
fn perpetual_with_maintenance() {
    // The distinguishing case: updates lapse, the software does not. A model
    // that could not separate these would force perpetual customers to be
    // told their licence expired, which is false and reads as the app
    // breaking.
    let perpetual = licence(
        LicencePayload::new("example")
            .with_entitlements(Entitlements::new().granting(id("all")))
            .with_update_until(at(365)),
    );

    assert_eq!(state(&perpetual, at(1_000)), Usability::Active);
    assert!(state(&perpetual, at(1_000)).is_usable());
    assert!(!perpetual.may_take_updates(at(400)));
    assert!(perpetual.may_take_updates(at(300)));
}

#[test]
fn trial() {
    // A use window and no update window: the trial runs out, and while it
    // runs it takes whatever ships.
    let trial = licence(
        LicencePayload::new("example")
            .with_entitlements(Entitlements::new().granting(id("all")))
            .with_use_until(at(14)),
    );

    assert_eq!(state(&trial, at(7)), Usability::Active);
    assert!(trial.may_take_updates(at(7)));
    assert_eq!(
        state(&trial, at(20)),
        Usability::UseWindowExpired { at: at(14) }
    );
}

#[test]
fn freemium() {
    // Neither window. A licence that never expires, granting a subset.
    let free = licence(
        LicencePayload::new("example")
            .with_entitlements(Entitlements::new().with(id("documents"), Limit::AtMost(3))),
    );

    assert_eq!(state(&free, at(10_000)), Usability::Active);
    assert!(free.may_take_updates(at(10_000)));
    assert!(free.entitlements().permits(&id("documents"), 3));
    assert!(!free.entitlements().permits(&id("documents"), 4));
    assert!(!free.entitlements().grants(&id("collaboration")));
}

#[test]
fn a_model_longhorn_never_anticipated() {
    // Site licence: perpetual use, updates for a fixed term, a seat cap, and
    // a lease so the seat count stays revocable. No Longhorn change needed,
    // which is the actual claim being tested.
    let site = licence(
        LicencePayload::new("example")
            .with_entitlements(
                Entitlements::new()
                    .granting(id("all"))
                    .with(id("seats"), Limit::AtMost(250)),
            )
            .with_update_until(at(730))
            .with_lease_until(at(90)),
    );

    assert_eq!(state(&site, at(60)), Usability::Active);
    assert!(site.entitlements().permits(&id("seats"), 250));
    assert!(!site.entitlements().permits(&id("seats"), 251));
    assert!(site.may_take_updates(at(700)));
    assert!(!site.may_take_updates(at(800)));
}
