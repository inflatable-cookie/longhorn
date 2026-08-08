import type { ConnectionFailure } from "@inflatable-cookie/longhorn/core";

import type { SettingsClient } from "@inflatable-cookie/longhorn/settings";
import type {
  SettingsRegistrySnapshot,
  SettingsRequestId,
  SettingsScopeId,
  SettingsScopeSnapshot,
} from "@inflatable-cookie/longhorn/settings/protocol";
import type { SettingsSubscription } from "@inflatable-cookie/longhorn/settings";
import type { SettingsScopeStatus } from "./types.ts";

export interface SettingsScopeStateOptions {
  readonly client: SettingsClient;
  readonly registry: SettingsRegistrySnapshot;
  readonly scopeId: SettingsScopeId;
  readonly nextRequestId: () => SettingsRequestId;
  readonly onFailure?: (failure: ConnectionFailure) => void;
}

export class SettingsScopeState {
  readonly #options: SettingsScopeStateOptions;
  #status = $state.raw<SettingsScopeStatus>({ kind: "idle" });
  #snapshot = $state.raw<SettingsScopeSnapshot | undefined>(undefined);
  #subscription:
    | SettingsSubscription<SettingsScopeSnapshot>
    | undefined;
  #startTask: Promise<void> | undefined;
  #stopTask: Promise<void> | undefined;
  #generation = 0;

  constructor(options: SettingsScopeStateOptions) {
    this.#options = options;
  }

  get scopeId(): SettingsScopeId {
    return this.#options.scopeId;
  }

  get status(): SettingsScopeStatus {
    return this.#status;
  }

  get snapshot(): SettingsScopeSnapshot | undefined {
    return this.#snapshot;
  }

  start(): Promise<void> {
    if (this.#startTask !== undefined) {
      return this.#startTask;
    }
    const afterStop = this.#stopTask;
    this.#startTask =
      afterStop === undefined
        ? this.#begin()
        : afterStop.then(() => this.#begin());
    return this.#startTask;
  }

  accept(snapshot: SettingsScopeSnapshot): void {
    this.#snapshot = snapshot;
    this.#status = { kind: "ready" };
  }

  stop(): Promise<void> {
    if (this.#stopTask !== undefined) {
      return this.#stopTask;
    }
    const subscription = this.#subscription;
    const priorFailure =
      this.#status.kind === "failed" ? this.#status.error : undefined;
    this.#subscription = undefined;
    this.#startTask = undefined;
    ++this.#generation;
    this.#snapshot = undefined;
    this.#status = { kind: "idle" };
    this.#stopTask = (async () => {
      try {
        try {
          await subscription?.dispose();
        } catch (error) {
          if (error !== priorFailure) throw error;
        }
      } finally {
        this.#stopTask = undefined;
      }
    })();
    return this.#stopTask;
  }

  async #begin(): Promise<void> {
    const generation = ++this.#generation;
    this.#status = { kind: "loading" };
    const subscription = this.#options.client.connectScope({
      registry: this.#options.registry,
      scopeId: this.#options.scopeId,
      nextRequestId: this.#options.nextRequestId,
      onSnapshot: (snapshot) => {
        if (generation === this.#generation) {
          this.accept(snapshot);
        }
      },
      onFailure: (failure) => {
        if (generation === this.#generation) {
          this.#status = { kind: "failed", error: failure.error };
          this.#options.onFailure?.(failure);
        }
      },
    });
    this.#subscription = subscription;
    try {
      const snapshot = await subscription.ready;
      if (generation === this.#generation) {
        this.accept(snapshot);
      }
    } catch (error) {
      if (generation === this.#generation) {
        this.#status = { kind: "failed", error };
      }
      throw error;
    }
  }
}
