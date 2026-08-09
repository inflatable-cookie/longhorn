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
    Artifact, GenerationMode, LabelMap, apply, exported_declaration, label_module,
    label_template_renderer, string_union_variants, tagged_variants,
};

mod fixture;

const GENERATED_PROTOCOL: &str = "packages/longhorn/src/config/generated/protocol.ts";
const GENERATED_BASE_PROTOCOL: &str = "packages/longhorn/src/config/generated/base.ts";
const GENERATED_RESTORE_PROTOCOL: &str = "packages/longhorn/src/config/generated/restore.ts";
const GOLDEN_FIXTURE: &str = "fixtures/config/protocol-v1.json";
const GENERATED_LABELS: &str = "packages/longhorn/src/config/generated/labels.ts";
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

pub fn run(mode: GenerationMode) -> Result<(), Box<dyn Error>> {
    let capability = ConfigOperationCapability::decl();
    let rejection = ConfigOperationRejectionCode::decl();
    let bootstrap = StorageBootstrapProjection::decl();
    let pending = BackupPendingState::decl();
    let encryption = BackupEncryptionState::decl();
    let inspect = StorageTransitionInspectOutcome::decl();
    let execute = StorageTransitionExecuteOutcome::decl();
    let recovery = StorageRecoveryOutcome::decl();
    let cleanup = StorageCleanupOutcome::decl();
    let create = BackupCreateOutcome::decl();
    let export = BackupExportOutcome::decl();
    let retention = BackupRetentionApplyOutcome::decl();
    let restore_selection = RestoreArchiveSelection::decl();
    let restore_identity = RestoreIdentityStatusProjection::decl();
    let restore_compatibility = RestoreDomainCompatibilityProjection::decl();
    let restore_participation = RestoreAdapterParticipationProjection::decl();
    let restore_current = RestoreCurrentEvidenceProjection::decl();
    let restore_inspect = RestoreInspectOutcome::decl();
    let restore_plan = RestorePlanOutcome::decl();
    let restore_execute = RestoreExecuteOutcome::decl();
    let restore_adapter = RestoreAdapterExecuteOutcome::decl();
    let restore_recovery = RestoreRecoveryOutcomeProjection::decl();

    let base_declarations = [
        ConfigProtocolVersion::decl(),
        ConfigGeneration::decl(),
        ConfigRequestId::decl(),
        capability.clone(),
        StorageProfileId::decl(),
        StorageLeafProvenanceProjection::decl(),
        StorageRootProjection::decl(),
        StorageLayoutProjection::decl(),
        bootstrap.clone(),
        StorageOperationsProjection::decl(),
        StorageTransitionDomainProjection::decl(),
        StorageTransitionConflictProjection::decl(),
        StorageTransitionPreviewProjection::decl(),
        StorageTransitionReceiptProjection::decl(),
        StorageRecoveryReceiptProjection::decl(),
        StorageCleanupReceiptProjection::decl(),
        BackupArchiveProjection::decl(),
        BackupInventoryEntryState::decl(),
        BackupInventoryEntry::decl(),
        BackupInventoryProjection::decl(),
        pending.clone(),
        encryption.clone(),
        BackupRetentionReasonProjection::decl(),
        BackupRetentionProjection::decl(),
        BackupOperationsProjection::decl(),
        ConfigOperationsSnapshot::decl(),
        ConfigSnapshotCommand::decl(),
        StorageTransitionInspectCommand::decl(),
        StorageTransitionExecuteCommand::decl(),
        StorageRecoveryCommand::decl(),
        StorageCleanupCommand::decl(),
        PendingBackupPolicy::decl(),
        BackupCreateCommand::decl(),
        BackupExportCommand::decl(),
        BackupRetentionApplyCommand::decl(),
        rejection.clone(),
        ConfigOperationRejection::decl(),
        inspect.clone(),
        execute.clone(),
        recovery.clone(),
        cleanup.clone(),
        BackupCaptureReceiptProjection::decl(),
        BackupPublicationReceiptProjection::decl(),
        create.clone(),
        export.clone(),
        retention.clone(),
    ]
    .map(exported_declaration);
    let restore_declarations = [
        RestoreOperationStateProjection::decl(),
        RestoreOperationsProjection::decl(),
        restore_selection.clone(),
        RestoreIntegrityProjection::decl(),
        RestoreAuthenticityProjection::decl(),
        restore_identity.clone(),
        RestoreIdentityProjection::decl(),
        RestoreConsistencyGroupProjection::decl(),
        restore_compatibility.clone(),
        restore_participation.clone(),
        RestoreDomainInspectionProjection::decl(),
        RestoreExclusionProjection::decl(),
        RestoreInspectionReceiptProjection::decl(),
        RestoreInspectionProjection::decl(),
        RestoreInspectCommand::decl(),
        restore_inspect.clone(),
        RestoreConflictChoiceProjection::decl(),
        RestoreDomainChoice::decl(),
        restore_current.clone(),
        RestorePlanEntryProjection::decl(),
        RestorePlanReceiptProjection::decl(),
        RestorePlanProjection::decl(),
        RestorePlanCommand::decl(),
        restore_plan.clone(),
        RestoreExecuteCommand::decl(),
        RestoreStagingReceiptProjection::decl(),
        RestoreExecutionReceiptProjection::decl(),
        RestoreExecutionFailureProjection::decl(),
        restore_execute.clone(),
        RestoreAdapterRequirementProjection::decl(),
        RestoreAdapterExecuteCommand::decl(),
        RestoreAdapterReceiptProjection::decl(),
        restore_adapter.clone(),
        RestoreRecoveryCommand::decl(),
        RestoreRecoveryReceiptProjection::decl(),
        restore_recovery.clone(),
    ]
    .map(exported_declaration);

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
