import type { InvokeTransport } from "@longhorn/core";
import { TauriTransport } from "@longhorn/tauri";

import type {
  BridgeHelloRequest,
  BridgeSessionId,
  DomainId,
} from "./generated/protocol.ts";
import type {
  BridgeOperationAdapter,
  BridgeOperationDescriptor,
} from "./operation.ts";
import {
  BridgeDomainClient,
} from "./operation.ts";
import type { BridgeNegotiationAdapter } from "./session.ts";
import {
  BridgeSession,
  BridgeSessionClient,
} from "./session.ts";

export const BRIDGE_HELLO_COMMAND = "longhorn_bridge_hello";
export const BRIDGE_AUTHORITY_COMMAND = "longhorn_bridge_authority";
export const BRIDGE_QUERY_COMMAND = "longhorn_bridge_query";
export const BRIDGE_COMMAND_COMMAND = "longhorn_bridge_command";
export const BRIDGE_CANCEL_COMMAND = "longhorn_bridge_cancel";
export const BRIDGE_RESYNC_COMMAND = "longhorn_bridge_resync";

/**
 * Generic bridge commands over the raw invoke-only Tauri edge.
 *
 * Domain routes and payload vocabulary arrive only through operation
 * descriptors owned by adapted domain packages.
 */
export class TauriBridgeAdapter
  implements BridgeNegotiationAdapter, BridgeOperationAdapter {
  readonly #transport: InvokeTransport;

  constructor(transport: InvokeTransport = new TauriTransport()) {
    this.#transport = transport;
  }

  negotiate(request: BridgeHelloRequest): Promise<unknown> {
    return this.#transport.invoke(BRIDGE_HELLO_COMMAND, { request });
  }

  authority(sessionId: BridgeSessionId): Promise<unknown> {
    return this.#transport.invoke(BRIDGE_AUTHORITY_COMMAND, { sessionId });
  }

  invoke<Request, Reply>(
    operation: BridgeOperationDescriptor<Request, Reply>,
    request: Request,
  ): Promise<unknown> {
    return this.#transport.invoke(commandFor(operation), {
      route: operation.route,
      request,
    });
  }

  resync(sessionId: BridgeSessionId, domainId: DomainId): Promise<unknown> {
    return this.#transport.invoke(BRIDGE_RESYNC_COMMAND, {
      sessionId,
      domainId,
    });
  }
}

export interface TauriBridgeConnection {
  readonly session: BridgeSession;
  readonly domain: BridgeDomainClient;
  readonly adapter: TauriBridgeAdapter;
}

/// Negotiates before exposing a Tauri-backed checked domain client.
export async function connectTauriBridge(
  request: BridgeHelloRequest,
  transport?: InvokeTransport,
): Promise<TauriBridgeConnection> {
  const adapter = new TauriBridgeAdapter(transport);
  const session = await new BridgeSessionClient(adapter).connect(request);
  return {
    session,
    domain: new BridgeDomainClient(session, adapter),
    adapter,
  };
}

function commandFor<Request, Reply>(
  operation: BridgeOperationDescriptor<Request, Reply>,
): string {
  switch (operation.kind) {
    case "query":
      return BRIDGE_QUERY_COMMAND;
    case "command":
      return BRIDGE_COMMAND_COMMAND;
    case "cancellation":
      return BRIDGE_CANCEL_COMMAND;
  }
}
