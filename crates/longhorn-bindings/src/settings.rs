use std::error::Error;

use longhorn_core::{
    SettingsActivationTargetId, SettingsAnchorId, SettingsApplyUnitId, SettingsAuthorityToken,
    SettingsCapabilityId, SettingsEntryId, SettingsModuleId, SettingsPageId,
    SettingsPolicySourceId, SettingsRendererId, SettingsRequestId, SettingsScopeId,
    SettingsSectionId,
};
use longhorn_settings::{
    SETTINGS_PROTOCOL_VERSION, SettingsActivationRequirement, SettingsActivationState,
    SettingsAnchorDefinition, SettingsApplyCommand, SettingsApplyUnitDefinition,
    SettingsAuthorityExpectation, SettingsCapabilityDefinition, SettingsConflict,
    SettingsDurabilityEvidence, SettingsEditability, SettingsEffectiveSource, SettingsLimits,
    SettingsLoadCommand, SettingsLoadOutcome, SettingsModuleDefinition, SettingsMutationOutcome,
    SettingsMutationReceipt, SettingsMutationResult, SettingsMutationTiming, SettingsOpaqueValue,
    SettingsPageDefinition, SettingsPageFeatures, SettingsPolicyEffect, SettingsPolicyProjection,
    SettingsProtocolVersion, SettingsRecoveryCode, SettingsRecoveryState,
    SettingsRegistryChangedEvent, SettingsRegistryDigest, SettingsRegistryGeneration,
    SettingsRegistrySnapshot, SettingsRejection, SettingsRejectionCode, SettingsRendererDefinition,
    SettingsResetCommand, SettingsScopeChangedEvent, SettingsScopeDefinition,
    SettingsScopeRevision, SettingsScopeSnapshot, SettingsSectionDefinition,
    SettingsSourceDiagnostic, SettingsValueProjection,
};
use ts_rs::TS;

use crate::generation::{
    Artifact, GenerationMode, apply, config, exported_declaration, field_map,
    string_union_variants, tagged_variants, variant_field_map,
};

mod fixture;

const GENERATED_PROTOCOL: &str = "packages/longhorn/src/settings/generated/protocol.ts";
const GENERATED_FIELDS: &str = "packages/longhorn/src/settings/generated/fields.ts";
const GENERATED_VARIANT_FIELDS: &str = "packages/longhorn/src/settings/generated/variant-fields.ts";
const GOLDEN_FIXTURE: &str = "fixtures/settings/protocol-v1.json";

struct RenderedProtocol {
    contents: String,
    fields: String,
    rejection_codes: Vec<String>,
    variant_fields: String,
}

/// Generates or checks the settings bindings and golden fixtures.
pub fn run(mode: GenerationMode) -> Result<(), Box<dyn Error>> {
    let protocol = render_protocol()?;
    let artifacts = [
        Artifact {
            relative_path: GENERATED_PROTOCOL,
            contents: protocol.contents,
        },
        Artifact {
            relative_path: GENERATED_FIELDS,
            contents: protocol.fields,
        },
        Artifact {
            relative_path: GENERATED_VARIANT_FIELDS,
            contents: protocol.variant_fields,
        },
        Artifact {
            relative_path: GOLDEN_FIXTURE,
            contents: fixture::render(&protocol.rejection_codes)?,
        },
    ];
    apply("settings", "generate:settings", mode, &artifacts)
}

