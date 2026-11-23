import type { ConnectionFailure } from "@inflatable-cookie/longhorn/core";

import type { SettingsClient } from "@inflatable-cookie/longhorn/settings";
import type {
  SettingsRegistrySnapshot,
  SettingsRequestId,
} from "@inflatable-cookie/longhorn/settings/protocol";
import type { SettingsSubscription } from "@inflatable-cookie/longhorn/settings";
import { SettingsRendererResolverUnavailableError } from "./errors.ts";
import { SettingsSessionDocument } from "./document.svelte.ts";
import type { SettingsRendererResolver, SettingsRoute, SettingsSessionStatus } from "./types.ts";

export interface SettingsRuntimeOptions {
  readonly client: SettingsClient;
  readonly nextRequestId: () => SettingsRequestId;
  readonly initialRoute?: SettingsRoute | undefined;
  readonly onError: (error: unknown) => void;
}

export class SettingsSessionRuntime {
  readonly document: SettingsSessionDocument;
  readonly #options: SettingsRuntimeOptions;
  #status = $state.raw<SettingsSessionStatus>({ kind: "idle" });
  #subscription: SettingsSubscription<SettingsRegistrySnapshot> | undefined;
  #resolver: SettingsRendererResolver | undefined;
  #startTask: Promise<void> | undefined;
  #stopTask: Promise<void> | undefined;
  #installTask: Promise<void> | undefined;
  #pendingRegistry: SettingsRegistrySnapshot | undefined;
  #requestedGeneration = 0;
  #generation = 0;

  constructor(options: SettingsRuntimeOptions) {
    this.#options = options;
    this.document = new SettingsSessionDocument({
      client: options.client,
      nextRequestId: options.nextRequestId,
      onScopeFailure: (error) => this.#fail(error),
      onMutationError: options.onError,
    });
  }

  get status(): SettingsSessionStatus {
    return this.#status;
  }

  start(resolver: SettingsRendererResolver): Promise<void> {
    if (this.#startTask !== undefined) {
      return this.#startTask;
    }
    this.#resolver = resolver;
    const afterStop = this.#stopTask;
    this.#startTask = afterStop === undefined ? this.#begin() : afterStop.then(() => this.#begin());
    return this.#startTask;
  }

  stop(): Promise<void> {
    if (this.#stopTask !== undefined) {
      return this.#stopTask;
    }
    const subscription = this.#subscription;
    const priorFailure = this.#status.kind === "failed" ? this.#status.error : undefined;
    this.#subscription = undefined;
    this.#startTask = undefined;
    this.#pendingRegistry = undefined;
    this.#requestedGeneration = 0;
    ++this.#generation;
    this.#status = { kind: "idle" };
    this.#stopTask = (async () => {
      try {
        await Promise.all([
          subscription?.dispose().catch((error: unknown) => {
            if (error !== priorFailure) {
              throw error;
            }
          }),
          this.document.stop(),
        ]);
      } finally {
        this.#stopTask = undefined;
      }
    })();
    return this.#stopTask;
  }

  async reconnect(): Promise<void> {
    const resolver = this.#requiredResolver();
    const registry = this.document.registry;
    if (registry === undefined) {
      await this.stop();
      await this.start(resolver);
      return;
    }
    // A reconnect is a replacement of the same generation, so it shares the
    // one install queue rather than clearing authority beside it.
    try {
      await this.#queueInstall(registry);
    } catch (error) {
      this.#fail(error);
      throw error;
    }
  }

  async #begin(): Promise<void> {
    const generation = ++this.#generation;
    this.#status = { kind: "loading" };
    const subscription = this.#options.client.connectRegistry(
      (registry) => {
        if (generation === this.#generation && this.#acceptsRegistry(registry)) {
          void this.#queueInstall(registry).catch((error: unknown) => {
            if (generation === this.#generation) {
              this.#fail(error);
            }
          });
        }
      },
      (failure: ConnectionFailure) => {
        if (generation === this.#generation) {
          this.#fail(failure.error);
        }
      },
    );
    this.#subscription = subscription;
    try {
      const registry = await subscription.ready;
      if (generation === this.#generation) {
        await this.#install(registry, this.#requiredResolver(), this.#options.initialRoute);
      }
    } catch (error) {
      if (generation === this.#generation) {
        this.#fail(error);
      }
      throw error;
    }
  }

  /**
   * Whether a registry change is worth queueing.
   *
   * Newer than what the document holds and newer than anything already
   * queued. A change that is not newer than both is one this session is
   * already past, and installing it would move the document backwards.
   */
  #acceptsRegistry(registry: SettingsRegistrySnapshot): boolean {
    const installed = this.document.registry;
    return (
      installed !== undefined &&
      registry.generation > Math.max(installed.generation, this.#requestedGeneration)
    );
  }

  /**
   * Queues one authority replacement, newest generation winning.
   *
   * Two registry changes can arrive while the scopes of the previous registry
   * are still being disposed. Clearing authority is not instant -- it waits for
   * every scope subscription to release -- and it empties the scope map before
   * it waits, so a second clear finds nothing to dispose, installs its registry
   * first, and is then overwritten when the older clear finishes. One install
   * runs at a time here, and each turn installs the newest queued registry, so
   * a replacement cannot land on top of a newer generation.
   */
  #queueInstall(registry: SettingsRegistrySnapshot): Promise<void> {
    if (
      this.#pendingRegistry === undefined ||
      registry.generation > this.#pendingRegistry.generation
    ) {
      this.#pendingRegistry = registry;
    }
    this.#requestedGeneration = Math.max(this.#requestedGeneration, registry.generation);
    if (this.#installTask === undefined) {
      this.#status = { kind: "reconnecting" };
      const generation = this.#generation;
      this.#installTask = this.#drainInstalls(generation).finally(() => {
        this.#installTask = undefined;
      });
    }
    return this.#installTask;
  }

  async #drainInstalls(generation: number): Promise<void> {
    while (this.#pendingRegistry !== undefined && generation === this.#generation) {
      const registry = this.#pendingRegistry;
      this.#pendingRegistry = undefined;
      const route = this.document.route;
      await this.document.clearAuthority();
      if (generation !== this.#generation) {
        return;
      }
      // A newer change arrived while this one was clearing. Installing it would
      // hand the document an older generation than the queue already holds, so
      // the loop takes the newer one instead.
      if (this.#pendingRegistry !== undefined) {
        continue;
      }
      await this.#install(registry, this.#requiredResolver(), route);
    }
  }

  async #install(
    registry: SettingsRegistrySnapshot,
    resolver: SettingsRendererResolver,
    route?: SettingsRoute,
  ): Promise<void> {
    this.#requestedGeneration = Math.max(this.#requestedGeneration, registry.generation);
    const result = await this.document.installRegistry(registry, resolver, route);
    this.#status =
      result === "ready"
        ? { kind: "ready" }
        : {
            kind: "unsupported",
            reason: "settings registry has no admitted pages",
          };
  }

  #requiredResolver(): SettingsRendererResolver {
    if (this.#resolver === undefined) {
      throw new SettingsRendererResolverUnavailableError();
    }
    return this.#resolver;
  }

  #fail(error: unknown): void {
    if (this.#status.kind === "failed" && this.#status.error === error) {
      return;
    }
    this.#status = { kind: "failed", error };
    this.#options.onError(error);
  }
}
