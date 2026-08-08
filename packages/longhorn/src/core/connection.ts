import type { Unlisten } from "./transport.ts";

export type SnapshotEventAction =
  | { readonly kind: "ignore" }
  | { readonly kind: "refresh" }
  | { readonly kind: "snapshot"; readonly value: unknown };

export type ConnectionFailurePhase =
  | "registration"
  | "event"
  | "snapshot"
  | "unlisten";

export interface ConnectionFailure {
  readonly phase: ConnectionFailurePhase;
  readonly error: unknown;
}

export type ConnectionFailureReporter = (
  failure: ConnectionFailure,
) => void;

export interface CheckedSnapshotConnectionOptions<Snapshot> {
  readonly listen: (
    listener: (payload: unknown) => void,
  ) => Promise<Unlisten>;
  readonly loadSnapshot: () => Promise<unknown>;
  readonly validateSnapshot: (value: unknown) => Snapshot;
  readonly handleEvent: (
    payload: unknown,
    current: Snapshot | undefined,
  ) => SnapshotEventAction;
  readonly isNewer: (
    candidate: Snapshot,
    current: Snapshot | undefined,
  ) => boolean;
  readonly onSnapshot?: (snapshot: Snapshot) => void;
  readonly onFailure?: ConnectionFailureReporter;
  readonly disposedBeforeReadyError?: () => Error;
}

export class CheckedSnapshotConnection<Snapshot> {
  readonly ready: Promise<Snapshot | undefined>;

  readonly #options: CheckedSnapshotConnectionOptions<Snapshot>;
  readonly #failures: ConnectionFailure[] = [];
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

  failures(): readonly ConnectionFailure[] {
    return [...this.#failures];
  }

  async dispose(): Promise<void> {
    this.#disposed = true;

    let cleanupError: unknown;
    try {
      await this.#closeListener();
    } catch (error) {
      cleanupError = error;
      this.#recordFailure("unlisten", error);
    }

    let readyError: unknown;
    try {
      await this.ready;
    } catch (error) {
      if (this.#failures.some((failure) => failure.error === error)) {
        readyError = error;
      }
    }

    if (cleanupError === undefined) {
      try {
        await this.#closeListener();
      } catch (error) {
        cleanupError = error;
        this.#recordFailure("unlisten", error);
      }
    }

    if (cleanupError !== undefined) {
      throw cleanupError;
    }
    if (readyError !== undefined) {
      throw readyError;
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

    try {
      const action = this.#options.handleEvent(payload, this.#current);
      switch (action.kind) {
        case "ignore":
          return;
        case "refresh":
          this.#refreshPending = true;
          if (this.#registered) {
            void this.#synchronize().catch((error) => {
              this.#recordFailure("snapshot", error);
              void this.#closeAfterFailure();
            });
          }
          return;
        case "snapshot":
          this.#accept(this.#options.validateSnapshot(action.value));
          return;
      }
    } catch (error) {
      this.#recordFailure("event", error);
      void this.#closeAfterFailure();
    }
  }

  #synchronize(): Promise<void> {
    if (
      !this.#registered ||
      this.#disposed ||
      this.#failures.length > 0
    ) {
      return Promise.resolve();
    }
    this.#pump ??= Promise.resolve().then(() => this.#runPump());
    return this.#pump;
  }

  async #runPump(): Promise<void> {
    try {
      while (
        this.#refreshPending &&
        !this.#disposed &&
        this.#failures.length === 0
      ) {
        this.#refreshPending = false;
        const value = await this.#options.loadSnapshot();
        if (this.#disposed || this.#failures.length > 0) {
          return;
        }
        this.#accept(this.#options.validateSnapshot(value));
      }
    } finally {
      this.#pump = undefined;
    }
  }

  #accept(candidate: Snapshot): void {
    if (!this.#options.isNewer(candidate, this.#current)) {
      return;
    }
    this.#current = candidate;
    this.#options.onSnapshot?.(candidate);
  }

  #recordFailure(
    phase: ConnectionFailurePhase,
    error: unknown,
  ): void {
    if (
      this.#failures.some(
        (failure) => failure.phase === phase && failure.error === error,
      )
    ) {
      return;
    }
    const failure = { phase, error };
    this.#failures.push(failure);
    this.#options.onFailure?.(failure);
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
