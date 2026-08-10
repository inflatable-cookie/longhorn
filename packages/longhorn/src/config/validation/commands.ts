import { CONFIG_FIELDS } from "../generated/fields.ts";
import type {
  BackupCreateCommand,
  BackupExportCommand,
  BackupRetentionApplyCommand,
  ConfigSnapshotCommand,
  RestoreAdapterExecuteCommand,
  RestoreExecuteCommand,
  RestoreInspectCommand,
  RestorePlanCommand,
  RestoreRecoveryCommand,
  StorageCleanupCommand,
  StorageRecoveryCommand,
  StorageTransitionExecuteCommand,
  StorageTransitionInspectCommand,
} from "../generated/protocol.ts";
import {
  RESTORE_ARCHIVE_SELECTION_SOURCES,
} from "../generated/protocol.ts";
import {
  array,
  baseCommand,
  boolean,
  digest,
  discriminant,
  finiteNumber,
  nonempty,
  record,
} from "./primitives.ts";

const STORAGE_PROFILES = [
  "platform-native-v1",
  "unified-app-root-v1",
  "shared-product-root-v1",
  "portable-v1",
] as const;
const RESTORE_CHOICES = ["useArchive", "keepCurrent"] as const;

export function assertCompatibleConfigSnapshotCommand(
  value: unknown,
): asserts value is ConfigSnapshotCommand {
  baseCommand(value, "$", CONFIG_FIELDS.ConfigSnapshotCommand);
}

export function assertCompatibleStorageTransitionInspectCommand(
  value: unknown,
): asserts value is StorageTransitionInspectCommand {
  const command = baseCommand(value, "$", CONFIG_FIELDS.StorageTransitionInspectCommand);
  discriminant(command.targetProfile, STORAGE_PROFILES, "$.targetProfile");
  boolean(command.includeLogs, "$.includeLogs");
}

export function assertCompatibleStorageTransitionExecuteCommand(
  value: unknown,
): asserts value is StorageTransitionExecuteCommand {
  generationConfirmationCommand(value, CONFIG_FIELDS.StorageTransitionExecuteCommand);
}

export function assertCompatibleStorageRecoveryCommand(
  value: unknown,
): asserts value is StorageRecoveryCommand {
  baseCommand(value, "$", CONFIG_FIELDS.StorageRecoveryCommand);
}

export function assertCompatibleStorageCleanupCommand(
  value: unknown,
): asserts value is StorageCleanupCommand {
  const command = baseCommand(value, "$", CONFIG_FIELDS.StorageCleanupCommand);
  nonempty(command.transitionId, "$.transitionId");
  digest(command.transitionReceiptDigest, "$.transitionReceiptDigest");
}

export function assertCompatibleBackupCreateCommand(
  value: unknown,
): asserts value is BackupCreateCommand {
  const command = baseCommand(value, "$", CONFIG_FIELDS.BackupCreateCommand);
  discriminant(command.pendingPolicy, ["refuse", "flush"], "$.pendingPolicy");
}

export function assertCompatibleBackupExportCommand(
  value: unknown,
): asserts value is BackupExportCommand {
  const command = baseCommand(value, "$", CONFIG_FIELDS.BackupExportCommand);
  digest(command.archiveSha256, "$.archiveSha256");
}

export function assertCompatibleBackupRetentionApplyCommand(
  value: unknown,
): asserts value is BackupRetentionApplyCommand {
  generationConfirmationCommand(value, CONFIG_FIELDS.BackupRetentionApplyCommand);
}

export function assertCompatibleRestoreInspectCommand(
  value: unknown,
): asserts value is RestoreInspectCommand {
  const command = baseCommand(value, "$", CONFIG_FIELDS.RestoreInspectCommand);
  const selection = record(command.selection, "$.selection");
  discriminant(
    selection.source,
    RESTORE_ARCHIVE_SELECTION_SOURCES,
    "$.selection.source",
  );
  if (selection.source === "inventory") {
    digest(selection.archiveSha256, "$.selection.archiveSha256");
  }
}

export function assertCompatibleRestorePlanCommand(
  value: unknown,
): asserts value is RestorePlanCommand {
  const command = baseCommand(value, "$", CONFIG_FIELDS.RestorePlanCommand);
  finiteNumber(command.generation, "$.generation");
  digest(command.archiveSha256, "$.archiveSha256");
  array(command.choices, "$.choices").forEach((choiceValue, index) => {
    const path = `$.choices[${index}]`;
    const choice = record(choiceValue, path, CONFIG_FIELDS.RestoreDomainChoice);
    nonempty(choice.domainId, `${path}.domainId`);
    discriminant(choice.choice, RESTORE_CHOICES, `${path}.choice`);
  });
}

export function assertCompatibleRestoreExecuteCommand(
  value: unknown,
): asserts value is RestoreExecuteCommand {
  generationConfirmationCommand(value, CONFIG_FIELDS.RestoreExecuteCommand);
}

export function assertCompatibleRestoreAdapterExecuteCommand(
  value: unknown,
): asserts value is RestoreAdapterExecuteCommand {
  const command = generationConfirmationCommand(value, CONFIG_FIELDS.RestoreAdapterExecuteCommand);
  digest(command.archiveSha256, "$.archiveSha256");
  nonempty(command.domainId, "$.domainId");
  discriminant(
    command.requirement,
    ["failureAtomic", "allowSeparate"],
    "$.requirement",
  );
}

export function assertCompatibleRestoreRecoveryCommand(
  value: unknown,
): asserts value is RestoreRecoveryCommand {
  baseCommand(value, "$", CONFIG_FIELDS.RestoreRecoveryCommand);
}

function generationConfirmationCommand(
  value: unknown,
  allowed?: readonly string[],
): Record<string, unknown> {
  const command = baseCommand(value, "$", allowed);
  finiteNumber(command.generation, "$.generation");
  digest(command.confirmationDigest, "$.confirmationDigest");
  return command;
}
