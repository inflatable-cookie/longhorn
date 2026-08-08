import type {
  DragSessionId,
  PanelSessionStartRequest,
  PanelTransferCommand,
  PanelTransferResponse,
  TransferCancelRequest,
  TransferCancelResponse,
  TransferClient,
  TransferClientSnapshot,
  TransferLeaseReceipt,
  TransferLeaseRequest,
  TransferLeaseResponse,
  TransferSessionResponse,
} from "@inflatable-cookie/longhorn/transfer";

import type { TimerScheduler } from "./scheduler.ts";

export type TransferPreparationState =
  | { readonly status: "idle" }
  | {
      readonly status: "preparing";
      readonly request: PanelSessionStartRequest;
    }
  | {
      readonly status: "prepared";
      readonly response: Extract<
        TransferSessionResponse,
        { status: "started" }
      >;
    }
  | {
      readonly status: "aborted";
      readonly response: Extract<
        TransferSessionResponse,
        { status: "aborted" }
      >;
    }
  | { readonly status: "timed_out"; readonly requestId: string }
  | { readonly status: "failed"; readonly error: unknown };

export type TransferLeaseState =
  | { readonly status: "idle" }
  | { readonly status: "publishing"; readonly request: TransferLeaseRequest }
  | {
      readonly status: "published";
      readonly response: Extract<
        TransferLeaseResponse,
        { status: "published" }
      >;
    }
  | {
      readonly status: "aborted";
      readonly response: Extract<
        TransferLeaseResponse,
        { status: "aborted" }
      >;
    }
  | { readonly status: "failed"; readonly error: unknown };

export type TransferCompletionState =
  | { readonly status: "idle" }
  | { readonly status: "committing"; readonly request: PanelTransferCommand }
  | {
      readonly status: "committed";
      readonly response: Extract<
        PanelTransferResponse,
        { status: "committed" }
      >;
    }
  | {
      readonly status: "aborted";
      readonly response: Extract<
        PanelTransferResponse,
        { status: "aborted" }
      >;
    }
  | { readonly status: "failed"; readonly error: unknown };

export type TransferCancellationState =
  | { readonly status: "idle" }
  | { readonly status: "cancelling"; readonly request: TransferCancelRequest }
  | {
      readonly status: "cancelled";
      readonly response: Extract<
        TransferCancelResponse,
        { status: "cancelled" }
      >;
    }
  | {
      readonly status: "aborted";
      readonly response: Extract<
        TransferCancelResponse,
        { status: "aborted" }
      >;
    }
  | { readonly status: "failed"; readonly error: unknown };

export interface TransferStateOptions {
  readonly client?: TransferClient;
  readonly unsupportedReason?: string;
  readonly makeCancellationRequest: (
    sessionId: DragSessionId,
  ) => TransferCancelRequest;
  readonly makeLeaseReleaseRequest?: (
    snapshot: TransferClientSnapshot,
    lease: TransferLeaseReceipt,
  ) => TransferLeaseRequest;
  readonly preparationTimeoutMs?: number;
  readonly scheduler?: TimerScheduler;
}

export class LeaseReleaseUnavailableError extends Error {
  constructor() {
    super("published transfer lease has no teardown request factory");
    this.name = "LeaseReleaseUnavailableError";
  }
}

export class InvalidLeaseReleaseRequestError extends Error {
  constructor() {
    super("lease teardown must publish a newer empty replacement");
    this.name = "InvalidLeaseReleaseRequestError";
  }
}

export class LeaseReleaseRejectedError extends Error {
  constructor() {
    super("lease teardown replacement was rejected");
    this.name = "LeaseReleaseRejectedError";
  }
}

export class TransferStateNotStartedError extends Error {
  constructor() {
    super("transfer state must be ready before operations");
    this.name = "TransferStateNotStartedError";
  }
}
