import type { EventTransport } from "@inflatable-cookie/longhorn/core";

import type {
  BackupCreateCommand,
  BackupCreateOutcome,
  BackupExportCommand,
  BackupExportOutcome,
  BackupRetentionApplyCommand,
  BackupRetentionApplyOutcome,
  ConfigOperationsSnapshot,
  ConfigSnapshotCommand,
  RestoreAdapterExecuteCommand,
  RestoreAdapterExecuteOutcome,
  RestoreExecuteCommand,
  RestoreExecuteOutcome,
  RestoreInspectCommand,
  RestoreInspectOutcome,
  RestorePlanCommand,
  RestorePlanOutcome,
  RestoreRecoveryCommand,
  RestoreRecoveryOutcomeProjection,
  StorageCleanupCommand,
  StorageCleanupOutcome,
  StorageRecoveryCommand,
  StorageRecoveryOutcome,
  StorageTransitionExecuteCommand,
  StorageTransitionExecuteOutcome,
  StorageTransitionInspectCommand,
  StorageTransitionInspectOutcome,
} from "./generated/protocol.ts";
import {
  assertValidBackupCreateCommand,
  assertValidBackupCreateOutcome,
  assertValidBackupExportCommand,
  assertValidBackupExportOutcome,
  assertValidBackupRetentionApplyCommand,
  assertValidBackupRetentionApplyOutcome,
  assertValidConfigOperationsSnapshot,
  assertValidConfigSnapshotCommand,
  assertValidRestoreAdapterExecuteCommand,
  assertValidRestoreAdapterExecuteOutcome,
  assertValidRestoreExecuteCommand,
  assertValidRestoreExecuteOutcome,
  assertValidRestoreInspectCommand,
  assertValidRestoreInspectOutcome,
  assertValidRestorePlanCommand,
  assertValidRestorePlanOutcome,
  assertValidRestoreRecoveryCommand,
  assertValidRestoreRecoveryOutcome,
  assertValidStorageCleanupCommand,
  assertValidStorageCleanupOutcome,
  assertValidStorageRecoveryCommand,
  assertValidStorageRecoveryOutcome,
  assertValidStorageTransitionExecuteCommand,
  assertValidStorageTransitionExecuteOutcome,
  assertValidStorageTransitionInspectCommand,
  assertValidStorageTransitionInspectOutcome,
} from "./validation.ts";
import {
  CONFIG_BACKUP_CREATE_COMMAND,
  CONFIG_BACKUP_EXPORT_COMMAND,
  CONFIG_BACKUP_RETENTION_COMMAND,
  CONFIG_SNAPSHOT_COMMAND,
  CONFIG_RESTORE_ADAPTER_EXECUTE_COMMAND,
  CONFIG_RESTORE_EXECUTE_COMMAND,
  CONFIG_RESTORE_INSPECT_COMMAND,
  CONFIG_RESTORE_PLAN_COMMAND,
  CONFIG_RESTORE_RECOVER_COMMAND,
  CONFIG_STORAGE_CLEANUP_COMMAND,
  CONFIG_STORAGE_EXECUTE_COMMAND,
  CONFIG_STORAGE_INSPECT_COMMAND,
  CONFIG_STORAGE_RECOVER_COMMAND,
} from "./names.ts";

export class ConfigOperationsClient {
  readonly #transport: EventTransport;

  constructor(transport: EventTransport) {
    this.#transport = transport;
  }

  async snapshot(command: ConfigSnapshotCommand): Promise<ConfigOperationsSnapshot> {
    assertValidConfigSnapshotCommand(command);
    const value = await this.#transport.invoke(CONFIG_SNAPSHOT_COMMAND, { command });
    assertValidConfigOperationsSnapshot(value);
    return value;
  }

