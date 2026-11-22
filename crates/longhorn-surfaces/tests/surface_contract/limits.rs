use longhorn_core::{
    OpaqueIdError, SurfaceId, SurfaceRequestId, SurfaceRevision, SurfaceRevisionOverflow,
};
use longhorn_surfaces::{SurfaceLimits, SurfaceLimitsError};

#[test]
fn identity_revision_and_limit_rejection_matrix_is_typed() {
    assert_eq!(SurfaceId::new(""), Err(OpaqueIdError::Empty));
    assert_eq!(
        SurfaceId::new("Surface:main"),
        Err(OpaqueIdError::InvalidCharacter { index: 0 })
    );
    assert_eq!(
        SurfaceId::new("x".repeat(129)),
        Err(OpaqueIdError::TooLong {
            maximum: 128,
            actual: 129,
        })
    );
    assert!(SurfaceRequestId::new("request:surface-create").is_ok());
    assert_eq!(
        SurfaceRevision::new(u64::MAX).checked_next(),
        Err(SurfaceRevisionOverflow)
    );

    assert_eq!(
        SurfaceLimits::builder().maximum_surfaces(0).build(),
        Err(SurfaceLimitsError::Zero {
            name: "maximum_surfaces"
        })
    );
    assert_eq!(
        SurfaceLimits::builder().maximum_windows(4_097).build(),
        Err(SurfaceLimitsError::ExceedsHardMaximum {
            name: "maximum_windows",
            maximum: 4_096,
            actual: 4_097,
        })
    );
    assert_eq!(
        SurfaceLimits::builder().maximum_label_bytes(16_385).build(),
        Err(SurfaceLimitsError::ExceedsHardMaximum {
            name: "maximum_label_bytes",
            maximum: 16_384,
            actual: 16_385,
        })
    );
    assert_eq!(
        SurfaceLimits::builder()
            .maximum_panel_instances(65_537)
            .build(),
        Err(SurfaceLimitsError::ExceedsHardMaximum {
            name: "maximum_panel_instances",
            maximum: 65_536,
            actual: 65_537,
        })
    );
}

#[test]
fn limits_and_ids_use_strict_serde() {
    let limits = SurfaceLimits::builder()
        .maximum_surfaces(2)
        .maximum_windows(2)
        .maximum_host_preferences_per_surface(2)
        .maximum_label_bytes(16)
        .build()
        .unwrap();
    let encoded = serde_json::to_string(&limits).unwrap();
    assert_eq!(
        serde_json::from_str::<SurfaceLimits>(&encoded).unwrap(),
        limits
    );
    assert!(
        serde_json::from_str::<SurfaceLimits>(
            r#"{
                "maximum_schemas":16,
                "maximum_regions_per_schema":32,
                "maximum_sizing_slots_per_schema":32,
                "maximum_panel_definitions":64,
                "maximum_surfaces":2,
                "maximum_windows":2,
                "maximum_host_preferences_per_surface":2,
                "maximum_label_bytes":16,
                "maximum_panel_instances":256,
                "maximum_panel_instances_per_region":64,
                "hidden_default":true
            }"#
        )
        .is_err()
    );
    assert!(serde_json::from_str::<SurfaceId>(r#""Surface""#).is_err());
}
