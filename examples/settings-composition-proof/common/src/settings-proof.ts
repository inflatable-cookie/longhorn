import type { EventTransport, Unlisten } from "@inflatable-cookie/longhorn-core";
import {
  SETTINGS_APPLY_COMMAND,
  SETTINGS_LOAD_COMMAND,
  SETTINGS_REGISTRY_CHANGED_EVENT,
  SETTINGS_REGISTRY_COMMAND,
  SETTINGS_RESET_COMMAND,
  SETTINGS_SCOPE_CHANGED_EVENT,
  SettingsClient,
  type SettingsApplyCommand,
  type SettingsMutationResult,
  type SettingsRegistrySnapshot,
  type SettingsScopeSnapshot,
} from "@inflatable-cookie/longhorn-settings";
import { SettingsSession } from "@inflatable-cookie/longhorn-settings/poodle";

import {
  createProofRegistry,
  createProofSnapshot,
  type ProofShape,
} from "./settings-registry.ts";

export {
  createProofRegistry,
  createProofSnapshot,
  type ProofShape,
} from "./settings-registry.ts";
export type MutationMode =
  | "applied"
  | "conflict"
  | "invalidIntent"
  | "policyBlocked"
  | "recoveryRequired";

export class SettingsProofTransport implements EventTransport {
  readonly registryValue: SettingsRegistrySnapshot;
  scopeValue: SettingsScopeSnapshot;
  mutationMode: MutationMode = "applied";
  readonly trace: string[] = [];
  readonly arguments: Record<string, unknown>[] = [];
  publications = 0;
  unlistenCount = 0;

  readonly #listeners = new Map<string, Set<(payload: unknown) => void>>();

  constructor(readonly shape: ProofShape) {
    this.registryValue = createProofRegistry(shape);
    this.scopeValue = createProofSnapshot(shape);
  }

  client(): SettingsClient {
    return new SettingsClient(this);
  }

  session(onClose?: () => void): SettingsSession {
    let request = 0;
    return new SettingsSession({
      client: this.client(),
      nextRequestId: () => `request:${this.shape}-${++request}`,
      onClose,
    });
  }

  async invoke(
    commandName: string,
    arguments_: Record<string, unknown>,
  ): Promise<unknown> {
    this.trace.push(`invoke:${commandName}`);
    this.arguments.push(structuredClone(arguments_));
    if (commandName === SETTINGS_REGISTRY_COMMAND) {
      return structuredClone(this.registryValue);
    }
    if (commandName === SETTINGS_LOAD_COMMAND) {
      return {
        status: "loaded",
        snapshot: structuredClone(this.scopeValue),
      };
    }
    if (
      commandName === SETTINGS_APPLY_COMMAND ||
      commandName === SETTINGS_RESET_COMMAND
    ) {
      return this.mutationResult(
        arguments_.command as SettingsApplyCommand,
      );
    }
    throw new Error(`unexpected settings command ${commandName}`);
  }

  async listen(
    event: string,
    listener: (payload: unknown) => void,
  ): Promise<Unlisten> {
    this.trace.push(`listen:${event}`);
    const listeners = this.#listeners.get(event) ?? new Set();
    listeners.add(listener);
    this.#listeners.set(event, listeners);
    let active = true;
    return () => {
      if (!active) return;
      active = false;
      listeners.delete(listener);
      this.unlistenCount += 1;
    };
  }

  calls(command: string): number {
    return this.trace.filter((item) => item === `invoke:${command}`).length;
  }

  activeListenerCount(): number {
    return [...this.#listeners.values()].reduce(
      (total, listeners) => total + listeners.size,
      0,
    );
  }

  setRecoveryRequired(): void {
    this.scopeValue = createProofSnapshot(this.shape, true);
    this.mutationMode = "recoveryRequired";
  }

  authorityLoadedBeforeMutation(): boolean {
    const registry = this.trace.indexOf(`invoke:${SETTINGS_REGISTRY_COMMAND}`);
    const scope = this.trace.indexOf(`invoke:${SETTINGS_LOAD_COMMAND}`);
    const mutation = this.trace.findIndex(
      (item) =>
        item === `invoke:${SETTINGS_APPLY_COMMAND}` ||
        item === `invoke:${SETTINGS_RESET_COMMAND}`,
    );
    return registry >= 0 && scope > registry && (mutation < 0 || mutation > scope);
  }

  private mutationResult(command: SettingsApplyCommand): SettingsMutationResult {
    const snapshot = structuredClone(this.scopeValue);
    if (this.mutationMode === "conflict") {
      return {
        status: "conflict",
        conflict: {
          expected: command.authority,
          actual: snapshot.authority,
        },
        snapshot,
      };
    }
    if (
      this.mutationMode === "invalidIntent" ||
      this.mutationMode === "policyBlocked" ||
      this.mutationMode === "recoveryRequired"
    ) {
      return {
        status: "rejected",
        rejection: {
          code: this.mutationMode,
          diagnostic: {
            codecVersion: 1,
            value: { published: false },
          },
        },
        snapshot,
      };
    }
    this.publications += 1;
    return {
      status: "applied",
      snapshot,
      receipt: {
        requestId: command.requestId,
        pageId: command.pageId,
        applyUnitId: command.applyUnitId,
        scopeId: command.scopeId,
        previousAuthority: command.authority,
        committedAuthority: snapshot.authority,
        outcome: "changed",
        durability: {
          kind: "confirmed",
          evidence: {
            codecVersion: 1,
            value: { publication: this.publications },
          },
        },
        activationRequirements: snapshot.activationRequirements,
      },
    };
  }
}

export const settingsEvents = {
  registry: SETTINGS_REGISTRY_CHANGED_EVENT,
  scope: SETTINGS_SCOPE_CHANGED_EVENT,
};
