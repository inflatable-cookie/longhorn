use longhorn_core::SurfaceRevision;
use longhorn_surfaces::{
    ParticipatingWindow, SurfaceDocument, SurfaceLimits, SurfaceValidationCode, normalize_document,
    validate_normalized_document,
};

use super::support::*;

#[test]
fn topology_rejects_duplicate_and_unknown_bindings() {
    let duplicate_surface = SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [
            surface("surface:a", None, [host("window:a", 0)]),
            surface("surface:a", None, [host("window:a", 1)]),
        ],
        [],
        [ParticipatingWindow::new(window_id("window:a"), None)],
    );
    assert_code(duplicate_surface, SurfaceValidationCode::DuplicateSurface);

    // Card 179 retired DuplicateLayoutContainerBinding. One container per
    // Surface was worth rejecting; two Surfaces sharing a schema is the point
    // of a schema, so there is nothing left to reject here.

    let duplicate_window = SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [surface("surface:a", None, [host("window:a", 0)])],
        [],
        [
            ParticipatingWindow::new(window_id("window:a"), None),
            ParticipatingWindow::new(window_id("window:a"), None),
        ],
    );
    assert_code(duplicate_window, SurfaceValidationCode::DuplicateWindow);

    let unknown_window = SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [surface("surface:a", None, [host("window:missing", 0)])],
        [],
        [ParticipatingWindow::new(window_id("window:a"), None)],
    );
    assert_code(unknown_window, SurfaceValidationCode::UnknownHostWindow);
}

#[test]
fn topology_rejects_missing_duplicate_and_incomplete_host_order() {
    let missing = SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [surface("surface:a", None, [])],
        [],
        [ParticipatingWindow::new(window_id("window:a"), None)],
    );
    assert_code(missing, SurfaceValidationCode::MissingHostPreference);

    let repeated_host = SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [surface(
            "surface:a",
            None,
            [host("window:a", 0), host("window:a", 1)],
        )],
        [],
        [ParticipatingWindow::new(window_id("window:a"), None)],
    );
    assert_code(
        repeated_host,
        SurfaceValidationCode::DuplicateHostPreference,
    );

    let duplicate_order = SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [
            surface("surface:a", None, [host("window:a", 0)]),
            surface("surface:b", None, [host("window:a", 0)]),
        ],
        [],
        [ParticipatingWindow::new(window_id("window:a"), None)],
    );
    assert_code(duplicate_order, SurfaceValidationCode::DuplicateHostOrder);

    let incomplete_order = SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [
            surface("surface:a", None, [host("window:a", 0)]),
            surface("surface:b", None, [host("window:a", 2)]),
        ],
        [],
        [ParticipatingWindow::new(window_id("window:a"), None)],
    );
    assert_code(incomplete_order, SurfaceValidationCode::IncompleteHostOrder);
}

#[test]
fn topology_rejects_bad_active_count_and_label_state() {
    let bad_active = SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [surface("surface:a", None, [host("window:a", 0)])],
        [],
        [ParticipatingWindow::new(
            window_id("window:a"),
            Some(surface_id("surface:missing")),
        )],
    );
    assert_code(bad_active, SurfaceValidationCode::ActiveSurfaceNotHosted);

    let long_label = SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [surface("surface:a", Some("12345"), [host("window:a", 0)])],
        [],
        [ParticipatingWindow::new(window_id("window:a"), None)],
    );
    assert_eq!(
        longhorn_surfaces::validate_document(SurfaceLimits::new(2, 2, 2, 4).unwrap(), &long_label)
            .unwrap_err()
            .code(),
        SurfaceValidationCode::LabelTooLong
    );

    let excessive = SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [
            surface("surface:a", None, [host("window:a", 0)]),
            surface("surface:b", None, [host("window:a", 1)]),
        ],
        [],
        [ParticipatingWindow::new(window_id("window:a"), None)],
    );
    assert_eq!(
        longhorn_surfaces::validate_document(SurfaceLimits::new(1, 2, 2, 16).unwrap(), &excessive)
            .unwrap_err()
            .code(),
        SurfaceValidationCode::TooManySurfaces
    );

    let too_many_windows = SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [surface("surface:a", None, [host("window:a", 0)])],
        [],
        [
            ParticipatingWindow::new(window_id("window:a"), None),
            ParticipatingWindow::new(window_id("window:b"), None),
        ],
    );
    assert_eq!(
        longhorn_surfaces::validate_document(
            SurfaceLimits::new(2, 1, 2, 16).unwrap(),
            &too_many_windows
        )
        .unwrap_err()
        .code(),
        SurfaceValidationCode::TooManyWindows
    );

    let too_many_preferences = SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [surface(
            "surface:a",
            None,
            [host("window:a", 0), host("window:b", 0)],
        )],
        [],
        [
            ParticipatingWindow::new(window_id("window:a"), None),
            ParticipatingWindow::new(window_id("window:b"), None),
        ],
    );
    assert_eq!(
        longhorn_surfaces::validate_document(
            SurfaceLimits::new(2, 2, 1, 16).unwrap(),
            &too_many_preferences
        )
        .unwrap_err()
        .code(),
        SurfaceValidationCode::TooManyHostPreferences
    );
}

#[test]
fn normalization_canonicalizes_structure_and_preserves_declared_host_priority() {
    let source = loophole_document();
    let permuted = SurfaceDocument::new(
        source.revision(),
        source.surfaces().iter().rev().cloned(),
        [],
        source.windows().iter().rev().cloned(),
    );

    let normalized = normalize_document(limits(), &permuted).unwrap();
    assert_eq!(
        normalized
            .surfaces()
            .iter()
            .map(|surface| surface.id().as_str())
            .collect::<Vec<_>>(),
        vec!["surface:edit", "surface:mix", "surface:plugins"]
    );
    assert_eq!(
        normalized
            .surface(&surface_id("surface:mix"))
            .unwrap()
            .host_preferences()
            .iter()
            .map(|preference| preference.window_id().as_str())
            .collect::<Vec<_>>(),
        vec!["window:main", "window:tools"]
    );
    assert_eq!(
        normalize_document(limits(), &normalized).unwrap(),
        normalized
    );
    validate_normalized_document(limits(), &normalized).unwrap();
    assert_eq!(
        validate_normalized_document(limits(), &permuted)
            .unwrap_err()
            .code(),
        SurfaceValidationCode::NonCanonicalDocument
    );

    let encoded = serde_json::to_vec(&normalized).unwrap();
    let decoded: SurfaceDocument = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, normalized);
    assert!(
        serde_json::from_value::<SurfaceDocument>(serde_json::json!({
            "revision": 0,
            "surfaces": [],
            "windows": [],
            "product_payload": {}
        }))
        .is_err()
    );
}
