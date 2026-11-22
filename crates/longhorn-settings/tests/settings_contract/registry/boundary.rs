use longhorn_settings::{
    SettingsApplyUnitDefinition, SettingsLimits, SettingsModuleDefinition, SettingsMutationTiming,
    SettingsPageFeatures, SettingsRegistryBuilder, SettingsRegistryErrorCode,
    SettingsRegistryGeneration, SettingsRendererDefinition, SettingsScopeDefinition,
};

use super::super::support::{
    capability_id, minimal_builder, module_id, register_module_page, renderer_id, scope_id,
    section_id, unit_id,
};
use super::page_with;

#[test]
fn limits_reject_declarations_before_admission() {
    let limits = SettingsLimits {
        maximum_pages: 1,
        ..SettingsLimits::default()
    };
    let mut builder = SettingsRegistryBuilder::new(SettingsRegistryGeneration::INITIAL, limits);
    register_module_page(
        &mut builder,
        "app",
        "General",
        0,
        SettingsMutationTiming::Staged,
        &[],
        SettingsPageFeatures::default(),
    );
    builder
        .register_page(page_with(
            module_id("app:module"),
            section_id("app:section"),
            renderer_id("app:renderer"),
            vec![scope_id("app:scope")],
            vec![unit_id("app:apply")],
        ))
        .unwrap();

    let error = builder.seal([]).unwrap_err();
    assert_eq!(error.code(), SettingsRegistryErrorCode::LimitExceeded);
}

#[test]
fn ownership_mismatch_and_unknown_composed_capability_are_typed() {
    let mut mismatched = minimal_builder();
    mismatched
        .register_module(SettingsModuleDefinition {
            id: module_id("other:module"),
            label: "Other".into(),
            order: 20,
        })
        .unwrap();
    mismatched
        .register_scope(SettingsScopeDefinition {
            id: scope_id("other:scope"),
            module_id: module_id("other:module"),
        })
        .unwrap();
    mismatched
        .register_apply_unit(SettingsApplyUnitDefinition {
            id: unit_id("app:mismatched"),
            module_id: module_id("app:module"),
            scope_id: scope_id("other:scope"),
            timing: SettingsMutationTiming::Staged,
            reset_supported: false,
        })
        .unwrap();
    let mismatch = mismatched.seal([]).unwrap_err();
    assert_eq!(
        mismatch.code(),
        SettingsRegistryErrorCode::OwnershipMismatch
    );

    let unknown = minimal_builder()
        .seal([capability_id("missing:capability")])
        .unwrap_err();
    assert_eq!(
        unknown.code(),
        SettingsRegistryErrorCode::UnknownComposedCapability
    );
}

#[test]
fn page_cannot_write_a_scope_it_cannot_read() {
    let mut builder = minimal_builder();
    builder
        .register_page(page_with(
            module_id("app:module"),
            section_id("app:section"),
            renderer_id("app:renderer"),
            vec![],
            vec![unit_id("app:apply")],
        ))
        .unwrap();

    let error = builder.seal([]).unwrap_err();
    assert_eq!(error.code(), SettingsRegistryErrorCode::MissingReference);
    assert!(error.to_string().contains("cannot read scope app:scope"));
}

#[test]
fn unregistered_renderer_owner_fails_seal() {
    let mut builder = SettingsRegistryBuilder::new(
        SettingsRegistryGeneration::INITIAL,
        SettingsLimits::default(),
    );
    builder
        .register_renderer(SettingsRendererDefinition {
            id: renderer_id("orphan:renderer"),
            module_id: module_id("orphan:module"),
        })
        .unwrap();
    let error = builder.seal([]).unwrap_err();
    assert_eq!(error.code(), SettingsRegistryErrorCode::MissingReference);
}
