import type { TransferClient } from "@inflatable-cookie/longhorn/transfer";
import type {
  DragSessionId,
  PanelSessionStartRequest,
  PanelTransferCommand,
  PanelTransferResponse,
  TransferCancelRequest,
  TransferCancelResponse,
  TransferClientSnapshot,
  TransferLeaseRequest,
  TransferLeaseResponse,
  TransferSessionResponse,
} from "@inflatable-cookie/longhorn/transfer";

import {
  ClientStateDestroyedError,
  ReactiveClientState,
  UnsupportedCapabilityError,
} from "./index.ts";
import type { ClientStatus } from "./index.ts";
import { StartStop } from "./start-stop.ts";
import { throwTeardownFailures } from "./teardown.ts";
import { systemTimerScheduler } from "./scheduler.ts";
import {
  InvalidLeaseReleaseRequestError,
  LeaseReleaseRejectedError,
  LeaseReleaseUnavailableError,
  TransferStateNotStartedError,
} from "./transfer-model.ts";
import type {
  TransferCancellationState,
  TransferCompletionState,
  TransferLeaseState,
  TransferPreparationState,
  TransferStateOptions,
} from "./transfer-model.ts";

export * from "./transfer-model.ts";
export * from "./transfer-actions.ts";
export type { TimerScheduler } from "./scheduler.ts";

export class TransferState {
  readonly #client: TransferClient | undefined;
  readonly #options: TransferStateOptions;
  readonly #lifecycle: ReactiveClientState<TransferClientSnapshot>;
  #preparation = $state.raw<TransferPreparationState>({ status: "idle" });
  #lease = $state.raw<TransferLeaseState>({ status: "idle" });
  #publishedLease: Extract<TransferLeaseResponse, { status: "published" }>["lease"] | undefined;
  #completion = $state.raw<TransferCompletionState>({ status: "idle" });
  #cancellation = $state.raw<TransferCancellationState>({ status: "idle" });
  #activeSessionId: DragSessionId | undefined;
  #preparationGeneration = 0;
  #preparationTimer: unknown;
  readonly #tasks = new Set<Promise<unknown>>();
  readonly #lifetime = new StartStop();
  #destroyed = false;
  #stopped = true;

