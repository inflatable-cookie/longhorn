//! Contract 020: "Display facts — known and observed displays, with scale
//! factors."
//!
//! GPUI's display API has three members: an id, a persistable UUID, and
//! logical bounds. It reports no scale factor, no work area, and no built-in
//! status. Longhorn's own `DisplayFacts` requires the first two and they are
//! not optional, so a GPUI host cannot meet this requirement alone.

use longhorn_display::DisplayBuiltinStatus;
use longhorn_gpui_windowing::{
    GPUI_DISPLAY_NAMESPACE, GpuiDisplayObservation, UnobtainableDisplayFact, observe_gpui_displays,
};

use super::support::{BareDisplayFacts, FakeGpuiHost, SuppliedDisplayFacts};

#[test]
fn gpui_alone_cannot_supply_the_display_facts_the_contract_requires() {
    let mut host = FakeGpuiHost::new();

    let displays = observe_gpui_displays(&mut host, &mut BareDisplayFacts).unwrap();

    assert_eq!(displays.len(), 1);
    assert!(displays[0].resolved().is_none());
    assert_eq!(
        displays[0].missing(),
        [
            UnobtainableDisplayFact::ScaleFactor,
            UnobtainableDisplayFact::WorkArea
        ]
    );
}

#[test]
fn a_product_that_supplies_the_missing_facts_gets_a_complete_observation() {
    // No silent fallback: the scale and work area come from an explicit
    // injected source, so a product that has neither gets the refusal above
    // rather than an invented number.
    let mut host = FakeGpuiHost::new();

    let displays = observe_gpui_displays(&mut host, &mut SuppliedDisplayFacts::new()).unwrap();

    let resolved = displays[0].resolved().expect("facts were supplied");
    assert_eq!(resolved.facts().scale().thousandths(), 2000);
    assert_eq!(resolved.facts().work_area().origin().y().get(), 25);
    assert_eq!(
        resolved.facts().builtin_status(),
        DisplayBuiltinStatus::BuiltIn
    );
    assert!(resolved.facts().is_main());
    assert_eq!(resolved.facts().full_bounds().size().width(), 1920);
}

#[test]
fn gpuis_display_uuid_is_stronger_identity_evidence_than_tauri_can_offer() {
    // The Tauri probe correlates monitors by name, position and size, and
    // carries `AmbiguousPrimaryMonitor` for the cases where that fails. GPUI
    // documents its UUID as stable across system restarts, so it is recorded
    // as a strong key. This is the one place the second backend supplies
    // better evidence than the first.
    let mut host = FakeGpuiHost::new();

    let displays = observe_gpui_displays(&mut host, &mut SuppliedDisplayFacts::new()).unwrap();
    let resolved = displays[0].resolved().unwrap();

    let strong: Vec<_> = resolved
        .evidence()
        .strong_keys()
        .iter()
        .map(|key| (key.namespace().to_owned(), key.value().to_owned()))
        .collect();
    assert_eq!(
        strong,
        [(
            GPUI_DISPLAY_NAMESPACE.to_owned(),
            "uuid:6d2f0e5c-0000-4000-8000-000000000001".to_owned()
        )]
    );
    assert_eq!(resolved.evidence().adapter_keys().len(), 1);
}

#[test]
fn an_unobtainable_display_still_reports_the_bounds_and_identity_gpui_does_have() {
    // Absence of evidence is recorded as absence, not as failure: the display
    // is still observed, and the facts GPUI does report are still carried.
    let mut host = FakeGpuiHost::new();

    let displays = observe_gpui_displays(&mut host, &mut BareDisplayFacts).unwrap();

    let GpuiDisplayObservation::Unobtainable {
        full_bounds,
        evidence,
        ..
    } = &displays[0]
    else {
        panic!("expected an unobtainable observation");
    };
    assert_eq!(full_bounds.size().width(), 1920);
    assert_eq!(evidence.strong_keys().len(), 1);
}