fn render_protocol() -> Result<RenderedProtocol, Box<dyn Error>> {
    let durability = SettingsDurabilityEvidence::decl(config());
    let mutation_result = SettingsMutationResult::decl(config());
    let load_outcome = SettingsLoadOutcome::decl(config());
    let rejection = SettingsRejectionCode::decl(config());
    let mutation_timing = SettingsMutationTiming::decl(config());
    let effective_source = SettingsEffectiveSource::decl(config());
    let editability = SettingsEditability::decl(config());
    let policy_effect = SettingsPolicyEffect::decl(config());
    let recovery_code = SettingsRecoveryCode::decl(config());
    let activation_state = SettingsActivationState::decl(config());
    let mutation_outcome = SettingsMutationOutcome::decl(config());

    let durability_kinds = tagged_variants(&durability, "kind")?;
    let mutation_statuses = tagged_variants(&mutation_result, "status")?;
    let load_statuses = tagged_variants(&load_outcome, "status")?;
    let rejection_codes = string_union_variants(&rejection)?;
    let mutation_timings = string_union_variants(&mutation_timing)?;
    let effective_sources = string_union_variants(&effective_source)?;
    let editabilities = string_union_variants(&editability)?;
    let policy_effects = string_union_variants(&policy_effect)?;
    let recovery_codes = string_union_variants(&recovery_code)?;
    let activation_states = string_union_variants(&activation_state)?;
    let mutation_outcomes = string_union_variants(&mutation_outcome)?;

    let declarations = [
        SettingsModuleId::decl(config()),
        SettingsSectionId::decl(config()),
        SettingsPageId::decl(config()),
        SettingsRendererId::decl(config()),
        SettingsAnchorId::decl(config()),
        SettingsScopeId::decl(config()),
        SettingsApplyUnitId::decl(config()),
        SettingsCapabilityId::decl(config()),
        SettingsActivationTargetId::decl(config()),
        SettingsEntryId::decl(config()),
        SettingsRequestId::decl(config()),
        SettingsPolicySourceId::decl(config()),
        SettingsAuthorityToken::decl(config()),
        SettingsProtocolVersion::decl(config()),
        SettingsScopeRevision::decl(config()),
        SettingsRegistryGeneration::decl(config()),
        SettingsRegistryDigest::decl(config()),
        SettingsLimits::decl(config()),
        SettingsOpaqueValue::decl(config()),
        SettingsModuleDefinition::decl(config()),
        SettingsSectionDefinition::decl(config()),
        SettingsRendererDefinition::decl(config()),
        SettingsScopeDefinition::decl(config()),
        SettingsCapabilityDefinition::decl(config()),
        mutation_timing,
        SettingsApplyUnitDefinition::decl(config()),
        SettingsAnchorDefinition::decl(config()),
        SettingsPageFeatures::decl(config()),
        SettingsPageDefinition::decl(config()),
        effective_source,
        editability,
        policy_effect,
        SettingsPolicyProjection::decl(config()),
        SettingsSourceDiagnostic::decl(config()),
        SettingsValueProjection::decl(config()),
        recovery_code,
        SettingsRecoveryState::decl(config()),
        activation_state,
        SettingsActivationRequirement::decl(config()),
        SettingsAuthorityExpectation::decl(config()),
        SettingsScopeSnapshot::decl(config()),
        SettingsRegistrySnapshot::decl(config()),
        SettingsRegistryChangedEvent::decl(config()),
        SettingsScopeChangedEvent::decl(config()),
        SettingsLoadCommand::decl(config()),
        SettingsApplyCommand::decl(config()),
        SettingsResetCommand::decl(config()),
        mutation_outcome,
        durability,
        SettingsMutationReceipt::decl(config()),
        SettingsConflict::decl(config()),
        rejection,
        SettingsRejection::decl(config()),
        mutation_result,
        load_outcome,
    ]
    .map(exported_declaration);

    // Inline format arguments take identifiers, not paths.
    let hard_maximum_text_bytes = SettingsLimits::HARD_MAXIMUM_TEXT_BYTES;
    let hard_maximum_opaque_value_bytes = SettingsLimits::HARD_MAXIMUM_OPAQUE_VALUE_BYTES;
    let contents = format!(
        "// @generated by `effigy generate:settings`; do not edit.\n\
         // Rust serde types are the wire authority.\n\n\
         export const SETTINGS_PROTOCOL_VERSION = {SETTINGS_PROTOCOL_VERSION} as const;\n\
         export const SETTINGS_HARD_MAXIMUM_TEXT_BYTES = {hard_maximum_text_bytes} as const;\n\
         export const SETTINGS_HARD_MAXIMUM_OPAQUE_VALUE_BYTES = {hard_maximum_opaque_value_bytes} as const;\n\
         export const SETTINGS_DURABILITY_KINDS = {} as const;\n\
         export const SETTINGS_MUTATION_RESULT_STATUSES = {} as const;\n\
         export const SETTINGS_LOAD_OUTCOME_STATUSES = {} as const;\n\
         export const SETTINGS_REJECTION_CODES = {} as const;\n\n\
         export const SETTINGS_MUTATION_TIMINGS = {} as const;\n\
         export const SETTINGS_EFFECTIVE_SOURCES = {} as const;\n\
         export const SETTINGS_EDITABILITIES = {} as const;\n\
         export const SETTINGS_POLICY_EFFECTS = {} as const;\n\
         export const SETTINGS_RECOVERY_CODES = {} as const;\n\
         export const SETTINGS_ACTIVATION_STATES = {} as const;\n\
         export const SETTINGS_MUTATION_OUTCOMES = {} as const;\n\n\
         {}\n",
        serde_json::to_string(&durability_kinds)?,
        serde_json::to_string(&mutation_statuses)?,
        serde_json::to_string(&load_statuses)?,
        serde_json::to_string(&rejection_codes)?,
        serde_json::to_string(&mutation_timings)?,
        serde_json::to_string(&effective_sources)?,
        serde_json::to_string(&editabilities)?,
        serde_json::to_string(&policy_effects)?,
        serde_json::to_string(&recovery_codes)?,
        serde_json::to_string(&activation_states)?,
        serde_json::to_string(&mutation_outcomes)?,
        declarations.join("\n\n")
    );
    let (fields, _skipped) = field_map("generate:settings", "SETTINGS_FIELDS", &declarations);

    let variant_fields = variant_field_map(
        "generate:settings",
        "SETTINGS_VARIANT_FIELDS",
        &declarations,
    );

    Ok(RenderedProtocol {
        variant_fields,
        contents,
        fields,
        rejection_codes,
    })
}
