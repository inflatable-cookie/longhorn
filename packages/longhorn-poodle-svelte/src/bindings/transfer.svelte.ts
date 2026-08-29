import {
  TransferClient,
  type DragSessionId,
  type PanelSessionStartRequest,
  type PanelTransferCommand,
  type PanelTransferResponse,
  type TransferCancelRequest,
  type TransferCancelResponse,
  type TransferClientSnapshot,
  type TransferLeaseRequest,
  type TransferLeaseResponse,
  type TransferSessionResponse,
} from "@inflatable-cookie/longhorn/transfer";

import {
  ReactiveClientState,
  UnsupportedCapabilityError,
  type ClientStatus,
} from "./index.ts";
import {
  systemTimerScheduler,
} from "./scheduler.ts";
import {
  InvalidLeaseReleaseRequestError,
  LeaseReleaseRejectedError,
  LeaseReleaseUnavailableError,
  TransferStateNotStartedError,
  type TransferCancellationState,
  type TransferCompletionState,
  type TransferLeaseState,
  type TransferPreparationState,
  type TransferStateOptions,
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
  #completion = $state.raw<TransferCompletionState>({ status: "idle" });
  #cancellation = $state.raw<TransferCancellationState>({ status: "idle" });
  #activeSessionId: DragSessionId | undefined;
  #preparationGeneration = 0;
  #preparationTimer: unknown;
  #preparationTasks = new Set<Promise<unknown>>();
  #stopTask: Promise<void> | undefined;
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
              reason:
                options.unsupportedReason ??
                "transfer capability is unavailable",
            }
          : {
              kind: "supported",
              connect: (listener, onFailure) =>
                client.connect(listener, onFailure),
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

  async start(): Promise<void> {
    this.#stopped = false;
    try {
      await this.#lifecycle.start();
    } catch (error) {
      this.#stopped = true;
      throw error;
    }
  }

  async reconnect(): Promise<void> {
    this.#stopped = false;
    try {
      await this.#lifecycle.reconnect();
    } catch (error) {
      this.#stopped = true;
      throw error;
    }
  }

  async preparePanel(
    request: PanelSessionStartRequest,
  ): Promise<TransferSessionResponse> {
    this.#assertRunning();
    const client = this.#requireClient();
    await this.#supersedePreparation();

    const generation = ++this.#preparationGeneration;
    this.#preparation = { status: "preparing", request };
    this.#schedulePreparationTimeout(generation, request.request_id);

    const task = client.startPanel(request).then(async (response) => {
      if (
        generation !== this.#preparationGeneration ||
        this.#stopped
      ) {
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
        response.status === "started"
          ? response.session.payload.session_id
          : undefined;
      return response;
    }).catch((error) => {
      if (generation === this.#preparationGeneration && !this.#stopped) {
        this.#clearPreparationTimer();
        this.#preparation = { status: "failed", error };
        this.#lifecycle.fail(error);
      }
      throw error;
    });
    this.#trackPreparation(task);
    return task;
  }

  async publishLease(
    request: TransferLeaseRequest,
  ): Promise<TransferLeaseResponse> {
    this.#assertRunning();
    const client = this.#requireClient();
    this.#lease = { status: "publishing", request };
    try {
      const response = await client.publishLease(request);
      this.#lease =
        response.status === "published"
          ? { status: "published", response }
          : { status: "aborted", response };
      return response;
    } catch (error) {
      this.#lease = { status: "failed", error };
      this.#lifecycle.fail(error);
      throw error;
    }
  }

  async commitPanel(
    request: PanelTransferCommand,
  ): Promise<PanelTransferResponse> {
    this.#assertRunning();
    const client = this.#requireClient();
    this.#completion = { status: "committing", request };
    try {
      const response = await client.commitPanel(request);
      this.#completion =
        response.status === "committed"
          ? { status: "committed", response }
          : { status: "aborted", response };
      if (
        response.status === "committed" ||
        response.abort.session_consumed
      ) {
        this.#activeSessionId = undefined;
      }
      return response;
    } catch (error) {
      this.#completion = { status: "failed", error };
      this.#lifecycle.fail(error);
      throw error;
    }
  }

  async cancel(
    request: TransferCancelRequest,
  ): Promise<TransferCancelResponse> {
    this.#assertRunning();
    return this.#cancelRequest(request);
  }

  cancelPreparation(): Promise<void> {
    return this.#supersedePreparation();
  }

  releaseLease(): Promise<void> {
    return this.#releaseLease();
  }

  stop(): Promise<void> {
    if (this.#stopTask !== undefined) {
      return this.#stopTask;
    }
    if (this.#stopped && this.#lifecycle.status.kind === "idle") {
      return Promise.resolve();
    }
    this.#stopTask = this.#performStop().finally(() => {
      this.#stopTask = undefined;
    });
    return this.#stopTask;
  }

  async #performStop(): Promise<void> {
    this.#stopped = true;
    ++this.#preparationGeneration;
    this.#clearPreparationTimer();

    const sessionId = this.#activeSessionId;
    this.#activeSessionId = undefined;
    if (sessionId !== undefined) {
      await this.#cancelSession(sessionId);
    }
    await Promise.allSettled([...this.#preparationTasks]);
    await this.#releaseLease();
    await this.#lifecycle.stop();

    this.#preparation = { status: "idle" };
    this.#lease = { status: "idle" };
    this.#completion = { status: "idle" };
    this.#cancellation = { status: "idle" };
  }

  async destroy(): Promise<void> {
    await this.stop();
    await this.#lifecycle.destroy();
  }

  async #supersedePreparation(): Promise<void> {
    ++this.#preparationGeneration;
    this.#clearPreparationTimer();
    const sessionId = this.#activeSessionId;
    this.#activeSessionId = undefined;
    if (sessionId !== undefined) {
      await this.#cancelSession(sessionId);
    }
    this.#preparation = { status: "idle" };
  }

  #schedulePreparationTimeout(
    generation: number,
    requestId: string,
  ): void {
    const timeoutMs = this.#options.preparationTimeoutMs;
    if (timeoutMs === undefined) {
      return;
    }
    if (!Number.isFinite(timeoutMs) || timeoutMs < 0) {
      throw new RangeError("preparation timeout must be finite and non-negative");
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
    scheduler.clear(this.#preparationTimer);
    this.#preparationTimer = undefined;
  }

  #trackPreparation(task: Promise<unknown>): void {
    this.#preparationTasks.add(task);
    void task.finally(() => {
      this.#preparationTasks.delete(task);
    }).catch(() => undefined);
  }

  #cancelSession(sessionId: DragSessionId): Promise<TransferCancelResponse> {
    return this.#cancelRequest(
      this.#options.makeCancellationRequest(sessionId),
    );
  }

  async #cancelRequest(
    request: TransferCancelRequest,
  ): Promise<TransferCancelResponse> {
    const client = this.#requireClient();
    this.#cancellation = { status: "cancelling", request };
    try {
      const response = await client.cancel(request);
      this.#cancellation =
        response.status === "cancelled"
          ? { status: "cancelled", response }
          : { status: "aborted", response };
      if (
        response.status === "cancelled" ||
        response.abort.session_consumed
      ) {
        this.#activeSessionId = undefined;
      }
      return response;
    } catch (error) {
      this.#cancellation = { status: "failed", error };
      this.#lifecycle.fail(error);
      throw error;
    }
  }

  async #releaseLease(): Promise<void> {
    if (this.#lease.status !== "published") {
      return;
    }
    const makeRequest = this.#options.makeLeaseReleaseRequest;
    const snapshot = this.#lifecycle.snapshot;
    if (makeRequest === undefined || snapshot === undefined) {
      throw new LeaseReleaseUnavailableError();
    }

    const receipt = this.#lease.response.lease;
    if (receipt.zone_count === 0) {
      this.#lease = { status: "idle" };
      return;
    }
    const request = makeRequest(snapshot, receipt);
    if (
      request.zones.length !== 0 ||
      request.generation <= receipt.generation
    ) {
      throw new InvalidLeaseReleaseRequestError();
    }
    const response = await this.#requireClient().publishLease(request);
    if (response.status !== "published") {
      this.#lease = { status: "aborted", response };
      throw new LeaseReleaseRejectedError();
    }
    this.#lease = { status: "idle" };
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
