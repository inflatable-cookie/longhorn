import { describe, expect, test } from "bun:test";

import {
  bridgeCodec,
  record,
  type BridgeSnapshotEnvelope,
  type BridgeStreamSource,
  connectBridgeStream,
  DirectBridgeStreamSource,
  SerializedLoopbackBridgeStreamSource,
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

describe("checked bridge stream lifetime", () => {
  for (const mode of ["direct", "loopback"] as const) {
    test(`${mode} listener-first load cannot lose an intervening event`, async () => {
      const oldSnapshot = structuredClone(fixture.snapshot) as
        BridgeSnapshotEnvelope<Snapshot>;
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
      let direct!: DirectBridgeStreamSource;
      direct = new DirectBridgeStreamSource(() => {
        loads += 1;
        if (loads === 1) {
          direct.emit(event);
          return oldSnapshot;
        }
        return current;
      });
      const source = mode === "direct"
        ? direct
        : new SerializedLoopbackBridgeStreamSource(direct);
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

  test("disposal during late registration removes the listener exactly once", async () => {
    let resolveRegistration:
      | ((unlisten: () => void) => void)
      | undefined;
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
