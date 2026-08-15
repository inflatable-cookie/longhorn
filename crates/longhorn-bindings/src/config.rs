use std::error::Error;

use longhorn_core::MAX_OPAQUE_ID_BYTES;

use longhorn_config::{
    BackupArchiveProjection, BackupCaptureReceiptProjection, BackupCreateCommand,
    BackupCreateOutcome, BackupEncryptionState, BackupExportCommand, BackupExportOutcome,
    BackupInventoryEntry, BackupInventoryEntryState, BackupInventoryProjection,
    BackupOperationsProjection, BackupPendingState, BackupPublicationReceiptProjection,
    BackupRetentionApplyCommand, BackupRetentionApplyOutcome, BackupRetentionProjection,
    BackupRetentionReasonProjection, CONFIG_OPERATIONS_PROTOCOL_VERSION, ConfigGeneration,
    ConfigOperationCapability, ConfigOperationRejection, ConfigOperationRejectionCode,
    ConfigOperationsSnapshot, ConfigProtocolVersion, ConfigSnapshotCommand, PendingBackupPolicy,
    RestoreAdapterExecuteCommand, RestoreAdapterExecuteOutcome,
    RestoreAdapterParticipationProjection, RestoreAdapterReceiptProjection,
    RestoreAdapterRequirementProjection, RestoreArchiveSelection, RestoreAuthenticityProjection,
    RestoreConflictChoiceProjection, RestoreConsistencyGroupProjection,
    RestoreCurrentEvidenceProjection, RestoreDomainChoice, RestoreDomainCompatibilityProjection,
    RestoreDomainInspectionProjection, RestoreExclusionProjection, RestoreExecuteCommand,
    RestoreExecuteOutcome, RestoreExecutionFailureProjection, RestoreExecutionReceiptProjection,
    RestoreIdentityProjection, RestoreIdentityStatusProjection, RestoreInspectCommand,
    RestoreInspectOutcome, RestoreInspectionProjection, RestoreInspectionReceiptProjection,
    RestoreIntegrityProjection, RestoreOperationStateProjection, RestoreOperationsProjection,
    RestorePlanCommand, RestorePlanEntryProjection, RestorePlanOutcome, RestorePlanProjection,
    RestorePlanReceiptProjection, RestoreRecoveryCommand, RestoreRecoveryOutcomeProjection,
    RestoreRecoveryReceiptProjection, RestoreStagingReceiptProjection, StorageBootstrapProjection,
    StorageCleanupCommand, StorageCleanupOutcome, StorageCleanupReceiptProjection,
    StorageLayoutProjection, StorageLeafProvenanceProjection, StorageOperationsProjection,
    StorageProfileId, StorageRecoveryCommand, StorageRecoveryOutcome,
    StorageRecoveryReceiptProjection, StorageRootProjection, StorageTransitionConflictProjection,
    StorageTransitionDomainProjection, StorageTransitionExecuteCommand,
    StorageTransitionExecuteOutcome, StorageTransitionInspectCommand,
    StorageTransitionInspectOutcome, StorageTransitionPreviewProjection,
    StorageTransitionReceiptProjection,
};
use longhorn_core::ConfigRequestId;
use ts_rs::TS;

use crate::generation::{
    Artifact, GenerationMode, LabelMap, apply, config, exported_declaration, field_map,
    label_module, label_template_renderer, string_union_variants, tagged_variants,
    variant_field_map,
};

mod fixture;

const GENERATED_PROTOCOL: &str = "packages/longhorn/src/config/generated/protocol.ts";
const GENERATED_BASE_PROTOCOL: &str = "packages/longhorn/src/config/generated/base.ts";
const GENERATED_RESTORE_PROTOCOL: &str = "packages/longhorn/src/config/generated/restore.ts";
const GOLDEN_FIXTURE: &str = "fixtures/config/protocol-v1.json";
const GENERATED_LABELS: &str = "packages/longhorn/src/config/generated/labels.ts";
const GENERATED_FIELDS: &str = "packages/longhorn/src/config/generated/fields.ts";
const GENERATED_VARIANT_FIELDS: &str = "packages/longhorn/src/config/generated/variant-fields.ts";
const GENERATED_LABEL_TEMPLATE: &str = "packages/longhorn/src/config/generated/label-template.ts";

