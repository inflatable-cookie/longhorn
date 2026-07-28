use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use longhorn_config::{ConfigStore, DurabilityRequirement, LoadOutcome, MutationOptions};
use serde_json::{Value, json};

use crate::common::{Fixture, config_domain, document};

fn options() -> MutationOptions {
    MutationOptions::new(Duration::from_secs(2), DurabilityRequirement::Atomic)
}

#[test]
fn two_store_instances_patch_the_fresh_value_without_lost_fields() {
    let fixture = Fixture::new();
    let initial = config_domain();
    fixture.write(
        &initial,
        &document(
            "example.preferences",
            3,
            json!({"name": "initial", "enabled": false}),
        ),
    );
    let roots_a = fixture.roots.clone();
    let roots_b = fixture.roots.clone();
    let coordination_a = fixture.coordination.clone();
    let coordination_b = fixture.coordination.clone();
    let (acquired_tx, acquired_rx) = mpsc::channel();

    let first = thread::spawn(move || {
        let domain = config_domain();
        let mut store = ConfigStore::new(roots_a, coordination_a);
        store.register(&domain).unwrap();
        store
            .mutate(&domain, options(), |value| {
                acquired_tx.send(()).unwrap();
                thread::sleep(Duration::from_millis(100));
                value.name = "changed".to_owned();
                Ok(())
            })
            .unwrap();
    });

    acquired_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let second = thread::spawn(move || {
        let domain = config_domain();
        let mut store = ConfigStore::new(roots_b, coordination_b);
        store.register(&domain).unwrap();
        store
            .mutate(&domain, options(), |value| {
                value.enabled = true;
                Ok(())
            })
            .unwrap();
    });

    first.join().unwrap();
    second.join().unwrap();

    let domain = config_domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
        panic!("expected current file");
    };
    assert_eq!(loaded.value.name, "changed");
    assert!(loaded.value.enabled);
}

#[test]
fn readers_observe_only_complete_old_or_new_documents() {
    let fixture = Fixture::new();
    let domain = config_domain();
    let path = fixture.write(
        &domain,
        &document(
            "example.preferences",
            3,
            json!({"name": "initial", "enabled": true}),
        ),
    );
    let roots = fixture.roots.clone();
    let coordination = fixture.coordination.clone();
    let finished = Arc::new(AtomicBool::new(false));
    let reader_finished = Arc::clone(&finished);

    let reader = thread::spawn(move || {
        loop {
            let bytes = fs::read(&path).unwrap();
            let document: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(document["domain"], "example.preferences");
            assert_eq!(document["schemaVersion"], 3);
            if reader_finished.load(Ordering::Acquire) {
                break;
            }
        }
    });

    let writer = thread::spawn(move || {
        let domain = config_domain();
        let mut store = ConfigStore::new(roots, coordination);
        store.register(&domain).unwrap();
        for index in 0..40 {
            store
                .mutate(&domain, options(), |value| {
                    value.name = format!("value-{index}");
                    Ok(())
                })
                .unwrap();
        }
    });

    writer.join().unwrap();
    finished.store(true, Ordering::Release);
    reader.join().unwrap();
}
