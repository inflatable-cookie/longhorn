use std::{
    sync::{Arc, Barrier},
    thread,
};

use longhorn_config::LoadOutcome;
use longhorn_surfaces::{EmptyWindowPolicy, SurfaceMutationRejectionCode};
use longhorn_surfaces_config::{SurfaceConfigMutationError, publish_surface_mutation};

use crate::support::{Fixture, domain, layout_document, options, rename_request, surface_id};

#[test]
fn immediate_publication_uses_fresh_complete_state() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let receipt = publish_surface_mutation(
        &store,
        &domain,
        options(),
        &layout_document(),
        EmptyWindowPolicy::Allow,
        &rename_request(7, "Renamed"),
    )
    .unwrap();
    assert_eq!(receipt.surface().committed_revision().get(), 8);
    assert_eq!(receipt.publication().path, fixture.path(&domain));

    let LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
        panic!("published Surface document should load");
    };
    assert_eq!(loaded.value.revision().get(), 8);
    assert_eq!(
        loaded
            .value
            .surface(&surface_id("surface:a"))
            .unwrap()
            .label(),
        Some("Renamed")
    );
    assert_eq!(loaded.value.surfaces().len(), 2);
}

#[test]
fn two_same_revision_writers_admit_exactly_one() {
    let fixture = Fixture::new();
    let domain = Arc::new(domain());
    let layout = Arc::new(layout_document());
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for request in [rename_request(7, "First"), rename_request(7, "Second")] {
        let domain = Arc::clone(&domain);
        let layout = Arc::clone(&layout);
        let barrier = Arc::clone(&barrier);
        let roots = fixture.roots();
        let coordination = fixture.coordination();
        handles.push(thread::spawn(move || {
            let mut store = longhorn_config::ConfigStore::new(roots, coordination);
            store.register(domain.as_ref()).unwrap();
            barrier.wait();
            publish_surface_mutation(
                &store,
                domain.as_ref(),
                options(),
                layout.as_ref(),
                EmptyWindowPolicy::Allow,
                &request,
            )
        }));
    }
    barrier.wait();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                Err(SurfaceConfigMutationError::Rejected(rejection))
                    if rejection.code() == SurfaceMutationRejectionCode::StaleRevision
            ))
            .count(),
        1
    );
}
