import fixtureJson from "../../../../fixtures/settings/protocol-v1.json";

import type { EventTransport, Unlisten } from "@inflatable-cookie/longhorn/core";
import {
  SETTINGS_APPLY_COMMAND,
  SETTINGS_LOAD_COMMAND,
  SETTINGS_REGISTRY_CHANGED_EVENT,
  SETTINGS_REGISTRY_COMMAND,
  SETTINGS_RESET_COMMAND,
  SETTINGS_SCOPE_CHANGED_EVENT,
  SettingsClient,
  type SettingsMutationResult,
  type SettingsRegistrySnapshot,
  type SettingsScopeSnapshot,
} from "../../../longhorn/src/settings/index.ts";
import type { SettingsPageRenderer } from "../../src/settings/svelte.ts";

export const fixture = fixtureJson;

export interface Deferred {
  readonly promise: Promise<void>;
  resolve(): void;
}

export function deferred(): Deferred {
  let resolve!: () => void;
  const promise = new Promise<void>((resolve_) => {
    resolve = resolve_;
  });
  return { promise, resolve };
}

export function registry(
  timing: "immediate" | "staged" = "staged",
): SettingsRegistrySnapshot {
  const value = structuredClone(
    fixture.registry,
  ) as SettingsRegistrySnapshot;
  value.applyUnits[0]!.timing = timing;
  return value;
}

export function twoPageRegistry(): SettingsRegistrySnapshot {
  const value = registry();
  value.pages.push({
    ...structuredClone(value.pages[0]!),
    id: "app:advanced",
    label: "Advanced",
    keywords: ["expert"],
    order: 20,
    anchors: [],
  });
  return value;
}

export function snapshot(): SettingsScopeSnapshot {
  return structuredClone(
    fixture.snapshots[0],
  ) as SettingsScopeSnapshot;
}

export function renderer(): SettingsPageRenderer {
  return (() => undefined) as unknown as SettingsPageRenderer;
}

export class FakeSettingsTransport implements EventTransport {
  registryValue = registry();
  scopeValue = snapshot();
  mutationValue = structuredClone(
    fixture.mutationResults[0],
  ) as SettingsMutationResult;
  loadError: unknown;
  mutationError: unknown;
  loadGate: Promise<void> | undefined;
  mutationGate: Promise<void> | undefined;
  deferRegistryListener = false;
  readonly trace: string[] = [];
  readonly arguments: Record<string, unknown>[] = [];
  unlistenCount = 0;

  readonly #listeners = new Map<
    string,
    Set<(payload: unknown) => void>
  >();
  #releaseRegistryListener: (() => void) | undefined;

  client(): SettingsClient {
    return new SettingsClient(this);
  }

  async invoke(
    command: string,
    arguments_: Record<string, unknown>,
  ): Promise<unknown> {
    this.trace.push(`invoke:${command}`);
    this.arguments.push(structuredClone(arguments_));
    switch (command) {
      case SETTINGS_REGISTRY_COMMAND:
        return structuredClone(this.registryValue);
      case SETTINGS_LOAD_COMMAND:
        await this.loadGate;
        if (this.loadError !== undefined) {
          throw this.loadError;
        }
        return {
          status: "loaded",
          snapshot: structuredClone(this.scopeValue),
        };
      case SETTINGS_APPLY_COMMAND:
      case SETTINGS_RESET_COMMAND:
        await this.mutationGate;
        if (this.mutationError !== undefined) {
          throw this.mutationError;
        }
        return structuredClone(this.mutationValue);
      default:
        throw new Error(`unexpected settings command ${command}`);
    }
  }

  async listen(
    event: string,
    listener: (payload: unknown) => void,
  ): Promise<Unlisten> {
    this.trace.push(`listen:${event}`);
    const listeners = this.#listeners.get(event) ?? new Set();
    listeners.add(listener);
    this.#listeners.set(event, listeners);
    if (
      event === SETTINGS_REGISTRY_CHANGED_EVENT &&
      this.deferRegistryListener
    ) {
      await new Promise<void>((resolve) => {
        this.#releaseRegistryListener = resolve;
      });
    }
    let active = true;
    return () => {
      if (!active) return;
      active = false;
      listeners.delete(listener);
      this.unlistenCount += 1;
    };
  }

  emitScope(snapshot_: SettingsScopeSnapshot): void {
    this.scopeValue = structuredClone(snapshot_);
    this.#emit(SETTINGS_SCOPE_CHANGED_EVENT, {
      protocolVersion: 1,
      registryGeneration: snapshot_.authority.registryGeneration,
      scopeId: snapshot_.scopeId,
      scopeRevision: snapshot_.authority.scopeRevision,
    });
  }

  releaseRegistryListener(): void {
    this.#releaseRegistryListener?.();
  }

  activeListenerCount(): number {
    return [...this.#listeners.values()].reduce(
      (total, listeners) => total + listeners.size,
      0,
    );
  }

  calls(command: string): number {
    return this.trace.filter((item) => item === `invoke:${command}`).length;
  }

  async drain(): Promise<void> {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
  }

  #emit(event: string, payload: unknown): void {
    for (const listener of this.#listeners.get(event) ?? []) {
      listener(structuredClone(payload));
    }
  }
}

export function requestIds(prefix: string): () => string {
  let number = 0;
  return () => `request:${prefix}-${++number}`;
}
