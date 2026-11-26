import type { ConnectionFailure, ConnectionFailureReporter } from "./connection.ts";
import { disposeAll } from "./listeners.ts";
import type { Unlisten } from "./transport.ts";

interface ControllerLifetime {
  readonly isCurrent: () => boolean;
  readonly own: (...unlisten: Array<Unlisten>) => Promise<void>;
  readonly reportFailure: ConnectionFailureReporter;
  readonly watch: (subscription: SnapshotSubscription, ready: () => void) => Promise<void>;
}

interface SnapshotSubscription {
  readonly ready: Promise<unknown>;
  failures(): ReadonlyArray<ConnectionFailure>;
  dispose(): Promise<void>;
}

interface ControllerStart {
  readonly loading: () => void;
  readonly connect: (lifetime: ControllerLifetime) => Promise<void>;
  readonly failed: (error: unknown) => void;
}

export class ControllerLifecycle {
  #unlisten: Array<Unlisten> | undefined;
  #revision = 0;
  #eventFailure: { readonly kind: "failed"; readonly error: unknown } | undefined;

  get revision(): number {
    return this.#revision;
  }

  get active(): boolean {
    return this.#unlisten !== undefined;
  }

  get eventFailure() {
    return this.#eventFailure;
  }

  isCurrent(revision: number): boolean {
    return this.active && this.#revision === revision;
  }

  async start({ loading, connect, failed }: ControllerStart): Promise<void> {
    if (this.active) {
      return;
    }
    const listeners: Array<Unlisten> = [];
    this.#unlisten = listeners;
    const revision = ++this.#revision;
    const isCurrent = () => this.isCurrent(revision);
    const own = async (...unlisten: Array<Unlisten>) => {
      if (isCurrent()) {
        listeners.push(...unlisten);
      } else {
        await disposeAll(unlisten);
      }
    };
    try {
      loading();
      if (isCurrent()) {
        await connect({
          isCurrent,
          own,
          reportFailure: ({ phase, error }) => {
            if (isCurrent() && phase !== "unlisten") {
              if (phase === "event") {
                this.#eventFailure ??= { kind: "failed", error };
              }
              failed(error);
            }
          },
          watch: async (subscription, ready) => {
            // The connection owns cleanup on registration and snapshot failures.
            await own(() => subscription.dispose());
            try {
              await subscription.ready;
              if (isCurrent() && subscription.failures().length === 0) {
                ready();
              }
            } catch (error) {
              if (isCurrent()) {
                failed(error);
              }
            }
          },
        });
      }
    } catch (error) {
      if (isCurrent()) {
        const unlisten = listeners.splice(0);
        try {
          failed(error);
        } finally {
          await disposeAll(unlisten);
        }
      }
    }
  }

  /** Resets the controller, then releases every owned listener before reporting a failure. */
  async stop(reset: () => void): Promise<void> {
    const listeners = this.#unlisten ?? [];
    this.#unlisten = undefined;
    this.#eventFailure = undefined;
    this.#revision += 1;
    let results: Array<PromiseSettledResult<void>>;
    try {
      reset();
    } finally {
      results = await disposeAll(listeners);
    }
    const failure = results.find((result) => result.status === "rejected");
    if (failure !== undefined) {
      throw failure.reason;
    }
  }
}
