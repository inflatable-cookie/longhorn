import { describe, expect, test } from "bun:test";

import {
  bridgeCodec,
  record,
  connectBridgeStream,
  BridgeStreamTracker,
  DirectBridgeStreamSource,
  SerializedLoopbackBridgeStreamSource,
} from "@inflatable-cookie/longhorn/bridge/stream";
import type {
  BridgeSnapshotEnvelope,
  BridgeStreamSource,
} from "@inflatable-cookie/longhorn/bridge/stream";

import { fixture } from "./support.ts";

interface Snapshot {
  readonly value: number;
}

interface Event {
  readonly delta: number;
}

const snapshotCodec = bridgeCodec<Snapshot>((value) => {
  const source = record(value, ["value"]);
  if (typeof source.value !== "number") {
    throw new TypeError("snapshot value must be a number");
  }
  return { value: source.value };
});

const eventCodec = bridgeCodec<Event>((value) => {
  const source = record(value, ["delta"]);
  if (typeof source.delta !== "number") {
    throw new TypeError("event delta must be a number");
  }
  return { delta: source.delta };
});

const settle = () =>
  new Promise<void>((resolve) => {
    setTimeout(resolve, 0);
  });

describe("checked bridge stream lifetime", () => {
  for (const mode of ["direct", "loopback"] as const) {
    test(`${mode} listener-first load cannot lose an intervening event`, async () => {
      const oldSnapshot = structuredClone(fixture.snapshot) as BridgeSnapshotEnvelope<Snapshot>;
      const event = {
        cursor: {
          ...oldSnapshot.cursor,
          sequence: oldSnapshot.cursor.sequence + 1,
        },
        payload: { delta: 1 },
      };
      const current = {
        cursor: event.cursor,
        payload: { value: oldSnapshot.payload.value + 1 },
      };
      let loads = 0;
      const direct = new DirectBridgeStreamSource(() => {
        loads += 1;
        if (loads === 1) {
          direct.emit(event);
          return oldSnapshot;
        }
        return current;
      });
      const source = mode === "direct" ? direct : new SerializedLoopbackBridgeStreamSource(direct);
      const connection = connectBridgeStream({
        sessionId: "session:fixture",
        domainId: "example.workspace",
        source,
        snapshot: snapshotCodec,
        event: eventCodec,
        apply: (snapshot, update) => ({
          value: snapshot.value + update.delta,
        }),
      });

      await connection.ready;
      expect(connection.current()).toEqual(current);
      expect(loads).toBe(2);
      await connection.dispose();
      expect(direct.listenerCount()).toBe(0);
    });
  }

  test("an older snapshot cannot move the accepted baseline back over a newer one", () => {
    const base = structuredClone(fixture.snapshot) as BridgeSnapshotEnvelope<Snapshot>;
    const start = base.cursor.sequence;
    const at = (sequence: number) => ({ ...base.cursor, sequence });
    const tracker = new BridgeStreamTracker(base.cursor.sessionId, base.cursor.domainId);

    expect(tracker.acceptSnapshot(at(start + 2))).toBe("accepted");
    expect(tracker.acceptSnapshot(at(start))).toBe("staleSnapshot");
    expect(tracker.acceptedCursor()?.sequence).toBe(start + 2);
    // An event already inside the baseline must not apply a second time.
    expect(tracker.classifyEvent(at(start + 1))).toBe("ignoreStale");
    expect(tracker.classifyEvent(at(start + 2))).toBe("ignoreDuplicate");
  });

  test("a stale resync reply after a newer event keeps the current snapshot and the stream live", async () => {
    const base = structuredClone(fixture.snapshot) as BridgeSnapshotEnvelope<Snapshot>;
    const start = base.cursor.sequence;
    const at = (sequence: number) => ({ ...base.cursor, sequence });
    const held: Array<(snapshot: BridgeSnapshotEnvelope<Snapshot>) => void> = [];
    let loads = 0;
    const direct = new DirectBridgeStreamSource(() => {
      loads += 1;
      if (loads === 1) {
        return { cursor: at(start), payload: { value: 0 } };
      }
      return new Promise<BridgeSnapshotEnvelope<Snapshot>>((resolve) => {
        held.push(resolve);
      });
    });
    const connection = connectBridgeStream({
      sessionId: "session:fixture",
      domainId: "example.workspace",
      source: direct,
      snapshot: snapshotCodec,
      event: eventCodec,
      apply: (snapshot, update) => ({ value: snapshot.value + update.delta }),
    });
    await connection.ready;

    // A gap starts a resync; an event that lands during it asks for another pass.
    direct.emit({ cursor: at(start + 2), payload: { delta: 2 } });
    await settle();
    expect(held).toHaveLength(1);
    direct.emit({ cursor: at(start + 3), payload: { delta: 3 } });
    held[0]?.({ cursor: at(start + 3), payload: { value: 3 } });
    await settle();
    expect(loads).toBe(3);
    expect(connection.current()?.payload.value).toBe(3);

    // The second pass starts clean, so an event applies before its reply lands.
    direct.emit({ cursor: at(start + 4), payload: { delta: 10 } });
    expect(connection.current()?.payload.value).toBe(13);
    held[1]?.({ cursor: at(start + 3), payload: { value: 3 } });
    await settle();

    expect(connection.current()?.payload.value).toBe(13);
    expect(connection.failures()).toHaveLength(0);
    expect(direct.listenerCount()).toBe(1);
    direct.emit({ cursor: at(start + 5), payload: { delta: 1 } });
    expect(connection.current()?.payload.value).toBe(14);
    await connection.dispose();
  });

  test("a stale resync reply cannot fail the initial subscription", async () => {
    const base = structuredClone(fixture.snapshot) as BridgeSnapshotEnvelope<Snapshot>;
    const start = base.cursor.sequence;
    const at = (sequence: number) => ({ ...base.cursor, sequence });
    const held: Array<(snapshot: BridgeSnapshotEnvelope<Snapshot>) => void> = [];
    const direct = new DirectBridgeStreamSource(
      () =>
        new Promise<BridgeSnapshotEnvelope<Snapshot>>((resolve) => {
          held.push(resolve);
        }),
    );
    const connection = connectBridgeStream({
      sessionId: "session:fixture",
      domainId: "example.workspace",
      source: direct,
      snapshot: snapshotCodec,
      event: eventCodec,
      apply: (snapshot, update) => ({ value: snapshot.value + update.delta }),
    });
    await settle();
    expect(held).toHaveLength(1);

    // An event during the initial load asks for a second pass before the first reply lands.
    direct.emit({ cursor: at(start + 1), payload: { delta: 1 } });
    held[0]?.({ cursor: at(start), payload: { value: 0 } });
    await settle();
    expect(held).toHaveLength(2);
    held[1]?.({ cursor: at(start + 1), payload: { value: 1 } });
    await expect(connection.ready).resolves.toBeDefined();
    expect(connection.current()?.payload.value).toBe(1);

    // A later gap starts a resync; an event during it asks for one more pass.
    direct.emit({ cursor: at(start + 5), payload: { delta: 0 } });
    await settle();
    expect(held).toHaveLength(3);
    direct.emit({ cursor: at(start + 6), payload: { delta: 6 } });
    held[2]?.({ cursor: at(start + 5), payload: { value: 5 } });
    await settle();
    expect(held).toHaveLength(4);

    // The older reply to the last pass resolves last and must not fail the stream.
    held[3]?.({ cursor: at(start + 1), payload: { value: 1 } });
    await settle();
    expect(connection.current()?.payload.value).toBe(5);
    expect(connection.failures()).toHaveLength(0);
    expect(direct.listenerCount()).toBe(1);
    await connection.dispose();
  });

  test("disposal during late registration removes the listener exactly once", async () => {
    let resolveRegistration: ((unlisten: () => void) => void) | undefined;
    let unlistenCalls = 0;
    const source: BridgeStreamSource = {
      listen: () =>
        new Promise((resolve) => {
          resolveRegistration = resolve;
        }),
      loadSnapshot: async () => fixture.snapshot,
    };
    const connection = connectBridgeStream({
      sessionId: "session:fixture",
      domainId: "example.workspace",
      source,
      snapshot: snapshotCodec,
      event: eventCodec,
      apply: (snapshot) => snapshot,
    });
    const disposal = connection.dispose();
    resolveRegistration?.(() => {
      unlistenCalls += 1;
    });
    await disposal;
    expect(unlistenCalls).toBe(1);
    expect(connection.current()).toBeUndefined();
  });
});
