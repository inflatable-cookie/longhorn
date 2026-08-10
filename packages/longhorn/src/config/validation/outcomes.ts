import { CONFIG_FIELDS } from "../generated/fields.ts";
import {
  BACKUP_CREATE_STATUSES,
  BACKUP_EXPORT_STATUSES,
  BACKUP_RETENTION_APPLY_STATUSES,
  CONFIG_OPERATION_REJECTION_CODES,
  RESTORE_ADAPTER_EXECUTE_STATUSES,
  RESTORE_EXECUTE_STATUSES,
  RESTORE_INSPECT_STATUSES,
  RESTORE_PLAN_STATUSES,
  RESTORE_RECOVERY_STATUSES,
  STORAGE_CLEANUP_STATUSES,
  STORAGE_RECOVERY_STATUSES,
  STORAGE_TRANSITION_EXECUTE_STATUSES,
  STORAGE_TRANSITION_INSPECT_STATUSES,
  type BackupCreateOutcome,
  type BackupExportOutcome,
  type BackupRetentionApplyOutcome,
  type RestoreAdapterExecuteOutcome,
  type RestoreExecuteOutcome,
  type RestoreInspectOutcome,
  type RestorePlanOutcome,
  type RestoreRecoveryOutcomeProjection,
  type StorageCleanupOutcome,
  type StorageRecoveryOutcome,
  type StorageTransitionExecuteOutcome,
  type StorageTransitionInspectOutcome,
} from "../generated/protocol.ts";
import {
  assertCompatibleConfigOperationsSnapshot,
  assertRestoreAdapterReceipt,
  assertRestoreExecutionReceipt,
  assertRestoreFailure,
  assertRestoreInspection,
  assertRestorePlan,
  assertRestoreRecoveryReceipt,
} from "./projection.ts";
import {
  array,
  digest,
  discriminant,
  finiteNumber,
  record,
  string,
} from "./primitives.ts";

export function assertCompatibleStorageTransitionInspectOutcome(
  value: unknown,
): asserts value is StorageTransitionInspectOutcome {
  outcome(value, STORAGE_TRANSITION_INSPECT_STATUSES, "$");
  const result = record(value, "$");
  if (result.status === "ready") {
    finiteNumber(result.generation, "$.generation");
    const preview = record(
      result.preview,
      "$.preview",
      CONFIG_FIELDS.StorageTransitionPreviewProjection,
    );
    [
      "sourceLayoutDigest",
      "targetLayoutDigest",
      "evidenceDigest",
      "confirmationDigest",
    ].forEach((key) => digest(preview[key], `$.preview.${key}`));
    array(preview.domains, "$.preview.domains");
    array(preview.unknownSourcePaths, "$.preview.unknownSourcePaths");
    array(preview.conflicts, "$.preview.conflicts");
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

export function assertCompatibleStorageTransitionExecuteOutcome(
  value: unknown,
): asserts value is StorageTransitionExecuteOutcome {
  mutationOutcome(value, STORAGE_TRANSITION_EXECUTE_STATUSES, "committed");
}

export function assertCompatibleStorageRecoveryOutcome(
  value: unknown,
): asserts value is StorageRecoveryOutcome {
  mutationOutcome(value, STORAGE_RECOVERY_STATUSES, "recovered");
}

export function assertCompatibleStorageCleanupOutcome(
  value: unknown,
): asserts value is StorageCleanupOutcome {
  mutationOutcome(value, STORAGE_CLEANUP_STATUSES, "applied");
}

export function assertCompatibleBackupCreateOutcome(
  value: unknown,
): asserts value is BackupCreateOutcome {
  mutationOutcome(value, BACKUP_CREATE_STATUSES, "published");
}

export function assertCompatibleBackupExportOutcome(
  value: unknown,
): asserts value is BackupExportOutcome {
  mutationOutcome(value, BACKUP_EXPORT_STATUSES, "published");
}

export function assertCompatibleBackupRetentionApplyOutcome(
  value: unknown,
): asserts value is BackupRetentionApplyOutcome {
  mutationOutcome(value, BACKUP_RETENTION_APPLY_STATUSES, "applied");
}

export function assertCompatibleRestoreInspectOutcome(
  value: unknown,
): asserts value is RestoreInspectOutcome {
  outcome(value, RESTORE_INSPECT_STATUSES, "$");
  const result = record(value, "$");
  if (result.status === "ready") {
    finiteNumber(result.generation, "$.generation");
    assertRestoreInspection(result.inspection, "$.inspection");
  } else if (result.status === "locked") {
    string(result.detail, "$.detail");
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

export function assertCompatibleRestorePlanOutcome(
  value: unknown,
): asserts value is RestorePlanOutcome {
  outcome(value, RESTORE_PLAN_STATUSES, "$");
  const result = record(value, "$");
  if (result.status === "ready") {
    finiteNumber(result.generation, "$.generation");
    assertRestorePlan(result.plan, "$.plan");
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

export function assertCompatibleRestoreExecuteOutcome(
  value: unknown,
): asserts value is RestoreExecuteOutcome {
  outcome(value, RESTORE_EXECUTE_STATUSES, "$");
  const result = record(value, "$");
  if (result.status === "succeeded") {
    assertRestoreExecutionReceipt(result.receipt, "$.receipt");
    assertCompatibleConfigOperationsSnapshot(result.snapshot);
  } else if (
    result.status === "rolledBack" ||
    result.status === "recoveryRequired"
  ) {
    assertRestoreFailure(result.failure, "$.failure");
    assertCompatibleConfigOperationsSnapshot(result.snapshot);
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

export function assertCompatibleRestoreAdapterExecuteOutcome(
  value: unknown,
): asserts value is RestoreAdapterExecuteOutcome {
  outcome(value, RESTORE_ADAPTER_EXECUTE_STATUSES, "$");
  const result = record(value, "$");
  if (result.status === "completed") {
    assertRestoreAdapterReceipt(result.receipt, "$.receipt");
    assertCompatibleConfigOperationsSnapshot(result.snapshot);
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

export function assertCompatibleRestoreRecoveryOutcome(
  value: unknown,
): asserts value is RestoreRecoveryOutcomeProjection {
  outcome(value, RESTORE_RECOVERY_STATUSES, "$");
  const result = record(value, "$");
  if (result.status === "recovered") {
    assertRestoreRecoveryReceipt(result.receipt, "$.receipt");
    assertCompatibleConfigOperationsSnapshot(result.snapshot);
  } else if (result.status === "recoveryRequired") {
    assertRestoreFailure(result.failure, "$.failure");
    assertCompatibleConfigOperationsSnapshot(result.snapshot);
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

function mutationOutcome(
  value: unknown,
  variants: readonly string[],
  success: string,
): void {
  outcome(value, variants, "$");
  const result = record(value, "$");
  if (result.status === success) {
    assertCompatibleConfigOperationsSnapshot(result.snapshot);
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

function rejection(value: unknown, path: string): void {
  const refused = record(value, path, CONFIG_FIELDS.ConfigOperationRejection);
  discriminant(refused.code, CONFIG_OPERATION_REJECTION_CODES, `${path}.code`);
  string(refused.detail, `${path}.detail`);
  if (refused.snapshot !== null) {
    assertCompatibleConfigOperationsSnapshot(refused.snapshot);
  }
}

function outcome(
  value: unknown,
  variants: readonly string[],
  path: string,
): void {
  const result = record(value, path);
  discriminant(result.status, variants, `${path}.status`);
}
