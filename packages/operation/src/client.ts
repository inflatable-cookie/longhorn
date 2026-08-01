import {
  CheckedSnapshotConnection,
  type ConnectionFailure,
  type ConnectionFailureReporter,
} from "@longhorn/core";

import {
  assertCompatibleOperationCancellationCommand,
  assertCompatibleOperationCancellationResult,
  assertCompatibleOperationChangedEvent,
  assertCompatibleOperationMutationCommand,
  assertCompatibleOperationMutationResult,
  assertCompatibleOperationSnapshotResponse,
} from "./compatibility.ts";
import {
  OPERATION_PROTOCOL_VERSION,
  type OperationCancellationCommand,
  type OperationCancellationResult,
  type OperationChangedEvent,
  type OperationMutationCommand,
  type OperationMutationResult,
  type OperationSnapshot,
  type OperationSnapshotResponse,
} from "./generated/protocol.ts";
import type { OperationPort } from "./ports.ts";

export interface OperationSubscription {
  readonly ready: Promise<void>;
  current(): OperationSnapshot | undefined;
  failures(): readonly ConnectionFailure[];
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
    assertCompatibleOperationSnapshotResponse(value);
    assertResponseCorrelation(value.requestId, requestId);
    return value;
  }

  async mutate(command: OperationMutationCommand): Promise<OperationMutationResult> {
    assertCompatibleOperationMutationCommand(command);
    const value = await this.#port.mutate(command);
    assertCompatibleOperationMutationResult(value);
    assertResponseCorrelation(value.requestId, command.requestId);
    return value;
  }

  async cancel(command: OperationCancellationCommand): Promise<OperationCancellationResult> {
    assertCompatibleOperationCancellationCommand(command);
    const value = await this.#port.cancel(command);
    assertCompatibleOperationCancellationResult(value);
    assertResponseCorrelation(value.requestId, command.requestId);
    return value;
  }

  subscribe(
    listener: (snapshot: OperationSnapshot) => void,
    onFailure?: ConnectionFailureReporter,
  ): OperationSubscription {
    return new CheckedOperationSubscription(this, this.#port, listener, onFailure);
  }
}

export class OperationResponseCorrelationError extends Error {
  constructor(readonly expectedRequestId: string, readonly receivedRequestId: string) {
    super(`operation response correlation mismatch: expected ${expectedRequestId}; received ${receivedRequestId}`);
    this.name = "OperationResponseCorrelationError";
  }
}

function assertResponseCorrelation(received: string, expected: string): void {
  if (received !== expected) throw new OperationResponseCorrelationError(expected, received);
}

class CheckedOperationSubscription implements OperationSubscription {
  readonly ready: Promise<void>;
  readonly #connection: CheckedSnapshotConnection<OperationSnapshot>;

  constructor(
    client: OperationClient,
    port: OperationPort,
    listener: (snapshot: OperationSnapshot) => void,
    onFailure?: ConnectionFailureReporter,
  ) {
    this.#connection = new CheckedSnapshotConnection({
      listen: async (receive) => {
        if (port.listen === undefined) return () => {};
        return port.listen(receive);
      },
      loadSnapshot: async () => (await client.snapshot()).snapshot,
      validateSnapshot: parseSnapshot,
      handleEvent: operationEventAction,
      isNewer: isNewerOperationSnapshot,
      onSnapshot: listener,
      onFailure,
    });
    this.ready = this.#connection.ready.then(() => undefined);
  }

  current(): OperationSnapshot | undefined {
    return this.#connection.current();
  }

  failures(): readonly ConnectionFailure[] {
    return this.#connection.failures();
  }

  dispose(): Promise<void> {
    return this.#connection.dispose();
  }
}

function parseSnapshot(value: unknown): OperationSnapshot {
  const response = { requestId: "request:validation", snapshot: value };
  assertCompatibleOperationSnapshotResponse(response);
  return value as OperationSnapshot;
}

export function operationEventAction(
  value: unknown,
  current: OperationSnapshot | undefined,
): { kind: "ignore" } | { kind: "refresh" } {
  assertCompatibleOperationChangedEvent(value);
  const event = value as OperationChangedEvent;
  if (current === undefined) return { kind: "refresh" };
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
  if (current === undefined) return true;
  if (candidate.authority.authorityId !== current.authority.authorityId) return false;
  if (candidate.authority.authorityEpoch !== current.authority.authorityEpoch) {
    return candidate.authority.authorityEpoch > current.authority.authorityEpoch;
  }
  return candidate.catalogueRevision > current.catalogueRevision;
}
