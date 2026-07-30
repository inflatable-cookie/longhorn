import type { EventTransport, Unlisten } from "@longhorn/core";
import {
  CONFIG_BACKUP_CREATE_COMMAND,
  CONFIG_BACKUP_EXPORT_COMMAND,
  CONFIG_BACKUP_RETENTION_COMMAND,
  CONFIG_RESTORE_ADAPTER_EXECUTE_COMMAND,
  CONFIG_RESTORE_EXECUTE_COMMAND,
  CONFIG_RESTORE_INSPECT_COMMAND,
  CONFIG_RESTORE_PLAN_COMMAND,
  CONFIG_RESTORE_RECOVER_COMMAND,
  CONFIG_SNAPSHOT_COMMAND,
  CONFIG_STORAGE_CLEANUP_COMMAND,
  CONFIG_STORAGE_EXECUTE_COMMAND,
  CONFIG_STORAGE_INSPECT_COMMAND,
  CONFIG_STORAGE_RECOVER_COMMAND,
  ConfigOperationsClient,
} from "@longhorn/config";

import configFixture from "./fixtures/config-protocol-v1.json";

export type RestoreTerminal = "succeeded" | "rolledBack" | "recoveryRequired";

export class ConfigProofTransport implements EventTransport {
  readonly trace: string[] = [];
  readonly arguments: Record<string, unknown>[] = [];
  restoreTerminal: RestoreTerminal = "succeeded";
  publications = 0;

  client(): ConfigOperationsClient {
    return new ConfigOperationsClient(this);
  }

  async invoke(
    command: string,
    arguments_: Record<string, unknown>,
  ): Promise<unknown> {
    this.trace.push(command);
    this.arguments.push(structuredClone(arguments_));
    const outcomes = configFixture.outcomes;
    switch (command) {
      case CONFIG_SNAPSHOT_COMMAND:
        return structuredClone(configFixture.snapshot);
      case CONFIG_STORAGE_INSPECT_COMMAND:
        return structuredClone(outcomes.inspectTransition);
      case CONFIG_STORAGE_EXECUTE_COMMAND:
        return this.publish(outcomes.executeTransition);
      case CONFIG_STORAGE_RECOVER_COMMAND:
        return this.publish(outcomes.recoverStorage);
      case CONFIG_STORAGE_CLEANUP_COMMAND:
        return this.publish(outcomes.cleanupStorage);
      case CONFIG_BACKUP_CREATE_COMMAND:
        return this.publish(outcomes.createBackup);
      case CONFIG_BACKUP_EXPORT_COMMAND:
        return this.publish(outcomes.exportBackup);
      case CONFIG_BACKUP_RETENTION_COMMAND:
        return this.publish(outcomes.applyRetention);
      case CONFIG_RESTORE_INSPECT_COMMAND:
        return structuredClone(configFixture.restoreInspectionStates[0]);
      case CONFIG_RESTORE_PLAN_COMMAND:
        return structuredClone(configFixture.restorePlanStates[0]);
      case CONFIG_RESTORE_EXECUTE_COMMAND:
        return this.publish(
          configFixture.restoreExecutionStates.find(
            ({ status }) => status === this.restoreTerminal,
          ),
        );
      case CONFIG_RESTORE_ADAPTER_EXECUTE_COMMAND:
        return this.publish(outcomes.executeAdapterRestore);
      case CONFIG_RESTORE_RECOVER_COMMAND:
        return this.publish(outcomes.recoverRestore);
      default:
        throw new Error(`unexpected config command ${command}`);
    }
  }

  async listen(): Promise<Unlisten> {
    return () => undefined;
  }

  calls(command: string): number {
    return this.trace.filter((entry) => entry === command).length;
  }

  private publish<T>(value: T): T {
    this.publications += 1;
    return structuredClone(value);
  }
}
