import {
  CheckedSnapshotConnection,
  type ConnectionFailure,
  type ConnectionFailureReporter,
  type EventTransport,
} from "@longhorn/core";

import type {
  SurfaceChangedEvent,
  SurfaceMutationRequest,
  SurfaceMutationResponse,
  SurfaceSnapshot,
} from "./generated/protocol.ts";
import {
  assertCompatibleSurfaceChangedEvent,
  assertCompatibleSurfaceMutationCommand,
  assertCompatibleSurfaceMutationResponse,
  assertCompatibleSurfaceSnapshot,
} from "./compatibility.ts";

export const SURFACE_SNAPSHOT_COMMAND = "longhorn_surfaces_snapshot";
export const SURFACE_MUTATE_COMMAND = "longhorn_surfaces_mutate";
export const SURFACE_CHANGED_EVENT = "longhorn://surfaces/changed";

export interface SurfaceSubscription {
  readonly ready: Promise<void>;
  current(): SurfaceSnapshot | undefined;
  failures(): readonly ConnectionFailure[];
  dispose(): Promise<void>;
}

export class SurfaceClient {
  readonly #transport: EventTransport;

  constructor(transport: EventTransport) {
    this.#transport = transport;
  }

  async snapshot(): Promise<SurfaceSnapshot> {
    const value = await this.#transport.invoke(SURFACE_SNAPSHOT_COMMAND, {});
    return parseSurfaceSnapshot(value);
  }

  async mutate(
    request: SurfaceMutationRequest,
  ): Promise<SurfaceMutationResponse> {
    assertCompatibleSurfaceMutationCommand(request.command);
    const value = await this.#transport.invoke(SURFACE_MUTATE_COMMAND, {
      request,
    });
    assertCompatibleSurfaceMutationResponse(value);
    return value;
  }

  subscribe(
    listener: (snapshot: SurfaceSnapshot) => void,
    onFailure?: ConnectionFailureReporter,
  ): SurfaceSubscription {
    return new SurfaceSnapshotSubscription(
      this,
      this.#transport,
      listener,
      onFailure,
    );
  }
}

class SurfaceSnapshotSubscription implements SurfaceSubscription {
  readonly ready: Promise<void>;
  readonly #connection: CheckedSnapshotConnection<SurfaceSnapshot>;

  constructor(
    client: SurfaceClient,
    transport: EventTransport,
    listener: (snapshot: SurfaceSnapshot) => void,
    onFailure?: ConnectionFailureReporter,
  ) {
    this.#connection = new CheckedSnapshotConnection({
      listen: (receive) =>
        transport.listen(SURFACE_CHANGED_EVENT, receive),
      loadSnapshot: () => client.snapshot(),
      validateSnapshot: parseSurfaceSnapshot,
      handleEvent: surfaceEventAction,
      isNewer: isNewerSurfaceSnapshot,
      onSnapshot: listener,
      onFailure,
    });
    this.ready = this.#connection.ready.then(() => undefined);
  }

  current(): SurfaceSnapshot | undefined {
    return this.#connection.current();
  }

  failures(): readonly ConnectionFailure[] {
    return this.#connection.failures();
  }

  dispose(): Promise<void> {
    return this.#connection.dispose();
  }
}

function parseSurfaceSnapshot(value: unknown): SurfaceSnapshot {
  assertCompatibleSurfaceSnapshot(value);
  return value;
}

function surfaceEventAction(
  value: unknown,
  current: SurfaceSnapshot | undefined,
): { kind: "ignore" } | { kind: "refresh" } {
  assertCompatibleSurfaceChangedEvent(value);
  if (current === undefined) {
    return { kind: "refresh" };
  }

  const event = value as SurfaceChangedEvent;
  if (
    event.epoch < current.epoch ||
    (event.epoch === current.epoch &&
      event.revision <= current.revision)
  ) {
    return { kind: "ignore" };
  }
  return { kind: "refresh" };
}

function isNewerSurfaceSnapshot(
  candidate: SurfaceSnapshot,
  current: SurfaceSnapshot | undefined,
): boolean {
  if (current === undefined) {
    return true;
  }
  if (candidate.epoch !== current.epoch) {
    return candidate.epoch > current.epoch;
  }
  return candidate.revision > current.revision;
}
