import type { ConnectionFailure } from "@inflatable-cookie/longhorn/core";

import { isNewerScope } from "@inflatable-cookie/longhorn/settings";
import type { SettingsClient } from "@inflatable-cookie/longhorn/settings";
import type {
  SettingsPageId,
  SettingsRegistrySnapshot,
  SettingsRequestId,
  SettingsScopeId,
  SettingsScopeSnapshot,
} from "@inflatable-cookie/longhorn/settings/protocol";
import type { SettingsSubscription } from "@inflatable-cookie/longhorn/settings";
import { StartStop } from "../../bindings/start-stop.ts";
import type { SettingsScopeStatus } from "./types.ts";

export interface SettingsScopeStateOptions {
  readonly client: SettingsClient;
  readonly registry: SettingsRegistrySnapshot;
  readonly pageId: SettingsPageId;
  readonly scopeId: SettingsScopeId;
  readonly nextRequestId: () => SettingsRequestId;
  readonly onFailure?: (failure: ConnectionFailure) => void;
}

export class SettingsScopeState {
  readonly #options: SettingsScopeStateOptions;
  #status = $state.raw<SettingsScopeStatus>({ kind: "idle" });
  #snapshot = $state.raw<SettingsScopeSnapshot | undefined>(undefined);
  #subscription: SettingsSubscription<SettingsScopeSnapshot> | undefined;
  readonly #lifecycle = new StartStop();

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
    return this.#lifecycle.start((generation) => this.#begin(generation));
  }

  accept(snapshot: SettingsScopeSnapshot): void {
    if (snapshot !== this.#snapshot && !isNewerScope(snapshot, this.#snapshot)) {
      return;
    }
    this.#snapshot = snapshot;
    this.#status = { kind: "ready" };
  }

  stop(): Promise<void> {
    return this.#lifecycle.stop(async () => {
      const priorFailure = this.#status.kind === "failed" ? this.#status.error : undefined;
      this.#snapshot = undefined;
      this.#status = { kind: "idle" };
      await Promise.resolve();
      const subscription = this.#subscription;
      this.#subscription = undefined;
      try {
        await subscription?.dispose();
      } catch (error) {
        if (error !== priorFailure) {
          throw error;
        }
      }
    });
  }

  async #begin(generation: number): Promise<void> {
    this.#status = { kind: "loading" };
    try {
      const subscription = this.#options.client.connectScope({
        registry: this.#options.registry,
        pageId: this.#options.pageId,
        scopeId: this.#options.scopeId,
        nextRequestId: this.#options.nextRequestId,
        onSnapshot: (snapshot) => {
          if (generation === this.#lifecycle.generation) {
            this.accept(snapshot);
          }
        },
        onFailure: (failure) => {
          if (generation === this.#lifecycle.generation) {
            this.#status = { kind: "failed", error: failure.error };
            this.#options.onFailure?.(failure);
          }
        },
      });
      this.#subscription = subscription;
      const snapshot = await subscription.ready;
      if (generation === this.#lifecycle.generation) {
        this.accept(snapshot);
      }
    } catch (error) {
      if (generation === this.#lifecycle.generation) {
        this.#status = { kind: "failed", error };
      }
      throw error;
    }
  }
}
