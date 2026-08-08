import type { InvokeTransport } from "@inflatable-cookie/longhorn/core";
import {
  assertCompatibleTransferCommitSelector,
  assertTransferProtocolVersion,
} from "@inflatable-cookie/longhorn/transfer";

import type {
  SurfaceSessionResponse,
  SurfaceSessionStartRequest,
  SurfaceTransferCommand,
  SurfaceTransferResponse,
} from "./generated/protocol.ts";
import {
  assertCompatibleSurfaceSessionResponse,
  assertCompatibleSurfaceTransferResponse,
} from "./compatibility.ts";

export const TRANSFER_START_SURFACE_COMMAND =
  "longhorn_transfer_start_surface";
export const TRANSFER_COMMIT_SURFACE_COMMAND =
  "longhorn_transfer_commit_surface";

export class SurfaceTransferClient {
  readonly #transport: InvokeTransport;

  constructor(transport: InvokeTransport) {
    this.#transport = transport;
  }

  async start(
    request: SurfaceSessionStartRequest,
  ): Promise<SurfaceSessionResponse> {
    assertTransferProtocolVersion(request.protocol_version);
    const value = await this.#transport.invoke(
      TRANSFER_START_SURFACE_COMMAND,
      { request },
    );
    assertCompatibleSurfaceSessionResponse(value);
    return value;
  }

  async commit(
    request: SurfaceTransferCommand,
  ): Promise<SurfaceTransferResponse> {
    assertTransferProtocolVersion(request.protocol_version);
    assertCompatibleTransferCommitSelector(request.selector);
    const value = await this.#transport.invoke(
      TRANSFER_COMMIT_SURFACE_COMMAND,
      { request },
    );
    assertCompatibleSurfaceTransferResponse(value);
    return value;
  }
}
