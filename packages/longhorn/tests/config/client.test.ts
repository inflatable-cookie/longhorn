import { describe, expect, test } from "bun:test";

import type { InvokeTransport } from "@inflatable-cookie/longhorn/core";
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
  ConfigOperationsClient,
} from "../../src/config/index.ts";
import type {
  BackupCreateCommand,
  RestoreAdapterExecuteCommand,
  RestoreInspectCommand,
  RestorePlanCommand,
  StorageTransitionInspectCommand,
} from "../../src/config/index.ts";
import { fixture } from "./support.ts";

describe("config operations client", () => {
  test("uses exact checked command envelopes through serialized transport", async () => {
    const transport = new FakeConfigTransport();
    const client = new ConfigOperationsClient(transport);

    await client.snapshot(fixture.commands.snapshot);
    await client.inspectStorageTransition(
      fixture.commands.inspectTransition as StorageTransitionInspectCommand,
    );
    await client.executeStorageTransition(fixture.commands.executeTransition);
    await client.recoverStorage(fixture.commands.recoverStorage);
    await client.cleanupStorage(fixture.commands.cleanupStorage);
    await client.createBackup(fixture.commands.createBackup as BackupCreateCommand);
    await client.exportBackup(fixture.commands.exportBackup);
    await client.applyBackupRetention(fixture.commands.applyRetention);
    await client.inspectRestore(fixture.commands.inspectRestore as RestoreInspectCommand);
    await client.planRestore(fixture.commands.planRestore as RestorePlanCommand);
    await client.executeRestore(fixture.commands.executeRestore);
    await client.executeAdapterRestore(
      fixture.commands.executeAdapterRestore as RestoreAdapterExecuteCommand,
    );
    await client.recoverRestore(fixture.commands.recoverRestore);

    expect(transport.commands).toEqual([
      CONFIG_SNAPSHOT_COMMAND,
      CONFIG_STORAGE_INSPECT_COMMAND,
      CONFIG_STORAGE_EXECUTE_COMMAND,
      CONFIG_STORAGE_RECOVER_COMMAND,
      CONFIG_STORAGE_CLEANUP_COMMAND,
      CONFIG_BACKUP_CREATE_COMMAND,
      CONFIG_BACKUP_EXPORT_COMMAND,
      CONFIG_BACKUP_RETENTION_COMMAND,
      CONFIG_RESTORE_INSPECT_COMMAND,
      CONFIG_RESTORE_PLAN_COMMAND,
      CONFIG_RESTORE_EXECUTE_COMMAND,
      CONFIG_RESTORE_ADAPTER_EXECUTE_COMMAND,
      CONFIG_RESTORE_RECOVER_COMMAND,
    ]);
    expect(transport.arguments.get(CONFIG_STORAGE_EXECUTE_COMMAND)).toEqual({
      command: fixture.commands.executeTransition,
    });
  });
});

class FakeConfigTransport implements InvokeTransport {
  readonly commands: Array<string> = [];
  readonly arguments = new Map<string, Record<string, unknown>>();

  async invoke(command: string, arguments_: Record<string, unknown>): Promise<unknown> {
    this.commands.push(command);
    this.arguments.set(command, clone(arguments_));
    const response = responses.get(command);
    if (response === undefined) {
      throw new Error(`unexpected command ${command}`);
    }
    return clone(response);
  }
}

const responses = new Map<string, unknown>(
  Object.entries({
    [CONFIG_SNAPSHOT_COMMAND]: fixture.snapshot,
    [CONFIG_STORAGE_INSPECT_COMMAND]: fixture.outcomes.inspectTransition,
    [CONFIG_STORAGE_EXECUTE_COMMAND]: fixture.outcomes.executeTransition,
    [CONFIG_STORAGE_RECOVER_COMMAND]: fixture.outcomes.recoverStorage,
    [CONFIG_STORAGE_CLEANUP_COMMAND]: fixture.outcomes.cleanupStorage,
    [CONFIG_BACKUP_CREATE_COMMAND]: fixture.outcomes.createBackup,
    [CONFIG_BACKUP_EXPORT_COMMAND]: fixture.outcomes.exportBackup,
    [CONFIG_BACKUP_RETENTION_COMMAND]: fixture.outcomes.applyRetention,
    [CONFIG_RESTORE_INSPECT_COMMAND]: fixture.outcomes.inspectRestore,
    [CONFIG_RESTORE_PLAN_COMMAND]: fixture.outcomes.planRestore,
    [CONFIG_RESTORE_EXECUTE_COMMAND]: fixture.outcomes.executeRestore,
    [CONFIG_RESTORE_ADAPTER_EXECUTE_COMMAND]: fixture.outcomes.executeAdapterRestore,
    [CONFIG_RESTORE_RECOVER_COMMAND]: fixture.outcomes.recoverRestore,
  }),
);

function clone<Value>(value: Value): Value {
  return structuredClone<Value>(value);
}
