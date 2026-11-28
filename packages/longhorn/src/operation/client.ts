import { CheckedSnapshotConnection, CheckedSubscription } from "@inflatable-cookie/longhorn/core";
import type {
  ConnectionFailure,
  ConnectionFailureReporter,
} from "@inflatable-cookie/longhorn/core";

import {
  assertValidOperationCancellationCommand,
  assertValidOperationCancellationResult,
  assertValidOperationChangedEvent,
  assertValidOperationMutationCommand,
  assertValidOperationMutationResult,
  assertValidOperationSnapshot,
  assertValidOperationSnapshotResponse,
} from "./validation.ts";
import { OPERATION_PROTOCOL_VERSION } from "./generated/protocol.ts";
import type {
  OperationCancellationCommand,
  OperationCancellationResult,
  OperationMutationCommand,
  OperationMutationResult,
  OperationSnapshot,
  OperationSnapshotResponse,
} from "./generated/protocol.ts";
import type { OperationPort } from "./ports.ts";

export interface OperationSubscription {
  readonly ready: Promise<void>;
  current(): OperationSnapshot | undefined;
  failures(): ReadonlyArray<ConnectionFailure>;
  dispose(): Promise<void>;
}

export class OperationClient {
  readonly #port: OperationPort;

  constructor(port: OperationPort) {
    this.#port = port;
  }

  nextRequestId(): string {
    return this.#port.nextRequestId();
  }

  async snapshot(): Promise<OperationSnapshotResponse> {
    const requestId = this.#port.nextRequestId();
    const value = await this.#port.snapshot({
      protocolVersion: OPERATION_PROTOCOL_VERSION,
      requestId,
    });
    assertValidOperationSnapshotResponse(value);
    assertResponseCorrelation(value.requestId, requestId);
    return value;
  }

  async mutate(command: OperationMutationCommand): Promise<OperationMutationResult> {
    assertValidOperationMutationCommand(command);
    const value = await this.#port.mutate(command);
    assertValidOperationMutationResult(value);
    assertResponseCorrelation(value.requestId, command.requestId);
    return value;
  }

  async cancel(command: OperationCancellationCommand): Promise<OperationCancellationResult> {
    assertValidOperationCancellationCommand(command);
    const value = await this.#port.cancel(command);
    assertValidOperationCancellationResult(value);
    assertResponseCorrelation(value.requestId, command.requestId);
    return value;
  }

  subscribe(
    listener: (snapshot: OperationSnapshot) => void,
    onFailure?: ConnectionFailureReporter,
  ): OperationSubscription {
    const port = this.#port;
    return new CheckedSubscription(
      new CheckedSnapshotConnection({
        listen: async (receive) => (port.listen === undefined ? () => {} : port.listen(receive)),
        loadSnapshot: async () => (await this.snapshot()).snapshot,
        validateSnapshot: parseSnapshot,
        handleEvent: operationEventAction,
        isNewer: isNewerOperationSnapshot,
        onSnapshot: listener,
        onFailure,
      }),
      () => {},
    );
  }
}

export class OperationResponseCorrelationError extends Error {
  constructor(
    readonly expectedRequestId: string,
    readonly receivedRequestId: string,
  ) {
    super(
      `operation response correlation mismatch: expected ${expectedRequestId}; received ${receivedRequestId}`,
    );
    this.name = "OperationResponseCorrelationError";
  }
}

function assertResponseCorrelation(received: string, expected: string): void {
  if (received !== expected) {
    throw new OperationResponseCorrelationError(expected, received);
  }
}

function parseSnapshot(value: unknown): OperationSnapshot {
  assertValidOperationSnapshot(value);
  return value;
}

export function operationEventAction(
  value: unknown,
  current: OperationSnapshot | undefined,
): { kind: "ignore" } | { kind: "refresh" } {
  assertValidOperationChangedEvent(value);
  const event = value;
  if (current === undefined) {
    return { kind: "refresh" };
  }
  if (event.authority.authorityId !== current.authority.authorityId) {
    return { kind: "ignore" };
  }
  if (event.authority.authorityEpoch < current.authority.authorityEpoch) {
    return { kind: "ignore" };
  }
  if (
    event.authority.authorityEpoch === current.authority.authorityEpoch &&
    event.committedCatalogueRevision <= current.catalogueRevision
  ) {
    return { kind: "ignore" };
  }
  return { kind: "refresh" };
}

export function isNewerOperationSnapshot(
  candidate: OperationSnapshot,
  current: OperationSnapshot | undefined,
): boolean {
  if (current === undefined) {
    return true;
  }
  if (candidate.authority.authorityId !== current.authority.authorityId) {
    return false;
  }
  if (candidate.authority.authorityEpoch !== current.authority.authorityEpoch) {
    return candidate.authority.authorityEpoch > current.authority.authorityEpoch;
  }
  return candidate.catalogueRevision > current.catalogueRevision;
}
