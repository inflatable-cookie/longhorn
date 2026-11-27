import { wireVariantFields } from "../../core/record.ts";
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
  STORAGE_CLASSES,
  STORAGE_PROFILE_IDS,
  STORAGE_RECOVERY_STATUSES,
  STORAGE_TRANSITION_EXECUTE_STATUSES,
  STORAGE_TRANSITION_INSPECT_STATUSES,
} from "../generated/protocol.ts";
import type {
  BackupCreateOutcome,
  BackupExportOutcome,
  BackupRetentionApplyOutcome,
  RestoreAdapterExecuteOutcome,
  RestoreExecuteOutcome,
  RestoreInspectOutcome,
  RestorePlanOutcome,
  RestoreRecoveryOutcomeProjection,
  StorageCleanupOutcome,
  StorageRecoveryOutcome,
  StorageTransitionExecuteOutcome,
  StorageTransitionInspectOutcome,
} from "../generated/protocol.ts";
import {
  assertBackupPublication,
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
  boolean,
  digest,
  discriminant,
  finiteNumber,
  nullableDigest,
  nullableString,
  record,
  string,
  fail,
} from "./primitives.ts";

export function assertValidStorageTransitionInspectOutcome(
  value: unknown,
): asserts value is StorageTransitionInspectOutcome {
  const result = outcome(
    value,
    STORAGE_TRANSITION_INSPECT_STATUSES,
    "$",
    "StorageTransitionInspectOutcome",
  );
  if (result.status === "ready") {
    finiteNumber(result.generation, "$.generation");
    assertStorageTransitionPreview(result.preview);
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

export function assertValidStorageTransitionExecuteOutcome(
  value: unknown,
): asserts value is StorageTransitionExecuteOutcome {
  mutationOutcome(
    value,
    STORAGE_TRANSITION_EXECUTE_STATUSES,
    "committed",
    "StorageTransitionExecuteOutcome",
    (result) => assertStorageTransitionReceipt(result.receipt),
  );
}

export function assertValidStorageRecoveryOutcome(
  value: unknown,
): asserts value is StorageRecoveryOutcome {
  mutationOutcome(
    value,
    STORAGE_RECOVERY_STATUSES,
    "recovered",
    "StorageRecoveryOutcome",
    (result) => assertStorageRecoveryReceipt(result.receipt),
  );
}

export function assertValidStorageCleanupOutcome(
  value: unknown,
): asserts value is StorageCleanupOutcome {
  mutationOutcome(value, STORAGE_CLEANUP_STATUSES, "applied", "StorageCleanupOutcome", (result) =>
    assertStorageCleanupReceipt(result.receipt),
  );
}

export function assertValidBackupCreateOutcome(
  value: unknown,
): asserts value is BackupCreateOutcome {
  mutationOutcome(value, BACKUP_CREATE_STATUSES, "published", "BackupCreateOutcome", (result) => {
    assertBackupCapture(result.capture);
    assertBackupPublication(result.publication, "$.publication");
  });
}

export function assertValidBackupExportOutcome(
  value: unknown,
): asserts value is BackupExportOutcome {
  mutationOutcome(value, BACKUP_EXPORT_STATUSES, "published", "BackupExportOutcome", (result) =>
    assertBackupPublication(result.publication, "$.publication"),
  );
}

export function assertValidBackupRetentionApplyOutcome(
  value: unknown,
): asserts value is BackupRetentionApplyOutcome {
  mutationOutcome(
    value,
    BACKUP_RETENTION_APPLY_STATUSES,
    "applied",
    "BackupRetentionApplyOutcome",
    (result) => stringArray(result.deletedPaths, "$.deletedPaths"),
  );
}

export function assertValidRestoreInspectOutcome(
  value: unknown,
): asserts value is RestoreInspectOutcome {
  const result = outcome(value, RESTORE_INSPECT_STATUSES, "$", "RestoreInspectOutcome");
  if (result.status === "ready") {
    finiteNumber(result.generation, "$.generation");
    assertRestoreInspection(result.inspection, "$.inspection");
  } else if (result.status === "locked") {
    string(result.detail, "$.detail");
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

export function assertValidRestorePlanOutcome(value: unknown): asserts value is RestorePlanOutcome {
  const result = outcome(value, RESTORE_PLAN_STATUSES, "$", "RestorePlanOutcome");
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
  const result = outcome(value, RESTORE_EXECUTE_STATUSES, "$", "RestoreExecuteOutcome");
  if (result.status === "succeeded") {
    assertRestoreExecutionReceipt(result.receipt, "$.receipt");
    assertValidConfigOperationsSnapshot(result.snapshot);
  } else if (result.status === "rolledBack" || result.status === "recoveryRequired") {
    assertRestoreFailure(result.failure, "$.failure");
    assertValidConfigOperationsSnapshot(result.snapshot);
  } else {
    rejection(result.rejection, "$.rejection");
  }
}

export function assertValidRestoreAdapterExecuteOutcome(
  value: unknown,
): asserts value is RestoreAdapterExecuteOutcome {
  const result = outcome(
    value,
    RESTORE_ADAPTER_EXECUTE_STATUSES,
    "$",
    "RestoreAdapterExecuteOutcome",
  );
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
  const result = outcome(value, RESTORE_RECOVERY_STATUSES, "$", "RestoreRecoveryOutcomeProjection");
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
  variants: ReadonlyArray<string>,
  success: string,
  type: string,
  checkSuccess: (result: Record<string, unknown>) => void,
): void {
  const result = outcome(value, variants, "$", type);
  if (result.status === success) {
    checkSuccess(result);
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
  variants: ReadonlyArray<string>,
  path: string,
  type: string,
): Record<string, unknown> {
  const result = record(value, path);
  discriminant(result.status, variants, `${path}.status`);
  return record(result, path, variantKeys(type, result, path));
}

function variantKeys(
  type: string,
  value: Record<string, unknown>,
  path: string,
): ReadonlyArray<string> {
  return wireVariantFields(
    CONFIG_VARIANT_FIELDS,
    CONFIG_VARIANT_FIELDS_DISCRIMINANTS,
    type,
    value,
    (discriminant) =>
      fail("invalid_payload", path, `no generated fields for ${type}.${String(discriminant)}`),
  );
}

function assertStorageTransitionPreview(value: unknown): void {
  const path = "$.preview";
  const preview = record(value, path, CONFIG_FIELDS.StorageTransitionPreviewProjection);
  discriminant(preview.targetProfile, STORAGE_PROFILE_IDS, `${path}.targetProfile`);
  for (const key of [
    "sourceLayoutDigest",
    "targetLayoutDigest",
    "evidenceDigest",
    "confirmationDigest",
  ]) {
    digest(preview[key], `${path}.${key}`);
  }
  for (const [index, value] of array(preview.domains, `${path}.domains`).entries()) {
    const domainPath = `${path}.domains[${index}]`;
    const domain = record(value, domainPath, CONFIG_FIELDS.StorageTransitionDomainProjection);
    string(domain.domainId, `${domainPath}.domainId`);
    discriminant(domain.storageClass, STORAGE_CLASSES, `${domainPath}.storageClass`);
    string(domain.action, `${domainPath}.action`);
    nullableString(domain.sourcePath, `${domainPath}.sourcePath`);
    nullableString(domain.targetPath, `${domainPath}.targetPath`);
    nullableDigest(domain.sourceSha256, `${domainPath}.sourceSha256`);
  }
  stringArray(preview.unknownSourcePaths, `${path}.unknownSourcePaths`);
  for (const [index, value] of array(preview.conflicts, `${path}.conflicts`).entries()) {
    const conflictPath = `${path}.conflicts[${index}]`;
    const conflict = record(value, conflictPath, CONFIG_FIELDS.StorageTransitionConflictProjection);
    string(conflict.kind, `${conflictPath}.kind`);
    nullableString(conflict.path, `${conflictPath}.path`);
    string(conflict.detail, `${conflictPath}.detail`);
  }
}

function assertStorageTransitionReceipt(value: unknown): void {
  const path = "$.receipt";
  const receipt = record(value, path, CONFIG_FIELDS.StorageTransitionReceiptProjection);
  string(receipt.transitionId, `${path}.transitionId`);
  string(receipt.outcome, `${path}.outcome`);
  digest(receipt.targetLayoutDigest, `${path}.targetLayoutDigest`);
  digest(receipt.receiptDigest, `${path}.receiptDigest`);
  stringArray(receipt.copiedDomainIds, `${path}.copiedDomainIds`);
  stringArray(receipt.customDomainIds, `${path}.customDomainIds`);
  stringArray(receipt.retainedSourcePaths, `${path}.retainedSourcePaths`);
}

function assertStorageRecoveryReceipt(value: unknown): void {
  const path = "$.receipt";
  const receipt = record(value, path, CONFIG_FIELDS.StorageRecoveryReceiptProjection);
  nullableString(receipt.transitionId, `${path}.transitionId`);
  string(receipt.outcome, `${path}.outcome`);
  digest(receipt.activeLayoutDigest, `${path}.activeLayoutDigest`);
  string(receipt.detail, `${path}.detail`);
}

function assertStorageCleanupReceipt(value: unknown): void {
  const path = "$.receipt";
  const receipt = record(value, path, CONFIG_FIELDS.StorageCleanupReceiptProjection);
  string(receipt.transitionId, `${path}.transitionId`);
  digest(receipt.transitionReceiptDigest, `${path}.transitionReceiptDigest`);
  stringArray(receipt.removedPaths, `${path}.removedPaths`);
}

function assertBackupCapture(value: unknown): void {
  const path = "$.capture";
  const receipt = record(value, path, CONFIG_FIELDS.BackupCaptureReceiptProjection);
  const { flushedPendingPublication, ...counters } = receipt;
  boolean(flushedPendingPublication, `${path}.flushedPendingPublication`);
  for (const [key, count] of Object.entries(counters)) {
    finiteNumber(count, `${path}.${key}`);
  }
}

function stringArray(value: unknown, path: string): void {
  for (const [index, item] of array(value, path).entries()) {
    string(item, `${path}[${index}]`);
  }
}
