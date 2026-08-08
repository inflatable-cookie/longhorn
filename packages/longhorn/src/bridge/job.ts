import type {
  BridgeJobId,
  BridgeJobTerminalDecision,
  BridgeJobTerminalEvent,
  BridgeProgressDecision,
  BridgeProgressEvent,
  BridgeRequestId,
} from "./generated/protocol.ts";

export class BridgeJobTracker {
  readonly #requestId: BridgeRequestId;
  readonly #jobId: BridgeJobId;
  #terminal = false;

  constructor(requestId: BridgeRequestId, jobId: BridgeJobId) {
    this.#requestId = requestId;
    this.#jobId = jobId;
  }

  classifyProgress<P>(
    event: BridgeProgressEvent<P>,
  ): BridgeProgressDecision {
    if (
      event.requestId !== this.#requestId ||
      event.jobId !== this.#jobId
    ) {
      return "ignoreWrongCorrelation";
    }
    return this.#terminal ? "ignoreAfterTerminal" : "accept";
  }

  classifyTerminal<S, D>(
    event: BridgeJobTerminalEvent<S, D>,
  ): BridgeJobTerminalDecision {
    if (
      event.requestId !== this.#requestId ||
      event.jobId !== this.#jobId
    ) {
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
