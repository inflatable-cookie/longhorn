import type {
  LayoutDocument,
  LayoutMutationRejection,
  LayoutMutationRequest,
  LayoutMutationReceipt,
} from "@inflatable-cookie/longhorn-layout";

import {
  OptimisticProjectionState,
  UnsupportedCapabilityError,
  type ClientStatus,
  type OptimisticProjector,
} from "./index.ts";

export type LayoutDispatchResult =
  | { readonly status: "committed"; readonly receipt: LayoutMutationReceipt }
  | {
      readonly status: "rejected";
      readonly rejection: LayoutMutationRejection;
    };

export type LayoutDispatcher = (
  request: LayoutMutationRequest,
) => Promise<LayoutDispatchResult>;

export interface LayoutStateOptions {
  readonly dispatch?: LayoutDispatcher;
  readonly unsupportedReason?: string;
}

export class LayoutState {
  readonly #options: LayoutStateOptions;
  readonly #projection = new OptimisticProjectionState<LayoutDocument>(
    (document) => document.revision,
  );
  #status = $state.raw<ClientStatus>({ kind: "idle" });
  #destroyed = false;

  constructor(options: LayoutStateOptions) {
    this.#options = options;
  }

  get status(): ClientStatus {
    return this.#status;
  }

  get authoritative(): LayoutDocument | undefined {
    return this.#projection.authoritative;
  }

  get projected(): LayoutDocument | undefined {
    return this.#projection.projected;
  }

  get pendingRequestIds(): readonly string[] {
    return this.#projection.pendingRequestIds;
  }

  async start(): Promise<void> {
    this.#assertAlive();
    this.#status =
      this.#options.dispatch === undefined
        ? {
            kind: "unsupported",
            reason:
              this.#options.unsupportedReason ??
              "layout dispatch is unavailable",
          }
        : { kind: "loading" };
  }

  accept(document: LayoutDocument): boolean {
    this.#assertAlive();
    const accepted = this.#projection.accept(document);
    if (accepted) {
      this.#status = { kind: "ready" };
    }
    return accepted;
  }

  reconnecting(): void {
    this.#assertAlive();
    this.#status =
      this.#projection.authoritative === undefined
        ? { kind: "loading" }
        : { kind: "reconnecting" };
  }

  async dispatch(
    request: LayoutMutationRequest,
    project: OptimisticProjector<LayoutDocument>,
  ): Promise<LayoutDispatchResult> {
    this.#assertAlive();
    const dispatch = this.#options.dispatch;
    if (dispatch === undefined) {
      const reason =
        this.#options.unsupportedReason ??
        "layout dispatch is unavailable";
      this.#status = { kind: "unsupported", reason };
      throw new UnsupportedCapabilityError(reason);
    }

    this.#projection.begin(request.request_id, project);
    try {
      const result = await dispatch(request);
      const document =
        result.status === "committed"
          ? result.receipt.authoritative_document
          : result.rejection.authoritative_document;
      this.#projection.settle(request.request_id, document);
      this.#status = { kind: "ready" };
      return result;
    } catch (error) {
      this.#projection.cancel(request.request_id);
      this.#status = { kind: "failed", error };
      throw error;
    }
  }

  cancel(requestId: string): void {
    this.#projection.cancel(requestId);
  }

  async stop(): Promise<void> {
    this.#projection.clear();
    this.#status = { kind: "idle" };
  }

  async destroy(): Promise<void> {
    if (!this.#destroyed) {
      await this.stop();
      this.#destroyed = true;
    }
  }

  #assertAlive(): void {
    if (this.#destroyed) {
      throw new Error("layout state has been destroyed");
    }
  }
}
