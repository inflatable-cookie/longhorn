import {
  DirectBridgeStreamSource,
  SerializedLoopbackBridgeStreamSource,
  connectBridgeStream,
} from "@inflatable-cookie/longhorn/bridge/stream";
import type { BridgeStreamSource } from "@inflatable-cookie/longhorn/bridge/stream";
import {
  BRIDGE_DOMAIN_EVENT,
  TauriBridgeStreamSource,
} from "@inflatable-cookie/longhorn-tauri/bridge-events";

import { declaration, declaredAt, queryParity, sameValues, unknownCodec } from "./common.ts";
import type { AdapterName } from "./common.ts";

interface StreamHarness {
  readonly source: BridgeStreamSource;
  readonly emit: (value: unknown) => void;
  readonly order: Array<string>;
  readonly secondLoad: Promise<void>;
}

type StreamTrace = Awaited<ReturnType<typeof streamTrace>>;

export async function runOrderedStreamsTrace() {
  const query = await queryParity("ordered-streams");
  const [direct, tauri, loopback] = await Promise.all([
    streamTrace("direct"),
    streamTrace("tauri"),
    streamTrace("loopback"),
  ]);
  const streams = { direct, tauri, loopback } satisfies Record<AdapterName, StreamTrace>;
  return {
    listenerFirst: Object.values(streams).every(
      ({ order }) => order[0] === "listen" && order[1] === "snapshot",
    ),
    gapResync: Object.values(streams).every(
      ({ snapshotSequences }) => JSON.stringify(snapshotSequences) === "[0,2]",
    ),
    adapterParity: sameValues({
      direct: parityFields(streams.direct),
      tauri: parityFields(streams.tauri),
      loopback: parityFields(streams.loopback),
    }),
    queryParity: sameValues(query.traces),
    streams,
  } as const;
}

function parityFields({ snapshotSequences, finalValue, failures }: StreamTrace) {
  return { snapshotSequences, finalValue, failures };
}

// The value is whatever the stream carried; sameValues compares it as JSON, so it stays unknown.
// oxlint-disable-next-line quality/no-unknown-returns -- see above.
function payloadValue(payload: unknown): unknown {
  return typeof payload === "object" && payload !== null && "value" in payload
    ? payload.value
    : undefined;
}

async function streamTrace(adapter: AdapterName) {
  const harness = streamHarness(adapter);
  const snapshots: Array<number> = [];
  const connection = connectBridgeStream({
    sessionId: "session:fixture-ordered-streams",
    domainId: declaredAt(declaration("ordered-streams").domains, 0, "domain").domainId,
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
    finalValue: payloadValue(current?.payload),
    failures: connection.failures().length,
  };
  await connection.dispose();
  return result;
}

function streamHarness(adapter: AdapterName): StreamHarness {
  const order: Array<string> = [];
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
        "session:fixture-ordered-streams",
        declaredAt(declaration("ordered-streams").domains, 0, "domain").domainId,
        transport,
      ),
      emit: (value) => listeners.get(BRIDGE_DOMAIN_EVENT)?.(value),
      order,
      secondLoad,
    };
  }

  const direct: DirectBridgeStreamSource = new DirectBridgeStreamSource(() => {
    if (direct.listenerCount() !== 1) {
      throw new Error("snapshot loaded before listener registration");
    }
    return load();
  });
  const source = adapter === "loopback" ? new SerializedLoopbackBridgeStreamSource(direct) : direct;
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
      sessionId: "session:fixture-ordered-streams",
      domainId: declaredAt(declaration("ordered-streams").domains, 0, "domain").domainId,
      authorityEpoch: 2,
      sequence,
    },
    payload: { value: sequence },
  };
}

function event(sequence: number) {
  return {
    cursor: {
      sessionId: "session:fixture-ordered-streams",
      domainId: declaredAt(declaration("ordered-streams").domains, 0, "domain").domainId,
      authorityEpoch: 2,
      sequence,
    },
    payload: { delta: 2 },
  };
}
