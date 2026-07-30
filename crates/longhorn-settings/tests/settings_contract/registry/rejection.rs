use longhorn_settings::{
    SettingsAnchorDefinition, SettingsApplyUnitDefinition, SettingsModuleDefinition,
    SettingsMutationTiming, SettingsPageDefinition, SettingsPageFeatures,
    SettingsRegistryErrorCode, SettingsSectionDefinition,
};

use super::super::support::{
    anchor_id, capability_id, minimal_builder, module_id, page_id, register_capability,
    renderer_id, scope_id, section_id, unit_id,
};
use super::page_with;

#[test]
fn duplicate_ids_fail_before_seal() {
    let mut builder = minimal_builder();
    let error = builder
        .register_module(SettingsModuleDefinition {
            id: module_id("app:module"),
            label: "Duplicate".into(),
            order: -1,
        })
        .unwrap_err();

    assert_eq!(error.code(), SettingsRegistryErrorCode::DuplicateId);
}

#[test]
fn duplicate_section_page_and_apply_unit_ids_fail_at_registration() {
    let mut builder = minimal_builder();
    let duplicate_section = builder
        .register_section(SettingsSectionDefinition {
            id: section_id("app:section"),
            module_id: module_id("app:module"),
            label: "Duplicate".into(),
            order: 99,
        })
        .unwrap_err();
    assert_eq!(
        duplicate_section.code(),
        SettingsRegistryErrorCode::DuplicateId
    );

    let duplicate_page = builder
        .register_page(SettingsPageDefinition {
            id: page_id("app:page"),
            ..page_with(
                module_id("app:module"),
                section_id("app:section"),
                renderer_id("app:renderer"),
                vec![scope_id("app:scope")],
                vec![unit_id("app:apply")],
            )
        })
        .unwrap_err();
    assert_eq!(
        duplicate_page.code(),
        SettingsRegistryErrorCode::DuplicateId
    );

    let duplicate_unit = builder
        .register_apply_unit(SettingsApplyUnitDefinition {
            id: unit_id("app:apply"),
            module_id: module_id("app:module"),
            scope_id: scope_id("app:scope"),
            timing: SettingsMutationTiming::Immediate,
            reset_supported: true,
        })
        .unwrap_err();
    assert_eq!(
        duplicate_unit.code(),
        SettingsRegistryErrorCode::DuplicateId
    );
}

#[test]
fn duplicate_anchors_fail_even_when_one_page_would_be_pruned() {
    let mut builder = minimal_builder();
    register_capability(&mut builder, "app", "app:optional");
    builder
        .register_page(SettingsPageDefinition {
            id: page_id("app:optional-page"),
            module_id: module_id("app:module"),
            section_id: section_id("app:section"),
            renderer_id: renderer_id("app:renderer"),
            label: "Optional".into(),
            keywords: vec![],
            order: 20,
            anchors: vec![SettingsAnchorDefinition {
                id: anchor_id("app:anchor"),
                label: None,
                order: 0,
            }],
            required_capabilities: vec![capability_id("app:optional")],
            readable_scope_ids: vec![scope_id("app:scope")],
            writable_apply_unit_ids: vec![unit_id("app:apply")],
            features: SettingsPageFeatures::default(),
        })
        .unwrap();

    let error = builder.seal([]).unwrap_err();
    assert_eq!(error.code(), SettingsRegistryErrorCode::DuplicateAnchor);
}

#[test]
fn missing_references_report_stable_error_categories() {
    let cases: Vec<(&str, SettingsPageDefinition)> = vec![
        (
            "owner",
            page_with(
                module_id("missing:module"),
                section_id("app:section"),
                renderer_id("app:renderer"),
                vec![scope_id("app:scope")],
                vec![unit_id("app:apply")],
            ),
        ),
        (
            "section",
            page_with(
                module_id("app:module"),
                section_id("missing:section"),
                renderer_id("app:renderer"),
                vec![scope_id("app:scope")],
                vec![unit_id("app:apply")],
            ),
        ),
        (
            "renderer",
            page_with(
                module_id("app:module"),
                section_id("app:section"),
                renderer_id("missing:renderer"),
                vec![scope_id("app:scope")],
                vec![unit_id("app:apply")],
            ),
        ),
        (
            "scope",
            page_with(
                module_id("app:module"),
                section_id("app:section"),
                renderer_id("app:renderer"),
                vec![scope_id("missing:scope")],
                vec![unit_id("app:apply")],
            ),
        ),
        (
            "apply unit",
            page_with(
                module_id("app:module"),
                section_id("app:section"),
                renderer_id("app:renderer"),
                vec![scope_id("app:scope")],
                vec![unit_id("missing:apply")],
            ),
        ),
    ];

    for (name, page) in cases {
        let mut builder = minimal_builder();
        builder.register_page(page).unwrap();
        let error = builder.seal([]).unwrap_err();
        assert_eq!(
            error.code(),
            SettingsRegistryErrorCode::MissingReference,
            "{name}: {}",
            error.detail()
        );
    }
}
