import {
  BridgeJobTracker,
  bridgeCodec,
  parseBridgeCommandEnvelope,
  parseBridgeCommandReply,
  parseBridgeJobTerminalEvent,
  parseBridgeQueryEnvelope,
  parseBridgeQueryReply,
} from "@inflatable-cookie/longhorn/bridge";
import type {
  BridgeCommandEnvelope,
  BridgeCommandReply,
  BridgeDomainClient,
  BridgeFailure,
  BridgeJobId,
  BridgeJobTerminalEvent,
  BridgeOperationDescriptor,
  BridgeProgressEvent,
  BridgeQueryEnvelope,
  BridgeQueryReply,
  BridgeRequestContext,
  BridgeRequestId,
  DomainId,
} from "@inflatable-cookie/longhorn/bridge";

import {
  assertValidOperationCancellationCommand,
  assertValidOperationCancellationResult,
  assertValidOperationChangedEvent,
  assertValidOperationMutationCommand,
  assertValidOperationMutationResult,
  assertValidOperationSnapshotQuery,
  assertValidOperationSnapshotResponse,
} from "./validation.ts";
import type {
  OperationCancellationCommand,
  OperationCancellationResult,
  OperationChangedEvent,
  OperationId,
  OperationMutationCommand,
  OperationMutationResult,
  OperationSnapshotQuery,
  OperationSnapshotResponse,
} from "./generated/protocol.ts";
import type { OperationPort } from "./ports.ts";

export interface BridgeOperationPortOptions {
  readonly client: BridgeDomainClient;
  readonly domainId: DomainId;
  readonly sessionId: string;
  readonly authorityEpoch: number;
  readonly nextBridgeRequestId: () => BridgeRequestId;
  readonly nextOperationRequestId: () => string;
}

export type BridgeOperationFailure =
  | {
      readonly kind: "rejected" | "indeterminate";
      readonly requestId: BridgeRequestId;
      readonly failure: BridgeFailure<null>;
    }
  | {
      readonly kind: "miscorrelated";
      readonly requestId: BridgeRequestId;
      readonly receivedRequestId: BridgeRequestId;
    }
  | {
      readonly kind: "failed";
      readonly requestId: BridgeRequestId;
      readonly jobId: BridgeJobId;
      readonly failure: BridgeFailure<null>;
    }
  | {
      readonly kind: "cancelled";
      readonly requestId: BridgeRequestId;
      readonly jobId: BridgeJobId;
    };

export class BridgeOperationError extends Error {
  constructor(readonly outcome: BridgeOperationFailure) {
    super(
      "failure" in outcome
        ? `operation bridge ${outcome.kind}: ${outcome.failure.message}`
        : `operation bridge ${outcome.kind}`,
    );
    this.name = "BridgeOperationError";
  }
}

export function createBridgeOperationPort(options: BridgeOperationPortOptions): OperationPort {
  return {
    snapshot: (query) => executeQuery(options, query),
    mutate: (command) => executeCommand(options, command),
    cancel: (command) => executeCancellation(options, command),
    nextRequestId: options.nextOperationRequestId,
  };
}

async function executeQuery(options: BridgeOperationPortOptions, query: OperationSnapshotQuery) {
  const context = nextContext(options);
  const descriptor = queryDescriptor(options.domainId);
  const reply = await options.client.execute(descriptor, { context, payload: query });
  return queryResult(context.requestId, reply);
}

async function executeCommand(
  options: BridgeOperationPortOptions,
  command: OperationMutationCommand,
) {
  const context = nextContext(options);
  const descriptor = commandDescriptor(options.domainId);
  const reply = await options.client.execute(descriptor, {
    context,
    authorityEpoch: options.authorityEpoch,
    expectedRevision:
      command.kind === "register" ||
      command.kind === "changeRetention" ||
      command.kind === "teardown"
        ? command.expectedCatalogueRevision
        : null,
    idempotencyKey: context.requestId,
    payload: command,
  });
  const { requestId } = context;
  checkCorrelation(requestId, reply.requestId);
  const { outcome } = reply;
  if ("rejected" in outcome) {
    throw new BridgeOperationError({ kind: "rejected", requestId, failure: outcome.rejected });
  }
  if ("indeterminate" in outcome) {
    throw new BridgeOperationError({
      kind: "indeterminate",
      requestId,
      failure: outcome.indeterminate,
    });
  }
  return outcome.applied;
}

async function executeCancellation(
  options: BridgeOperationPortOptions,
  command: OperationCancellationCommand,
) {
  const context = nextContext(options);
  const descriptor = cancellationDescriptor(options.domainId);
  const reply = await options.client.execute(descriptor, { context, payload: command });
  return queryResult(context.requestId, reply);
}

