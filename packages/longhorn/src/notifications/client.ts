import { CheckedSnapshotConnection, type ConnectionFailure, type ConnectionFailureReporter } from "@inflatable-cookie/longhorn/core";

import {
  assertCompatibleNotificationChangedEvent,
  assertCompatibleNotificationMutationCommand,
  assertCompatibleNotificationMutationResult,
  assertCompatibleNotificationSnapshotResponse,
} from "./compatibility.ts";
import {
  NOTIFICATION_PROTOCOL_VERSION,
  type NotificationChangedEvent,
  type NotificationMutationCommand,
  type NotificationMutationResult,
  type NotificationSnapshot,
  type NotificationSnapshotResponse,
} from "./generated/protocol.ts";
import type { NotificationPort } from "./ports.ts";

export interface NotificationSubscription {
  readonly ready: Promise<void>;
  current(): NotificationSnapshot | undefined;
  failures(): readonly ConnectionFailure[];
  dispose(): Promise<void>;
}

export class NotificationClient {
  constructor(readonly port: NotificationPort) {}

  nextRequestId(): string { return this.port.nextRequestId(); }

  async snapshot(offset = 0, limit = 100): Promise<NotificationSnapshotResponse> {
    const requestId = this.port.nextRequestId();
    const value = await this.port.snapshot({ protocolVersion: NOTIFICATION_PROTOCOL_VERSION, requestId, offset, limit });
    assertCompatibleNotificationSnapshotResponse(value);
    assertCorrelation(value.requestId, requestId);
    return value;
  }

  async mutate(command: NotificationMutationCommand): Promise<NotificationMutationResult> {
    assertCompatibleNotificationMutationCommand(command);
    const value = await this.port.mutate(command);
    assertCompatibleNotificationMutationResult(value);
    assertCorrelation(value.requestId, command.requestId);
    return value;
  }

  subscribe(listener: (snapshot: NotificationSnapshot) => void, onFailure?: ConnectionFailureReporter, limit = 100): NotificationSubscription {
    return new CheckedNotificationSubscription(this, this.port, listener, onFailure, limit);
  }
}

export class NotificationResponseCorrelationError extends Error {
  constructor(readonly expectedRequestId: string, readonly receivedRequestId: string) {
    super(`notification response correlation mismatch: expected ${expectedRequestId}; received ${receivedRequestId}`);
    this.name = "NotificationResponseCorrelationError";
  }
}

class CheckedNotificationSubscription implements NotificationSubscription {
  readonly ready: Promise<void>;
  readonly connection: CheckedSnapshotConnection<NotificationSnapshot>;

  constructor(client: NotificationClient, port: NotificationPort, listener: (snapshot: NotificationSnapshot) => void, onFailure: ConnectionFailureReporter | undefined, limit: number) {
    this.connection = new CheckedSnapshotConnection({
      listen: async (receive) => port.listen === undefined ? () => {} : port.listen(receive),
      loadSnapshot: async () => (await client.snapshot(0, limit)).snapshot,
      validateSnapshot: parseSnapshot,
      handleEvent: notificationEventAction,
      isNewer: isNewerNotificationSnapshot,
      onSnapshot: listener,
      onFailure,
    });
    this.ready = this.connection.ready.then(() => undefined);
  }

  current(): NotificationSnapshot | undefined { return this.connection.current(); }
  failures(): readonly ConnectionFailure[] { return this.connection.failures(); }
  dispose(): Promise<void> { return this.connection.dispose(); }
}

function parseSnapshot(value: unknown): NotificationSnapshot {
  const response = { requestId: "request:validation", snapshot: value };
  assertCompatibleNotificationSnapshotResponse(response);
  return value as NotificationSnapshot;
}

export function notificationEventAction(value: unknown, current: NotificationSnapshot | undefined): { kind: "ignore" } | { kind: "refresh" } {
  assertCompatibleNotificationChangedEvent(value);
  const event = value as NotificationChangedEvent;
  if (current === undefined) return { kind: "refresh" };
  if (event.authority.authorityId !== current.authority.authorityId) return { kind: "ignore" };
  if (event.authority.authorityEpoch < current.authority.authorityEpoch) return { kind: "ignore" };
  if (event.authority.authorityEpoch === current.authority.authorityEpoch && event.committedLedgerRevision <= current.ledgerRevision) return { kind: "ignore" };
  return { kind: "refresh" };
}

export function isNewerNotificationSnapshot(candidate: NotificationSnapshot, current: NotificationSnapshot | undefined): boolean {
  if (current === undefined) return true;
  if (candidate.authority.authorityId !== current.authority.authorityId) return false;
  if (candidate.authority.authorityEpoch !== current.authority.authorityEpoch) return candidate.authority.authorityEpoch > current.authority.authorityEpoch;
  return candidate.ledgerRevision > current.ledgerRevision;
}

function assertCorrelation(received: string, expected: string): void {
  if (received !== expected) throw new NotificationResponseCorrelationError(expected, received);
}
