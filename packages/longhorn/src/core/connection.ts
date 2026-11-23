import type { Unlisten } from "./transport.ts";

export type SnapshotEventAction =
  | { readonly kind: "ignore" }
  | { readonly kind: "refresh" }
  | { readonly kind: "snapshot"; readonly value: unknown };

export type ConnectionFailurePhase = "registration" | "event" | "snapshot" | "unlisten";

export interface ConnectionFailure {
  readonly phase: ConnectionFailurePhase;
  readonly error: unknown;
}

export type ConnectionFailureReporter = (failure: ConnectionFailure) => void;

export interface CheckedSnapshotConnectionOptions<Snapshot> {
  readonly listen: (listener: (payload: unknown) => void) => Promise<Unlisten>;
  readonly loadSnapshot: () => Promise<unknown>;
  readonly validateSnapshot: (value: unknown) => Snapshot;
  readonly handleEvent: (payload: unknown, current: Snapshot | undefined) => SnapshotEventAction;
  readonly isNewer: (candidate: Snapshot, current: Snapshot | undefined) => boolean;
  readonly onSnapshot?: ((snapshot: Snapshot) => void) | undefined;
  readonly onFailure?: ConnectionFailureReporter | undefined;
  readonly disposedBeforeReadyError?: () => Error;
}

export class CheckedSnapshotConnection<Snapshot> {
  readonly ready: Promise<Snapshot | undefined>;

  readonly #options: CheckedSnapshotConnectionOptions<Snapshot>;
  readonly #failures: Array<ConnectionFailure> = [];
  #current: Snapshot | undefined;
  #unlisten: Unlisten | undefined;
  #cleanup: Promise<void> | undefined;
  #pump: Promise<void> | undefined;
  #registered = false;
  #refreshPending = false;
  #disposed = false;

  constructor(options: CheckedSnapshotConnectionOptions<Snapshot>) {
    this.#options = options;
    this.ready = this.#start();
  }

  current(): Snapshot | undefined {
    return this.#current;
  }

  failures(): ReadonlyArray<ConnectionFailure> {
    return [...this.#failures];
  }

  /**
   * Accepts a snapshot the caller obtained from its own request (for example,
   * a mutation result), under the same `isNewer` rule the refresh pump uses.
   * It is a no-op after dispose.
   */
  offer(candidate: Snapshot): void {
    if (!this.#disposed && this.#accept(candidate)) {
      this.#options.onSnapshot?.(candidate);
    }
  }

  async dispose(): Promise<void> {
    this.#disposed = true;

    let cleanupFailure: { error: unknown } | undefined;
    try {
      await this.#closeListener();
    } catch (error) {
      cleanupFailure = { error };
      this.#recordFailure("unlisten", error);
    }

    let readyFailure: ConnectionFailure | undefined;
    try {
      await this.ready;
    } catch (error) {
      readyFailure = this.#failures.find((failure) => Object.is(failure.error, error));
    }

    if (cleanupFailure === undefined) {
      try {
        await this.#closeListener();
      } catch (error) {
        cleanupFailure = { error };
        this.#recordFailure("unlisten", error);
      }
    }

    const failure = cleanupFailure ?? readyFailure;
    if (failure !== undefined) {
      throw failure.error;
    }
  }

  async #start(): Promise<Snapshot | undefined> {
    let unlisten: Unlisten;
    try {
      unlisten = await this.#options.listen((payload) => {
        this.#handleEvent(payload);
      });
    } catch (error) {
      this.#recordFailure("registration", error);
      throw error;
    }

    this.#unlisten = unlisten;
    this.#registered = true;

    if (this.#disposed) {
      await this.#closeAfterFailure();
      const disposedError = this.#options.disposedBeforeReadyError?.();
      if (disposedError !== undefined) {
        throw disposedError;
      }
      return undefined;
    }

    if (this.#failures.length > 0) {
      await this.#closeAfterFailure();
      return this.#current;
    }

    this.#refreshPending = true;
    try {
      await this.#synchronize();
    } catch (error) {
      this.#recordFailure("snapshot", error);
      await this.#closeAfterFailure();
      throw error;
    }

    if (this.#disposed) {
      await this.#closeAfterFailure();
      const disposedError = this.#options.disposedBeforeReadyError?.();
      if (disposedError !== undefined) {
        throw disposedError;
      }
    }

    return this.#current;
  }

  #handleEvent(payload: unknown): void {
    if (this.#disposed || this.#failures.length > 0) {
      return;
    }

    let candidate: Snapshot;
    try {
      const action = this.#options.handleEvent(payload, this.#current);
      switch (action.kind) {
        case "ignore":
          return;
        case "refresh":
          this.#refreshPending = true;
          if (this.#registered) {
            void this.#synchronize().catch((error: unknown) => {
              this.#recordFailure("snapshot", error);
              void this.#closeAfterFailure();
            });
          }
          return;
        case "snapshot":
          candidate = this.#options.validateSnapshot(action.value);
          if (!this.#accept(candidate)) {
            return;
          }
          break;
      }
    } catch (error) {
      this.#recordFailure("event", error);
      void this.#closeAfterFailure();
      return;
    }
    this.#notifySnapshot(candidate, this.#registered && this.#pump === undefined);
  }

  #synchronize(): Promise<void> {
    if (!this.#registered || this.#disposed || this.#failures.length > 0) {
      return Promise.resolve();
    }
    this.#pump ??= Promise.resolve().then(() => this.#runPump());
    return this.#pump;
  }

  async #runPump(): Promise<void> {
    try {
      while (this.#refreshPending && !this.#disposed && this.#failures.length === 0) {
        this.#refreshPending = false;
        const value = await this.#options.loadSnapshot();
        if (this.#disposed || this.#failures.length > 0) {
          return;
        }
        const candidate = this.#options.validateSnapshot(value);
        if (this.#accept(candidate)) {
          this.#notifySnapshot(candidate, false);
        }
      }
    } finally {
      this.#pump = undefined;
    }
  }

  #accept(candidate: Snapshot): boolean {
    if (!this.#options.isNewer(candidate, this.#current)) {
      return false;
    }
    this.#current = candidate;
    return true;
  }

  #notifySnapshot(candidate: Snapshot, propagate: boolean): void {
    try {
      this.#options.onSnapshot?.(candidate);
    } catch (error) {
      // A callback during registration or a query can otherwise reject that operation.
      if (propagate) {
        throw error;
      }
    }
  }

  #recordFailure(phase: ConnectionFailurePhase, error: unknown): void {
    if (
      this.#failures.some((failure) => failure.phase === phase && Object.is(failure.error, error))
    ) {
      return;
    }
    const failure = { phase, error };
    this.#failures.push(failure);
    try {
      this.#options.onFailure?.(failure);
    } catch {
      // A failure reporter cannot interrupt teardown or replace the recorded error.
    }
  }

  async #closeAfterFailure(): Promise<void> {
    try {
      await this.#closeListener();
    } catch (error) {
      this.#recordFailure("unlisten", error);
    }
  }

  #closeListener(): Promise<void> {
    if (this.#cleanup !== undefined) {
      return this.#cleanup;
    }
    const unlisten = this.#unlisten;
    if (unlisten === undefined) {
      return Promise.resolve();
    }
    this.#unlisten = undefined;
    this.#cleanup = Promise.resolve().then(() => unlisten());
    return this.#cleanup;
  }
}