function queryResult<Result>(requestId: BridgeRequestId, reply: BridgeQueryReply<Result, null>) {
  checkCorrelation(requestId, reply.requestId);
  if ("rejected" in reply.outcome) {
    throw new BridgeOperationError({
      kind: "rejected",
      requestId,
      failure: reply.outcome.rejected,
    });
  }
  return reply.outcome.success;
}

function checkCorrelation(requestId: BridgeRequestId, receivedRequestId: BridgeRequestId): void {
  if (receivedRequestId !== requestId) {
    throw new BridgeOperationError({ kind: "miscorrelated", requestId, receivedRequestId });
  }
}

function nextContext(options: BridgeOperationPortOptions): BridgeRequestContext {
  return {
    requestId: options.nextBridgeRequestId(),
    sessionId: options.sessionId,
    domainId: options.domainId,
  };
}

function codec<Value>(assertValue: (value: unknown) => asserts value is Value) {
  return bridgeCodec((value) => {
    assertValue(value);
    return value;
  });
}

const noDetails = bridgeCodec((value): null => {
  if (value !== null) {
    throw new Error("operation bridge failure details must be null");
  }
  return null;
});

function queryDescriptor(
  domainId: DomainId,
): BridgeOperationDescriptor<
  BridgeQueryEnvelope<OperationSnapshotQuery>,
  BridgeQueryReply<OperationSnapshotResponse, null>
> {
  return {
    route: "longhorn.operation.snapshot",
    domainId,
    kind: "query",
    requiredCapability: "longhorn.operation.read",
    request: bridgeCodec((value) =>
      parseBridgeQueryEnvelope(value, codec(assertValidOperationSnapshotQuery)),
    ),
    reply: bridgeCodec((value) =>
      parseBridgeQueryReply(value, codec(assertValidOperationSnapshotResponse), noDetails),
    ),
  };
}

function commandDescriptor(
  domainId: DomainId,
): BridgeOperationDescriptor<
  BridgeCommandEnvelope<OperationMutationCommand>,
  BridgeCommandReply<OperationMutationResult, null>
> {
  return {
    route: "longhorn.operation.mutate",
    domainId,
    kind: "command",
    requiredCapability: "longhorn.operation.manage",
    request: bridgeCodec((value) =>
      parseBridgeCommandEnvelope(value, codec(assertValidOperationMutationCommand)),
    ),
    reply: bridgeCodec((value) =>
      parseBridgeCommandReply(value, codec(assertValidOperationMutationResult), noDetails),
    ),
  };
}

function cancellationDescriptor(
  domainId: DomainId,
): BridgeOperationDescriptor<
  BridgeQueryEnvelope<OperationCancellationCommand>,
  BridgeQueryReply<OperationCancellationResult, null>
> {
  return {
    route: "longhorn.operation.cancel",
    domainId,
    kind: "cancellation",
    requiredCapability: "longhorn.operation.cancel",
    request: bridgeCodec((value) =>
      parseBridgeQueryEnvelope(value, codec(assertValidOperationCancellationCommand)),
    ),
    reply: bridgeCodec((value) =>
      parseBridgeQueryReply(value, codec(assertValidOperationCancellationResult), noDetails),
    ),
  };
}

export interface BridgeOperationCorrelation {
  readonly bridgeRequestId: BridgeRequestId;
  readonly bridgeJobId: BridgeJobId;
  readonly operationId: OperationId;
}

export class BridgeOperationJobCorrelation {
  readonly #tracker: BridgeJobTracker;
  readonly #operationId: OperationId;

  constructor(correlation: BridgeOperationCorrelation) {
    this.#tracker = new BridgeJobTracker(correlation.bridgeRequestId, correlation.bridgeJobId);
    this.#operationId = correlation.operationId;
  }

  acceptProgress(
    event: BridgeProgressEvent<OperationChangedEvent>,
  ): OperationChangedEvent | undefined {
    if (this.#tracker.classifyProgress(event) !== "accept") {
      return undefined;
    }
    assertValidOperationChangedEvent(event.progress);
    return event.progress.operationId === this.#operationId ? event.progress : undefined;
  }

  /** Returns a correlated success or throws for a correlated failed or cancelled job. */
  acceptTerminal(
    event: BridgeJobTerminalEvent<OperationChangedEvent, null>,
  ): OperationChangedEvent | undefined {
    if (this.#tracker.classifyTerminal(event) !== "accept") {
      return undefined;
    }
    const { requestId, jobId, outcome } = parseBridgeJobTerminalEvent(
      event,
      codec(assertValidOperationChangedEvent),
      noDetails,
    );
    if (outcome === "cancelled") {
      throw new BridgeOperationError({ kind: "cancelled", requestId, jobId });
    }
    if ("failed" in outcome) {
      throw new BridgeOperationError({ kind: "failed", requestId, jobId, failure: outcome.failed });
    }
    return outcome.succeeded.operationId === this.#operationId ? outcome.succeeded : undefined;
  }
}
