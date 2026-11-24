import type {
  BridgeJobId,
  BridgeJobTerminalDecision,
  BridgeJobTerminalEvent,
  BridgeProgressDecision,
  BridgeProgressEvent,
  BridgeRequestId,
} from "./generated/protocol.ts";

/** The identifiers that tie a progress or terminal event to one job. */
export interface BridgeJobCorrelation {
  readonly requestId: BridgeRequestId;
  readonly jobId: BridgeJobId;
}

export class BridgeJobTracker {
  readonly #requestId: BridgeRequestId;
  readonly #jobId: BridgeJobId;
  #terminal = false;

  constructor(requestId: BridgeRequestId, jobId: BridgeJobId) {
    this.#requestId = requestId;
    this.#jobId = jobId;
  }

  correlates(event: BridgeJobCorrelation): boolean {
    return event.requestId === this.#requestId && event.jobId === this.#jobId;
  }

  classifyProgress<P>(event: BridgeProgressEvent<P>): BridgeProgressDecision {
    if (!this.correlates(event)) {
      return "ignoreWrongCorrelation";
    }
    return this.#terminal ? "ignoreAfterTerminal" : "accept";
  }

  classifyTerminal<S, D>(event: BridgeJobTerminalEvent<S, D>): BridgeJobTerminalDecision {
    if (!this.correlates(event)) {
      return "ignoreWrongCorrelation";
    }
    if (this.#terminal) {
      return "ignoreAlreadyTerminal";
    }
    this.#terminal = true;
    return "accept";
  }

  isTerminal(): boolean {
    return this.#terminal;
  }
}
