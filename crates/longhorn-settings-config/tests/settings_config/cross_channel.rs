//! Settings inherit configuration's cross-channel refusal.
//!
//! Settings have no persistence of their own — a settings apply unit is bound
//! to a configuration domain and stores through it. That inheritance is the
//! reason contract 018 treats settings as covered, so it is proved here rather
//! than assumed.

use std::fs;

use longhorn_config::{LoadOutcome, RecoveryKind};
use longhorn_core::{CompatibilityStore, FutureSchemaRefused};
use serde_json::json;

use super::support::{Fixture, PreferencesDomain};

#[test]
fn a_settings_domain_refuses_a_future_schema_and_preserves_the_file() {
    let fixture = Fixture::new();
    let domain = PreferencesDomain::new("preferences.settings", "preferences/settings.json");
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    // The domain registers at schema 1, so 2 is what a newer channel leaves.
    let written = serde_json::to_vec_pretty(&json!({
        "domain": "preferences.settings",
        "schemaVersion": 2,
        "value": {"theme": "written-by-a-newer-build"},
    }))
    .unwrap();
    let path = fixture.config_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, &written).unwrap();

    let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
        panic!("a future settings schema must not load");
    };
    assert_eq!(recovery.kind, RecoveryKind::FutureSchema);
    assert_eq!(
        recovery
            .future_schema_refusal()
            .map(|refusal| refusal.store),
        Some(CompatibilityStore::Configuration),
        "settings classify as configuration because that is what they store through"
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        written,
        "a refused settings load must not rewrite the store"
    );
}
