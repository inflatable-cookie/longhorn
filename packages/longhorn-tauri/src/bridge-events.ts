import type { EventTransport, Unlisten } from "@inflatable-cookie/longhorn/core";
import { TauriEventTransport } from "@inflatable-cookie/longhorn-tauri/events";

import type { BridgeCodec } from "@inflatable-cookie/longhorn/bridge";
import {
  parseBridgeJobTerminalEvent,
  parseBridgeProgressEvent,
} from "@inflatable-cookie/longhorn/bridge";
import { BridgeJobTracker } from "@inflatable-cookie/longhorn/bridge";
import type {
  BridgeJobId,
  BridgeJobTerminalEvent,
  BridgeProgressEvent,
  BridgeRequestId,
  BridgeSessionId,
  DomainId,
} from "@inflatable-cookie/longhorn/bridge/protocol";
import type { BridgeStreamSource } from "@inflatable-cookie/longhorn/bridge/stream";
import { TauriBridgeAdapter } from "./bridge.ts";

export const BRIDGE_DOMAIN_EVENT = "longhorn://bridge/domain";
export const BRIDGE_PROGRESS_EVENT = "longhorn://bridge/progress";
export const BRIDGE_TERMINAL_EVENT = "longhorn://bridge/terminal";

/**
 * Event-capable stream source. `connectBridgeStream` installs `listen` before
 * calling `loadSnapshot`, preserving the required subscription handshake.
 */
export class TauriBridgeStreamSource implements BridgeStreamSource {
  readonly #transport: EventTransport;
  readonly #adapter: TauriBridgeAdapter;
  readonly #sessionId: BridgeSessionId;
  readonly #domainId: DomainId;

  constructor(
    sessionId: BridgeSessionId,
    domainId: DomainId,
    transport: EventTransport = new TauriEventTransport(),
  ) {
    this.#sessionId = sessionId;
    this.#domainId = domainId;
    this.#transport = transport;
    this.#adapter = new TauriBridgeAdapter(transport);
  }

  listen(listener: (value: unknown) => void): Promise<Unlisten> {
    return this.#transport.listen(BRIDGE_DOMAIN_EVENT, listener);
  }

  loadSnapshot(): Promise<unknown> {
    return this.#adapter.resync(this.#sessionId, this.#domainId);
  }
}

export interface BridgeJobListeners<Progress, Success, Detail> {
  readonly progressCodec: BridgeCodec<Progress>;
  readonly successCodec: BridgeCodec<Success>;
  readonly detailCodec: BridgeCodec<Detail>;
  readonly progress?: (event: BridgeProgressEvent<Progress>) => void;
  readonly terminal: (
    event: BridgeJobTerminalEvent<Success, Detail>,
  ) => void;
}

/**
 * Installs request/job-correlated progress and terminal listeners.
 * Wrong-correlation and post-terminal messages never reach consumers.
 */
export async function listenTauriBridgeJob<Progress, Success, Detail>(
  requestId: BridgeRequestId,
  jobId: BridgeJobId,
  listeners: BridgeJobListeners<Progress, Success, Detail>,
  transport: EventTransport = new TauriEventTransport(),
): Promise<Unlisten> {
  const tracker = new BridgeJobTracker(requestId, jobId);
  const disposeProgress = await transport.listen(
    BRIDGE_PROGRESS_EVENT,
    (value) => {
      const event = parseBridgeProgressEvent(
        value,
        listeners.progressCodec,
      );
      if (tracker.classifyProgress(event) === "accept") {
        listeners.progress?.(event);
      }
    },
  );
  let disposeTerminal: Unlisten;
  try {
    disposeTerminal = await transport.listen(
      BRIDGE_TERMINAL_EVENT,
      (value) => {
        const event = parseBridgeJobTerminalEvent(
          value,
          listeners.successCodec,
          listeners.detailCodec,
        );
        if (tracker.classifyTerminal(event) === "accept") {
          listeners.terminal(event);
        }
      },
    );
  } catch (error) {
    await disposeProgress();
    throw error;
  }
  return async () => {
    await disposeTerminal();
    await disposeProgress();
  };
}
