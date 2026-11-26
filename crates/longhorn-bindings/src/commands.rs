use std::error::Error;

use longhorn_command::{
    CommandArgumentField, CommandArgumentKind, CommandArgumentSchema, CommandArgumentValue,
    CommandArguments, CommandAvailability, CommandAvailabilityReason,
    CommandAvailabilityReasonCode, CommandAvailabilityRecord, CommandAvailabilitySnapshot,
    CommandAvailabilityState, CommandBindingCandidate, CommandBindingDefinition,
    CommandBindingReplacement, CommandBindingSource, CommandBindingWinner,
    CommandCandidateDisposition, CommandContextRevision, CommandDiagnostic, CommandDiscoveryRecord,
    CommandEffectiveBinding, CommandFiniteNumber, CommandInvocation, CommandKeyChord,
    CommandKeyResolution, CommandKeyTrigger, CommandKeyboardGate, CommandKeyboardInput,
    CommandKeyboardMode, CommandKeymapConflict, CommandKeymapOverride, CommandKeyword,
    CommandModifiers, CommandPhysicalCode, CommandPlatform, CommandPlatformScope,
    CommandRegistryDigest, CommandRegistryGeneration, CommandSearchHit, CommandShortcutRecord,
    CommandTextInputPolicy, CommandTriggerModifiers, CommandVisibility,
    MAXIMUM_PHYSICAL_CODE_BYTES,
};
use longhorn_command_config::{
    COMMAND_KEYMAP_PROTOCOL_VERSION, CommandCatalogueChangedEvent, CommandCatalogueSnapshot,
    CommandKeymapChangedEvent, CommandKeymapCommit, CommandKeymapCommitEvidence,
    CommandKeymapDiagnostic, CommandKeymapDurability, CommandKeymapLoadOrigin,
    CommandKeymapLoadOutcome, CommandKeymapMutationOutcome, CommandKeymapMutationReceipt,
    CommandKeymapMutationResult, CommandKeymapPatch, CommandKeymapPatchDigest,
    CommandKeymapPresetRecord, CommandKeymapPreview, CommandKeymapPreviewResult,
    CommandKeymapProtocolVersion, CommandKeymapRecovery, CommandKeymapRecoveryCode,
    CommandKeymapRejection, CommandKeymapRejectionCode, CommandKeymapReset, CommandKeymapRevision,
    CommandKeymapSnapshot, CommandKeymapState,
};
use longhorn_core::{
    CommandAvailabilityReasonId, CommandBindingId, CommandCategoryId, CommandContextId,
    CommandEnumValueId, CommandFieldId, CommandId, CommandKeymapPresetId, CommandRequestId,
    SchemaVersion,
};
use ts_rs::TS;

use crate::{
    generation::{
        Artifact, DomainArtifacts, GENERATE_TASK, build_domain_artifacts, config,
        exported_declaration, tagged_variants_json,
    },
    serde_shape::enum_names_json,
};

mod fixture;

// Compile the adapter's wire declarations without linking the Tauri runtime.
#[allow(dead_code)]
#[path = "../../longhorn-tauri-command/src/error.rs"]
mod host_error;

const ARTIFACTS: DomainArtifacts = DomainArtifacts {
    protocol: "packages/longhorn/src/commands/generated/protocol.ts",
    fields: "packages/longhorn/src/commands/generated/fields.ts",
    fields_constant: "COMMANDS_FIELDS",
    variant_fields: "packages/longhorn/src/commands/generated/variant-fields.ts",
    variant_fields_constant: "COMMAND_VARIANT_FIELDS",
    fixture: "fixtures/commands/protocol-v1.json",
};