  async inspectStorageTransition(
    command: StorageTransitionInspectCommand,
  ): Promise<StorageTransitionInspectOutcome> {
    assertValidStorageTransitionInspectCommand(command);
    const value = await this.#transport.invoke(CONFIG_STORAGE_INSPECT_COMMAND, {
      command,
    });
    assertValidStorageTransitionInspectOutcome(value);
    return value;
  }

  async executeStorageTransition(
    command: StorageTransitionExecuteCommand,
  ): Promise<StorageTransitionExecuteOutcome> {
    assertValidStorageTransitionExecuteCommand(command);
    const value = await this.#transport.invoke(CONFIG_STORAGE_EXECUTE_COMMAND, {
      command,
    });
    assertValidStorageTransitionExecuteOutcome(value);
    return value;
  }

  async recoverStorage(
    command: StorageRecoveryCommand,
  ): Promise<StorageRecoveryOutcome> {
    assertValidStorageRecoveryCommand(command);
    const value = await this.#transport.invoke(CONFIG_STORAGE_RECOVER_COMMAND, {
      command,
    });
    assertValidStorageRecoveryOutcome(value);
    return value;
  }

  async cleanupStorage(
    command: StorageCleanupCommand,
  ): Promise<StorageCleanupOutcome> {
    assertValidStorageCleanupCommand(command);
    const value = await this.#transport.invoke(CONFIG_STORAGE_CLEANUP_COMMAND, {
      command,
    });
    assertValidStorageCleanupOutcome(value);
    return value;
  }

  async createBackup(command: BackupCreateCommand): Promise<BackupCreateOutcome> {
    assertValidBackupCreateCommand(command);
    const value = await this.#transport.invoke(CONFIG_BACKUP_CREATE_COMMAND, {
      command,
    });
    assertValidBackupCreateOutcome(value);
    return value;
  }

  async exportBackup(command: BackupExportCommand): Promise<BackupExportOutcome> {
    assertValidBackupExportCommand(command);
    const value = await this.#transport.invoke(CONFIG_BACKUP_EXPORT_COMMAND, {
      command,
    });
    assertValidBackupExportOutcome(value);
    return value;
  }

  async applyBackupRetention(
    command: BackupRetentionApplyCommand,
  ): Promise<BackupRetentionApplyOutcome> {
    assertValidBackupRetentionApplyCommand(command);
    const value = await this.#transport.invoke(CONFIG_BACKUP_RETENTION_COMMAND, {
      command,
    });
    assertValidBackupRetentionApplyOutcome(value);
    return value;
  }

  async inspectRestore(
    command: RestoreInspectCommand,
  ): Promise<RestoreInspectOutcome> {
    assertValidRestoreInspectCommand(command);
    const value = await this.#transport.invoke(CONFIG_RESTORE_INSPECT_COMMAND, {
      command,
    });
    assertValidRestoreInspectOutcome(value);
    return value;
  }

  async planRestore(command: RestorePlanCommand): Promise<RestorePlanOutcome> {
    assertValidRestorePlanCommand(command);
    const value = await this.#transport.invoke(CONFIG_RESTORE_PLAN_COMMAND, {
      command,
    });
    assertValidRestorePlanOutcome(value);
    return value;
  }

  async executeRestore(
    command: RestoreExecuteCommand,
  ): Promise<RestoreExecuteOutcome> {
    assertValidRestoreExecuteCommand(command);
    const value = await this.#transport.invoke(CONFIG_RESTORE_EXECUTE_COMMAND, {
      command,
    });
    assertValidRestoreExecuteOutcome(value);
    return value;
  }

  async executeAdapterRestore(
    command: RestoreAdapterExecuteCommand,
  ): Promise<RestoreAdapterExecuteOutcome> {
    assertValidRestoreAdapterExecuteCommand(command);
    const value = await this.#transport.invoke(
      CONFIG_RESTORE_ADAPTER_EXECUTE_COMMAND,
      { command },
    );
    assertValidRestoreAdapterExecuteOutcome(value);
    return value;
  }

  async recoverRestore(
    command: RestoreRecoveryCommand,
  ): Promise<RestoreRecoveryOutcomeProjection> {
    assertValidRestoreRecoveryCommand(command);
    const value = await this.#transport.invoke(CONFIG_RESTORE_RECOVER_COMMAND, {
      command,
    });
    assertValidRestoreRecoveryOutcome(value);
    return value;
  }
}
