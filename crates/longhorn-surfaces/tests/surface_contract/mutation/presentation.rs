use longhorn_core::{PanelDefinitionId, SurfaceRevision};
use longhorn_surfaces::{
    EmptyWindowPolicy, SurfaceDocument, SurfaceMutationCommand, SurfaceMutationEngine,
    SurfaceMutationOutcome, SurfaceMutationRejectionCode, SurfaceMutationRequest,
    SurfacePresentation,
};

use crate::support::{
    layout_containers, limits, loophole_document, request_id, surface_id, window_id,
};

fn panel(value: &str) -> PanelDefinitionId {
    PanelDefinitionId::new(value).unwrap()
}

// The engine borrows its container inventory, so it is built per call rather
// than returned from a helper.
fn attempt(
    document: &SurfaceDocument,
    revision: u64,
    suffix: &str,
    command: SurfaceMutationCommand,
) -> Result<longhorn_surfaces::SurfaceMutationReceipt, longhorn_surfaces::SurfaceMutationRejection>
{
    SurfaceMutationEngine::new(limits(), &layout_containers(), EmptyWindowPolicy::Allow).apply(
        document,
        &SurfaceMutationRequest::new(
            request_id(&format!("request:{suffix}")),
            SurfaceRevision::new(revision),
            command,
        ),
    )
}

fn apply(
    document: &SurfaceDocument,
    revision: u64,
    suffix: &str,
    command: SurfaceMutationCommand,
) -> longhorn_surfaces::SurfaceMutationReceipt {
    attempt(document, revision, suffix, command).unwrap()
}

fn focus(surface: &str, panel_id: &str) -> SurfaceMutationCommand {
    SurfaceMutationCommand::SetSurfacePresentation {
        surface_id: surface_id(surface),
        presentation: SurfacePresentation::FocusedPanel {
            panel_definition_id: panel(panel_id),
        },
    }
}

#[test]
fn surfaces_are_regional_until_focused() {
    let document = loophole_document();
    for surface in document.surfaces() {
        assert_eq!(surface.presentation(), &SurfacePresentation::Regional);
        assert!(surface.presentation().is_regional());
        assert!(surface.presentation().focused_panel().is_none());
    }
}

#[test]
fn focusing_a_panel_commits_and_reports_the_replaced_presentation() {
    let source = loophole_document();
    let receipt = apply(&source, 11, "focus", focus("surface:edit", "panel:console"));

    match receipt.outcome() {
        SurfaceMutationOutcome::SurfacePresentationSet {
            surface_id: id,
            presentation,
            previous_presentation,
        } => {
            assert_eq!(id, &surface_id("surface:edit"));
            assert_eq!(presentation.focused_panel(), Some(&panel("panel:console")));
            assert_eq!(previous_presentation, &SurfacePresentation::Regional);
        }
        other => panic!("unexpected outcome {other:?}"),
    }

    let focused = receipt
        .authoritative_document()
        .surface(&surface_id("surface:edit"))
        .unwrap();
    assert_eq!(
        focused.presentation().focused_panel(),
        Some(&panel("panel:console"))
    );

    // Focus is per Surface, so nothing else moved.
    let untouched = receipt
        .authoritative_document()
        .surface(&surface_id("surface:mix"))
        .unwrap();
    assert!(untouched.presentation().is_regional());
}

#[test]
fn a_focused_surface_returns_to_regional() {
    let source = loophole_document();
    let focused = apply(&source, 11, "focus", focus("surface:edit", "panel:console"));
    let restored = apply(
        focused.authoritative_document(),
        12,
        "regional",
        SurfaceMutationCommand::SetSurfacePresentation {
            surface_id: surface_id("surface:edit"),
            presentation: SurfacePresentation::Regional,
        },
    );

    match restored.outcome() {
        SurfaceMutationOutcome::SurfacePresentationSet {
            previous_presentation,
            ..
        } => assert_eq!(
            previous_presentation.focused_panel(),
            Some(&panel("panel:console"))
        ),
        other => panic!("unexpected outcome {other:?}"),
    }
    assert!(
        restored
            .authoritative_document()
            .surface(&surface_id("surface:edit"))
            .unwrap()
            .presentation()
            .is_regional()
    );
}

#[test]
fn focusing_an_unknown_surface_is_rejected_without_changing_state() {
    let source = loophole_document();
    let rejection = attempt(
        &source,
        11,
        "missing",
        focus("surface:absent", "panel:console"),
    )
    .unwrap_err();

    assert_eq!(
        rejection.code(),
        SurfaceMutationRejectionCode::UnknownSurface
    );
    assert_eq!(rejection.current_revision(), source.revision());
    assert_eq!(rejection.authoritative_document(), &source);
}

#[test]
fn focus_survives_a_document_round_trip() {
    let source = loophole_document();
    let focused = apply(&source, 11, "focus", focus("surface:edit", "panel:console"));
    let document = focused.authoritative_document();

    let encoded = serde_json::to_string(document).unwrap();
    let decoded: SurfaceDocument = serde_json::from_str(&encoded).unwrap();

    assert_eq!(&decoded, document);
    assert_eq!(
        decoded
            .surface(&surface_id("surface:edit"))
            .unwrap()
            .presentation()
            .focused_panel(),
        Some(&panel("panel:console"))
    );
}

/// A document written before presentation existed must still load. This is the
/// whole reason the field carries a serde default: the stored schema version
/// belongs to the consumer's migration hook, and requiring a migration for an
/// additive field would have made `NoSurfaceMigration` wrong for every
/// consumer that already has state on disk.
#[test]
fn a_document_without_presentation_loads_as_regional() {
    let stored = serde_json::json!({
        "revision": 7,
        "surfaces": [{
            "id": "surface:edit",
            "layout_container_id": "container:edit",
            "label": "Edit",
            "host_preferences": [{ "window_id": "window:main", "order": 0 }]
        }],
        "windows": [{ "id": "window:main", "active_surface_id": "surface:edit" }]
    });

    let document: SurfaceDocument = serde_json::from_value(stored).unwrap();
    let surface = document.surface(&surface_id("surface:edit")).unwrap();
    assert_eq!(surface.presentation(), &SurfacePresentation::Regional);
    assert_eq!(
        document.window(&window_id("window:main")).unwrap().id(),
        &window_id("window:main")
    );
}
