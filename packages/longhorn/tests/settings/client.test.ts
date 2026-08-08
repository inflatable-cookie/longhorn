import { describe, expect, test } from "bun:test";

import type { EventTransport, Unlisten } from "@inflatable-cookie/longhorn/core";
import {
  SETTINGS_APPLY_COMMAND,
  SETTINGS_LOAD_COMMAND,
  SETTINGS_REGISTRY_CHANGED_EVENT,
  SETTINGS_REGISTRY_COMMAND,
  SETTINGS_RESET_COMMAND,
  SETTINGS_SCOPE_CHANGED_EVENT,
  SettingsClient,
  type SettingsApplyCommand,
  type SettingsResetCommand,
} from "../../src/settings/index.ts";
import { fixture, registry, snapshot } from "./support.ts";

describe("settings client", () => {
  test("installs registry listener before snapshot and rejects stale generations", async () => {
    const transport = new FakeSettingsTransport();
    const seen: number[] = [];
    const client = new SettingsClient(transport);
    const connection = client.connectRegistry((value) => {
      seen.push(value.generation);
    });

    await connection.ready;
    expect(transport.trace.slice(0, 2)).toEqual([
      `listen:${SETTINGS_REGISTRY_CHANGED_EVENT}`,
      `invoke:${SETTINGS_REGISTRY_COMMAND}`,
    ]);
    transport.registryValue = registry(8);
    transport.emit(SETTINGS_REGISTRY_CHANGED_EVENT, {
      protocolVersion: 1,
      registryGeneration: 8,
    });
    await transport.drain();
    transport.registryValue = registry(7);
    transport.emit(SETTINGS_REGISTRY_CHANGED_EVENT, {
      protocolVersion: 1,
      registryGeneration: 9,
    });
    await transport.drain();
    expect(seen).toEqual([7, 8]);
    expect(connection.current()?.generation).toBe(8);
    await connection.dispose();
    await connection.dispose();
    expect(transport.unlistenCount).toBe(1);
  });

  test("scope connection reloads hints and preserves newer authority", async () => {
    const transport = new FakeSettingsTransport();
    transport.scopeValue = snapshot(3);
    const client = new SettingsClient(transport);
    let request = 0;
    const seen: number[] = [];
    const connection = client.connectScope({
      registry: registry(),
      scopeId: "app:preferences",
      nextRequestId: () => `request:scope-${++request}`,
      onSnapshot: (value) => seen.push(value.authority.scopeRevision),
    });

    await connection.ready;
    expect(transport.trace.slice(0, 2)).toEqual([
      `listen:${SETTINGS_SCOPE_CHANGED_EVENT}`,
      `invoke:${SETTINGS_LOAD_COMMAND}`,
    ]);
    transport.scopeValue = snapshot(5);
    transport.emit(SETTINGS_SCOPE_CHANGED_EVENT, {
      protocolVersion: 1,
      registryGeneration: 7,
      scopeId: "app:preferences",
      scopeRevision: 5,
    });
    await transport.drain();
    transport.scopeValue = snapshot(4);
    transport.emit(SETTINGS_SCOPE_CHANGED_EVENT, {
      protocolVersion: 1,
      registryGeneration: 7,
      scopeId: "app:preferences",
      scopeRevision: 6,
    });
    await transport.drain();
    expect(seen).toEqual([3, 5]);
    expect(connection.current()?.authority.scopeRevision).toBe(5);
    await connection.dispose();
  });

  test("late listener registration unlistens exactly once without loading", async () => {
    const transport = new FakeSettingsTransport();
    transport.deferListener = true;
    const client = new SettingsClient(transport);
    const connection = client.connectRegistry();
    const ready = connection.ready;
    const disposal = connection.dispose();
    transport.releaseListener();
    await expect(ready).rejects.toThrow(/disposed/);
    await disposal;
    expect(
      transport.trace.filter((item) =>
        item.startsWith(`invoke:${SETTINGS_REGISTRY_COMMAND}`),
      ),
    ).toEqual([]);
    expect(transport.unlistenCount).toBe(1);
  });

  test("uses exact checked command envelopes through serialized transport", async () => {
    const transport = new FakeSettingsTransport();
    transport.serialize = true;
    const client = new SettingsClient(transport);
    const apply = structuredClone(
      fixture.applyCommands[0],
    ) as SettingsApplyCommand;
    const reset = structuredClone(
      fixture.resetCommands[0],
    ) as SettingsResetCommand;
    transport.mutationValue = fixture.mutationResults[0]!;
    expect((await client.apply(registry(), apply)).status).toBe("applied");
    transport.mutationValue = fixture.mutationResults[1]!;
    expect((await client.reset(registry(), reset)).status).toBe("applied");
    expect(transport.argumentsFor(SETTINGS_APPLY_COMMAND)).toEqual({
      command: apply,
    });
    expect(transport.argumentsFor(SETTINGS_RESET_COMMAND)).toEqual({
      command: reset,
    });
  });
});

class FakeSettingsTransport implements EventTransport {
  readonly trace: string[] = [];
  registryValue = registry();
  scopeValue = snapshot();
  mutationValue: unknown = fixture.mutationResults[0]!;
  serialize = false;
  deferListener = false;
  unlistenCount = 0;

  readonly #listeners = new Map<string, (payload: unknown) => void>();
  readonly #arguments = new Map<string, Record<string, unknown>>();
  #release: (() => void) | undefined;

  async invoke(
    command: string,
    arguments_: Record<string, unknown>,
  ): Promise<unknown> {
    this.trace.push(`invoke:${command}`);
    this.#arguments.set(command, clone(arguments_));
    let response: unknown;
    switch (command) {
      case SETTINGS_REGISTRY_COMMAND:
        response = this.registryValue;
        break;
      case SETTINGS_LOAD_COMMAND:
        response = { status: "loaded", snapshot: this.scopeValue };
        break;
      case SETTINGS_APPLY_COMMAND:
      case SETTINGS_RESET_COMMAND:
        response = this.mutationValue;
        break;
      default:
        throw new Error(`unexpected command ${command}`);
    }
    return this.serialize ? clone(response) : response;
  }

  async listen(
    event: string,
    listener: (payload: unknown) => void,
  ): Promise<Unlisten> {
    this.trace.push(`listen:${event}`);
    this.#listeners.set(event, listener);
    if (this.deferListener) {
      await new Promise<void>((resolve) => {
        this.#release = resolve;
      });
    }
    let active = true;
    return () => {
      if (active) {
        active = false;
        this.unlistenCount += 1;
        this.#listeners.delete(event);
      }
    };
  }

  emit(event: string, payload: unknown): void {
    this.#listeners.get(event)?.(clone(payload));
  }

  releaseListener(): void {
    this.#release?.();
  }

  argumentsFor(command: string): Record<string, unknown> | undefined {
    return this.#arguments.get(command);
  }

  async drain(): Promise<void> {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
  }
}

function clone<Value>(value: Value): Value {
  return JSON.parse(JSON.stringify(value)) as Value;
}
