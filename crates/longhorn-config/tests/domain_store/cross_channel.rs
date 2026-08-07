//! Cross-channel store compatibility.
//!
//! All update channels ship under one bundle identity, so a nightly build and
//! a production build read and write the same files. Every nightly install
//! eventually rejoins production, at which point an older reader opens a store
//! a newer build wrote. These tests hold the two properties that make that
//! safe: the read is refused, and the bytes on disk are left exactly as found.

use std::{fs, time::Duration};

use longhorn_config::{
    DurabilityRequirement, LoadOutcome, MutationError, MutationOptions, MutationRefusal,
    RecoveryKind,
};
use longhorn_core::{CompatibilityStore, FutureSchemaRefused};
use serde_json::json;

use crate::common::{Fixture, config_domain, document};

/// The schema a hypothetical newer channel writes. `config_domain` registers
/// at 3, so 4 stands in for what a future build would leave behind.
const FUTURE_SCHEMA: u32 = 4;

fn future_document() -> Vec<u8> {
    document(
        "example.preferences",
        FUTURE_SCHEMA,
        json!({"name": "written by a newer build", "enabled": true}),
    )
}

#[test]
fn refused_future_schema_load_leaves_the_file_byte_identical() {
    let fixture = Fixture::new();
    let domain = config_domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let written = future_document();
    let path = fixture.write(&domain, &written);

    let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
        panic!("a future schema must not load");
    };
    assert_eq!(recovery.kind, RecoveryKind::FutureSchema);

    assert_eq!(
        fs::read(&path).unwrap(),
        written,
        "a refused load must not rewrite the store"
    );
}

#[test]
fn refused_future_schema_mutation_leaves_the_file_byte_identical() {
    let fixture = Fixture::new();
    let domain = config_domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let written = future_document();
    let path = fixture.write(&domain, &written);

    // The destructive path is not the read but the write that follows it: a
    // reader that fell back to defaults would persist those defaults over the
    // newer data. Mutation has to refuse for the same reason loading does.
    let error = store
        .mutate(
            &domain,
            MutationOptions::new(Duration::from_secs(10), DurabilityRequirement::Atomic),
            |value| {
                value.name.push_str(" clobbered");
                Ok(())
            },
        )
        .expect_err("a future schema must not be mutated");

    assert!(matches!(
        error,
        MutationError::Refused(MutationRefusal::Recovery(_))
    ));
    assert_eq!(
        fs::read(&path).unwrap(),
        written,
        "a refused mutation must not rewrite the store"
    );
}

#[test]
fn future_schema_recovery_classifies_for_the_update_surface() {
    let fixture = Fixture::new();
    let domain = config_domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    fixture.write(&domain, &future_document());

    let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
        panic!("a future schema must not load");
    };

    let refusal = recovery
        .future_schema_refusal()
        .expect("a future-schema recovery must classify as one");
    assert_eq!(refusal.store, CompatibilityStore::Configuration);
}

#[test]
fn other_recoveries_do_not_classify_as_future_schema() {
    let fixture = Fixture::new();
    let domain = config_domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    fixture.write(&domain, b"{ definitely not json");

    let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
        panic!("corrupt source must not load");
    };

    assert_eq!(recovery.kind, RecoveryKind::CorruptDocument);
    assert_eq!(
        recovery.future_schema_refusal(),
        None,
        "a corrupt document is not a version problem and must not be reported as one"
    );
}
