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
    Artifact, GenerationMode, apply, exported_declaration, field_map, string_union_variants,
    tagged_variants, variant_field_map,
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
    let durability = SettingsDurabilityEvidence::decl();
    let mutation_result = SettingsMutationResult::decl();
    let load_outcome = SettingsLoadOutcome::decl();
    let rejection = SettingsRejectionCode::decl();
    let mutation_timing = SettingsMutationTiming::decl();
    let effective_source = SettingsEffectiveSource::decl();
    let editability = SettingsEditability::decl();
    let policy_effect = SettingsPolicyEffect::decl();
    let recovery_code = SettingsRecoveryCode::decl();
    let activation_state = SettingsActivationState::decl();
    let mutation_outcome = SettingsMutationOutcome::decl();

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
        SettingsModuleId::decl(),
        SettingsSectionId::decl(),
        SettingsPageId::decl(),
        SettingsRendererId::decl(),
        SettingsAnchorId::decl(),
        SettingsScopeId::decl(),
        SettingsApplyUnitId::decl(),
        SettingsCapabilityId::decl(),
        SettingsActivationTargetId::decl(),
        SettingsEntryId::decl(),
        SettingsRequestId::decl(),
        SettingsPolicySourceId::decl(),
        SettingsAuthorityToken::decl(),
        SettingsProtocolVersion::decl(),
        SettingsScopeRevision::decl(),
        SettingsRegistryGeneration::decl(),
        SettingsRegistryDigest::decl(),
        SettingsLimits::decl(),
        SettingsOpaqueValue::decl(),
        SettingsModuleDefinition::decl(),
        SettingsSectionDefinition::decl(),
        SettingsRendererDefinition::decl(),
        SettingsScopeDefinition::decl(),
        SettingsCapabilityDefinition::decl(),
        mutation_timing,
        SettingsApplyUnitDefinition::decl(),
        SettingsAnchorDefinition::decl(),
        SettingsPageFeatures::decl(),
        SettingsPageDefinition::decl(),
        effective_source,
        editability,
        policy_effect,
        SettingsPolicyProjection::decl(),
        SettingsSourceDiagnostic::decl(),
        SettingsValueProjection::decl(),
        recovery_code,
        SettingsRecoveryState::decl(),
        activation_state,
        SettingsActivationRequirement::decl(),
        SettingsAuthorityExpectation::decl(),
        SettingsScopeSnapshot::decl(),
        SettingsRegistrySnapshot::decl(),
        SettingsRegistryChangedEvent::decl(),
        SettingsScopeChangedEvent::decl(),
        SettingsLoadCommand::decl(),
        SettingsApplyCommand::decl(),
        SettingsResetCommand::decl(),
        mutation_outcome,
        durability,
        SettingsMutationReceipt::decl(),
        SettingsConflict::decl(),
        rejection,
        SettingsRejection::decl(),
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