/// Emits the three restore label maps.
///
/// Compatibility carries *templates* rather than finished strings, because six
/// of its thirteen classifications interpolate their own fields — a table
/// entry cannot say "Migration required (3 → 7)". Rust renders from the same
/// templates, so one source still decides the wording. See Card 170.
fn render_labels() -> String {
    let integrity: Vec<(&str, &str)> = RestoreIntegrityProjection::ALL
        .iter()
        .map(|state| (state.wire_name(), state.label()))
        .collect();
    let authenticity: Vec<(&str, &str)> = RestoreAuthenticityProjection::ALL
        .iter()
        .map(|state| (state.wire_name(), state.label()))
        .collect();

    label_module(
        "generate:config",
        &[
            LabelMap {
                constant: "RESTORE_INTEGRITY_LABELS",
                import: "RestoreIntegrityProjection",
                key_type: "RestoreIntegrityProjection",
                entries: &integrity,
            },
            LabelMap {
                constant: "RESTORE_AUTHENTICITY_LABELS",
                import: "RestoreAuthenticityProjection",
                key_type: "RestoreAuthenticityProjection",
                entries: &authenticity,
            },
            LabelMap {
                constant: "RESTORE_COMPATIBILITY_LABEL_TEMPLATES",
                import: "RestoreDomainCompatibilityProjection",
                // A tagged union's discriminant is not importable on its own,
                // so the record is keyed by indexing the union.
                key_type: "RestoreDomainCompatibilityProjection[\"status\"]",
                entries: &RestoreDomainCompatibilityProjection::TEMPLATES,
            },
        ],
    )
}

