import { useSessionLifecycle } from "./mount.ts";

import type {
  ConnectionFailure,
  ConnectionFailureReporter,
} from "@inflatable-cookie/longhorn/core";

import { StartStop } from "./start-stop.ts";

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

export class ReactiveClientState<Snapshot> implements StartableClientState {
  readonly #options: ReactiveClientStateOptions<Snapshot>;
  #status = $state.raw<ClientStatus>({ kind: "idle" });
  #snapshot = $state.raw<Snapshot | undefined>(undefined);
  #connection: ReactiveConnection<Snapshot> | undefined;
  readonly #lifecycle = new StartStop();
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
    return this.#lifecycle.start((generation) => this.#begin(generation, false), true);
  }

  reconnect(): Promise<void> {
    if (this.#destroyed) {
      return Promise.reject(new ClientStateDestroyedError());
    }
    if (this.#status.kind === "unsupported") {
      return Promise.resolve();
    }

    void this.#stop(false).catch(() => undefined);
    return this.#lifecycle.start((generation) => this.#begin(generation, true), true);
  }

  stop(): Promise<void> {
    return this.#stop(true);
  }

  async destroy(): Promise<void> {
    this.#destroyed = true;
    await this.#stop(true);
  }

  fail(error: unknown): void {
    if (!this.#destroyed) {
      this.#status = { kind: "failed", error };
    }
  }

  async #begin(generation: number, reconnecting: boolean): Promise<void> {
    const capability = this.#options.capability;
    if (capability.kind === "unsupported") {
      this.#status = {
        kind: "unsupported",
        reason: capability.reason,
      };
      return;
    }

    this.#status =
      reconnecting || this.#snapshot !== undefined ? { kind: "reconnecting" } : { kind: "loading" };

    let connection: ReactiveConnection<Snapshot>;
    try {
      connection = capability.connect(
        (snapshot) => {
          if (generation === this.#lifecycle.generation && !this.#destroyed) {
            this.#accept(snapshot);
          }
        },
        (failure) => {
          if (generation === this.#lifecycle.generation && !this.#destroyed) {
            this.#status = { kind: "failed", error: failure.error };
            this.#options.onFailure?.(failure);
          }
        },
      );
    } catch (error) {
      if (generation === this.#lifecycle.generation) {
        this.#status = { kind: "failed", error };
      }
      throw error;
    }
    this.#connection = connection;

    try {
      await connection.ready;
      if (generation !== this.#lifecycle.generation || this.#destroyed) {
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
      if (generation === this.#lifecycle.generation) {
        this.#status = { kind: "failed", error };
        this.#connection = undefined;
        try {
          await connection.dispose();
        } catch {
          // The connection retains exact teardown failure evidence.
        }
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
    if (clearSnapshot) {
      this.#snapshot = undefined;
    }
    this.#status = { kind: clearSnapshot ? "idle" : "reconnecting" };
    return this.#lifecycle.stop(async () => {
      // A synchronous connect callback can call stop before connect returns.
      // Capture its connection after that call has finished publishing it.
      await Promise.resolve();
      const connection = this.#connection;
      this.#connection = undefined;
      try {
        await connection?.dispose();
      } catch (error) {
        this.#status = { kind: "failed", error };
        throw error;
      }
    });
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
  useSessionLifecycle(() => state, reportFailure, reportFailure);
}
