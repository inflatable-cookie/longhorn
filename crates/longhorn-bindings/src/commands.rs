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

use crate::generation::{
    Artifact, GenerationMode, apply, exported_declaration, field_map, string_union_variants,
    tagged_variants,
};

mod fixture;

const GENERATED_PROTOCOL: &str = "packages/longhorn/src/commands/generated/protocol.ts";
const GENERATED_FIELDS: &str = "packages/longhorn/src/commands/generated/fields.ts";
const GOLDEN_FIXTURE: &str = "fixtures/commands/protocol-v1.json";

struct RenderedProtocol {
    contents: String,
    fields: String,
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
            relative_path: GOLDEN_FIXTURE,
            contents: fixture::render()?,
        },
    ];
    apply("commands", "generate:commands", mode, &artifacts)
}

fn render_protocol() -> Result<RenderedProtocol, Box<dyn Error>> {
    let load_outcome = CommandKeymapLoadOutcome::decl();
    let preview_result = CommandKeymapPreviewResult::decl();
    let mutation_result = CommandKeymapMutationResult::decl();
    let load_origin = CommandKeymapLoadOrigin::decl();
    let rejection_code = CommandKeymapRejectionCode::decl();
    let recovery_code = CommandKeymapRecoveryCode::decl();
    let durability = CommandKeymapDurability::decl();
    let mutation_outcome = CommandKeymapMutationOutcome::decl();
    let override_kind = CommandKeymapOverride::decl();

    let declarations = [
        CommandBindingId::decl(),
        CommandCategoryId::decl(),
        CommandContextId::decl(),
        CommandEnumValueId::decl(),
        CommandFieldId::decl(),
        CommandId::decl(),
        CommandKeymapPresetId::decl(),
        CommandRequestId::decl(),
        SchemaVersion::decl(),
        CommandRegistryGeneration::decl(),
        CommandRegistryDigest::decl(),
        CommandContextRevision::decl(),
        CommandKeymapProtocolVersion::decl(),
        CommandKeymapRevision::decl(),
        CommandKeymapPatchDigest::decl(),
        CommandFiniteNumber::decl(),
        CommandArgumentKind::decl(),
        CommandArgumentField::decl(),
        CommandArgumentSchema::decl(),
        CommandArgumentValue::decl(),
        CommandArguments::decl(),
        CommandVisibility::decl(),
        CommandKeyword::decl(),
        CommandTextInputPolicy::decl(),
        CommandDiscoveryRecord::decl(),
        CommandAvailabilityReasonId::decl(),
        CommandDiagnostic::decl(),
        CommandAvailabilityReasonCode::decl(),
        CommandAvailabilityReason::decl(),
        CommandAvailabilityState::decl(),
        CommandAvailability::decl(),
        CommandAvailabilityRecord::decl(),
        CommandAvailabilitySnapshot::decl(),
        CommandSearchHit::decl(),
        CommandPlatform::decl(),
        CommandPlatformScope::decl(),
        CommandPhysicalCode::decl(),
        CommandModifiers::decl(),
        CommandTriggerModifiers::decl(),
        CommandKeyTrigger::decl(),
        CommandKeyChord::decl(),
        CommandKeyboardInput::decl(),
        CommandKeyboardMode::decl(),
        CommandBindingDefinition::decl(),
        CommandBindingReplacement::decl(),
        override_kind.clone(),
        CommandInvocation::decl(),
        CommandBindingSource::decl(),
        CommandEffectiveBinding::decl(),
        CommandShortcutRecord::decl(),
        CommandCandidateDisposition::decl(),
        CommandBindingCandidate::decl(),
        CommandBindingWinner::decl(),
        CommandKeyboardGate::decl(),
        CommandKeyResolution::decl(),
        CommandKeymapConflict::decl(),
        CommandKeymapPresetRecord::decl(),
        CommandCatalogueSnapshot::decl(),
        CommandCatalogueChangedEvent::decl(),
        CommandKeymapState::decl(),
        CommandKeymapPatch::decl(),
        CommandKeymapCommitEvidence::decl(),
        CommandKeymapPreview::decl(),
        CommandKeymapCommit::decl(),
        CommandKeymapReset::decl(),
        load_origin.clone(),
        CommandKeymapDiagnostic::decl(),
        CommandKeymapSnapshot::decl(),
        CommandKeymapChangedEvent::decl(),
        rejection_code.clone(),
        CommandKeymapRejection::decl(),
        preview_result.clone(),
        recovery_code.clone(),
        CommandKeymapRecovery::decl(),
        load_outcome.clone(),
        mutation_outcome.clone(),
        durability.clone(),
        CommandKeymapMutationReceipt::decl(),
        mutation_result.clone(),
    ]
    .map(exported_declaration);

    let contents = format!(
        "// @generated by `effigy generate:commands`; do not edit.\n\
         // Rust serde types are the wire authority.\n\n\
         export const COMMAND_KEYMAP_PROTOCOL_VERSION = {COMMAND_KEYMAP_PROTOCOL_VERSION} as const;\n\
         export const COMMAND_MAXIMUM_PHYSICAL_CODE_BYTES = {MAXIMUM_PHYSICAL_CODE_BYTES} as const;\n\
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
        serde_json::to_string(&tagged_variants(&load_outcome, "status")?)?,
        serde_json::to_string(&tagged_variants(&preview_result, "status")?)?,
        serde_json::to_string(&tagged_variants(&mutation_result, "status")?)?,
        serde_json::to_string(&tagged_variants(&load_origin, "kind")?)?,
        serde_json::to_string(&string_union_variants(&rejection_code)?)?,
        serde_json::to_string(&string_union_variants(&recovery_code)?)?,
        serde_json::to_string(&string_union_variants(&durability)?)?,
        serde_json::to_string(&string_union_variants(&mutation_outcome)?)?,
        serde_json::to_string(&tagged_variants(&override_kind, "kind")?)?,
        declarations.join("\n\n")
    );
    let (fields, skipped) = field_map("generate:commands", "COMMANDS_FIELDS", &declarations);
    if !skipped.is_empty() {
        eprintln!(
            "[commands] tagged unions not in the field map: {}",
            skipped.join(", ")
        );
    }

    Ok(RenderedProtocol { contents, fields })
}
