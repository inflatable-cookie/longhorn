import type {
  ConnectionFailureReporter,
  EventTransport,
  Unlisten,
} from "@inflatable-cookie/longhorn/core";
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
  readonly terminal: (event: BridgeJobTerminalEvent<Success, Detail>) => void;
  /**
   * Reports a malformed event for this job or a teardown failure, mirroring
   * `CheckedSnapshotConnection`. A malformed terminal event is the only way
   * the job would otherwise never terminate for the consumer, so one parse
   * failure reports `{ phase: "event" }` and tears both listeners down.
   */
  readonly onFailure?: ConnectionFailureReporter;
}

/**
 * Installs request/job-correlated progress and terminal listeners.
 * Wrong-correlation messages are dropped before their payload is decoded, and
 * post-terminal messages are dropped without decoding, so neither can fail
 * this job or reach consumers.
 *
 * Contract 010 makes events projections, not durable delivery, and the bridge
 * has no job-status query. A terminal that fires before its listener is
 * registered (the jobId is only known once the job has started) is therefore
 * not recovered here; see docs/knowledge/questions.md.
 */
export async function listenTauriBridgeJob<Progress, Success, Detail>(
  requestId: BridgeRequestId,
  jobId: BridgeJobId,
  listeners: BridgeJobListeners<Progress, Success, Detail>,
  transport: EventTransport = new TauriEventTransport(),
): Promise<Unlisten> {
  const tracker = new BridgeJobTracker(requestId, jobId);
  // Set once the job has failed or been closed. Nothing reaches the consumer
  // after that, and a registration that completes later is released at once.
  let stopped = false;
  let registered = false;
  const unlistens: Array<Unlisten> = [];

  // Last registered, first released.
  const teardown = async (): Promise<void> => {
    const pending = unlistens.splice(0).reverse();
    const results = await Promise.allSettled(
      pending.map((unlisten) => Promise.resolve().then(() => unlisten())),
    );
    for (const result of results) {
      if (result.status === "rejected") {
        throw result.reason;
      }
    }
  };

  const reportUnlistenFailure = (error: unknown): void => {
    try {
      listeners.onFailure?.({ phase: "unlisten", error });
    } catch {
      // A secondary reporter failure cannot replace the original error.
    }
  };

  const notify = (callback: () => void): void => {
    try {
      callback();
    } catch (error) {
      // A transport may deliver inline before returning its unlisten.
      if (registered) {
        throw error;
      }
    }
  };

  const fail = (error: unknown): void => {
    if (stopped) {
      return;
    }
    stopped = true;
    void teardown().catch(reportUnlistenFailure);
    notify(() => listeners.onFailure?.({ phase: "event", error }));
  };

  const adopt = async (unlisten: Unlisten): Promise<void> => {
    if (!stopped) {
      unlistens.push(unlisten);
      return;
    }
    try {
      await unlisten();
    } catch (error) {
      reportUnlistenFailure(error);
    }
  };

  // The channels are shared by every job, so admission comes before any
  // job-specific decode: another job's payload cannot fail this one. A value
  // from another request is refused on its request id alone. Identifiers that
  // cannot be read as part of this request are malformed.
  const admits = (value: unknown): boolean => {
    try {
      return tracker.admits(value);
    } catch (error) {
      fail(error);
      return false;
    }
  };

  const onProgress = (value: unknown): void => {
    if (stopped || tracker.isTerminal() || !admits(value)) {
      return;
    }
    let event: BridgeProgressEvent<Progress>;
    try {
      event = parseBridgeProgressEvent(value, listeners.progressCodec);
    } catch (error) {
      fail(error);
      return;
    }
    if (tracker.classifyProgress(event) === "accept") {
      notify(() => listeners.progress?.(event));
    }
  };

  const onTerminal = (value: unknown): void => {
    // After this job's terminal, input is not decoded at all: a late or
    // malformed event cannot fail a job that has already settled.
    if (stopped || tracker.isTerminal() || !admits(value)) {
      return;
    }
    let event: BridgeJobTerminalEvent<Success, Detail>;
    try {
      event = parseBridgeJobTerminalEvent(value, listeners.successCodec, listeners.detailCodec);
    } catch (error) {
      fail(error);
      return;
    }
    if (tracker.classifyTerminal(event) === "accept") {
      notify(() => listeners.terminal(event));
    }
  };

  const close = async (): Promise<void> => {
    stopped = true;
    await teardown();
  };

  try {
    await adopt(await transport.listen(BRIDGE_PROGRESS_EVENT, onProgress));
    await adopt(await transport.listen(BRIDGE_TERMINAL_EVENT, onTerminal));
    registered = true;
  } catch (error) {
    stopped = true;
    await teardown().catch(reportUnlistenFailure);
    throw error;
  }
  return close;
}
