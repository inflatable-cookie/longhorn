use longhorn_core::WindowId;
use longhorn_surfaces_config::merge_pre_card179_state;
use serde_json::json;

fn host() -> WindowId {
    WindowId::new("window:main").expect("window id is valid")
}

fn layout() -> serde_json::Value {
    json!({
        "revision": 7,
        "containers": [
            {
                "id": "container:edit",
                "schema_id": "schema:loophole",
                "regions": [{
                    "region_id": "region:main",
                    "panel_instance_ids": ["panel-instance:mix"],
                    "active_panel_instance_id": "panel-instance:mix",
                    "collapsed": null
                }],
                "sizing_slots": [{ "sizing_slot_id": "slot:left", "ratio": 300000 }]
            },
            {
                "id": "container:spare",
                "schema_id": "schema:loophole",
                "regions": [],
                "sizing_slots": []
            }
        ],
        "panel_instances": [
            { "id": "panel-instance:mix", "definition_id": "panel:mix" }
        ]
    })
}

fn surfaces() -> serde_json::Value {
    json!({
        "revision": 11,
        "surfaces": [{
            "id": "surface:edit",
            "layout_container_id": "container:edit",
            "label": "Edit",
            "host_preferences": [{ "window_id": "window:studio", "order": 0 }]
        }],
        "windows": [{ "id": "window:studio", "active_surface_id": "surface:edit" }]
    })
}

#[test]
fn a_bound_container_keeps_its_surface_identity_label_and_hosting() {
    let merged = merge_pre_card179_state(&layout(), Some(&surfaces()), &host()).unwrap();

    let edit = merged
        .surfaces()
        .iter()
        .find(|surface| surface.id().as_str() == "surface:edit")
        .expect("the bound container became its Surface");
    assert_eq!(edit.label(), Some("Edit"));
    assert_eq!(edit.schema_id().as_str(), "schema:loophole");
    assert_eq!(edit.host_preferences().len(), 1);
    assert_eq!(
        edit.host_preferences()[0].window_id().as_str(),
        "window:studio"
    );

    // The layout the container held survives onto the Surface.
    assert_eq!(edit.regions().len(), 1);
    assert_eq!(edit.regions()[0].panel_instance_ids().len(), 1);
    assert_eq!(edit.sizing_slots().len(), 1);
    assert_eq!(edit.sizing_slots()[0].ratio().millionths(), 300_000);
}

#[test]
fn an_unbound_container_becomes_an_unlabelled_surface_in_the_named_window() {
    let merged = merge_pre_card179_state(&layout(), Some(&surfaces()), &host()).unwrap();

    let spare = merged
        .surfaces()
        .iter()
        .find(|surface| surface.id().as_str() == "container:spare")
        .expect("the unbound container became a Surface");
    assert_eq!(spare.label(), None);
    assert_eq!(spare.host_preferences().len(), 1);
    assert_eq!(
        spare.host_preferences()[0].window_id().as_str(),
        "window:main"
    );
}

#[test]
fn panels_windows_and_the_higher_revision_carry_across() {
    let merged = merge_pre_card179_state(&layout(), Some(&surfaces()), &host()).unwrap();

    assert_eq!(merged.panel_instances().len(), 1);
    assert_eq!(
        merged.panel_instances()[0].id().as_str(),
        "panel-instance:mix"
    );
    assert_eq!(merged.windows().len(), 1);
    // 11 from the Surface side beats 7 from the layout side, so neither
    // document's expected-revision history appears to move backwards.
    assert_eq!(merged.revision().get(), 11);
}

#[test]
fn a_layout_with_no_surface_document_becomes_one_surface_per_container() {
    let merged = merge_pre_card179_state(&layout(), None, &host()).unwrap();

    assert_eq!(merged.surfaces().len(), 2);
    assert!(
        merged
            .surfaces()
            .iter()
            .all(|surface| surface.label().is_none())
    );
    assert!(
        merged
            .surfaces()
            .iter()
            .all(|surface| surface.host_preferences()[0].window_id() == &host())
    );
    assert_eq!(merged.windows().len(), 0);
    assert_eq!(merged.revision().get(), 7);
}

/// A Surface naming a container the layout document does not hold means the two
/// files were not saved together. Guessing would silently lose a panel
/// arrangement, so this refuses.
#[test]
fn a_surface_naming_an_absent_container_is_refused() {
    let orphaned = json!({
        "revision": 11,
        "surfaces": [{
            "id": "surface:ghost",
            "layout_container_id": "container:absent",
            "label": null,
            "host_preferences": [{ "window_id": "window:studio", "order": 0 }]
        }],
        "windows": []
    });

    let error = merge_pre_card179_state(&layout(), Some(&orphaned), &host()).unwrap_err();
    assert!(
        error.message.contains("container:absent"),
        "the error should name the missing container, got: {}",
        error.message
    );
}

#[test]
fn two_surfaces_sharing_one_container_are_refused() {
    let doubled = json!({
        "revision": 11,
        "surfaces": [
            {
                "id": "surface:one",
                "layout_container_id": "container:edit",
                "label": null,
                "host_preferences": [{ "window_id": "window:studio", "order": 0 }]
            },
            {
                "id": "surface:two",
                "layout_container_id": "container:edit",
                "label": null,
                "host_preferences": [{ "window_id": "window:studio", "order": 1 }]
            }
        ],
        "windows": []
    });

    let error = merge_pre_card179_state(&layout(), Some(&doubled), &host()).unwrap_err();
    assert!(
        error.message.contains("more than one Surface"),
        "unexpected error: {}",
        error.message
    );
}
