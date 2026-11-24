import { isRecord } from "../core/record.ts";
import type {
  BridgeJobId,
  BridgeJobTerminalDecision,
  BridgeJobTerminalEvent,
  BridgeProgressDecision,
  BridgeProgressEvent,
  BridgeRequestId,
} from "./generated/protocol.ts";
import { incompatible, opaqueId } from "./validation/base.ts";

/** The identifiers that tie a progress or terminal event to one job. */
interface BridgeJobCorrelation {
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

  /**
   * Whether a raw progress or terminal event belongs to this job, read before
   * any payload is decoded. The request id is checked first and a different
   * request ends the check, so another job's event is refused whatever its job
   * id or payload. Identifiers that cannot be read throw as malformed.
   */
  admits(value: unknown): boolean {
    if (!isRecord(value)) {
      incompatible("invalid_object", value);
    }
    if (opaqueId(value.requestId) !== this.#requestId) {
      return false;
    }
    return opaqueId(value.jobId) === this.#jobId;
  }

  #matches(event: BridgeJobCorrelation): boolean {
    return event.requestId === this.#requestId && event.jobId === this.#jobId;
  }

  classifyProgress<P>(event: BridgeProgressEvent<P>): BridgeProgressDecision {
    if (!this.#matches(event)) {
      return "ignoreWrongCorrelation";
    }
    return this.#terminal ? "ignoreAfterTerminal" : "accept";
  }

  classifyTerminal<S, D>(event: BridgeJobTerminalEvent<S, D>): BridgeJobTerminalDecision {
    if (!this.#matches(event)) {
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
