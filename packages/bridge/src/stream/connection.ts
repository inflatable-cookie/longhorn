import {
  CheckedSnapshotConnection,
  type ConnectionFailure,
  type ConnectionFailureReporter,
} from "@longhorn/core";

import type {
  BridgeEventEnvelope,
  BridgeSnapshotEnvelope,
  BridgeStreamDecision,
  DomainId,
  BridgeSessionId,
} from "../generated/protocol.ts";
import type { BridgeCodec } from "../compatibility/base.ts";
import {
  parseBridgeEventEnvelope,
  parseBridgeSnapshotEnvelope,
} from "../compatibility/streams.ts";
import { BridgeStreamTracker, newer } from "./tracker.ts";
import type { BridgeStreamSource } from "./source.ts";

export interface BridgeStreamConnectionOptions<Snapshot, Event> {
  readonly sessionId: BridgeSessionId;
  readonly domainId: DomainId;
  readonly source: BridgeStreamSource;
  readonly snapshot: BridgeCodec<Snapshot>;
  readonly event: BridgeCodec<Event>;
  readonly apply: (
    current: Snapshot,
    event: Event,
  ) => Snapshot;
  readonly onSnapshot?: (
    snapshot: BridgeSnapshotEnvelope<Snapshot>,
  ) => void;
  readonly onFailure?: ConnectionFailureReporter;
}

export class BridgeStreamConnection<Snapshot> {
  readonly ready: Promise<BridgeSnapshotEnvelope<Snapshot> | undefined>;
  readonly #connection: CheckedSnapshotConnection<
    BridgeSnapshotEnvelope<Snapshot>
  >;

  constructor(
    connection: CheckedSnapshotConnection<
      BridgeSnapshotEnvelope<Snapshot>
    >,
  ) {
    this.#connection = connection;
    this.ready = connection.ready;
  }

  current(): BridgeSnapshotEnvelope<Snapshot> | undefined {
    return this.#connection.current();
  }

  failures(): readonly ConnectionFailure[] {
    return this.#connection.failures();
  }

  dispose(): Promise<void> {
    return this.#connection.dispose();
  }
}

export function connectBridgeStream<Snapshot, Event>(
  options: BridgeStreamConnectionOptions<Snapshot, Event>,
): BridgeStreamConnection<Snapshot> {
  const tracker = new BridgeStreamTracker(
    options.sessionId,
    options.domainId,
  );
  const connection = new CheckedSnapshotConnection<
    BridgeSnapshotEnvelope<Snapshot>
  >({
    listen: (listener) => options.source.listen(listener),
    loadSnapshot: () => options.source.loadSnapshot(),
    validateSnapshot: (value) => {
      const snapshot = parseBridgeSnapshotEnvelope(value, options.snapshot);
      const decision = tracker.acceptSnapshot(snapshot.cursor);
      if (
        decision === "supersededSession" ||
        decision === "wrongDomain"
      ) {
        throw new BridgeStreamAuthorityError(decision);
      }
      return snapshot;
    },
    handleEvent: (value, current) => {
      const event = parseBridgeEventEnvelope(value, options.event);
      const decision = tracker.classifyEvent(event.cursor);
      return eventAction(decision, current, event, options.apply);
    },
    isNewer: (candidate, current) =>
      current === undefined || newer(candidate.cursor, current.cursor),
    onSnapshot: options.onSnapshot,
    onFailure: options.onFailure,
  });
  return new BridgeStreamConnection(connection);
}

export class BridgeStreamAuthorityError extends Error {
  readonly decision: "supersededSession" | "wrongDomain";

  constructor(decision: "supersededSession" | "wrongDomain") {
    super(`bridge stream snapshot rejected: ${decision}`);
    this.name = "BridgeStreamAuthorityError";
    this.decision = decision;
  }
}

function eventAction<Snapshot, Event>(
  decision: BridgeStreamDecision,
  current: BridgeSnapshotEnvelope<Snapshot> | undefined,
  event: BridgeEventEnvelope<Event>,
  apply: (current: Snapshot, event: Event) => Snapshot,
):
  | { readonly kind: "ignore" }
  | { readonly kind: "refresh" }
  | {
    readonly kind: "snapshot";
    readonly value: BridgeSnapshotEnvelope<Snapshot>;
  } {
  switch (decision) {
    case "apply":
      if (current === undefined) {
        return { kind: "refresh" };
      }
      return {
        kind: "snapshot",
        value: {
          cursor: event.cursor,
          payload: apply(current.payload, event.payload),
        },
      };
    case "resnapshotGap":
    case "resnapshotNewEpoch":
    case "resnapshotRequired":
      return { kind: "refresh" };
    case "ignoreDuplicate":
    case "ignoreStale":
    case "ignoreSupersededSession":
    case "ignoreWrongDomain":
      return { kind: "ignore" };
  }
}