/// Generates or checks the config bindings and golden fixtures.
pub fn run(mode: GenerationMode) -> Result<(), Box<dyn Error>> {
    let capability = ConfigOperationCapability::decl(config());
    let rejection = ConfigOperationRejectionCode::decl(config());
    let bootstrap = StorageBootstrapProjection::decl(config());
    let pending = BackupPendingState::decl(config());
    let encryption = BackupEncryptionState::decl(config());
    let inspect = StorageTransitionInspectOutcome::decl(config());
    let execute = StorageTransitionExecuteOutcome::decl(config());
    let recovery = StorageRecoveryOutcome::decl(config());
    let cleanup = StorageCleanupOutcome::decl(config());
    let create = BackupCreateOutcome::decl(config());
    let export = BackupExportOutcome::decl(config());
    let retention = BackupRetentionApplyOutcome::decl(config());
    let restore_selection = RestoreArchiveSelection::decl(config());
    let restore_identity = RestoreIdentityStatusProjection::decl(config());
    let restore_compatibility = RestoreDomainCompatibilityProjection::decl(config());
    let restore_participation = RestoreAdapterParticipationProjection::decl(config());
    let restore_current = RestoreCurrentEvidenceProjection::decl(config());
    let restore_inspect = RestoreInspectOutcome::decl(config());
    let restore_plan = RestorePlanOutcome::decl(config());
    let restore_execute = RestoreExecuteOutcome::decl(config());
    let restore_adapter = RestoreAdapterExecuteOutcome::decl(config());
    let restore_recovery = RestoreRecoveryOutcomeProjection::decl(config());

    let base_declarations = [
        ConfigProtocolVersion::decl(config()),
        ConfigGeneration::decl(config()),
        ConfigRequestId::decl(config()),
        capability.clone(),
        StorageProfileId::decl(config()),
        StorageLeafProvenanceProjection::decl(config()),
        StorageRootProjection::decl(config()),
        StorageLayoutProjection::decl(config()),
        bootstrap.clone(),
        StorageOperationsProjection::decl(config()),
        StorageTransitionDomainProjection::decl(config()),
        StorageTransitionConflictProjection::decl(config()),
        StorageTransitionPreviewProjection::decl(config()),
        StorageTransitionReceiptProjection::decl(config()),
        StorageRecoveryReceiptProjection::decl(config()),
        StorageCleanupReceiptProjection::decl(config()),
        BackupArchiveProjection::decl(config()),
        BackupInventoryEntryState::decl(config()),
        BackupInventoryEntry::decl(config()),
        BackupInventoryProjection::decl(config()),
        pending.clone(),
        encryption.clone(),
        BackupRetentionReasonProjection::decl(config()),
        BackupRetentionProjection::decl(config()),
        BackupOperationsProjection::decl(config()),
        ConfigOperationsSnapshot::decl(config()),
        ConfigSnapshotCommand::decl(config()),
        StorageTransitionInspectCommand::decl(config()),
        StorageTransitionExecuteCommand::decl(config()),
        StorageRecoveryCommand::decl(config()),
        StorageCleanupCommand::decl(config()),
        PendingBackupPolicy::decl(config()),
        BackupCreateCommand::decl(config()),
        BackupExportCommand::decl(config()),
        BackupRetentionApplyCommand::decl(config()),
        rejection.clone(),
        ConfigOperationRejection::decl(config()),
        inspect.clone(),
        execute.clone(),
        recovery.clone(),
        cleanup.clone(),
        BackupCaptureReceiptProjection::decl(config()),
        BackupPublicationReceiptProjection::decl(config()),
        create.clone(),
        export.clone(),
        retention.clone(),
    ]
    .map(exported_declaration);
    let restore_declarations = [
        RestoreOperationStateProjection::decl(config()),
        RestoreOperationsProjection::decl(config()),
        restore_selection.clone(),
        RestoreIntegrityProjection::decl(config()),
        RestoreAuthenticityProjection::decl(config()),
        restore_identity.clone(),
        RestoreIdentityProjection::decl(config()),
        RestoreConsistencyGroupProjection::decl(config()),
        restore_compatibility.clone(),
        restore_participation.clone(),
        RestoreDomainInspectionProjection::decl(config()),
        RestoreExclusionProjection::decl(config()),
        RestoreInspectionReceiptProjection::decl(config()),
        RestoreInspectionProjection::decl(config()),
        RestoreInspectCommand::decl(config()),
        restore_inspect.clone(),
        RestoreConflictChoiceProjection::decl(config()),
        RestoreDomainChoice::decl(config()),
        restore_current.clone(),
        RestorePlanEntryProjection::decl(config()),
        RestorePlanReceiptProjection::decl(config()),
        RestorePlanProjection::decl(config()),
        RestorePlanCommand::decl(config()),
        restore_plan.clone(),
        RestoreExecuteCommand::decl(config()),
        RestoreStagingReceiptProjection::decl(config()),
        RestoreExecutionReceiptProjection::decl(config()),
        RestoreExecutionFailureProjection::decl(config()),
        restore_execute.clone(),
        RestoreAdapterRequirementProjection::decl(config()),
        RestoreAdapterExecuteCommand::decl(config()),
        RestoreAdapterReceiptProjection::decl(config()),
        restore_adapter.clone(),
        RestoreRecoveryCommand::decl(config()),
        RestoreRecoveryReceiptProjection::decl(config()),
        restore_recovery.clone(),
    ]
    .map(exported_declaration);

    // Both modules in one map: the validators import from `protocol.ts`,
    // which re-exports both, so splitting the map would only make callers
    // guess which half a type lives in.
    let mut all_declarations = base_declarations.to_vec();
    all_declarations.extend_from_slice(&restore_declarations);
    let (fields_contents, _skipped) =
        field_map("generate:config", "CONFIG_FIELDS", &all_declarations);
    let variant_fields_contents = variant_field_map(
        "generate:config",
        "CONFIG_VARIANT_FIELDS",
        &all_declarations,
    );

    let protocol_contents = "// @generated by `effigy generate:config`; do not edit.\n\
         export * from \"./base.ts\";\n\
         export * from \"./restore.ts\";\n"
        .to_owned();
    let base_contents = format!(
        "// @generated by `effigy generate:config`; do not edit.\n\
         // Rust serde types are the wire authority.\n\
         import type {{ RestoreOperationsProjection }} from \"./restore.ts\";\n\n\
         export const CONFIG_OPERATIONS_PROTOCOL_VERSION = \
         {CONFIG_OPERATIONS_PROTOCOL_VERSION} as const;\n\
         export const CONFIG_MAXIMUM_OPAQUE_ID_BYTES = {MAX_OPAQUE_ID_BYTES} as const;\n\
         export const CONFIG_OPERATION_CAPABILITIES = {} as const;\n\
         export const CONFIG_OPERATION_REJECTION_CODES = {} as const;\n\
         export const STORAGE_BOOTSTRAP_STATES = {} as const;\n\
         export const BACKUP_PENDING_STATES = {} as const;\n\
         export const BACKUP_ENCRYPTION_STATES = {} as const;\n\
         export const STORAGE_TRANSITION_INSPECT_STATUSES = {} as const;\n\
         export const STORAGE_TRANSITION_EXECUTE_STATUSES = {} as const;\n\
         export const STORAGE_RECOVERY_STATUSES = {} as const;\n\
         export const STORAGE_CLEANUP_STATUSES = {} as const;\n\
         export const BACKUP_CREATE_STATUSES = {} as const;\n\
         export const BACKUP_EXPORT_STATUSES = {} as const;\n\
         export const BACKUP_RETENTION_APPLY_STATUSES = {} as const;\n\n\
         {}\n",
        serde_json::to_string(&string_union_variants(&capability)?)?,
        serde_json::to_string(&string_union_variants(&rejection)?)?,
        serde_json::to_string(&tagged_variants(&bootstrap, "state")?)?,
        serde_json::to_string(&tagged_variants(&pending, "state")?)?,
        serde_json::to_string(&tagged_variants(&encryption, "state")?)?,
        serde_json::to_string(&tagged_variants(&inspect, "status")?)?,
        serde_json::to_string(&tagged_variants(&execute, "status")?)?,
        serde_json::to_string(&tagged_variants(&recovery, "status")?)?,
        serde_json::to_string(&tagged_variants(&cleanup, "status")?)?,
        serde_json::to_string(&tagged_variants(&create, "status")?)?,
        serde_json::to_string(&tagged_variants(&export, "status")?)?,
        serde_json::to_string(&tagged_variants(&retention, "status")?)?,
        base_declarations.join("\n\n")
    );
    let restore_contents = format!(
        "// @generated by `effigy generate:config`; do not edit.\n\
         // Rust serde types are the wire authority.\n\
         import type {{ BackupPublicationReceiptProjection, ConfigGeneration, \
         ConfigOperationRejection, ConfigOperationsSnapshot, ConfigProtocolVersion, \
         ConfigRequestId }} from \"./base.ts\";\n\n\
         export const RESTORE_ARCHIVE_SELECTION_SOURCES = {} as const;\n\
         export const RESTORE_IDENTITY_STATUSES = {} as const;\n\
         export const RESTORE_DOMAIN_COMPATIBILITY_STATUSES = {} as const;\n\
         export const RESTORE_ADAPTER_PARTICIPATION_KINDS = {} as const;\n\
         export const RESTORE_CURRENT_EVIDENCE_STATES = {} as const;\n\
         export const RESTORE_INSPECT_STATUSES = {} as const;\n\
         export const RESTORE_PLAN_STATUSES = {} as const;\n\
         export const RESTORE_EXECUTE_STATUSES = {} as const;\n\
         export const RESTORE_ADAPTER_EXECUTE_STATUSES = {} as const;\n\
         export const RESTORE_RECOVERY_STATUSES = {} as const;\n\n\
         {}\n",
        serde_json::to_string(&tagged_variants(&restore_selection, "source")?)?,
        serde_json::to_string(&tagged_variants(&restore_identity, "status")?)?,
        serde_json::to_string(&tagged_variants(&restore_compatibility, "status")?)?,
        serde_json::to_string(&tagged_variants(&restore_participation, "kind")?)?,
        serde_json::to_string(&tagged_variants(&restore_current, "state")?)?,
        serde_json::to_string(&tagged_variants(&restore_inspect, "status")?)?,
        serde_json::to_string(&tagged_variants(&restore_plan, "status")?)?,
        serde_json::to_string(&tagged_variants(&restore_execute, "status")?)?,
        serde_json::to_string(&tagged_variants(&restore_adapter, "status")?)?,
        serde_json::to_string(&tagged_variants(&restore_recovery, "status")?)?,
        restore_declarations.join("\n\n")
    );
    let artifacts = [
        Artifact {
            relative_path: GENERATED_PROTOCOL,
            contents: protocol_contents,
        },
        Artifact {
            relative_path: GENERATED_BASE_PROTOCOL,
            contents: base_contents,
        },
        Artifact {
            relative_path: GENERATED_RESTORE_PROTOCOL,
            contents: restore_contents,
        },
        Artifact {
            relative_path: GENERATED_FIELDS,
            contents: fields_contents,
        },
        Artifact {
            relative_path: GENERATED_VARIANT_FIELDS,
            contents: variant_fields_contents,
        },
        Artifact {
            relative_path: GENERATED_LABELS,
            contents: render_labels(),
        },
        Artifact {
            relative_path: GENERATED_LABEL_TEMPLATE,
            contents: label_template_renderer("generate:config"),
        },
        Artifact {
            relative_path: GOLDEN_FIXTURE,
            contents: fixture::render()?,
        },
    ];
    apply("config", "generate:config", mode, &artifacts)
}
