import { CheckedSnapshotConnection, CheckedSubscription } from "@inflatable-cookie/longhorn/core";
import type {
  ConnectionFailure,
  ConnectionFailureReporter,
  EventTransport,
} from "@inflatable-cookie/longhorn/core";

import type {
  SurfaceMutationRequest,
  SurfaceMutationResponse,
  SurfaceSnapshot,
} from "./generated/protocol.ts";
import {
  assertValidSurfaceChangedEvent,
  assertValidSurfaceMutationCommand,
  assertValidSurfaceMutationResponse,
  assertValidSurfaceSnapshot,
} from "./validation.ts";

export const SURFACE_SNAPSHOT_COMMAND = "longhorn_surfaces_snapshot";
export const SURFACE_MUTATE_COMMAND = "longhorn_surfaces_mutate";
export const SURFACE_CHANGED_EVENT = "longhorn://surfaces/changed";

export interface SurfaceSubscription {
  readonly ready: Promise<void>;
  current(): SurfaceSnapshot | undefined;
  failures(): ReadonlyArray<ConnectionFailure>;
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

  async mutate(request: SurfaceMutationRequest): Promise<SurfaceMutationResponse> {
    assertValidSurfaceMutationCommand(request.command);
    const value = await this.#transport.invoke(SURFACE_MUTATE_COMMAND, {
      request,
    });
    assertValidSurfaceMutationResponse(value);
    return value;
  }

  subscribe(
    listener: (snapshot: SurfaceSnapshot) => void,
    onFailure?: ConnectionFailureReporter,
  ): SurfaceSubscription {
    const transport = this.#transport;
    return new CheckedSubscription(
      new CheckedSnapshotConnection({
        listen: (receive) => transport.listen(SURFACE_CHANGED_EVENT, receive),
        loadSnapshot: () => this.snapshot(),
        validateSnapshot: parseSurfaceSnapshot,
        handleEvent: surfaceEventAction,
        isNewer: isNewerSurfaceSnapshot,
        onSnapshot: listener,
        onFailure,
      }),
      () => {},
    );
  }
}

function parseSurfaceSnapshot(value: unknown): SurfaceSnapshot {
  assertValidSurfaceSnapshot(value);
  return value;
}

function surfaceEventAction(
  value: unknown,
  current: SurfaceSnapshot | undefined,
): { kind: "ignore" } | { kind: "refresh" } {
  assertValidSurfaceChangedEvent(value);
  if (current === undefined) {
    return { kind: "refresh" };
  }

  const event = value;
  if (
    event.epoch < current.epoch ||
    (event.epoch === current.epoch && event.revision <= current.revision)
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
