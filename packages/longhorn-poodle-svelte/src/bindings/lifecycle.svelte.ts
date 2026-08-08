import { onMount } from "svelte";

import type {
  ConnectionFailure,
  ConnectionFailureReporter,
} from "@inflatable-cookie/longhorn/core";

export type ClientStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "ready" }
  | { readonly kind: "reconnecting" }
  | { readonly kind: "unsupported"; readonly reason: string }
  | { readonly kind: "failed"; readonly error: unknown };

export interface ReactiveConnection<Snapshot> {
  readonly ready: Promise<unknown>;
  current(): Snapshot | undefined;
  dispose(): Promise<void>;
}

export type ReactiveConnectionFactory<Snapshot> = (
  listener: (snapshot: Snapshot) => void,
  onFailure: ConnectionFailureReporter,
) => ReactiveConnection<Snapshot>;

export type ClientCapability<Snapshot> =
  | {
      readonly kind: "supported";
      readonly connect: ReactiveConnectionFactory<Snapshot>;
    }
  | { readonly kind: "unsupported"; readonly reason: string };

export interface ReactiveClientStateOptions<Snapshot> {
  readonly capability: ClientCapability<Snapshot>;
  readonly onSnapshot?: (snapshot: Snapshot) => void;
  readonly onFailure?: (failure: ConnectionFailure) => void;
}

export interface StartableClientState {
  start(): Promise<void>;
  stop(): Promise<void>;
}

export class ReactiveClientState<Snapshot>
  implements StartableClientState
{
  readonly #options: ReactiveClientStateOptions<Snapshot>;
  #status = $state.raw<ClientStatus>({ kind: "idle" });
  #snapshot = $state.raw<Snapshot | undefined>(undefined);
  #connection: ReactiveConnection<Snapshot> | undefined;
  #startTask: Promise<void> | undefined;
  #stopTask: Promise<void> | undefined;
  #generation = 0;
  #destroyed = false;

  constructor(options: ReactiveClientStateOptions<Snapshot>) {
    this.#options = options;
  }

  get status(): ClientStatus {
    return this.#status;
  }

  get snapshot(): Snapshot | undefined {
    return this.#snapshot;
  }

  start(): Promise<void> {
    if (this.#destroyed) {
      return Promise.reject(new ClientStateDestroyedError());
    }
    if (this.#startTask !== undefined) {
      return this.#startTask;
    }

    const afterStop = this.#stopTask;
    this.#startTask =
      afterStop === undefined
        ? this.#begin(false)
        : afterStop.then(() => this.#begin(false));
    return this.#startTask;
  }

  reconnect(): Promise<void> {
    if (this.#destroyed) {
      return Promise.reject(new ClientStateDestroyedError());
    }
    if (this.#status.kind === "unsupported") {
      return Promise.resolve();
    }

    const task = this.#stop(false).then(() => this.#begin(true));
    this.#startTask = task;
    return task;
  }

  stop(): Promise<void> {
    return this.#stop(true);
  }

  async destroy(): Promise<void> {
    if (this.#destroyed) {
      await this.#stopTask;
      return;
    }
    this.#destroyed = true;
    await this.#stop(true);
  }

  fail(error: unknown): void {
    if (!this.#destroyed) {
      this.#status = { kind: "failed", error };
    }
  }

  async #begin(reconnecting: boolean): Promise<void> {
    const capability = this.#options.capability;
    if (capability.kind === "unsupported") {
      this.#status = {
        kind: "unsupported",
        reason: capability.reason,
      };
      return;
    }

    const generation = ++this.#generation;
    this.#status =
      reconnecting || this.#snapshot !== undefined
        ? { kind: "reconnecting" }
        : { kind: "loading" };

    let connection: ReactiveConnection<Snapshot>;
    try {
      connection = capability.connect(
        (snapshot) => {
          if (generation === this.#generation && !this.#destroyed) {
            this.#accept(snapshot);
          }
        },
        (failure) => {
          if (generation === this.#generation && !this.#destroyed) {
            this.#status = { kind: "failed", error: failure.error };
            this.#options.onFailure?.(failure);
          }
        },
      );
    } catch (error) {
      this.#status = { kind: "failed", error };
      throw error;
    }
    this.#connection = connection;

    try {
      await connection.ready;
      if (generation !== this.#generation || this.#destroyed) {
        return;
      }
      const snapshot = connection.current();
      if (snapshot === undefined) {
        throw new ClientSnapshotUnavailableError();
      }
      if (this.#snapshot !== snapshot) {
        this.#accept(snapshot);
      } else {
        this.#status = { kind: "ready" };
      }
    } catch (error) {
      if (generation === this.#generation && !this.#destroyed) {
        this.#status = { kind: "failed", error };
      }
      try {
        await connection.dispose();
      } catch {
        // The connection retains exact teardown failure evidence.
      }
      throw error;
    }
  }

  #accept(snapshot: Snapshot): void {
    this.#snapshot = snapshot;
    this.#status = { kind: "ready" };
    this.#options.onSnapshot?.(snapshot);
  }

  #stop(clearSnapshot: boolean): Promise<void> {
    if (this.#stopTask !== undefined) {
      return this.#stopTask;
    }

    const connection = this.#connection;
    this.#connection = undefined;
    this.#startTask = undefined;
    ++this.#generation;
    if (clearSnapshot) {
      this.#snapshot = undefined;
    }
    this.#status = { kind: "idle" };

    this.#stopTask = (async () => {
      try {
        await connection?.dispose();
      } catch (error) {
        this.#status = { kind: "failed", error };
        throw error;
      } finally {
        this.#stopTask = undefined;
      }
    })();
    return this.#stopTask;
  }
}

export class ClientStateDestroyedError extends Error {
  constructor() {
    super("client state has been destroyed");
    this.name = "ClientStateDestroyedError";
  }
}

export class ClientSnapshotUnavailableError extends Error {
  constructor() {
    super("client connection became ready without a snapshot");
    this.name = "ClientSnapshotUnavailableError";
  }
}

export class UnsupportedCapabilityError extends Error {
  constructor(reason: string) {
    super(`capability is unsupported: ${reason}`);
    this.name = "UnsupportedCapabilityError";
  }
}

export function useClientState(
  state: StartableClientState,
  reportFailure?: (error: unknown) => void,
): void {
  onMount(() => {
    void state.start().catch((error) => reportFailure?.(error));
    return () => {
      void state.stop().catch((error) => reportFailure?.(error));
    };
  });
}
