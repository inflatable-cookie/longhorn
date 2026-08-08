import type { EventTransport, Unlisten } from "@inflatable-cookie/longhorn-core";
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
  type ConfigOperationsSnapshot,
} from "../src/index.ts";
import fixtureJson from "../../../fixtures/config/protocol-v1.json";

export const fixture = fixtureJson;

export class PageTransport implements EventTransport {
  readonly calls: string[] = [];
  readonly #held = new Map<string, Promise<void>>();
  readonly #release = new Map<string, () => void>();
  readonly responses = new Map<string, unknown>([
    [CONFIG_SNAPSHOT_COMMAND, fixture.snapshot],
    [CONFIG_STORAGE_INSPECT_COMMAND, fixture.outcomes.inspectTransition],
    [CONFIG_STORAGE_EXECUTE_COMMAND, fixture.outcomes.executeTransition],
    [CONFIG_STORAGE_RECOVER_COMMAND, fixture.outcomes.recoverStorage],
    [CONFIG_STORAGE_CLEANUP_COMMAND, fixture.outcomes.cleanupStorage],
    [CONFIG_BACKUP_CREATE_COMMAND, fixture.outcomes.createBackup],
    [CONFIG_BACKUP_EXPORT_COMMAND, fixture.outcomes.exportBackup],
    [CONFIG_BACKUP_RETENTION_COMMAND, fixture.outcomes.applyRetention],
    [CONFIG_RESTORE_INSPECT_COMMAND, fixture.outcomes.inspectRestore],
    [CONFIG_RESTORE_PLAN_COMMAND, fixture.outcomes.planRestore],
    [CONFIG_RESTORE_EXECUTE_COMMAND, fixture.outcomes.executeRestore],
    [CONFIG_RESTORE_ADAPTER_EXECUTE_COMMAND, fixture.outcomes.executeAdapterRestore],
    [CONFIG_RESTORE_RECOVER_COMMAND, fixture.outcomes.recoverRestore],
  ]);

  hold(command: string): void {
    if (this.#held.has(command)) return;
    this.#held.set(
      command,
      new Promise<void>((resolve) => this.#release.set(command, resolve)),
    );
  }

  release(command: string): void {
    this.#release.get(command)?.();
    this.#held.delete(command);
    this.#release.delete(command);
  }

  async invoke(command: string): Promise<unknown> {
    this.calls.push(command);
    await this.#held.get(command);
    const response = this.responses.get(command);
    if (response === undefined) throw new Error(`unexpected command ${command}`);
    return structuredClone(response);
  }

  async listen(): Promise<Unlisten> {
    throw new Error("config pages do not use ambient event listeners");
  }
}

export function pageFixture(): {
  client: ConfigOperationsClient;
  snapshot: ConfigOperationsSnapshot;
  transport: PageTransport;
} {
  const transport = new PageTransport();
  return {
    client: new ConfigOperationsClient(transport),
    snapshot: structuredClone(fixture.snapshot) as ConfigOperationsSnapshot,
    transport,
  };
}
