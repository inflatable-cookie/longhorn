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
   * any payload is decoded. An identifier that reads as another request or job
   * makes the event foreign, whatever the other identifier is, so another job's
   * event is refused without its payload being decoded. An event is admitted
   * only when both identifiers read and match. One that names no other job and
   * has an unreadable identifier throws as malformed.
   */
  admits(value: unknown): boolean {
    if (!isRecord(value)) {
      incompatible("invalid_object", value);
    }
    const requestId = readOpaqueId(value.requestId);
    const jobId = readOpaqueId(value.jobId);
    if (
      (requestId !== undefined && requestId !== this.#requestId) ||
      (jobId !== undefined && jobId !== this.#jobId)
    ) {
      return false;
    }
    if (requestId === undefined || jobId === undefined) {
      incompatible("invalid_id", value);
    }
    return true;
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

/** The identifier, or `undefined` when it is not a readable opaque id. */
function readOpaqueId(value: unknown): string | undefined {
  try {
    return opaqueId(value);
  } catch {
    return undefined;
  }
}
