import {
  BridgeJobTracker,
  type BridgeCommandEnvelope,
  type BridgeCommandReply,
  type BridgeDomainClient,
  type BridgeJobId,
  type BridgeJobTerminalEvent,
  type BridgeOperationDescriptor,
  type BridgeProgressEvent,
  type BridgeQueryEnvelope,
  type BridgeQueryReply,
  type BridgeRequestContext,
  type BridgeRequestId,
  type DomainId,
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
  if (reply.requestId !== context.requestId || !("success" in reply.outcome)) {
    throw new Error("operation bridge query was rejected or miscorrelated");
  }
  return reply.outcome.success;
}

async function executeCommand(options: BridgeOperationPortOptions, command: OperationMutationCommand) {
  const context = nextContext(options);
  const descriptor = commandDescriptor(options.domainId);
  const reply = await options.client.execute(descriptor, {
    context,
    authorityEpoch: options.authorityEpoch,
    expectedRevision: command.kind === "register" || command.kind === "changeRetention" || command.kind === "teardown"
      ? command.expectedCatalogueRevision
      : null,
    idempotencyKey: context.requestId,
    payload: command,
  });
  if (reply.requestId !== context.requestId || !("applied" in reply.outcome)) {
    throw new Error("operation bridge command was rejected, indeterminate, or miscorrelated");
  }
  return reply.outcome.applied;
}

async function executeCancellation(options: BridgeOperationPortOptions, command: OperationCancellationCommand) {
  const context = nextContext(options);
  const descriptor = cancellationDescriptor(options.domainId);
  const reply = await options.client.execute(descriptor, { context, payload: command });
  if (reply.requestId !== context.requestId || !("success" in reply.outcome)) {
    throw new Error("operation bridge cancellation was rejected or miscorrelated");
  }
  return reply.outcome.success;
}

function nextContext(options: BridgeOperationPortOptions): BridgeRequestContext {
  return { requestId: options.nextBridgeRequestId(), sessionId: options.sessionId, domainId: options.domainId };
}

function codec<Value>(assertValue: (value: unknown) => void) {
  return { parse(value: unknown): Value { assertValue(value); return value as Value; } };
}

function queryDescriptor(domainId: DomainId): BridgeOperationDescriptor<BridgeQueryEnvelope<OperationSnapshotQuery>, BridgeQueryReply<OperationSnapshotResponse, null>> {
  return {
    route: "longhorn.operation.snapshot", domainId, kind: "query",
    requiredCapability: "longhorn.operation.read",
    request: codec((value) => {
      const payload = (value as BridgeQueryEnvelope<unknown>).payload;
      assertValidOperationSnapshotQuery(payload);
    }),
    reply: codec((value) => {
      const outcome = (value as BridgeQueryReply<unknown, null>).outcome;
      if ("success" in outcome) assertValidOperationSnapshotResponse(outcome.success);
    }),
  };
}

function commandDescriptor(domainId: DomainId): BridgeOperationDescriptor<BridgeCommandEnvelope<OperationMutationCommand>, BridgeCommandReply<OperationMutationResult, null>> {
  return {
    route: "longhorn.operation.mutate", domainId, kind: "command",
    requiredCapability: "longhorn.operation.manage",
    request: codec((value) => assertValidOperationMutationCommand((value as BridgeCommandEnvelope<unknown>).payload)),
    reply: codec((value) => {
      const outcome = (value as BridgeCommandReply<unknown, null>).outcome;
      if ("applied" in outcome) assertValidOperationMutationResult(outcome.applied);
    }),
  };
}

function cancellationDescriptor(domainId: DomainId): BridgeOperationDescriptor<BridgeQueryEnvelope<OperationCancellationCommand>, BridgeQueryReply<OperationCancellationResult, null>> {
  return {
    route: "longhorn.operation.cancel", domainId, kind: "cancellation",
    requiredCapability: "longhorn.operation.cancel",
    request: codec((value) => assertValidOperationCancellationCommand((value as BridgeQueryEnvelope<unknown>).payload)),
    reply: codec((value) => {
      const outcome = (value as BridgeQueryReply<unknown, null>).outcome;
      if ("success" in outcome) assertValidOperationCancellationResult(outcome.success);
    }),
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

  acceptProgress(event: BridgeProgressEvent<OperationChangedEvent>): OperationChangedEvent | undefined {
    if (this.#tracker.classifyProgress(event) !== "accept") return undefined;
    assertValidOperationChangedEvent(event.progress);
    return event.progress.operationId === this.#operationId ? event.progress : undefined;
  }

  acceptTerminal(event: BridgeJobTerminalEvent<OperationChangedEvent, null>): OperationChangedEvent | undefined {
    if (this.#tracker.classifyTerminal(event) !== "accept") return undefined;
    if (typeof event.outcome !== "object" || event.outcome === null || !("succeeded" in event.outcome)) return undefined;
    assertValidOperationChangedEvent(event.outcome.succeeded);
    return event.outcome.succeeded.operationId === this.#operationId ? event.outcome.succeeded : undefined;
  }
}
