import {
  SurfaceClient,
  type SurfaceMutationRequest,
  type SurfaceMutationResponse,
  type SurfaceSnapshot,
} from "@inflatable-cookie/longhorn/surfaces";

import {
  OptimisticProjectionState,
  ReactiveClientState,
  UnsupportedCapabilityError,
  type ClientStatus,
  type OptimisticProjector,
} from "./index.ts";

export type SurfaceMutationOperation =
  | { readonly requestId: string; readonly status: "pending" }
  | {
      readonly requestId: string;
      readonly status: "committed" | "rejected";
      readonly response: SurfaceMutationResponse;
    }
  | {
      readonly requestId: string;
      readonly status: "failed";
      readonly error: unknown;
    };

export interface SurfaceStateOptions {
  readonly client?: SurfaceClient;
  readonly unsupportedReason?: string;
}

export class SurfaceState {
  readonly #client: SurfaceClient | undefined;
  readonly #lifecycle: ReactiveClientState<SurfaceSnapshot>;
  readonly #projection = new OptimisticProjectionState<
    SurfaceSnapshot["document"]
  >((document) => document.revision);
  #operations = $state.raw<readonly SurfaceMutationOperation[]>([]);
  #epoch: number | undefined;

  constructor(options: SurfaceStateOptions) {
    this.#client = options.client;
    this.#lifecycle = new ReactiveClientState({
      capability:
        options.client === undefined
          ? {
              kind: "unsupported",
              reason:
                options.unsupportedReason ??
                "Surface capability is unavailable",
            }
          : {
              kind: "supported",
              connect: (listener, onFailure) =>
                options.client!.subscribe(listener, onFailure),
            },
      onSnapshot: (snapshot) => this.#acceptSnapshot(snapshot),
    });
  }

  get status(): ClientStatus {
    return this.#lifecycle.status;
  }

  get snapshot(): SurfaceSnapshot | undefined {
    const snapshot = this.#lifecycle.snapshot;
    const document = this.#projection.authoritative;
    if (snapshot === undefined || document === undefined) {
      return snapshot;
    }
    return {
      ...snapshot,
      revision: document.revision,
      document,
    };
  }

  get projectedDocument(): SurfaceSnapshot["document"] | undefined {
    return this.#projection.projected;
  }

  get pendingRequestIds(): readonly string[] {
    return this.#projection.pendingRequestIds;
  }

  get operations(): readonly SurfaceMutationOperation[] {
    return this.#operations;
  }

  start(): Promise<void> {
    return this.#lifecycle.start();
  }

  reconnect(): Promise<void> {
    return this.#lifecycle.reconnect();
  }

  async mutate(
    request: SurfaceMutationRequest,
    project: OptimisticProjector<SurfaceSnapshot["document"]>,
  ): Promise<SurfaceMutationResponse> {
    const client = this.#client;
    if (client === undefined) {
      const reason =
        this.#lifecycle.status.kind === "unsupported"
          ? this.#lifecycle.status.reason
          : "Surface capability is unavailable";
      throw new UnsupportedCapabilityError(reason);
    }

    const epoch = this.#epoch;
    this.#projection.begin(request.request_id, project);
    this.#setOperation({
      requestId: request.request_id,
      status: "pending",
    });
    try {
      const response = await client.mutate(request);
      const document =
        response.status === "committed"
          ? response.receipt.authoritative_document
          : response.rejection.authoritative_document;
      if (epoch !== undefined && this.#epoch === epoch) {
        this.#projection.settle(request.request_id, document);
      } else {
        this.#projection.cancel(request.request_id);
      }
      this.#setOperation({
        requestId: request.request_id,
        status: response.status,
        response,
      });
      return response;
    } catch (error) {
      this.#projection.cancel(request.request_id);
      this.#setOperation({
        requestId: request.request_id,
        status: "failed",
        error,
      });
      this.#lifecycle.fail(error);
      throw error;
    }
  }

  async stop(): Promise<void> {
    this.#projection.clear();
    this.#operations = [];
    this.#epoch = undefined;
    await this.#lifecycle.stop();
  }

  async destroy(): Promise<void> {
    this.#projection.clear();
    this.#operations = [];
    this.#epoch = undefined;
    await this.#lifecycle.destroy();
  }

  #acceptSnapshot(snapshot: SurfaceSnapshot): void {
    if (this.#epoch !== snapshot.epoch) {
      this.#projection.clear();
      this.#epoch = snapshot.epoch;
    }
    this.#projection.accept(snapshot.document);
  }

  #setOperation(operation: SurfaceMutationOperation): void {
    this.#operations = [
      ...this.#operations.filter(
        ({ requestId }) => requestId !== operation.requestId,
      ),
      operation,
    ];
  }
}
