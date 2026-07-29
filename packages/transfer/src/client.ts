import {
  isEventTransport,
  type ConnectionFailureReporter,
  type InvokeTransport,
} from "@longhorn/core";

import {
  type PanelSessionStartRequest,
  type PanelTransferCommand,
  type PanelTransferResponse,
  type TransferCancelRequest,
  type TransferCancelResponse,
  type TransferClientSnapshot,
  type TransferLeaseRequest,
  type TransferLeaseResponse,
  type TransferSessionResponse,
} from "./generated/protocol.ts";
import {
  assertCompatiblePanelTransferResponse,
  assertCompatibleTransferClientSnapshot,
  assertCompatibleTransferCancelResponse,
  assertCompatibleTransferCommitSelector,
  assertCompatibleTransferLeaseResponse,
  assertCompatibleTransferSessionResponse,
  assertCompatibleTransferTargetBinding,
  assertTransferProtocolVersion,
} from "./compatibility.ts";
import {
  TransferClientConnection,
  type TransferClientSnapshotListener,
} from "./connection.ts";

export const TRANSFER_SNAPSHOT_COMMAND = "longhorn_transfer_snapshot";
export const TRANSFER_START_PANEL_COMMAND =
  "longhorn_transfer_start_panel";
export const TRANSFER_PUBLISH_LEASE_COMMAND =
  "longhorn_transfer_publish_lease";
export const TRANSFER_COMMIT_PANEL_COMMAND =
  "longhorn_transfer_commit_panel";
export const TRANSFER_CANCEL_COMMAND = "longhorn_transfer_cancel";

export class TransferClient {
  readonly #transport: InvokeTransport;

  constructor(transport: InvokeTransport) {
    this.#transport = transport;
  }

  async snapshot(): Promise<TransferClientSnapshot> {
    const value = await this.#transport.invoke(TRANSFER_SNAPSHOT_COMMAND, {});
    assertCompatibleTransferClientSnapshot(value);
    return value;
  }

  connect(
    listener?: TransferClientSnapshotListener,
    onFailure?: ConnectionFailureReporter,
  ): TransferClientConnection {
    if (!isEventTransport(this.#transport)) {
      throw new TypeError(
        "transfer connection requires an event-capable transport",
      );
    }
    return new TransferClientConnection(
      this.#transport,
      () => this.snapshot(),
      listener,
      onFailure,
    );
  }

  async startPanel(
    request: PanelSessionStartRequest,
  ): Promise<TransferSessionResponse> {
    assertTransferProtocolVersion(request.protocol_version);
    return this.#invoke(
      TRANSFER_START_PANEL_COMMAND,
      request,
      assertCompatibleTransferSessionResponse,
    );
  }

  async publishLease(
    request: TransferLeaseRequest,
  ): Promise<TransferLeaseResponse> {
    assertTransferProtocolVersion(request.protocol_version);
    for (const zone of request.zones) {
      assertCompatibleTransferTargetBinding(zone.target);
    }
    return this.#invoke(
      TRANSFER_PUBLISH_LEASE_COMMAND,
      request,
      assertCompatibleTransferLeaseResponse,
    );
  }

  async commitPanel(
    request: PanelTransferCommand,
  ): Promise<PanelTransferResponse> {
    assertTransferProtocolVersion(request.protocol_version);
    assertCompatibleTransferCommitSelector(request.selector);
    return this.#invoke(
      TRANSFER_COMMIT_PANEL_COMMAND,
      request,
      assertCompatiblePanelTransferResponse,
    );
  }

  async cancel(
    request: TransferCancelRequest,
  ): Promise<TransferCancelResponse> {
    assertTransferProtocolVersion(request.protocol_version);
    return this.#invoke(
      TRANSFER_CANCEL_COMMAND,
      request,
      assertCompatibleTransferCancelResponse,
    );
  }

  async #invoke<T>(
    command: string,
    request:
      | PanelSessionStartRequest
      | TransferLeaseRequest
      | PanelTransferCommand
      | TransferCancelRequest,
    guard: (value: unknown) => asserts value is T,
  ): Promise<T> {
    const value = await this.#transport.invoke(command, { request });
    guard(value);
    return value;
  }
}
