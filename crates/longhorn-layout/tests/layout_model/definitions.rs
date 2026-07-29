use longhorn_layout::{
    DefinitionErrorCode, EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutLimits,
    LayoutLimitsError, LayoutRatio, LayoutSchemaDefinition, PanelDefinition, PanelInstancePolicy,
    PlacementSelector, RegionDefinition, SizingSlotDefinition,
};

use super::support::*;

#[test]
fn finite_limits_reject_zero_and_excessive_values() {
    assert_eq!(
        LayoutLimits::new(0, 1, 1, 1, 1, 1, 1),
        Err(LayoutLimitsError::Zero {
            name: "maximum_schemas"
        })
    );
    assert!(matches!(
        LayoutLimits::new(4_097, 1, 1, 1, 1, 1, 1),
        Err(LayoutLimitsError::ExceedsHardMaximum {
            name: "maximum_schemas",
            maximum: 4_096,
            actual: 4_097,
        })
    ));

    let serialized = serde_json::to_value(limits()).unwrap();
    assert_eq!(
        serde_json::from_value::<LayoutLimits>(serialized).unwrap(),
        limits()
    );

    let mut invalid = serde_json::to_value(limits()).unwrap();
    invalid
        .as_object_mut()
        .unwrap()
        .insert("maximum_containers".into(), 0.into());
    assert!(serde_json::from_value::<LayoutLimits>(invalid).is_err());
}

#[test]
fn ratio_is_a_bounded_integer_wire_value() {
    assert_eq!(
        LayoutRatio::from_millionths(1_000_001)
            .unwrap_err()
            .actual(),
        1_000_001
    );
    assert_eq!(serde_json::to_string(&ratio(740_000)).unwrap(), "740000");
    assert_eq!(
        serde_json::from_str::<LayoutRatio>("740000").unwrap(),
        ratio(740_000)
    );
}

#[test]
fn registry_canonicalizes_schema_order_and_resolves_policy() {
    let registry = registry();
    let schema = registry.schema(&schema_id("schema:workspace")).unwrap();

    assert_eq!(
        schema
            .regions()
            .iter()
            .map(|region| region.id().as_str())
            .collect::<Vec<_>>(),
        vec!["left", "center", "right"]
    );
    assert_eq!(
        registry
            .eligible_regions(&schema_id("schema:workspace"), &definition_id("panel:tool"))
            .unwrap(),
        vec![region_id("center"), region_id("right")]
    );
    assert_eq!(
        registry
            .default_region(&schema_id("schema:workspace"), &definition_id("panel:tool"))
            .unwrap(),
        Some(region_id("right"))
    );
}

#[test]
fn duplicate_and_invalid_schema_facts_fail_typed() {
    let duplicate_regions = LayoutSchemaDefinition::new(
        schema_id("schema:duplicate"),
        [
            RegionDefinition::new(
                region_id("same"),
                family_id("workspace"),
                0,
                EmptyRegionPolicy::KeepVisible,
                false,
            ),
            RegionDefinition::new(
                region_id("same"),
                family_id("workspace"),
                1,
                EmptyRegionPolicy::KeepVisible,
                false,
            ),
        ],
        [],
    );
    assert_eq!(
        LayoutDefinitionRegistry::new(limits(), [duplicate_regions], [])
            .unwrap_err()
            .code(),
        DefinitionErrorCode::DuplicateRegion
    );

    let duplicate_order = LayoutSchemaDefinition::new(
        schema_id("schema:order"),
        [
            RegionDefinition::new(
                region_id("one"),
                family_id("workspace"),
                7,
                EmptyRegionPolicy::KeepVisible,
                false,
            ),
            RegionDefinition::new(
                region_id("two"),
                family_id("workspace"),
                7,
                EmptyRegionPolicy::KeepVisible,
                false,
            ),
        ],
        [],
    );
    assert_eq!(
        LayoutDefinitionRegistry::new(limits(), [duplicate_order], [])
            .unwrap_err()
            .code(),
        DefinitionErrorCode::DuplicateRegionOrder
    );

    let invalid_sizing = LayoutSchemaDefinition::new(
        schema_id("schema:sizing"),
        [RegionDefinition::new(
            region_id("center"),
            family_id("workspace"),
            0,
            EmptyRegionPolicy::KeepVisible,
            false,
        )],
        [SizingSlotDefinition::new(
            slot_id("split"),
            0,
            ratio(800_000),
            ratio(500_000),
            ratio(900_000),
        )],
    );
    assert_eq!(
        LayoutDefinitionRegistry::new(limits(), [invalid_sizing], [])
            .unwrap_err()
            .code(),
        DefinitionErrorCode::InvalidSizingBounds
    );
}

#[test]
fn missing_or_inconsistent_panel_policy_fails_closed() {
    let no_allowed = PanelDefinition::new(
        definition_id("panel:none"),
        [PlacementSelector::Region(region_id("center"))],
        [],
        PanelInstancePolicy::Multiple,
        true,
        true,
    );
    assert_eq!(
        LayoutDefinitionRegistry::new(limits(), [workspace_schema()], [no_allowed])
            .unwrap_err()
            .code(),
        DefinitionErrorCode::EmptyAllowedPlacement
    );

    let disallowed_default = PanelDefinition::new(
        definition_id("panel:wrong"),
        [PlacementSelector::Region(region_id("left"))],
        [PlacementSelector::Family(family_id("workspace"))],
        PanelInstancePolicy::Multiple,
        true,
        true,
    );
    assert_eq!(
        LayoutDefinitionRegistry::new(limits(), [workspace_schema()], [disallowed_default])
            .unwrap_err()
            .code(),
        DefinitionErrorCode::DefaultPlacementNotAllowed
    );

    let invalid_limit = PanelDefinition::new(
        definition_id("panel:bounded"),
        [PlacementSelector::Region(region_id("center"))],
        [PlacementSelector::Family(family_id("workspace"))],
        PanelInstancePolicy::Bounded {
            maximum_per_document: 1,
            maximum_per_container: 2,
        },
        true,
        true,
    );
    assert_eq!(
        LayoutDefinitionRegistry::new(limits(), [workspace_schema()], [invalid_limit])
            .unwrap_err()
            .code(),
        DefinitionErrorCode::InvalidInstanceLimit
    );
}

#[test]
fn serialized_panel_definition_requires_instance_policy() {
    let raw = r#"{
        "id":"panel:tool",
        "default_placements":[{"kind":"region","id":"center"}],
        "allowed_placements":[{"kind":"family","id":"workspace"}],
        "movable":true,
        "closeable":true
    }"#;

    assert!(serde_json::from_str::<PanelDefinition>(raw).is_err());

    let mut panel = serde_json::to_value(tool_definition()).unwrap();
    panel
        .as_object_mut()
        .unwrap()
        .insert("implicit_policy".into(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<PanelDefinition>(panel).is_err());
}
