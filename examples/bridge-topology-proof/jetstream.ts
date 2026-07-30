import {
  DirectBridgeStreamSource,
  SerializedLoopbackBridgeStreamSource,
  connectBridgeStream,
  type BridgeStreamSource,
} from "@longhorn/bridge/stream";
import {
  BRIDGE_DOMAIN_EVENT,
  TauriBridgeStreamSource,
} from "@longhorn/bridge/tauri-events";

import {
  declaration,
  queryParity,
  sameValues,
  unknownCodec,
  type AdapterName,
} from "./common.ts";

interface StreamHarness {
  readonly source: BridgeStreamSource;
  readonly emit: (value: unknown) => void;
  readonly order: string[];
  readonly secondLoad: Promise<void>;
}

export async function runJetstreamTrace() {
  const query = await queryParity("jetstream");
  const streams = Object.fromEntries(
    await Promise.all(
      (["direct", "tauri", "loopback"] as const).map(async (adapter) => [
        adapter,
        await streamTrace(adapter),
      ]),
    ),
  ) as Record<AdapterName, Awaited<ReturnType<typeof streamTrace>>>;
  return {
    listenerFirst: Object.values(streams).every(
      ({ order }) => order[0] === "listen" && order[1] === "snapshot",
    ),
    gapResync: Object.values(streams).every(
      ({ snapshotSequences }) =>
        JSON.stringify(snapshotSequences) === "[0,2]",
    ),
    adapterParity: sameValues(
      Object.fromEntries(
        Object.entries(streams).map(([adapter, trace]) => [
          adapter,
          {
            snapshotSequences: trace.snapshotSequences,
            finalValue: trace.finalValue,
            failures: trace.failures,
          },
        ]),
      ) as Record<AdapterName, unknown>,
    ),
    queryParity: sameValues(query.traces),
    streams,
  } as const;
}

async function streamTrace(adapter: AdapterName) {
  const harness = streamHarness(adapter);
  const snapshots: number[] = [];
  const connection = connectBridgeStream({
    sessionId: "session:fixture-jetstream",
    domainId: declaration("jetstream").domains[0]!.domainId,
    source: harness.source,
    snapshot: unknownCodec,
    event: unknownCodec,
    apply: (current) => current,
    onSnapshot: (snapshot) => snapshots.push(snapshot.cursor.sequence),
  });
  await connection.ready;
  harness.emit(event(2));
  await harness.secondLoad;
  await Promise.resolve();
  await Promise.resolve();
  const current = connection.current();
  const result = {
    order: harness.order,
    snapshotSequences: snapshots,
    finalValue: (
      current?.payload as { readonly value?: unknown } | undefined
    )?.value,
    failures: connection.failures().length,
  };
  await connection.dispose();
  return result;
}

function streamHarness(adapter: AdapterName): StreamHarness {
  const order: string[] = [];
  let loads = 0;
  let resolveSecond!: () => void;
  const secondLoad = new Promise<void>((resolve) => {
    resolveSecond = resolve;
  });
  const load = () => {
    order.push("snapshot");
    const value = snapshot(loads === 0 ? 0 : 2);
    loads += 1;
    if (loads === 2) {
      resolveSecond();
    }
    return value;
  };

  if (adapter === "tauri") {
    const listeners = new Map<string, (value: unknown) => void>();
    const transport = {
      invoke: () => Promise.resolve(load()),
      listen(eventName: string, listener: (value: unknown) => void) {
        order.push("listen");
        listeners.set(eventName, listener);
        return Promise.resolve(() => {
          listeners.delete(eventName);
        });
      },
    };
    return {
      source: new TauriBridgeStreamSource(
        "session:fixture-jetstream",
        declaration("jetstream").domains[0]!.domainId,
        transport,
      ),
      emit: (value) => listeners.get(BRIDGE_DOMAIN_EVENT)?.(value),
      order,
      secondLoad,
    };
  }

  let direct!: DirectBridgeStreamSource;
  direct = new DirectBridgeStreamSource(() => {
    if (direct.listenerCount() !== 1) {
      throw new Error("snapshot loaded before listener registration");
    }
    return load();
  });
  const source = adapter === "loopback"
    ? new SerializedLoopbackBridgeStreamSource(direct)
    : direct;
  const originalListen = source.listen.bind(source);
  const tracked: BridgeStreamSource = {
    listen(listener) {
      order.push("listen");
      return originalListen(listener);
    },
    loadSnapshot: () => source.loadSnapshot(),
  };
  return {
    source: tracked,
    emit: (value) => direct.emit(value),
    order,
    secondLoad,
  };
}

function snapshot(sequence: number) {
  return {
    cursor: {
      sessionId: "session:fixture-jetstream",
      domainId: declaration("jetstream").domains[0]!.domainId,
      authorityEpoch: 2,
      sequence,
    },
    payload: { value: sequence },
  };
}

function event(sequence: number) {
  return {
    cursor: {
      sessionId: "session:fixture-jetstream",
      domainId: declaration("jetstream").domains[0]!.domainId,
      authorityEpoch: 2,
      sequence,
    },
    payload: { delta: 2 },
  };
}
