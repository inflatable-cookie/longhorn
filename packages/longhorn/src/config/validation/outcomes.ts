import {
  CONFIG_VARIANT_FIELDS,
  CONFIG_VARIANT_FIELDS_DISCRIMINANTS,
} from "../generated/variant-fields.ts";
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
  assertValidConfigOperationsSnapshot,
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
  fail,
} from "./primitives.ts";

export function assertValidStorageTransitionInspectOutcome(
  value: unknown,
): asserts value is StorageTransitionInspectOutcome {
  outcome(value, STORAGE_TRANSITION_INSPECT_STATUSES, "$", "StorageTransitionInspectOutcome");
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

export function assertValidStorageTransitionExecuteOutcome(
  value: unknown,
): asserts value is StorageTransitionExecuteOutcome {
  mutationOutcome(value, STORAGE_TRANSITION_EXECUTE_STATUSES, "committed", "StorageTransitionExecuteOutcome");
}

export function assertValidStorageRecoveryOutcome(
  value: unknown,
): asserts value is StorageRecoveryOutcome {
  mutationOutcome(value, STORAGE_RECOVERY_STATUSES, "recovered", "StorageRecoveryOutcome");
}

export function assertValidStorageCleanupOutcome(
  value: unknown,
): asserts value is StorageCleanupOutcome {
  mutationOutcome(value, STORAGE_CLEANUP_STATUSES, "applied", "StorageCleanupOutcome");
}

export function assertValidBackupCreateOutcome(
  value: unknown,
): asserts value is BackupCreateOutcome {
  mutationOutcome(value, BACKUP_CREATE_STATUSES, "published", "BackupCreateOutcome");
}

export function assertValidBackupExportOutcome(
  value: unknown,
): asserts value is BackupExportOutcome {
  mutationOutcome(value, BACKUP_EXPORT_STATUSES, "published", "BackupExportOutcome");
}

export function assertValidBackupRetentionApplyOutcome(
  value: unknown,
): asserts value is BackupRetentionApplyOutcome {
  mutationOutcome(value, BACKUP_RETENTION_APPLY_STATUSES, "applied", "BackupRetentionApplyOutcome");
}

export function assertValidRestoreInspectOutcome(
  value: unknown,
): asserts value is RestoreInspectOutcome {
  outcome(value, RESTORE_INSPECT_STATUSES, "$", "RestoreInspectOutcome");
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

export function assertValidRestorePlanOutcome(
  value: unknown,
): asserts value is RestorePlanOutcome {
  outcome(value, RESTORE_PLAN_STATUSES, "$", "RestorePlanOutcome");
  const result = record(value, "$");
  if (result.status === "ready") {
    finiteNumber(result.generation, "$.generation");
    assertRestorePlan(result.plan, "$.plan");
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

export function assertValidRestoreExecuteOutcome(
  value: unknown,
): asserts value is RestoreExecuteOutcome {
  outcome(value, RESTORE_EXECUTE_STATUSES, "$", "RestoreExecuteOutcome");
  const result = record(value, "$");
  if (result.status === "succeeded") {
    assertRestoreExecutionReceipt(result.receipt, "$.receipt");
    assertValidConfigOperationsSnapshot(result.snapshot);
  } else if (
    result.status === "rolledBack" ||
    result.status === "recoveryRequired"
  ) {
    assertRestoreFailure(result.failure, "$.failure");
    assertValidConfigOperationsSnapshot(result.snapshot);
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

export function assertValidRestoreAdapterExecuteOutcome(
  value: unknown,
): asserts value is RestoreAdapterExecuteOutcome {
  outcome(value, RESTORE_ADAPTER_EXECUTE_STATUSES, "$", "RestoreAdapterExecuteOutcome");
  const result = record(value, "$");
  if (result.status === "completed") {
    assertRestoreAdapterReceipt(result.receipt, "$.receipt");
    assertValidConfigOperationsSnapshot(result.snapshot);
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

export function assertValidRestoreRecoveryOutcome(
  value: unknown,
): asserts value is RestoreRecoveryOutcomeProjection {
  outcome(value, RESTORE_RECOVERY_STATUSES, "$", "RestoreRecoveryOutcomeProjection");
  const result = record(value, "$");
  if (result.status === "recovered") {
    assertRestoreRecoveryReceipt(result.receipt, "$.receipt");
    assertValidConfigOperationsSnapshot(result.snapshot);
  } else if (result.status === "recoveryRequired") {
    assertRestoreFailure(result.failure, "$.failure");
    assertValidConfigOperationsSnapshot(result.snapshot);
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

function mutationOutcome(
  value: unknown,
  variants: readonly string[],
  success: string,
  type: string,
): void {
  outcome(value, variants, "$", type);
  const result = record(value, "$");
  if (result.status === success) {
    assertValidConfigOperationsSnapshot(result.snapshot);
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

function rejection(value: unknown, path: string): void {
  const refused = record(value, path, CONFIG_FIELDS.ConfigOperationRejection);
  discriminant(refused.code, CONFIG_OPERATION_REJECTION_CODES, `${path}.code`);
  string(refused.detail, `${path}.detail`);
  if (refused.snapshot !== null) {
    assertValidConfigOperationsSnapshot(refused.snapshot);
  }
}

function outcome(
  value: unknown,
  variants: readonly string[],
  path: string,
  type: string,
): void {
  const result = record(value, path);
  discriminant(result.status, variants, `${path}.status`);
  // The discriminant is checked above, so a missing map entry means the
  // generator failed rather than that a caller sent something odd.
  record(result, path, variantKeys(type, result, path));
}

/**
 * Allowed keys for one tagged-union variant, from the generated map, with the
 * discriminant's name read from the map too. This domain tags on `status`,
 * `state`, `kind` and `source`.
 */
function variantKeys(
  type: string,
  value: Record<string, unknown>,
  path: string,
): readonly string[] {
  const discriminant = value[CONFIG_VARIANT_FIELDS_DISCRIMINANTS[type] ?? "status"];
  const keys = CONFIG_VARIANT_FIELDS[type]?.[discriminant as string];
  if (keys === undefined) {
    fail("invalid_payload", path, `no generated fields for ${type}.${String(discriminant)}`);
  }
  return keys;
}