  constructor(options: TransferStateOptions) {
    this.#options = options;
    const client = options.client;
    this.#client = client;
    this.#lifecycle = new ReactiveClientState({
      capability:
        client === undefined
          ? {
              kind: "unsupported",
              reason: options.unsupportedReason ?? "transfer capability is unavailable",
            }
          : {
              kind: "supported",
              connect: (listener, onFailure) => client.connect(listener, onFailure),
            },
    });
  }

  get status(): ClientStatus {
    return this.#lifecycle.status;
  }

  get snapshot(): TransferClientSnapshot | undefined {
    return this.#lifecycle.snapshot;
  }

  get preparation(): TransferPreparationState {
    return this.#preparation;
  }

  get lease(): TransferLeaseState {
    return this.#lease;
  }

  get completion(): TransferCompletionState {
    return this.#completion;
  }

  get cancellation(): TransferCancellationState {
    return this.#cancellation;
  }

  start(): Promise<void> {
    return this.#start(false);
  }

  reconnect(): Promise<void> {
    return this.#stopped ? this.#start(true) : this.#connect(this.#lifetime.generation, true);
  }

  #start(reconnect: boolean): Promise<void> {
    if (this.#destroyed) {
      return Promise.reject(new ClientStateDestroyedError());
    }
    return this.#lifetime.start((generation) => this.#connect(generation, reconnect), true);
  }

  async #connect(generation: number, reconnect: boolean): Promise<void> {
    this.#stopped = false;
    try {
      await (reconnect ? this.#lifecycle.reconnect() : this.#lifecycle.start());
    } catch (error) {
      if (!reconnect && generation === this.#lifetime.generation) {
        this.#stopped = true;
      }
      throw error;
    }
  }

  async preparePanel(request: PanelSessionStartRequest): Promise<TransferSessionResponse> {
    this.#assertRunning();
    const timeoutMs = this.#options.preparationTimeoutMs;
    if (timeoutMs !== undefined && (!Number.isFinite(timeoutMs) || timeoutMs < 0)) {
      throw new RangeError("preparation timeout must be finite and non-negative");
    }
    return this.#track(this.#preparePanel(request, timeoutMs));
  }

  async #preparePanel(
    request: PanelSessionStartRequest,
    timeoutMs: number | undefined,
  ): Promise<TransferSessionResponse> {
    const client = this.#requireClient();
    const lifetime = this.#lifetime.generation;
    const superseded = this.#supersedePreparation();
    const generation = this.#preparationGeneration;
    await superseded;
    if (lifetime === this.#lifetime.generation && generation === this.#preparationGeneration) {
      this.#preparation = { status: "preparing", request };
      try {
        this.#schedulePreparationTimeout(generation, request.request_id, timeoutMs);
      } catch (error) {
        this.#preparation = { status: "failed", error };
        throw error;
      }
    }

    return client
      .startPanel(request)
      .then(async (response) => {
        if (generation !== this.#preparationGeneration || this.#stopped) {
          if (response.status === "started") {
            await this.#cancelSession(response.session.payload.session_id);
          }
          return response;
        }

        this.#clearPreparationTimer();
        this.#preparation =
          response.status === "started"
            ? { status: "prepared", response }
            : { status: "aborted", response };
        this.#activeSessionId =
          response.status === "started" ? response.session.payload.session_id : undefined;
        return response;
      })
      .catch((error: unknown) => {
        if (generation === this.#preparationGeneration && !this.#stopped) {
          this.#clearPreparationTimer();
          this.#preparation = { status: "failed", error };
        }
        throw error;
      });
  }

  async publishLease(request: TransferLeaseRequest): Promise<TransferLeaseResponse> {
    this.#assertRunning();
    return this.#track(this.#publishLease(request));
  }

  async #publishLease(request: TransferLeaseRequest): Promise<TransferLeaseResponse> {
    const client = this.#requireClient();
    this.#lease = { status: "publishing", request };
    try {
      const response = await client.publishLease(request);
      if (response.status === "published") {
        const prior = this.#publishedLease;
        if (prior === undefined || response.lease.generation >= prior.generation) {
          this.#publishedLease = response.lease;
        }
        this.#lease = { status: "published", response };
      } else {
        this.#lease = { status: "aborted", response };
      }
      return response;
    } catch (error) {
      this.#lease = { status: "failed", error };
      throw error;
    }
  }

  async commitPanel(request: PanelTransferCommand): Promise<PanelTransferResponse> {
    this.#assertRunning();
    return this.#track(this.#commitPanel(request));
  }

  async #commitPanel(request: PanelTransferCommand): Promise<PanelTransferResponse> {
    const client = this.#requireClient();
    this.#completion = { status: "committing", request };
    try {
      const response = await client.commitPanel(request);
      this.#completion =
        response.status === "committed"
          ? { status: "committed", response }
          : { status: "aborted", response };
      if (
        this.#activeSessionId === request.session_id &&
        (response.status === "committed" || response.abort.session_consumed)
      ) {
        this.#activeSessionId = undefined;
      }
      return response;
    } catch (error) {
      this.#completion = { status: "failed", error };
      throw error;
    }
  }

  async cancel(request: TransferCancelRequest): Promise<TransferCancelResponse> {
    this.#assertRunning();
    return this.#track(this.#cancelRequest(request));
  }

  cancelPreparation(): Promise<void> {
    return this.#stopped ? Promise.resolve() : this.#track(this.#supersedePreparation());
  }

  releaseLease(): Promise<void> {
    return this.#stopped ? Promise.resolve() : this.#track(this.#releaseLease());
  }

  stop(): Promise<void> {
    return this.#lifetime.stop(() => this.#performStop());
  }

  async #performStop(): Promise<void> {
    if (this.#stopped && this.#lifecycle.status.kind === "idle") {
      return;
    }
    this.#stopped = true;
    ++this.#preparationGeneration;

    const sessionId = this.#activeSessionId;
    this.#activeSessionId = undefined;
    const results = await Promise.allSettled([this.#cancelForStop(sessionId), ...this.#tasks]);
    const lease = await Promise.allSettled([this.#releaseLease()]);
    const subscription = await Promise.allSettled([this.#lifecycle.stop()]);

    this.#preparation = { status: "idle" };
    this.#lease = { status: "idle" };
    this.#publishedLease = undefined;
    this.#completion = { status: "idle" };
    this.#cancellation = { status: "idle" };
    throwTeardownFailures([...results, ...lease, ...subscription]);
  }

  async #cancelForStop(sessionId: DragSessionId | undefined): Promise<void> {
    try {
      this.#clearPreparationTimer();
    } finally {
      if (sessionId !== undefined) {
        await this.#cancelSession(sessionId);
      }
    }
  }

  async destroy(): Promise<void> {
    this.#destroyed = true;
    try {
      await this.stop();
    } finally {
      await this.#lifecycle.destroy();
    }
  }

  async #supersedePreparation(): Promise<void> {
    ++this.#preparationGeneration;
    this.#clearPreparationTimer();
    const sessionId = this.#activeSessionId;
    this.#activeSessionId = undefined;
    this.#preparation = { status: "idle" };
    if (sessionId !== undefined) {
      await this.#cancelSession(sessionId);
    }
  }

  #schedulePreparationTimeout(
    generation: number,
    requestId: string,
    timeoutMs: number | undefined,
  ): void {
    if (timeoutMs === undefined) {
      return;
    }
    const scheduler = this.#options.scheduler ?? systemTimerScheduler;
    this.#preparationTimer = scheduler.set(timeoutMs, () => {
      if (generation !== this.#preparationGeneration || this.#stopped) {
        return;
      }
      ++this.#preparationGeneration;
      this.#preparationTimer = undefined;
      this.#preparation = { status: "timed_out", requestId };
    });
  }

  #clearPreparationTimer(): void {
    if (this.#preparationTimer === undefined) {
      return;
    }
    const scheduler = this.#options.scheduler ?? systemTimerScheduler;
    const timer = this.#preparationTimer;
    this.#preparationTimer = undefined;
    scheduler.clear(timer);
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

  #cancelSession(sessionId: DragSessionId): Promise<TransferCancelResponse> {
    return this.#cancelRequest(this.#options.makeCancellationRequest(sessionId));
  }

  async #cancelRequest(request: TransferCancelRequest): Promise<TransferCancelResponse> {
    const client = this.#requireClient();
    this.#cancellation = { status: "cancelling", request };
    try {
      const response = await client.cancel(request);
      this.#cancellation =
        response.status === "cancelled"
          ? { status: "cancelled", response }
          : { status: "aborted", response };
      if (
        this.#activeSessionId === request.session_id &&
        (response.status === "cancelled" || response.abort.session_consumed)
      ) {
        this.#activeSessionId = undefined;
      }
      return response;
    } catch (error) {
      this.#cancellation = { status: "failed", error };
      throw error;
    }
  }

  async #releaseLease(): Promise<void> {
    const receipt = this.#publishedLease;
    if (receipt === undefined) {
      return;
    }
    if (receipt.zone_count === 0) {
      this.#publishedLease = undefined;
      this.#lease = { status: "idle" };
      return;
    }
    const makeRequest = this.#options.makeLeaseReleaseRequest;
    const snapshot = this.#lifecycle.snapshot;
    if (makeRequest === undefined || snapshot === undefined) {
      throw new LeaseReleaseUnavailableError();
    }
    const request = makeRequest(snapshot, receipt);
    if (request.zones.length !== 0 || request.generation <= receipt.generation) {
      throw new InvalidLeaseReleaseRequestError();
    }
    const response = await this.#requireClient().publishLease(request);
    if (response.status !== "published") {
      this.#lease = { status: "aborted", response };
      throw new LeaseReleaseRejectedError();
    }
    if (this.#publishedLease === receipt) {
      this.#publishedLease = undefined;
      this.#lease = { status: "idle" };
    }
  }

  #requireClient(): TransferClient {
    if (this.#client !== undefined) {
      return this.#client;
    }
    const reason =
      this.#lifecycle.status.kind === "unsupported"
        ? this.#lifecycle.status.reason
        : "transfer capability is unavailable";
    throw new UnsupportedCapabilityError(reason);
  }

  #assertRunning(): void {
    if (this.#stopped || this.#lifecycle.status.kind !== "ready") {
      throw new TransferStateNotStartedError();
    }
  }
}
