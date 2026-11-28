import { CheckedSnapshotConnection, CheckedSubscription } from "@inflatable-cookie/longhorn/core";
import type { ConnectionFailureReporter } from "@inflatable-cookie/longhorn/core";

import type {
  BridgeEventEnvelope,
  BridgeSnapshotEnvelope,
  BridgeStreamDecision,
  DomainId,
  BridgeSessionId,
} from "../generated/protocol.ts";
import type { BridgeCodec } from "../validation/base.ts";
import { parseBridgeEventEnvelope, parseBridgeSnapshotEnvelope } from "../validation/streams.ts";
import { BridgeStreamTracker, newer } from "./tracker.ts";
import type { BridgeStreamSource } from "./source.ts";

export interface BridgeStreamConnectionOptions<Snapshot, Event> {
  readonly sessionId: BridgeSessionId;
  readonly domainId: DomainId;
  readonly source: BridgeStreamSource;
  readonly snapshot: BridgeCodec<Snapshot>;
  readonly event: BridgeCodec<Event>;
  readonly apply: (current: Snapshot, event: Event) => Snapshot;
  readonly onSnapshot?: (snapshot: BridgeSnapshotEnvelope<Snapshot>) => void;
  readonly onFailure?: ConnectionFailureReporter;
}

export class BridgeStreamConnection<Snapshot> extends CheckedSubscription<
  BridgeSnapshotEnvelope<Snapshot>,
  BridgeSnapshotEnvelope<Snapshot> | undefined
> {
  constructor(connection: CheckedSnapshotConnection<BridgeSnapshotEnvelope<Snapshot>>) {
    super(connection, (snapshot) => snapshot);
  }
}

export function connectBridgeStream<Snapshot, Event>(
  options: BridgeStreamConnectionOptions<Snapshot, Event>,
): BridgeStreamConnection<Snapshot> {
  const tracker = new BridgeStreamTracker(options.sessionId, options.domainId);
  const connection = new CheckedSnapshotConnection<BridgeSnapshotEnvelope<Snapshot>>({
    listen: (listener) => options.source.listen(listener),
    loadSnapshot: () => options.source.loadSnapshot(),
    validateSnapshot: (value) => {
      const snapshot = parseBridgeSnapshotEnvelope(value, options.snapshot);
      const decision = tracker.acceptSnapshot(snapshot.cursor);
      if (decision === "supersededSession" || decision === "wrongDomain") {
        throw new BridgeStreamAuthorityError(decision);
      }
      // The reply is behind an event already seen, and the events in between
      // were not applied. Nothing else will ask again if the stream goes quiet.
      if (decision === "acceptedResnapshotRequired") {
        connection.requestRefresh();
      }
      // A stale reply is returned unchanged. The tracker kept its baseline, and
      // the connection's isNewer check drops the reply, so the stream stays live.
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
