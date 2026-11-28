import {
  RESTORE_COMPATIBILITY_LABEL_TEMPLATES,
  renderLabelTemplate,
} from "@inflatable-cookie/longhorn/config";
import type {
  BackupConsistencyMode,
  BackupSourceState,
  RestoreAdapterOutcomeProjection,
  RestoreConflictChoiceProjection,
  RestoreDomainCompatibilityProjection,
  RestoreDomainInspectionProjection,
  RestoreExecutionStageProjection,
  RestoreFailureTerminalProjection,
  RestoreIdentityStatusProjection,
  RestoreRecoveryResultProjection,
  StorageClass,
} from "@inflatable-cookie/longhorn/config";

export type RestoreChoice = "" | "useArchive" | "keepCurrent";

export const restoreChoiceLabels: Record<RestoreConflictChoiceProjection, string> = {
  useArchive: "Use archive",
  keepCurrent: "Keep current",
};

export const restoreActionLabels = new Map([
  ["create", "Create file"],
  ["replace", "Replace file"],
  ["delete", "Delete file"],
  ["migrate", "Migrate archive data"],
  ["unchanged", "No change"],
]);

export const restoreStageLabels: Record<RestoreExecutionStageProjection, string> = {
  validateSafetyBackup: "Validate the safety backup",
  recoverPrevious: "Recover the previous restore",
  recheckCurrent: "Recheck current data",
  captureRollback: "Capture rollback data",
  captureSafetyBackup: "Capture the safety backup",
  encodeSafetyBackup: "Encode the safety backup",
  publishSafetyBackup: "Publish the safety backup",
  publishJournal: "Write the restore journal",
  publishTarget: "Publish restored data",
  verifyTarget: "Verify restored data",
  rollback: "Roll back changes",
  cleanup: "Clean up restore files",
};

export const restoreFailureLabels: Record<RestoreFailureTerminalProjection, string> = {
  noLiveMutation: "Restore stopped before changing live data",
  rolledBack: "Restore failed and changes were rolled back",
  recoveryRequired: "Restore needs recovery",
};

export const restoreAdapterLabels: Record<RestoreAdapterOutcomeProjection, string> = {
  verified: "Restore verified",
  rolledBack: "Changes rolled back",
  recoveryRequired: "Recovery required",
};

export const restoreRecoveryLabels: Record<RestoreRecoveryResultProjection, string> = {
  noRecoveryNeeded: "No recovery needed",
  rolledBack: "Changes rolled back",
  terminalCleanup: "Recovery cleanup complete",
};

export const consistencyModeLabels: Record<BackupConsistencyMode, string> = {
  "coordinated-bounded": "Coordinated capture",
  "external-snapshot": "External snapshot",
};

export const storageClassLabels: Record<StorageClass, string> = {
  defaults: "Defaults",
  policy: "Policy",
  "user-config": "User settings",
  "machine-state": "Machine state",
  "workspace-local": "Local workspace data",
  "project-shared": "Shared project data",
  secret: "Secrets",
  cache: "Cache",
  runtime: "Runtime data",
  log: "Logs",
};

export const sourceStateLabels: Record<BackupSourceState, string> = {
  present: "Included in archive",
  absent: "Recorded as absent",
  "source-preserved": "Source preserved",
};

export function canUseArchive(domain: RestoreDomainInspectionProjection): boolean {
  return (
    domain.compatibility.status === "ready" || domain.compatibility.status === "migrationRequired"
  );
}

/**
 * Renders a compatibility classification from the generated template table.
 *
 * The templates come from Rust, where the wording lives on the classification
 * itself, and Rust renders from the same table — so the two backends cannot
 * word a classification differently. Six of the thirteen interpolate their own
 * fields, which is why this is a template table rather than a finished-string
 * map. See memo 022, D2, and Card 170.
 */
export function compatibilityLabel(compatibility: RestoreDomainCompatibilityProjection): string {
  const template = RESTORE_COMPATIBILITY_LABEL_TEMPLATES[compatibility.status];
  return renderLabelTemplate(template, fieldsOf(compatibility));
}

function fieldsOf(compatibility: RestoreDomainCompatibilityProjection) {
  const fields: Record<string, string> = {};
  for (const [key, value] of Object.entries(compatibility)) {
    if (key !== "status" && (typeof value === "string" || typeof value === "number")) {
      fields[key] = String(value);
    }
  }
  return fields;
}

/**
 * Renders one identity comparison, naming both sides when they differ.
 *
 * Checked against the Rust projection by the shared parity fixture.
 */
export function identityLabel(status: RestoreIdentityStatusProjection): string {
  return status.status === "compatible"
    ? "Compatible"
    : `Mismatch: host expects ${status.expected}, archive declares ${status.archive}`;
}
