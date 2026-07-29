import {
  SurfaceTransferClient,
  type SurfaceSessionResponse,
  type SurfaceSessionStartRequest,
  type SurfaceTransferCommand,
  type SurfaceTransferResponse,
} from "@longhorn/surface-transfer";
import type { DragSessionId } from "@longhorn/transfer";

import {
  UnsupportedCapabilityError,
  type ClientStatus,
} from "./index.ts";

export type SurfaceTransferPreparationState =
  | { readonly status: "idle" }
  | {
      readonly status: "preparing";
      readonly request: SurfaceSessionStartRequest;
    }
  | {
      readonly status: "prepared" | "aborted";
      readonly response: SurfaceSessionResponse;
    }
  | { readonly status: "failed"; readonly error: unknown };

export type SurfaceTransferCompletionState =
  | { readonly status: "idle" }
  | { readonly status: "committing"; readonly request: SurfaceTransferCommand }
  | {
      readonly status: "committed" | "aborted";
      readonly response: SurfaceTransferResponse;
    }
  | { readonly status: "failed"; readonly error: unknown };

export interface SurfaceTransferStateOptions {
  readonly client?: SurfaceTransferClient;
  readonly unsupportedReason?: string;
  readonly cancelSession: (sessionId: DragSessionId) => Promise<void>;
}

export class SurfaceTransferState {
  readonly #options: SurfaceTransferStateOptions;
  #status = $state.raw<ClientStatus>({ kind: "idle" });
  #preparation = $state.raw<SurfaceTransferPreparationState>({
    status: "idle",
  });
  #completion = $state.raw<SurfaceTransferCompletionState>({
    status: "idle",
  });
  #activeSessionId: DragSessionId | undefined;
  #generation = 0;
  #tasks = new Set<Promise<unknown>>();
  #stopTask: Promise<void> | undefined;
  #stopped = true;
  #destroyed = false;

  constructor(options: SurfaceTransferStateOptions) {
    this.#options = options;
  }

  get status(): ClientStatus {
    return this.#status;
  }

  get preparation(): SurfaceTransferPreparationState {
    return this.#preparation;
  }

  get completion(): SurfaceTransferCompletionState {
    return this.#completion;
  }

  async start(): Promise<void> {
    this.#assertAlive();
    this.#stopped = false;
    this.#status =
      this.#options.client === undefined
        ? {
            kind: "unsupported",
            reason:
              this.#options.unsupportedReason ??
              "Surface transfer capability is unavailable",
          }
        : { kind: "ready" };
  }

  async prepare(
    request: SurfaceSessionStartRequest,
  ): Promise<SurfaceSessionResponse> {
    this.#assertRunning();
    const client = this.#requireClient();
    await this.#cancelActive();
    const generation = ++this.#generation;
    this.#preparation = { status: "preparing", request };

    const task = client.start(request).then(async (response) => {
      if (generation !== this.#generation || this.#stopped) {
        if (response.status === "started") {
          await this.#options.cancelSession(
            response.session.payload.session_id,
          );
        }
        return response;
      }
      this.#preparation = {
        status:
          response.status === "started" ? "prepared" : "aborted",
        response,
      };
      this.#activeSessionId =
        response.status === "started"
          ? response.session.payload.session_id
          : undefined;
      return response;
    }).catch((error) => {
      if (generation === this.#generation && !this.#stopped) {
        this.#preparation = { status: "failed", error };
        this.#status = { kind: "failed", error };
      }
      throw error;
    });
    this.#track(task);
    return task;
  }

  async commit(
    request: SurfaceTransferCommand,
  ): Promise<SurfaceTransferResponse> {
    this.#assertRunning();
    const client = this.#requireClient();
    this.#completion = { status: "committing", request };
    try {
      const response = await client.commit(request);
      this.#completion = {
        status:
          response.status === "committed" ? "committed" : "aborted",
        response,
      };
      if (
        response.status === "committed" ||
        response.abort.session_consumed
      ) {
        this.#activeSessionId = undefined;
      }
      return response;
    } catch (error) {
      this.#completion = { status: "failed", error };
      this.#status = { kind: "failed", error };
      throw error;
    }
  }

  cancelPreparation(): Promise<void> {
    ++this.#generation;
    return this.#cancelActive();
  }

  stop(): Promise<void> {
    if (this.#stopTask !== undefined) {
      return this.#stopTask;
    }
    if (this.#stopped) {
      return Promise.resolve();
    }
    this.#stopTask = this.#performStop().finally(() => {
      this.#stopTask = undefined;
    });
    return this.#stopTask;
  }

  async #performStop(): Promise<void> {
    this.#stopped = true;
    ++this.#generation;
    await this.#cancelActive();
    await Promise.allSettled([...this.#tasks]);
    this.#status = { kind: "idle" };
    this.#preparation = { status: "idle" };
    this.#completion = { status: "idle" };
  }

  async destroy(): Promise<void> {
    if (!this.#destroyed) {
      await this.stop();
      this.#destroyed = true;
    }
  }

  async #cancelActive(): Promise<void> {
    const sessionId = this.#activeSessionId;
    this.#activeSessionId = undefined;
    if (sessionId !== undefined) {
      await this.#options.cancelSession(sessionId);
    }
    this.#preparation = { status: "idle" };
  }

  #track(task: Promise<unknown>): void {
    this.#tasks.add(task);
    void task.finally(() => {
      this.#tasks.delete(task);
    }).catch(() => undefined);
  }

  #requireClient(): SurfaceTransferClient {
    if (this.#options.client !== undefined) {
      return this.#options.client;
    }
    const reason =
      this.#status.kind === "unsupported"
        ? this.#status.reason
        : "Surface transfer capability is unavailable";
    throw new UnsupportedCapabilityError(reason);
  }

  #assertAlive(): void {
    if (this.#destroyed) {
      throw new Error("Surface transfer state has been destroyed");
    }
  }

  #assertRunning(): void {
    this.#assertAlive();
    if (this.#stopped || this.#status.kind !== "ready") {
      throw new Error("Surface transfer state must be started");
    }
  }
}

export * from "./surface-transfer-actions.ts";
