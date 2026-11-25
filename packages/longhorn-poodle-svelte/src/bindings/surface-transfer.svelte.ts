import type { SurfaceTransferClient } from "@inflatable-cookie/longhorn/surface-transfer";
import type {
  SurfaceSessionResponse,
  SurfaceSessionStartRequest,
  SurfaceTransferCommand,
  SurfaceTransferResponse,
} from "@inflatable-cookie/longhorn/surface-transfer";
import type { DragSessionId } from "@inflatable-cookie/longhorn/transfer";

import { UnsupportedCapabilityError } from "./index.ts";
import type { ClientStatus } from "./index.ts";
import { StartStop } from "./start-stop.ts";
import { throwTeardownFailures } from "./teardown.ts";

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
  readonly #tasks = new Set<Promise<unknown>>();
  readonly #lifetime = new StartStop();
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
    if (!this.#stopped && this.#status.kind === "failed") {
      this.#status = { kind: "ready" };
    }
    await this.#lifetime.start(async () => {
      this.#stopped = false;
      this.#status =
        this.#options.client === undefined
          ? {
              kind: "unsupported",
              reason:
                this.#options.unsupportedReason ?? "Surface transfer capability is unavailable",
            }
          : { kind: "ready" };
    });
  }

  async prepare(request: SurfaceSessionStartRequest): Promise<SurfaceSessionResponse> {
    this.#assertRunning();
    return this.#track(this.#prepare(request));
  }

  async #prepare(request: SurfaceSessionStartRequest): Promise<SurfaceSessionResponse> {
    const client = this.#requireClient();
    const generation = ++this.#generation;
    await this.#cancelActive();
    if (generation === this.#generation) {
      this.#preparation = { status: "preparing", request };
    }

    return client
      .start(request)
      .then(async (response) => {
        if (generation !== this.#generation || this.#stopped) {
          if (response.status === "started") {
            await this.#options.cancelSession(response.session.payload.session_id);
          }
          return response;
        }
        this.#preparation = {
          status: response.status === "started" ? "prepared" : "aborted",
          response,
        };
        this.#activeSessionId =
          response.status === "started" ? response.session.payload.session_id : undefined;
        return response;
      })
      .catch((error: unknown) => {
        if (generation === this.#generation && !this.#stopped) {
          this.#preparation = { status: "failed", error };
          this.#status = { kind: "failed", error };
        }
        throw error;
      });
  }

  async commit(request: SurfaceTransferCommand): Promise<SurfaceTransferResponse> {
    this.#assertRunning();
    return this.#track(this.#commit(request));
  }

  async #commit(request: SurfaceTransferCommand): Promise<SurfaceTransferResponse> {
    const client = this.#requireClient();
    this.#completion = { status: "committing", request };
    try {
      const response = await client.commit(request);
      this.#completion = {
        status: response.status === "committed" ? "committed" : "aborted",
        response,
      };
      if (
        this.#activeSessionId === request.session_id &&
        (response.status === "committed" || response.abort.session_consumed)
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
    if (this.#stopped) {
      return Promise.resolve();
    }
    ++this.#generation;
    return this.#track(this.#cancelActive());
  }

  stop(): Promise<void> {
    return this.#lifetime.stop(async () => {
      this.#stopped = true;
      ++this.#generation;
      this.#status = { kind: "idle" };
      const results = await Promise.allSettled([this.#cancelActive(), ...this.#tasks]);
      this.#status = { kind: "idle" };
      this.#preparation = { status: "idle" };
      this.#completion = { status: "idle" };
      throwTeardownFailures(results);
    });
  }

  async destroy(): Promise<void> {
    this.#destroyed = true;
    await this.stop();
  }

  async #cancelActive(): Promise<void> {
    const sessionId = this.#activeSessionId;
    this.#activeSessionId = undefined;
    this.#preparation = { status: "idle" };
    if (sessionId !== undefined) {
      await this.#options.cancelSession(sessionId);
    }
  }

  #track<Result>(task: Promise<Result>): Promise<Result> {
    this.#tasks.add(task);
    void task
      .finally(() => {
        this.#tasks.delete(task);
      })
      .catch(() => undefined);
    return task;
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
