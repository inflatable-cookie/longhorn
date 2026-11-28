use std::error::Error;

use longhorn_config::{
    BackupArchiveProjection, BackupCaptureReceiptProjection, BackupConsistencyMode,
    BackupCreateCommand, BackupCreateOutcome, BackupEncryptionState, BackupExportCommand,
    BackupExportOutcome, BackupInventoryEntry, BackupInventoryEntryState,
    BackupInventoryProjection, BackupKind, BackupOperationsProjection, BackupPendingState,
    BackupPublicationDurability, BackupPublicationReceiptProjection, BackupRetentionApplyCommand,
    BackupRetentionApplyOutcome, BackupRetentionProjection, BackupRetentionReasonProjection,
    BackupSourceState, CONFIG_OPERATIONS_PROTOCOL_VERSION, ConfigGeneration,
    ConfigOperationCapability, ConfigOperationRejection, ConfigOperationRejectionCode,
    ConfigOperationsSnapshot, ConfigProtocolVersion, ConfigSnapshotCommand, PendingBackupPolicy,
    RestoreAdapterExecuteCommand, RestoreAdapterExecuteOutcome, RestoreAdapterOutcomeProjection,
    RestoreAdapterParticipationProjection, RestoreAdapterReceiptProjection,
    RestoreAdapterRequirementProjection, RestoreArchiveSelection, RestoreAuthenticityProjection,
    RestoreConflictChoiceProjection, RestoreConsistencyGroupProjection,
    RestoreCurrentEvidenceProjection, RestoreDomainChoice, RestoreDomainCompatibilityProjection,
    RestoreDomainInspectionProjection, RestoreExclusionProjection, RestoreExecuteCommand,
    RestoreExecuteOutcome, RestoreExecutionFailureProjection, RestoreExecutionReceiptProjection,
    RestoreExecutionStageProjection, RestoreFailureTerminalProjection, RestoreIdentityProjection,
    RestoreIdentityStatusProjection, RestoreInspectCommand, RestoreInspectOutcome,
    RestoreInspectionProjection, RestoreInspectionReceiptProjection, RestoreIntegrityProjection,
    RestoreOperationStateProjection, RestoreOperationsProjection, RestorePlanCommand,
    RestorePlanEntryProjection, RestorePlanOutcome, RestorePlanProjection,
    RestorePlanReceiptProjection, RestoreRecoveryCommand, RestoreRecoveryOutcomeProjection,
    RestoreRecoveryReceiptProjection, RestoreRecoveryResultProjection,
    RestoreStagingReceiptProjection, StorageBootstrapProjection, StorageClass,
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

use crate::{
    generation::{
        Artifact, GENERATE_TASK, LabelMap, config, exported_declaration, field_map, label_module,
        label_template_renderer, tagged_variants_json, variant_field_map,
    },
    serde_shape::enum_names_json,
};

mod fixture;

// Compile the adapter's wire declarations without linking the Tauri runtime.
#[allow(dead_code)]
#[path = "../../longhorn-tauri-config/src/error.rs"]
mod host_error;

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
        GENERATE_TASK,
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

/// Renders the config bindings and golden fixtures.
///
/// # Errors
///
/// Returns an error if declaration inspection or fixture serialization fails.
pub fn render() -> Result<Vec<Artifact>, Box<dyn Error>> {
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

    let [host_error_code, host_error] =
        crate::serde_shape::error_declarations::<_, host_error::ConfigOperationsHostErrorCode>(
            host_error::ConfigOperationsHostError::state_unavailable(),
            "ConfigOperationsHostError",
            "ConfigOperationsHostErrorCode",
        )?;

    let base_declarations = [
        ConfigProtocolVersion::decl(config()),
        ConfigGeneration::decl(config()),
        ConfigRequestId::decl(config()),
        ConfigOperationCapability::decl(config()),
        StorageProfileId::decl(config()),
        StorageLeafProvenanceProjection::decl(config()),
        StorageRootProjection::decl(config()),
        StorageLayoutProjection::decl(config()),
        bootstrap.clone(),
        StorageOperationsProjection::decl(config()),
        StorageClass::decl(config()),
        StorageTransitionDomainProjection::decl(config()),
        StorageTransitionConflictProjection::decl(config()),
        StorageTransitionPreviewProjection::decl(config()),
        StorageTransitionReceiptProjection::decl(config()),
        StorageRecoveryReceiptProjection::decl(config()),
        StorageCleanupReceiptProjection::decl(config()),
        BackupKind::decl(config()),
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
        ConfigOperationRejectionCode::decl(config()),
        ConfigOperationRejection::decl(config()),
        inspect.clone(),
        execute.clone(),
        recovery.clone(),
        cleanup.clone(),
        BackupCaptureReceiptProjection::decl(config()),
        BackupPublicationDurability::decl(config()),
        BackupPublicationReceiptProjection::decl(config()),
        create.clone(),
        export.clone(),
        retention.clone(),
        host_error_code,
        host_error,
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
        BackupConsistencyMode::decl(config()),
        RestoreConsistencyGroupProjection::decl(config()),
        restore_compatibility.clone(),
        restore_participation.clone(),
        BackupSourceState::decl(config()),
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
        RestoreExecutionStageProjection::decl(config()),
        RestoreFailureTerminalProjection::decl(config()),
        RestoreExecutionFailureProjection::decl(config()),
        restore_execute.clone(),
        RestoreAdapterRequirementProjection::decl(config()),
        RestoreAdapterExecuteCommand::decl(config()),
        RestoreAdapterOutcomeProjection::decl(config()),
        RestoreAdapterReceiptProjection::decl(config()),
        restore_adapter.clone(),
        RestoreRecoveryCommand::decl(config()),
        RestoreRecoveryResultProjection::decl(config()),
        RestoreRecoveryReceiptProjection::decl(config()),
        restore_recovery.clone(),
    ]
    .map(exported_declaration);

    // Both modules in one map: the validators import from `protocol.ts`,
    // which re-exports both, so splitting the map would only make callers
    // guess which half a type lives in.
    let mut all_declarations = base_declarations.to_vec();
    all_declarations.extend_from_slice(&restore_declarations);
    let fields_contents = field_map(GENERATE_TASK, "CONFIG_FIELDS", &all_declarations);
    let variant_fields_contents =
        variant_field_map(GENERATE_TASK, "CONFIG_VARIANT_FIELDS", &all_declarations)?;

    let protocol_contents = format!(
        "// @generated by `effigy {GENERATE_TASK}`; do not edit.\n\
         export * from \"./base.ts\";\n\
         export * from \"./restore.ts\";\n"
    );
    let base_contents = format!(
        "// @generated by `effigy {GENERATE_TASK}`; do not edit.\n\
         // Rust serde types are the wire authority.\n\
         import type {{ RestoreOperationsProjection }} from \"./restore.ts\";\n\n\
         export const CONFIG_OPERATIONS_PROTOCOL_VERSION = \
         {CONFIG_OPERATIONS_PROTOCOL_VERSION} as const;\n\
         export const CONFIG_OPERATION_CAPABILITIES = {} as const;\n\
         export const CONFIG_OPERATION_REJECTION_CODES = {} as const;\n\
         export const STORAGE_PROFILE_IDS = {} as const;\n\
         export const STORAGE_LEAF_PROVENANCES = {} as const;\n\
         export const BACKUP_KINDS = {} as const;\n\
         export const STORAGE_CLASSES = {} as const;\n\
         export const BACKUP_INVENTORY_ENTRY_STATES = {} as const;\n\
         export const BACKUP_PENDING_POLICIES = {} as const;\n\
         export const STORAGE_BOOTSTRAP_STATES = {} as const;\n\
         export const BACKUP_PENDING_STATES = {} as const;\n\
         export const BACKUP_ENCRYPTION_STATES = {} as const;\n\
         export const STORAGE_TRANSITION_INSPECT_STATUSES = {} as const;\n\
         export const STORAGE_TRANSITION_EXECUTE_STATUSES = {} as const;\n\
         export const STORAGE_RECOVERY_STATUSES = {} as const;\n\
         export const STORAGE_CLEANUP_STATUSES = {} as const;\n\
         export const BACKUP_CREATE_STATUSES = {} as const;\n\
         export const BACKUP_EXPORT_STATUSES = {} as const;\n\
         export const BACKUP_PUBLICATION_DURABILITIES = {} as const;\n\
         export const BACKUP_RETENTION_APPLY_STATUSES = {} as const;\n\n\
         {}\n",
        enum_names_json::<ConfigOperationCapability>()?,
        enum_names_json::<ConfigOperationRejectionCode>()?,
        enum_names_json::<StorageProfileId>()?,
        enum_names_json::<StorageLeafProvenanceProjection>()?,
        enum_names_json::<BackupKind>()?,
        enum_names_json::<StorageClass>()?,
        enum_names_json::<BackupInventoryEntryState>()?,
        enum_names_json::<PendingBackupPolicy>()?,
        tagged_variants_json(&bootstrap)?,
        tagged_variants_json(&pending)?,
        tagged_variants_json(&encryption)?,
        tagged_variants_json(&inspect)?,
        tagged_variants_json(&execute)?,
        tagged_variants_json(&recovery)?,
        tagged_variants_json(&cleanup)?,
        tagged_variants_json(&create)?,
        tagged_variants_json(&export)?,
        enum_names_json::<BackupPublicationDurability>()?,
        tagged_variants_json(&retention)?,
        base_declarations.join("\n\n")
    );
    let restore_contents = format!(
        "// @generated by `effigy {GENERATE_TASK}`; do not edit.\n\
         // Rust serde types are the wire authority.\n\
         import type {{ BackupPublicationReceiptProjection, ConfigGeneration, \
         ConfigOperationRejection, ConfigOperationsSnapshot, ConfigProtocolVersion, \
         ConfigRequestId, BackupKind, StorageClass }} from \"./base.ts\";\n\n\
         export const BACKUP_CONSISTENCY_MODES = {} as const;\n\
         export const BACKUP_SOURCE_STATES = {} as const;\n\
         export const RESTORE_INTEGRITY_VALUES = {} as const;\n\
         export const RESTORE_AUTHENTICITY_VALUES = {} as const;\n\
         export const RESTORE_CONFLICT_CHOICES = {} as const;\n\
         export const RESTORE_OPERATION_STATES = {} as const;\n\
         export const RESTORE_ADAPTER_REQUIREMENTS = {} as const;\n\
         export const RESTORE_ARCHIVE_SELECTION_SOURCES = {} as const;\n\
         export const RESTORE_IDENTITY_STATUSES = {} as const;\n\
         export const RESTORE_DOMAIN_COMPATIBILITY_STATUSES = {} as const;\n\
         export const RESTORE_ADAPTER_PARTICIPATION_KINDS = {} as const;\n\
         export const RESTORE_CURRENT_EVIDENCE_STATES = {} as const;\n\
         export const RESTORE_INSPECT_STATUSES = {} as const;\n\
         export const RESTORE_PLAN_STATUSES = {} as const;\n\
         export const RESTORE_EXECUTION_STAGES = {} as const;\n\
         export const RESTORE_FAILURE_TERMINALS = {} as const;\n\
         export const RESTORE_ADAPTER_OUTCOMES = {} as const;\n\
         export const RESTORE_RECOVERY_RESULTS = {} as const;\n\
         export const RESTORE_EXECUTE_STATUSES = {} as const;\n\
         export const RESTORE_ADAPTER_EXECUTE_STATUSES = {} as const;\n\
         export const RESTORE_RECOVERY_STATUSES = {} as const;\n\n\
         {}\n",
        enum_names_json::<BackupConsistencyMode>()?,
        enum_names_json::<BackupSourceState>()?,
        enum_names_json::<RestoreIntegrityProjection>()?,
        enum_names_json::<RestoreAuthenticityProjection>()?,
        enum_names_json::<RestoreConflictChoiceProjection>()?,
        enum_names_json::<RestoreOperationStateProjection>()?,
        enum_names_json::<RestoreAdapterRequirementProjection>()?,
        tagged_variants_json(&restore_selection)?,
        tagged_variants_json(&restore_identity)?,
        tagged_variants_json(&restore_compatibility)?,
        tagged_variants_json(&restore_participation)?,
        tagged_variants_json(&restore_current)?,
        tagged_variants_json(&restore_inspect)?,
        tagged_variants_json(&restore_plan)?,
        enum_names_json::<RestoreExecutionStageProjection>()?,
        enum_names_json::<RestoreFailureTerminalProjection>()?,
        enum_names_json::<RestoreAdapterOutcomeProjection>()?,
        enum_names_json::<RestoreRecoveryResultProjection>()?,
        tagged_variants_json(&restore_execute)?,
        tagged_variants_json(&restore_adapter)?,
        tagged_variants_json(&restore_recovery)?,
        restore_declarations.join("\n\n")
    );
    Ok(vec![
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
            contents: label_template_renderer(GENERATE_TASK),
        },
        Artifact {
            relative_path: GOLDEN_FIXTURE,
            contents: fixture::render()?,
        },
        crate::serde_shape::error_fixture(
            "fixtures/config/host-errors.json",
            &[
                host_error::ConfigOperationsHostError::authority(
                    "Host temporarily unavailable",
                    true,
                ),
                host_error::ConfigOperationsHostError::state_unavailable(),
            ],
        )?,
    ])
}
