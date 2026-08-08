import { describe, expect, test } from "bun:test";

import {
  NativeContentClient,
  SerializedNativeContentPort,
} from "../../src/native-content/index.ts";
import {
  MemoryNativeContentHost,
  baseSnapshot,
  flush,
  nextUpdate,
} from "./support.ts";

async function trace(serialized: boolean) {
  const host = new MemoryNativeContentHost();
  const direct = host.port();
  const port = serialized ? new SerializedNativeContentPort(direct) : direct;
  const snapshots: Array<[number, number, number]> = [];
  const connection = new NativeContentClient(port, "island:fixture").connect(
    (snapshot) => {
      snapshots.push([
        snapshot.cursor.client_epoch,
        snapshot.cursor.desired_revision,
        snapshot.cursor.observed_revision,
      ]);
    },
  );
  const initial = await connection.ready;
  const updated = await connection.updateDesired(nextUpdate(initial, 32));
  host.admitObservation();
  await flush();
  const current = connection.current();
  if (current === undefined) throw new Error("missing current snapshot");
  const decision = await connection.decideContentSize(
    {
      generation: current.cursor.attach_generation,
      desired_revision: current.cursor.desired_revision,
      size: { width: 800, height: 600 },
    },
    { kind: "constrained", size: { width: 768, height: 576 } },
  );
  await connection.dispose();
  return {
    listenerFirst: host.calls.slice(0, 2),
    initial: initial.cursor,
    updateStatus: updated.status,
    decisionStatus: decision.status,
    current: connection.current()?.cursor,
    snapshots,
    unlistenCount: host.unlistenCount,
  };
}

describe("transport conformance", () => {
  test("direct and serialized loopback traces match", async () => {
    const direct = await trace(false);
    const serialized = await trace(true);
    expect(serialized).toEqual(direct);
    expect(direct.listenerFirst).toEqual(["listen", "connect"]);
    expect(direct.updateStatus).toBe("committed");
    expect(direct.decisionStatus).toBe("decided");
    expect(direct.current?.observed_revision).toBe(1);
    expect(direct.unlistenCount).toBe(1);
  });

  test("root package state stays JSON-safe", () => {
    const snapshot = baseSnapshot();
    expect(JSON.parse(JSON.stringify(snapshot))).toEqual(snapshot);
  });
});
