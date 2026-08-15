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
    Artifact, GenerationMode, apply, config, exported_declaration, field_map,
    string_union_variants, tagged_variants, variant_field_map,
};

mod fixture;

const GENERATED_PROTOCOL: &str = "packages/longhorn/src/commands/generated/protocol.ts";
const GENERATED_FIELDS: &str = "packages/longhorn/src/commands/generated/fields.ts";
const GENERATED_VARIANT_FIELDS: &str = "packages/longhorn/src/commands/generated/variant-fields.ts";
const GOLDEN_FIXTURE: &str = "fixtures/commands/protocol-v1.json";

struct RenderedProtocol {
    contents: String,
    fields: String,
    variant_fields: String,
}

/// Generates or checks the command bindings and golden fixtures.
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
            contents: fixture::render()?,
        },
    ];
    apply("commands", "generate:commands", mode, &artifacts)
}

fn render_protocol() -> Result<RenderedProtocol, Box<dyn Error>> {
    let load_outcome = CommandKeymapLoadOutcome::decl(config());
    let preview_result = CommandKeymapPreviewResult::decl(config());
    let mutation_result = CommandKeymapMutationResult::decl(config());
    let load_origin = CommandKeymapLoadOrigin::decl(config());
    let rejection_code = CommandKeymapRejectionCode::decl(config());
    let recovery_code = CommandKeymapRecoveryCode::decl(config());
    let durability = CommandKeymapDurability::decl(config());
    let mutation_outcome = CommandKeymapMutationOutcome::decl(config());
    let override_kind = CommandKeymapOverride::decl(config());

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
        CommandArgumentSchema::decl(config()),
        CommandArgumentValue::decl(config()),
        CommandArguments::decl(config()),
        CommandVisibility::decl(config()),
        CommandKeyword::decl(config()),
        CommandTextInputPolicy::decl(config()),
        CommandDiscoveryRecord::decl(config()),
        CommandAvailabilityReasonId::decl(config()),
        CommandDiagnostic::decl(config()),
        CommandAvailabilityReasonCode::decl(config()),
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
        CommandBindingSource::decl(config()),
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
        rejection_code.clone(),
        CommandKeymapRejection::decl(config()),
        preview_result.clone(),
        recovery_code.clone(),
        CommandKeymapRecovery::decl(config()),
        load_outcome.clone(),
        mutation_outcome.clone(),
        durability.clone(),
        CommandKeymapMutationReceipt::decl(config()),
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
    let (fields, _skipped) = field_map("generate:commands", "COMMANDS_FIELDS", &declarations);

    let variant_fields =
        variant_field_map("generate:commands", "COMMAND_VARIANT_FIELDS", &declarations);

    Ok(RenderedProtocol {
        contents,
        fields,
        variant_fields,
    })
}