/// Renders the command bindings and golden fixtures.
///
/// # Errors
///
/// Returns an error if declaration inspection or fixture serialization fails.
pub fn render() -> Result<Vec<Artifact>, Box<dyn Error>> {
    let load_outcome = CommandKeymapLoadOutcome::decl(config());
    let preview_result = CommandKeymapPreviewResult::decl(config());
    let mutation_result = CommandKeymapMutationResult::decl(config());
    let load_origin = CommandKeymapLoadOrigin::decl(config());
    let reason_code = CommandAvailabilityReasonCode::decl(config());
    let binding_source = CommandBindingSource::decl(config());
    let argument_schema = CommandArgumentSchema::decl(config());
    let override_kind = CommandKeymapOverride::decl(config());

    let [host_error_code, host_error] =
        crate::serde_shape::error_declarations::<_, host_error::CommandHostErrorCode>(
            host_error::CommandHostError::authority("fixture", true),
            "CommandHostError",
            "CommandHostErrorCode",
        )?;

    let declarations = [
        CommandBindingId::decl(config()),
        CommandCategoryId::decl(config()),
        CommandContextId::decl(config()),
        CommandEnumValueId::decl(config()),
        CommandFieldId::decl(config()),
        CommandId::decl(config()),
        CommandKeymapPresetId::decl(config()),
        CommandRequestId::decl(config()),
        SchemaVersion::decl(config()),
        CommandRegistryGeneration::decl(config()),
        CommandRegistryDigest::decl(config()),
        CommandContextRevision::decl(config()),
        CommandKeymapProtocolVersion::decl(config()),
        CommandKeymapRevision::decl(config()),
        CommandKeymapPatchDigest::decl(config()),
        CommandFiniteNumber::decl(config()),
        CommandArgumentKind::decl(config()),
        CommandArgumentField::decl(config()),
        argument_schema.clone(),
        CommandArgumentValue::decl(config()),
        CommandArguments::decl(config()),
        CommandVisibility::decl(config()),
        CommandKeyword::decl(config()),
        CommandTextInputPolicy::decl(config()),
        CommandDiscoveryRecord::decl(config()),
        CommandAvailabilityReasonId::decl(config()),
        CommandDiagnostic::decl(config()),
        reason_code.clone(),
        CommandAvailabilityReason::decl(config()),
        CommandAvailabilityState::decl(config()),
        CommandAvailability::decl(config()),
        CommandAvailabilityRecord::decl(config()),
        CommandAvailabilitySnapshot::decl(config()),
        CommandSearchHit::decl(config()),
        CommandPlatform::decl(config()),
        CommandPlatformScope::decl(config()),
        CommandPhysicalCode::decl(config()),
        CommandModifiers::decl(config()),
        CommandTriggerModifiers::decl(config()),
        CommandKeyTrigger::decl(config()),
        CommandKeyChord::decl(config()),
        CommandKeyboardInput::decl(config()),
        CommandKeyboardMode::decl(config()),
        CommandBindingDefinition::decl(config()),
        CommandBindingReplacement::decl(config()),
        override_kind.clone(),
        CommandInvocation::decl(config()),
        binding_source.clone(),
        CommandEffectiveBinding::decl(config()),
        CommandShortcutRecord::decl(config()),
        CommandCandidateDisposition::decl(config()),
        CommandBindingCandidate::decl(config()),
        CommandBindingWinner::decl(config()),
        CommandKeyboardGate::decl(config()),
        CommandKeyResolution::decl(config()),
        CommandKeymapConflict::decl(config()),
        CommandKeymapPresetRecord::decl(config()),
        CommandCatalogueSnapshot::decl(config()),
        CommandCatalogueChangedEvent::decl(config()),
        CommandKeymapState::decl(config()),
        CommandKeymapPatch::decl(config()),
        CommandKeymapCommitEvidence::decl(config()),
        CommandKeymapPreview::decl(config()),
        CommandKeymapCommit::decl(config()),
        CommandKeymapReset::decl(config()),
        load_origin.clone(),
        CommandKeymapDiagnostic::decl(config()),
        CommandKeymapSnapshot::decl(config()),
        CommandKeymapChangedEvent::decl(config()),
        CommandKeymapRejectionCode::decl(config()),
        CommandKeymapRejection::decl(config()),
        preview_result.clone(),
        CommandKeymapRecoveryCode::decl(config()),
        CommandKeymapRecovery::decl(config()),
        load_outcome.clone(),
        CommandKeymapMutationOutcome::decl(config()),
        CommandKeymapDurability::decl(config()),
        CommandKeymapMutationReceipt::decl(config()),
        mutation_result.clone(),
        host_error_code,
        host_error,
    ]
    .map(exported_declaration);

    let contents = format!(
        "// @generated by `effigy {GENERATE_TASK}`; do not edit.\n\
         // Rust serde types are the wire authority.\n\n\
         export const COMMAND_KEYMAP_PROTOCOL_VERSION = {COMMAND_KEYMAP_PROTOCOL_VERSION} as const;\n\
         export const COMMAND_MAXIMUM_PHYSICAL_CODE_BYTES = {MAXIMUM_PHYSICAL_CODE_BYTES} as const;\n\
         export const COMMAND_AVAILABILITY_STATES = {} as const;\n\
         export const COMMAND_AVAILABILITY_REASON_KINDS = {} as const;\n\
         export const COMMAND_BINDING_SOURCE_KINDS = {} as const;\n\
         export const COMMAND_ARGUMENT_SCHEMA_SHAPES = {} as const;\n\
         export const COMMAND_KEYMAP_LOAD_STATUSES = {} as const;\n\
         export const COMMAND_KEYMAP_PREVIEW_STATUSES = {} as const;\n\
         export const COMMAND_KEYMAP_MUTATION_STATUSES = {} as const;\n\
         export const COMMAND_KEYMAP_LOAD_ORIGINS = {} as const;\n\
         export const COMMAND_KEYMAP_REJECTION_CODES = {} as const;\n\
         export const COMMAND_KEYMAP_RECOVERY_CODES = {} as const;\n\
         export const COMMAND_KEYMAP_DURABILITIES = {} as const;\n\
         export const COMMAND_KEYMAP_MUTATION_OUTCOMES = {} as const;\n\
         export const COMMAND_KEYMAP_OVERRIDE_KINDS = {} as const;\n\n\
         {}\n",
        enum_names_json::<CommandAvailabilityState>()?,
        tagged_variants_json(&reason_code)?,
        tagged_variants_json(&binding_source)?,
        tagged_variants_json(&argument_schema)?,
        tagged_variants_json(&load_outcome)?,
        tagged_variants_json(&preview_result)?,
        tagged_variants_json(&mutation_result)?,
        tagged_variants_json(&load_origin)?,
        enum_names_json::<CommandKeymapRejectionCode>()?,
        enum_names_json::<CommandKeymapRecoveryCode>()?,
        enum_names_json::<CommandKeymapDurability>()?,
        enum_names_json::<CommandKeymapMutationOutcome>()?,
        tagged_variants_json(&override_kind)?,
        declarations.join("\n\n")
    );

    let mut artifacts =
        build_domain_artifacts(&ARTIFACTS, contents, &declarations, fixture::render()?)?;
    artifacts.push(crate::serde_shape::error_fixture(
        "fixtures/commands/host-errors.json",
        &[
            host_error::CommandHostError::authority("Host temporarily unavailable", true),
            host_error::CommandHostError::state_unavailable(),
        ],
    )?);
    Ok(artifacts)
}
