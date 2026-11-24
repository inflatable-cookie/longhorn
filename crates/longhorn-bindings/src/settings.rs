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

use crate::{
    generation::{
        Artifact, DomainArtifacts, GENERATE_TASK, build_domain_artifacts, config,
        exported_declaration, tagged_variants_json,
    },
    serde_shape::{enum_names, enum_names_json},
};

mod fixture;

// Compile the adapter's wire declarations without linking the Tauri runtime.
#[allow(dead_code)]
#[path = "../../longhorn-tauri-settings/src/error.rs"]
mod host_error;

const ARTIFACTS: DomainArtifacts = DomainArtifacts {
    protocol: "packages/longhorn/src/settings/generated/protocol.ts",
    fields: "packages/longhorn/src/settings/generated/fields.ts",
    fields_constant: "SETTINGS_FIELDS",
    variant_fields: "packages/longhorn/src/settings/generated/variant-fields.ts",
    variant_fields_constant: "SETTINGS_VARIANT_FIELDS",
    fixture: "fixtures/settings/protocol-v1.json",
};

/// Renders the settings bindings and golden fixtures.
///
/// # Errors
///
/// Returns an error if declaration inspection or fixture serialization fails.
pub fn render() -> Result<Vec<Artifact>, Box<dyn Error>> {
    let durability = SettingsDurabilityEvidence::decl(config());
    let mutation_result = SettingsMutationResult::decl(config());
    let load_outcome = SettingsLoadOutcome::decl(config());
    let rejection_codes = enum_names::<SettingsRejectionCode>()?;

    let [host_error_code, host_error] =
        crate::serde_shape::error_declarations::<_, host_error::SettingsHostErrorCode>(
            host_error::SettingsHostError::state_unavailable(),
            "SettingsHostError",
            "SettingsHostErrorCode",
        )?;

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
        SettingsMutationTiming::decl(config()),
        SettingsApplyUnitDefinition::decl(config()),
        SettingsAnchorDefinition::decl(config()),
        SettingsPageFeatures::decl(config()),
        SettingsPageDefinition::decl(config()),
        SettingsEffectiveSource::decl(config()),
        SettingsEditability::decl(config()),
        SettingsPolicyEffect::decl(config()),
        SettingsPolicyProjection::decl(config()),
        SettingsSourceDiagnostic::decl(config()),
        SettingsValueProjection::decl(config()),
        SettingsRecoveryCode::decl(config()),
        SettingsRecoveryState::decl(config()),
        SettingsActivationState::decl(config()),
        SettingsActivationRequirement::decl(config()),
        SettingsAuthorityExpectation::decl(config()),
        SettingsScopeSnapshot::decl(config()),
        SettingsRegistrySnapshot::decl(config()),
        SettingsRegistryChangedEvent::decl(config()),
        SettingsScopeChangedEvent::decl(config()),
        SettingsLoadCommand::decl(config()),
        SettingsApplyCommand::decl(config()),
        SettingsResetCommand::decl(config()),
        SettingsMutationOutcome::decl(config()),
        durability.clone(),
        SettingsMutationReceipt::decl(config()),
        SettingsConflict::decl(config()),
        SettingsRejectionCode::decl(config()),
        SettingsRejection::decl(config()),
        mutation_result.clone(),
        load_outcome.clone(),
        host_error_code,
        host_error,
    ]
    .map(exported_declaration);

    // Inline format arguments take identifiers, not paths.
    let hard_maximum_text_bytes = SettingsLimits::HARD_MAXIMUM_TEXT_BYTES;
    let hard_maximum_opaque_value_bytes = SettingsLimits::HARD_MAXIMUM_OPAQUE_VALUE_BYTES;
    let contents = format!(
        "// @generated by `effigy {GENERATE_TASK}`; do not edit.\n\
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
        tagged_variants_json(&durability)?,
        tagged_variants_json(&mutation_result)?,
        tagged_variants_json(&load_outcome)?,
        serde_json::to_string(rejection_codes)?,
        enum_names_json::<SettingsMutationTiming>()?,
        enum_names_json::<SettingsEffectiveSource>()?,
        enum_names_json::<SettingsEditability>()?,
        enum_names_json::<SettingsPolicyEffect>()?,
        enum_names_json::<SettingsRecoveryCode>()?,
        enum_names_json::<SettingsActivationState>()?,
        enum_names_json::<SettingsMutationOutcome>()?,
        declarations.join("\n\n")
    );

    let mut artifacts = build_domain_artifacts(
        &ARTIFACTS,
        contents,
        &declarations,
        fixture::render(rejection_codes)?,
    )?;
    artifacts.push(crate::serde_shape::error_fixture(
        "fixtures/settings/host-errors.json",
        &[
            host_error::SettingsHostError::authority("Host temporarily unavailable", true),
            host_error::SettingsHostError::state_unavailable(),
        ],
    )?);
    Ok(artifacts)
}
