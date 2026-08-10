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
  assertCompatibleBackupCreateCommand,
  assertCompatibleBackupCreateOutcome,
  assertCompatibleBackupExportCommand,
  assertCompatibleBackupExportOutcome,
  assertCompatibleBackupRetentionApplyCommand,
  assertCompatibleBackupRetentionApplyOutcome,
  assertCompatibleConfigOperationsSnapshot,
  assertCompatibleConfigSnapshotCommand,
  assertCompatibleRestoreAdapterExecuteCommand,
  assertCompatibleRestoreAdapterExecuteOutcome,
  assertCompatibleRestoreExecuteCommand,
  assertCompatibleRestoreExecuteOutcome,
  assertCompatibleRestoreInspectCommand,
  assertCompatibleRestoreInspectOutcome,
  assertCompatibleRestorePlanCommand,
  assertCompatibleRestorePlanOutcome,
  assertCompatibleRestoreRecoveryCommand,
  assertCompatibleRestoreRecoveryOutcome,
  assertCompatibleStorageCleanupCommand,
  assertCompatibleStorageCleanupOutcome,
  assertCompatibleStorageRecoveryCommand,
  assertCompatibleStorageRecoveryOutcome,
  assertCompatibleStorageTransitionExecuteCommand,
  assertCompatibleStorageTransitionExecuteOutcome,
  assertCompatibleStorageTransitionInspectCommand,
  assertCompatibleStorageTransitionInspectOutcome,
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
    assertCompatibleConfigSnapshotCommand(command);
    const value = await this.#transport.invoke(CONFIG_SNAPSHOT_COMMAND, { command });
    assertCompatibleConfigOperationsSnapshot(value);
    return value;
  }

  async inspectStorageTransition(
    command: StorageTransitionInspectCommand,
  ): Promise<StorageTransitionInspectOutcome> {
    assertCompatibleStorageTransitionInspectCommand(command);
    const value = await this.#transport.invoke(CONFIG_STORAGE_INSPECT_COMMAND, {
      command,
    });
    assertCompatibleStorageTransitionInspectOutcome(value);
    return value;
  }

  async executeStorageTransition(
    command: StorageTransitionExecuteCommand,
  ): Promise<StorageTransitionExecuteOutcome> {
    assertCompatibleStorageTransitionExecuteCommand(command);
    const value = await this.#transport.invoke(CONFIG_STORAGE_EXECUTE_COMMAND, {
      command,
    });
    assertCompatibleStorageTransitionExecuteOutcome(value);
    return value;
  }

  async recoverStorage(
    command: StorageRecoveryCommand,
  ): Promise<StorageRecoveryOutcome> {
    assertCompatibleStorageRecoveryCommand(command);
    const value = await this.#transport.invoke(CONFIG_STORAGE_RECOVER_COMMAND, {
      command,
    });
    assertCompatibleStorageRecoveryOutcome(value);
    return value;
  }

  async cleanupStorage(
    command: StorageCleanupCommand,
  ): Promise<StorageCleanupOutcome> {
    assertCompatibleStorageCleanupCommand(command);
    const value = await this.#transport.invoke(CONFIG_STORAGE_CLEANUP_COMMAND, {
      command,
    });
    assertCompatibleStorageCleanupOutcome(value);
    return value;
  }

  async createBackup(command: BackupCreateCommand): Promise<BackupCreateOutcome> {
    assertCompatibleBackupCreateCommand(command);
    const value = await this.#transport.invoke(CONFIG_BACKUP_CREATE_COMMAND, {
      command,
    });
    assertCompatibleBackupCreateOutcome(value);
    return value;
  }

  async exportBackup(command: BackupExportCommand): Promise<BackupExportOutcome> {
    assertCompatibleBackupExportCommand(command);
    const value = await this.#transport.invoke(CONFIG_BACKUP_EXPORT_COMMAND, {
      command,
    });
    assertCompatibleBackupExportOutcome(value);
    return value;
  }

  async applyBackupRetention(
    command: BackupRetentionApplyCommand,
  ): Promise<BackupRetentionApplyOutcome> {
    assertCompatibleBackupRetentionApplyCommand(command);
    const value = await this.#transport.invoke(CONFIG_BACKUP_RETENTION_COMMAND, {
      command,
    });
    assertCompatibleBackupRetentionApplyOutcome(value);
    return value;
  }

  async inspectRestore(
    command: RestoreInspectCommand,
  ): Promise<RestoreInspectOutcome> {
    assertCompatibleRestoreInspectCommand(command);
    const value = await this.#transport.invoke(CONFIG_RESTORE_INSPECT_COMMAND, {
      command,
    });
    assertCompatibleRestoreInspectOutcome(value);
    return value;
  }

  async planRestore(command: RestorePlanCommand): Promise<RestorePlanOutcome> {
    assertCompatibleRestorePlanCommand(command);
    const value = await this.#transport.invoke(CONFIG_RESTORE_PLAN_COMMAND, {
      command,
    });
    assertCompatibleRestorePlanOutcome(value);
    return value;
  }

  async executeRestore(
    command: RestoreExecuteCommand,
  ): Promise<RestoreExecuteOutcome> {
    assertCompatibleRestoreExecuteCommand(command);
    const value = await this.#transport.invoke(CONFIG_RESTORE_EXECUTE_COMMAND, {
      command,
    });
    assertCompatibleRestoreExecuteOutcome(value);
    return value;
  }

  async executeAdapterRestore(
    command: RestoreAdapterExecuteCommand,
  ): Promise<RestoreAdapterExecuteOutcome> {
    assertCompatibleRestoreAdapterExecuteCommand(command);
    const value = await this.#transport.invoke(
      CONFIG_RESTORE_ADAPTER_EXECUTE_COMMAND,
      { command },
    );
    assertCompatibleRestoreAdapterExecuteOutcome(value);
    return value;
  }

  async recoverRestore(
    command: RestoreRecoveryCommand,
  ): Promise<RestoreRecoveryOutcomeProjection> {
    assertCompatibleRestoreRecoveryCommand(command);
    const value = await this.#transport.invoke(CONFIG_RESTORE_RECOVER_COMMAND, {
      command,
    });
    assertCompatibleRestoreRecoveryOutcome(value);
    return value;
  }
}
