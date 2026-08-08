import type { ConnectionFailure } from "@inflatable-cookie/longhorn-core";

import type { SettingsClient } from "../client.ts";
import type {
  SettingsRegistrySnapshot,
  SettingsRequestId,
} from "../generated/protocol.ts";
import type { SettingsSubscription } from "../connection.ts";
import { SettingsRendererResolverUnavailableError } from "./errors.ts";
import { SettingsSessionDocument } from "./document.svelte.ts";
import type {
  SettingsRendererResolver,
  SettingsRoute,
  SettingsSessionStatus,
} from "./types.ts";

export interface SettingsRuntimeOptions {
  readonly client: SettingsClient;
  readonly nextRequestId: () => SettingsRequestId;
  readonly initialRoute?: SettingsRoute;
  readonly onError: (error: unknown) => void;
}

export class SettingsSessionRuntime {
  readonly document: SettingsSessionDocument;
  readonly #options: SettingsRuntimeOptions;
  #status = $state.raw<SettingsSessionStatus>({ kind: "idle" });
  #subscription:
    | SettingsSubscription<SettingsRegistrySnapshot>
    | undefined;
  #resolver: SettingsRendererResolver | undefined;
  #startTask: Promise<void> | undefined;
  #stopTask: Promise<void> | undefined;
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
    if (this.#startTask !== undefined) return this.#startTask;
    this.#resolver = resolver;
    const afterStop = this.#stopTask;
    this.#startTask =
      afterStop === undefined
        ? this.#begin()
        : afterStop.then(() => this.#begin());
    return this.#startTask;
  }

  stop(): Promise<void> {
    if (this.#stopTask !== undefined) return this.#stopTask;
    const subscription = this.#subscription;
    const priorFailure =
      this.#status.kind === "failed" ? this.#status.error : undefined;
    this.#subscription = undefined;
    this.#startTask = undefined;
    ++this.#generation;
    this.#status = { kind: "idle" };
    this.#stopTask = (async () => {
      try {
        await Promise.all([
          subscription?.dispose().catch((error) => {
            if (error !== priorFailure) throw error;
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
    const route = this.document.route;
    this.#status = { kind: "reconnecting" };
    try {
      await this.document.clearAuthority();
      await this.#install(registry, resolver, route);
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
        const current = this.document.registry;
        if (
          generation === this.#generation &&
          current !== undefined &&
          registry.generation > current.generation
        ) {
          void this.#replace(registry, generation);
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
        await this.#install(
          registry,
          this.#requiredResolver(),
          this.#options.initialRoute,
        );
      }
    } catch (error) {
      if (generation === this.#generation) this.#fail(error);
      throw error;
    }
  }

  async #replace(
    registry: SettingsRegistrySnapshot,
    generation: number,
  ): Promise<void> {
    this.#status = { kind: "reconnecting" };
    try {
      const route = this.document.route;
      await this.document.clearAuthority();
      if (generation === this.#generation) {
        await this.#install(registry, this.#requiredResolver(), route);
      }
    } catch (error) {
      if (generation === this.#generation) this.#fail(error);
    }
  }

  async #install(
    registry: SettingsRegistrySnapshot,
    resolver: SettingsRendererResolver,
    route?: SettingsRoute,
  ): Promise<void> {
    const result = await this.document.installRegistry(
      registry,
      resolver,
      route,
    );
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
    if (this.#status.kind === "failed" && this.#status.error === error) return;
    this.#status = { kind: "failed", error };
    this.#options.onError(error);
  }
}
