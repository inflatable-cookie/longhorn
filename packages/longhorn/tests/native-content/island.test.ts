import { describe, expect, test } from "bun:test";

import {
  NativeContentClient,
  NativeContentIslandCorrelationError,
} from "../../src/native-content/index.ts";
import type { NativeContentPort } from "../../src/native-content/index.ts";
import { MemoryNativeContentHost, flush, nextUpdate } from "./support.ts";

const OTHER = "island:other";

interface Snapshot {
  cursor: { island_id: string };
  desired: { island_id: string };
}

/** Rewrites the state a reply carries so that it describes another island. */
function foreign(reply: unknown, part: "snapshot" | "event" = "snapshot"): unknown {
  const copy = structuredClone(reply) as {
    snapshot: Snapshot;
    event?: { cursor: { island_id: string } };
  };
  if (part === "snapshot") {
    copy.snapshot.cursor.island_id = OTHER;
    copy.snapshot.desired.island_id = OTHER;
  } else if (copy.event !== undefined) {
    copy.event.cursor.island_id = OTHER;
  }
  return copy;
}

const connect = (port: NativeContentPort) =>
  new NativeContentClient(port, "island:fixture").connect();

describe("native-content island correlation", () => {
  test("a connect reply for another island never becomes current", async () => {
    const direct = new MemoryNativeContentHost().port();
    const connection = connect({
      ...direct,
      connect: async (request) => foreign(await direct.connect(request)),
    });

    await expect(connection.ready).rejects.toBeInstanceOf(NativeContentIslandCorrelationError);
    expect(connection.current()).toBeUndefined();
    await connection.dispose();
  });

  test("a refresh reply for another island fails the connection and keeps the state", async () => {
    const host = new MemoryNativeContentHost();
    const direct = host.port();
    const connection = connect({
      ...direct,
      snapshot: async (request) => foreign(await direct.snapshot(request)),
    });
    const initial = await connection.ready;

    host.admitObservation();
    for (let attempt = 0; attempt < 100 && connection.failures().length === 0; attempt += 1) {
      await flush();
    }

    expect(connection.failures()).toEqual([
      { phase: "snapshot", error: expect.any(NativeContentIslandCorrelationError) },
    ]);
    expect(connection.current()).toEqual(initial);
    await connection.dispose();
  });

  test.each(["snapshot", "event"] as const)(
    "a mutation result with another island's %s is refused",
    async (part) => {
      const direct = new MemoryNativeContentHost().port();
      const connection = connect({
        ...direct,
        // Silent, so only the reply can move the connection's state.
        listen: () => () => {},
        updateDesired: async (request) => foreign(await direct.updateDesired(request), part),
        decideContentSize: async (request) =>
          foreign(await direct.decideContentSize(request), part),
      });
      const initial = await connection.ready;

      await expect(connection.updateDesired(nextUpdate(initial, 48))).rejects.toBeInstanceOf(
        NativeContentIslandCorrelationError,
      );
      await expect(
        connection.decideContentSize(
          {
            desired_revision: initial.cursor.desired_revision,
            generation: initial.cursor.attach_generation,
            size: { width: 640, height: 480 },
          },
          { kind: "accepted" },
        ),
      ).rejects.toBeInstanceOf(NativeContentIslandCorrelationError);
      expect(connection.current()).toEqual(initial);
      await connection.dispose();
    },
  );
});
