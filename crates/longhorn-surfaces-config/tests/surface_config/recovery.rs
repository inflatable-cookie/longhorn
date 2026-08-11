use longhorn_config::{ConfigDomain, LoadOutcome};
use longhorn_surfaces_config::{LayoutFallback, load_or_default};
use serde_json::json;

use crate::support::{Fixture, document, domain, envelope};

#[test]
fn a_readable_document_is_returned_unchanged() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = domain();
    store.register(&domain).unwrap();
    let encoded = domain.encode(&document()).unwrap();
    fixture.write(&domain, &envelope("surfaces.workspace", 1, encoded));

    let (value, fallback) = load_or_default(&store, &domain).unwrap();
    assert_eq!(fallback, LayoutFallback::None);
    assert!(!fallback.discarded_stored_state());
    assert_eq!(value, document());
}

#[test]
fn an_unparseable_document_opens_on_the_default_arrangement() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = domain();
    store.register(&domain).unwrap();
    fixture.write(&domain, b"{ not json");

    let (value, fallback) = load_or_default(&store, &domain).unwrap();
    assert_eq!(fallback, LayoutFallback::RecoveredToDefault);
    assert!(fallback.discarded_stored_state());
    assert_eq!(value, domain.default_value());
}

/// The interesting case: the file parses, but the document it describes is not
/// a valid Surface document. Before this helper that stopped the application;
/// now it opens on the default arrangement.
#[test]
fn a_structurally_invalid_document_opens_on_the_default_arrangement() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = domain();
    store.register(&domain).unwrap();
    // Well-formed JSON, wrong shape: a Surface with no id and no schema.
    let nonsense = json!({
        "revision": 3,
        "surfaces": [{ "label": "orphan" }],
        "panel_instances": [],
        "windows": []
    });
    fixture.write(&domain, &envelope("surfaces.workspace", 1, nonsense));

    let (value, fallback) = load_or_default(&store, &domain).unwrap();
    assert_eq!(fallback, LayoutFallback::RecoveredToDefault);
    assert_eq!(value, domain.default_value());
}

/// Falling back must not overwrite the file. One bad session should cost the
/// arrangement, not the record of it.
#[test]
fn the_unreadable_source_is_left_on_disk() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = domain();
    store.register(&domain).unwrap();
    fixture.write(&domain, b"{ not json");

    let _ = load_or_default(&store, &domain).unwrap();

    assert_eq!(
        std::fs::read(fixture.path(&domain)).unwrap(),
        b"{ not json",
        "the helper must not rewrite what it declined to read"
    );
    let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
        panic!("the source should still be recoverable after a fallback load");
    };
    assert_eq!(recovery.source.unwrap().bytes, b"{ not json");
}
