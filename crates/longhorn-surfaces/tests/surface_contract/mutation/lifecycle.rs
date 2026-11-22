use longhorn_core::{PanelDefinitionId, SurfaceRevision};
use longhorn_surfaces::{
    EmptyWindowPolicy, SurfaceMutationCommand, SurfaceMutationEngine, SurfaceMutationOutcome,
    SurfaceMutationRequest, SurfacePresentation, validate_document,
};

use crate::support::{
    host, loophole_document, registry, request_id, schema_id, surface_id, window_id,
};

fn apply(
    document: &longhorn_surfaces::SurfaceDocument,
    revision: u64,
    suffix: &str,
    command: SurfaceMutationCommand,
) -> longhorn_surfaces::SurfaceMutationReceipt {
    let reg = registry();
    let receipt = SurfaceMutationEngine::new(&reg, EmptyWindowPolicy::Allow)
        .apply(
            document,
            &SurfaceMutationRequest::new(
                request_id(&format!("request:{suffix}")),
                SurfaceRevision::new(revision),
                command,
            ),
        )
        .unwrap();
    validate_document(&reg, receipt.authoritative_document()).unwrap();
    receipt
}

#[test]
fn create_duplicate_rename_activate_and_reorder_commit_once() {
    let source = loophole_document();
    let created = apply(
        &source,
        11,
        "create",
        SurfaceMutationCommand::CreateSurface {
            surface_id: surface_id("surface:new"),
            schema_id: schema_id(),
            label: Some("New".to_owned()),
            host_preferences: vec![host("window:main", 2), host("window:tools", 3)],
        },
    );
    assert_eq!(created.previous_revision().get(), 11);
    assert_eq!(created.committed_revision().get(), 12);
    assert_eq!(source.revision().get(), 11);

    let duplicated = apply(
        created.authoritative_document(),
        12,
        "duplicate",
        SurfaceMutationCommand::DuplicateSurface {
            source_surface_id: surface_id("surface:mix"),
            surface_id: surface_id("surface:mix-copy"),
        },
    );
    let copy = duplicated
        .authoritative_document()
        .surface(&surface_id("surface:mix-copy"))
        .unwrap();
    assert_eq!(copy.label(), Some("Mix"));
    assert_eq!(copy.schema_id(), &schema_id());
    assert_eq!(
        copy.host_preferences()
            .iter()
            .map(|preference| preference.order())
            .collect::<Vec<_>>(),
        [1, 2]
    );

    let renamed = apply(
        duplicated.authoritative_document(),
        13,
        "rename",
        SurfaceMutationCommand::RenameSurface {
            surface_id: surface_id("surface:mix-copy"),
            label: Some("Second Mix".to_owned()),
        },
    );
    let activated = apply(
        renamed.authoritative_document(),
        14,
        "activate",
        SurfaceMutationCommand::ActivateSurface {
            window_id: window_id("window:main"),
            surface_id: surface_id("surface:mix-copy"),
        },
    );
    assert!(matches!(
        activated.outcome(),
        SurfaceMutationOutcome::SurfaceActivated {
            previous_active_surface_id: Some(previous),
            ..
        } if previous == &surface_id("surface:edit")
    ));

    let members = vec![
        surface_id("surface:new"),
        surface_id("surface:mix-copy"),
        surface_id("surface:edit"),
        surface_id("surface:mix"),
    ];
    let reordered = apply(
        activated.authoritative_document(),
        15,
        "reorder",
        SurfaceMutationCommand::ReorderWindow {
            window_id: window_id("window:main"),
            surface_ids: members.clone(),
        },
    );
    for (index, surface_id) in members.iter().enumerate() {
        let order = reordered
            .authoritative_document()
            .surface(surface_id)
            .unwrap()
            .host_preferences()
            .iter()
            .find(|preference| preference.window_id() == &window_id("window:main"))
            .unwrap()
            .order();
        assert_eq!(order, u32::try_from(index).unwrap());
    }
    assert_eq!(
        reordered
            .authoritative_document()
            .window(&window_id("window:main"))
            .unwrap()
            .active_surface_id(),
        Some(&surface_id("surface:mix-copy"))
    );
}

#[test]
fn duplicate_preserves_focused_presentation() {
    let focused_panel = PanelDefinitionId::new("panel:console").unwrap();
    let focused = apply(
        &loophole_document(),
        11,
        "focus",
        SurfaceMutationCommand::SetSurfacePresentation {
            surface_id: surface_id("surface:mix"),
            presentation: SurfacePresentation::FocusedPanel {
                panel_definition_id: focused_panel.clone(),
            },
        },
    );
    let duplicated = apply(
        focused.authoritative_document(),
        12,
        "duplicate-focused",
        SurfaceMutationCommand::DuplicateSurface {
            source_surface_id: surface_id("surface:mix"),
            surface_id: surface_id("surface:mix-copy"),
        },
    );
    let copy = duplicated
        .authoritative_document()
        .surface(&surface_id("surface:mix-copy"))
        .unwrap();
    assert_eq!(
        copy.presentation(),
        &SurfacePresentation::FocusedPanel {
            panel_definition_id: focused_panel,
        }
    );
}
