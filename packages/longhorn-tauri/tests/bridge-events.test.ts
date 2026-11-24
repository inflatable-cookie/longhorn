import { describe, expect, test } from "bun:test";

import type { ConnectionFailure, EventTransport, Unlisten } from "@inflatable-cookie/longhorn/core";
import { bridgeCodec } from "@inflatable-cookie/longhorn/bridge";
import {
  connectBridgeStream,
  DirectBridgeStreamSource,
  parseBridgeSnapshotEnvelope,
} from "@inflatable-cookie/longhorn/bridge/stream";
import {
  BRIDGE_DOMAIN_EVENT,
  BRIDGE_PROGRESS_EVENT,
  BRIDGE_TERMINAL_EVENT,
  listenTauriBridgeJob,
  TauriBridgeStreamSource,
} from "@inflatable-cookie/longhorn-tauri/bridge-events";

import { fixture, jsonCodec, values } from "../../longhorn/tests/bridge/support.ts";

const sessionId = "session:fixture";
const domainId = "example.workspace";

class RecordingEventTransport implements EventTransport {
  readonly order: Array<string> = [];
  readonly listeners = new Map<string, (payload: unknown) => void>();

  invoke(command: string): Promise<unknown> {
    this.order.push(command);
    return Promise.resolve(fixture.snapshot);
  }

  listen(event: string, listener: (payload: unknown) => void): Promise<Unlisten> {
    this.order.push(event);
    this.listeners.set(event, listener);
    return Promise.resolve(() => {
      this.listeners.delete(event);
    });
  }

  emit(event: string, payload: unknown): void {
    this.listeners.get(event)?.(payload);
  }
}

test("subscription composition listens before requesting resync", async () => {
  const transport = new RecordingEventTransport();
  const source = new TauriBridgeStreamSource(sessionId, domainId, transport);
  const connection = connectBridgeStream({
    sessionId,
    domainId,
    source,
    snapshot: jsonCodec,
    event: jsonCodec,
    apply: (current) => current,
  });

  await connection.ready;
  expect(transport.order).toEqual([BRIDGE_DOMAIN_EVENT, "longhorn_bridge_resync"]);
  await connection.dispose();
});

const streamRoutes = [
  {
    name: "direct",
    open() {
      const source = new DirectBridgeStreamSource(() => fixture.snapshot);
      return {
        source,
        emit: (payload: unknown) => source.emit(payload),
        listenerCount: () => source.listenerCount(),
      };
    },
  },
  {
    name: "Tauri-backed",
    open() {
      const transport = new RecordingEventTransport();
      return {
        source: new TauriBridgeStreamSource(sessionId, domainId, transport),
        emit: (payload: unknown) => transport.emit(BRIDGE_DOMAIN_EVENT, payload),
        listenerCount: () => transport.listeners.size,
      };
    },
  },
];

describe.each(streamRoutes)("$name stream subscription conformance", ({ open }) => {
  const initial = parseBridgeSnapshotEnvelope(fixture.snapshot, jsonCodec);
  const update = parseBridgeSnapshotEnvelope(values(fixture.events)[0], jsonCodec);
  type Snapshot = typeof initial;

  function subscribe(onSnapshot: (snapshot: Snapshot) => void) {
    const route = open();
    const failures: Array<ConnectionFailure> = [];
    const connection = connectBridgeStream({
      sessionId,
      domainId,
      source: route.source,
      snapshot: jsonCodec,
      event: jsonCodec,
      apply: (_current, payload) => payload,
      onSnapshot,
      onFailure: (failure) => failures.push(failure),
    });
    return { ...route, connection, failures };
  }

  test("delivers valid snapshots and releases the listener on disposal", async () => {
    const observed: Array<Snapshot> = [];
    const { connection, emit, listenerCount, failures } = subscribe((value) =>
      observed.push(value),
    );

    expect(await connection.ready).toEqual(initial);
    emit(update);
    expect(connection.current()).toEqual(update);
    expect(observed).toEqual([initial, update]);
    expect(failures).toEqual([]);
    await connection.dispose();
    expect(listenerCount()).toBe(0);
  });

  test("reports malformed data and stops delivery", async () => {
    const observed: Array<Snapshot> = [];
    const { connection, emit, listenerCount, failures } = subscribe((value) =>
      observed.push(value),
    );

    await connection.ready;
    emit(null);
    emit(update);
    await tick();
    expect(observed).toEqual([initial]);
    expect(failures).toEqual([{ phase: "event", error: expect.any(Error) }]);
    expect(listenerCount()).toBe(0);
  });

  test("a throwing initial callback leaves readiness and the listener intact", async () => {
    const observed: Array<Snapshot> = [];
    const { connection, emit, listenerCount, failures } = subscribe((value) => {
      observed.push(value);
      if (value.cursor.sequence === initial.cursor.sequence) {
        throw new Error("initial callback failed");
      }
    });

    expect(await connection.ready).toEqual(initial);
    emit(update);
    expect(observed).toEqual([initial, update]);
    expect(failures).toEqual([]);
    expect(listenerCount()).toBe(1);
    await connection.dispose();
  });

  test("a throwing event callback leaves the accepted snapshot and listener intact", async () => {
    const callbackError = new Error("event callback failed");
    const { connection, emit, listenerCount, failures } = subscribe((value) => {
      if (value.cursor.sequence === update.cursor.sequence) {
        throw callbackError;
      }
    });

    await connection.ready;
    expect(() => emit(update)).toThrow(callbackError);
    expect(connection.current()).toEqual(update);
    const next = { ...update, cursor: { ...update.cursor, sequence: update.cursor.sequence + 1 } };
    emit(next);
    expect(connection.current()).toEqual(next);
    expect(failures).toEqual([]);
    expect(listenerCount()).toBe(1);
    await connection.dispose();
  });
});

test("job listeners admit only matching correlation and one terminal", async () => {
  const transport = new RecordingEventTransport();
  const progress: Array<unknown> = [];
  const terminal: Array<unknown> = [];
  const dispose = await listenTauriBridgeJob(
    "request:scan",
    "job:scan",
    {
      progressCodec: jsonCodec,
      successCodec: jsonCodec,
      detailCodec: jsonCodec,
      progress: (event) => progress.push(event),
      terminal: (event) => terminal.push(event),
    },
    transport,
  );

  transport.emit(BRIDGE_PROGRESS_EVENT, {
    requestId: "request:other",
    jobId: "job:scan",
    progress: 10,
  });
  transport.emit(BRIDGE_PROGRESS_EVENT, {
    requestId: "request:scan",
    jobId: "job:scan",
    progress: 20,
  });
  const terminalEvent = {
    requestId: "request:scan",
    jobId: "job:scan",
    outcome: { succeeded: { value: 3 } },
  };
  transport.emit(BRIDGE_TERMINAL_EVENT, terminalEvent);
  transport.emit(BRIDGE_TERMINAL_EVENT, terminalEvent);
  transport.emit(BRIDGE_PROGRESS_EVENT, {
    requestId: "request:scan",
    jobId: "job:scan",
    progress: 30,
  });

  expect(progress).toHaveLength(1);
  expect(terminal).toEqual([terminalEvent]);
  await dispose();
  expect(transport.listeners.size).toBe(0);
});

function tick(): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
}

// A channel that fans every event out to all of its listeners, as Tauri does
// for jobs that share an event name.
class SharedChannelTransport implements EventTransport {
  readonly listeners = new Map<string, Set<(payload: unknown) => void>>();

  invoke(): Promise<unknown> {
    return Promise.resolve(undefined);
  }

  listen(event: string, listener: (payload: unknown) => void): Promise<Unlisten> {
    const set = this.listeners.get(event) ?? new Set<(payload: unknown) => void>();
    set.add(listener);
    this.listeners.set(event, set);
    return Promise.resolve(() => {
      set.delete(listener);
    });
  }

  emit(event: string, payload: unknown): void {
    for (const listener of this.listeners.get(event) ?? []) {
      listener(payload);
    }
  }

  count(): number {
    return [...this.listeners.values()].reduce((total, set) => total + set.size, 0);
  }
}

// Two incompatible job shapes on one channel: the first job's codecs accept
// only a plain number, the second's only an object.
const numberCodec = bridgeCodec<number>((value) => {
  if (typeof value !== "number") {
    throw new TypeError("expected a number");
  }
  return value;
});

const objectCodec = bridgeCodec<Record<string, unknown>>((value) => {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError("expected an object");
  }
  return value as Record<string, unknown>;
});

test("two active jobs with incompatible shapes never fail each other", async () => {
  const transport = new SharedChannelTransport();
  const failures: Array<ConnectionFailure> = [];
  const numeric: Array<unknown> = [];
  const numericTerminal: Array<unknown> = [];
  const shaped: Array<unknown> = [];
  const shapedTerminal: Array<unknown> = [];
  const numericJob = await listenTauriBridgeJob(
    "request:count",
    "job:count",
    {
      progressCodec: numberCodec,
      successCodec: numberCodec,
      detailCodec: numberCodec,
      progress: (event) => numeric.push(event.progress),
      terminal: (event) => numericTerminal.push(event),
      onFailure: (failure) => failures.push(failure),
    },
    transport,
  );
  const shapedJob = await listenTauriBridgeJob(
    "request:shape",
    "job:shape",
    {
      progressCodec: objectCodec,
      successCodec: objectCodec,
      detailCodec: numberCodec,
      progress: (event) => shaped.push(event.progress),
      terminal: (event) => shapedTerminal.push(event),
      onFailure: (failure) => failures.push(failure),
    },
    transport,
  );

  transport.emit(BRIDGE_PROGRESS_EVENT, {
    requestId: "request:shape",
    jobId: "job:shape",
    progress: { percent: 10 },
  });
  transport.emit(BRIDGE_PROGRESS_EVENT, {
    requestId: "request:count",
    jobId: "job:count",
    progress: 20,
  });
  transport.emit(BRIDGE_TERMINAL_EVENT, {
    requestId: "request:count",
    jobId: "job:count",
    outcome: { succeeded: 3 },
  });
  transport.emit(BRIDGE_TERMINAL_EVENT, {
    requestId: "request:shape",
    jobId: "job:shape",
    outcome: { succeeded: { rows: [1, 2] } },
  });
  transport.emit(BRIDGE_TERMINAL_EVENT, {
    requestId: "request:shape",
    jobId: "job:shape",
    outcome: {
      failed: {
        code: "shape.failed",
        message: "shape job failed",
        retryClass: "never",
        phase: "execution",
        details: { reason: "not-a-number" },
      },
    },
  });
  await tick();

  expect(failures).toEqual([]);
  expect(numeric).toEqual([20]);
  expect(numericTerminal).toEqual([
    { requestId: "request:count", jobId: "job:count", outcome: { succeeded: 3 } },
  ]);
  expect(shaped).toEqual([{ percent: 10 }]);
  expect(shapedTerminal).toEqual([
    { requestId: "request:shape", jobId: "job:shape", outcome: { succeeded: { rows: [1, 2] } } },
  ]);
  // Four listeners stay registered until each job is disposed.
  expect(transport.count()).toBe(4);
  await numericJob();
  expect(transport.count()).toBe(2);
  await shapedJob();
  expect(transport.count()).toBe(0);
});

test("a foreign request with an unreadable job id is dropped, not malformed", async () => {
  const transport = new RecordingEventTransport();
  const failures: Array<ConnectionFailure> = [];
  const progress: Array<unknown> = [];
  const dispose = await listenTauriBridgeJob(
    "request:scan",
    "job:scan",
    {
      progressCodec: jsonCodec,
      successCodec: jsonCodec,
      detailCodec: jsonCodec,
      progress: (event) => progress.push(event.progress),
      terminal: () => {},
      onFailure: (failure) => failures.push(failure),
    },
    transport,
  );

  transport.emit(BRIDGE_PROGRESS_EVENT, {
    requestId: "request:other",
    jobId: "Not An Id!",
    progress: 10,
  });
  transport.emit(BRIDGE_TERMINAL_EVENT, {
    requestId: "request:other",
    jobId: "Not An Id!",
    outcome: "cancelled",
  });
  await tick();

  expect(failures).toEqual([]);
  expect(progress).toEqual([]);
  expect(transport.listeners.size).toBe(2);
  await dispose();

  // The same unreadable job id under this job's own request is malformed.
  const own = new RecordingEventTransport();
  const ownFailures: Array<ConnectionFailure> = [];
  await listenTauriBridgeJob(
    "request:scan",
    "job:scan",
    {
      progressCodec: jsonCodec,
      successCodec: jsonCodec,
      detailCodec: jsonCodec,
      terminal: () => {},
      onFailure: (failure) => ownFailures.push(failure),
    },
    own,
  );
  own.emit(BRIDGE_PROGRESS_EVENT, {
    requestId: "request:scan",
    jobId: "Not An Id!",
    progress: 10,
  });
  await tick();
  expect(ownFailures.map((failure) => failure.phase)).toEqual(["event"]);
  expect(own.listeners.size).toBe(0);
});

test("events after this job's terminal are neither decoded nor reported", async () => {
  const transport = new RecordingEventTransport();
  const failures: Array<ConnectionFailure> = [];
  const progress: Array<unknown> = [];
  const terminal: Array<unknown> = [];
  const dispose = await listenTauriBridgeJob(
    "request:scan",
    "job:scan",
    {
      progressCodec: numberCodec,
      successCodec: numberCodec,
      detailCodec: numberCodec,
      progress: (event) => progress.push(event.progress),
      terminal: (event) => terminal.push(event),
      onFailure: (failure) => failures.push(failure),
    },
    transport,
  );

  transport.emit(BRIDGE_TERMINAL_EVENT, {
    requestId: "request:scan",
    jobId: "job:scan",
    outcome: { succeeded: 3 },
  });
  // Late input for the settled job, malformed in both payload and envelope.
  transport.emit(BRIDGE_PROGRESS_EVENT, {
    requestId: "request:scan",
    jobId: "job:scan",
    progress: "late",
  });
  transport.emit(BRIDGE_TERMINAL_EVENT, {
    requestId: "request:scan",
    jobId: "job:scan",
    outcome: { succeeded: "late" },
  });
  transport.emit(BRIDGE_TERMINAL_EVENT, "not-an-event");
  await tick();

  expect(failures).toEqual([]);
  expect(progress).toEqual([]);
  expect(terminal).toHaveLength(1);
  expect(transport.listeners.size).toBe(2);
  await dispose();
});

test("a malformed terminal event reaches onFailure and ends the job", async () => {
  const transport = new RecordingEventTransport();
  const failures: Array<ConnectionFailure> = [];
  const terminal: Array<unknown> = [];
  await listenTauriBridgeJob(
    "request:scan",
    "job:scan",
    {
      progressCodec: jsonCodec,
      successCodec: jsonCodec,
      detailCodec: jsonCodec,
      terminal: (event) => terminal.push(event),
      onFailure: (failure) => failures.push(failure),
    },
    transport,
  );

  transport.emit(BRIDGE_TERMINAL_EVENT, "not-an-event");
  // A second malformed event is the same failure, not a new one.
  transport.emit(BRIDGE_TERMINAL_EVENT, null);
  await tick();

  expect(failures).toHaveLength(1);
  expect(failures[0]?.phase).toBe("event");
  // The job terminates: both listeners are torn down, so a well-formed
  // terminal arriving now never reaches the consumer.
  expect(transport.listeners.size).toBe(0);
  transport.emit(BRIDGE_TERMINAL_EVENT, {
    requestId: "request:scan",
    jobId: "job:scan",
    outcome: { succeeded: { value: 3 } },
  });
  expect(terminal).toHaveLength(0);
});

test("a malformed progress event reaches onFailure and ends the job", async () => {
  const transport = new RecordingEventTransport();
  const failures: Array<ConnectionFailure> = [];
  const progress: Array<unknown> = [];
  await listenTauriBridgeJob(
    "request:scan",
    "job:scan",
    {
      progressCodec: jsonCodec,
      successCodec: jsonCodec,
      detailCodec: jsonCodec,
      progress: (event) => progress.push(event),
      terminal: () => {},
      onFailure: (failure) => failures.push(failure),
    },
    transport,
  );

  transport.emit(BRIDGE_PROGRESS_EVENT, 42);
  await tick();

  expect(failures).toHaveLength(1);
  expect(failures[0]?.phase).toBe("event");
  expect(transport.listeners.size).toBe(0);
  transport.emit(BRIDGE_PROGRESS_EVENT, {
    requestId: "request:scan",
    jobId: "job:scan",
    progress: 20,
  });
  expect(progress).toHaveLength(0);
});

test.each(["throws", "rejects"])(
  "dispose runs both unlistens when terminal disposal %s",
  async (mode) => {
    const disposed: Array<string> = [];
    const terminalError = new Error("terminal unlisten failed");
    const transport: EventTransport = {
      invoke: () => Promise.resolve(undefined),
      listen: (event) =>
        Promise.resolve(() => {
          disposed.push(event);
          if (event === BRIDGE_TERMINAL_EVENT) {
            if (mode === "throws") {
              throw terminalError;
            }
            return Promise.reject(terminalError);
          }
          return undefined;
        }),
    };
    const dispose = await listenTauriBridgeJob(
      "request:scan",
      "job:scan",
      {
        progressCodec: jsonCodec,
        successCodec: jsonCodec,
        detailCodec: jsonCodec,
        terminal: () => {},
      },
      transport,
    );

    await expect(dispose()).rejects.toBe(terminalError);
    expect(disposed).toEqual([BRIDGE_TERMINAL_EVENT, BRIDGE_PROGRESS_EVENT]);
  },
);

test("a throwing failure reporter leaves both job listeners released", async () => {
  const transport = new RecordingEventTransport();
  const failures: Array<ConnectionFailure> = [];
  const reportError = new Error("reporter failed");
  await listenTauriBridgeJob(
    "request:scan",
    "job:scan",
    {
      progressCodec: jsonCodec,
      successCodec: jsonCodec,
      detailCodec: jsonCodec,
      terminal: () => {},
      onFailure(failure) {
        failures.push(failure);
        throw reportError;
      },
    },
    transport,
  );

  expect(() => transport.emit(BRIDGE_TERMINAL_EVENT, null)).toThrow(reportError);
  await tick();
  expect(failures).toEqual([{ phase: "event", error: expect.any(Error) }]);
  expect(transport.listeners.size).toBe(0);
});

test("registration keeps its original rejection when teardown also fails", async () => {
  const registrationError = new Error("terminal registration failed");
  const teardownError = new Error("progress unlisten failed");
  const failures: Array<ConnectionFailure> = [];
  const transport: EventTransport = {
    invoke: () => Promise.resolve(undefined),
    async listen(event) {
      if (event === BRIDGE_TERMINAL_EVENT) {
        throw registrationError;
      }
      return () => {
        throw teardownError;
      };
    },
  };
  const registration = listenTauriBridgeJob(
    "request:scan",
    "job:scan",
    {
      progressCodec: jsonCodec,
      successCodec: jsonCodec,
      detailCodec: jsonCodec,
      terminal: () => {},
      onFailure(failure) {
        failures.push(failure);
        throw new Error("reporter failed");
      },
    },
    transport,
  );

  await expect(registration).rejects.toBe(registrationError);
  expect(failures).toEqual([{ phase: "unlisten", error: teardownError }]);
});

test("a malformed progress event during terminal registration releases the terminal listener", async () => {
  const released: Array<string> = [];
  const handlers = new Map<string, (payload: unknown) => void>();
  let completeTerminal!: (unlisten: Unlisten) => void;
  const transport: EventTransport = {
    invoke: () => Promise.resolve(undefined),
    listen: (event, listener) => {
      handlers.set(event, listener);
      if (event === BRIDGE_TERMINAL_EVENT) {
        return new Promise<Unlisten>((resolve) => {
          completeTerminal = resolve;
        });
      }
      return Promise.resolve(() => {
        released.push(event);
      });
    },
  };
  const failures: Array<ConnectionFailure> = [];
  const registration = listenTauriBridgeJob(
    "request:scan",
    "job:scan",
    {
      progressCodec: jsonCodec,
      successCodec: jsonCodec,
      detailCodec: jsonCodec,
      terminal: () => {},
      onFailure: (failure) => failures.push(failure),
    },
    transport,
  );
  // Progress is registered; terminal registration is still in flight.
  await tick();
  handlers.get(BRIDGE_PROGRESS_EVENT)?.(42);
  completeTerminal(() => {
    released.push(BRIDGE_TERMINAL_EVENT);
  });
  await registration;
  await tick();

  expect(failures.map((failure) => failure.phase)).toEqual(["event"]);
  // Both registrations are released, the late terminal one as soon as it lands.
  expect(released).toEqual([BRIDGE_PROGRESS_EVENT, BRIDGE_TERMINAL_EVENT]);
});

test("a throwing consumer callback is not reported as a malformed event", async () => {
  const transport = new RecordingEventTransport();
  const failures: Array<ConnectionFailure> = [];
  const terminal: Array<unknown> = [];
  const dispose = await listenTauriBridgeJob(
    "request:scan",
    "job:scan",
    {
      progressCodec: jsonCodec,
      successCodec: jsonCodec,
      detailCodec: jsonCodec,
      progress: () => {
        throw new Error("consumer bug");
      },
      terminal: (event) => terminal.push(event),
      onFailure: (failure) => failures.push(failure),
    },
    transport,
  );

  expect(() =>
    transport.emit(BRIDGE_PROGRESS_EVENT, {
      requestId: "request:scan",
      jobId: "job:scan",
      progress: 20,
    }),
  ).toThrow("consumer bug");
  transport.emit(BRIDGE_TERMINAL_EVENT, {
    requestId: "request:scan",
    jobId: "job:scan",
    outcome: { succeeded: { value: 3 } },
  });
  await tick();

  expect(failures).toHaveLength(0);
  expect(transport.listeners.size).toBe(2);
  expect(terminal).toHaveLength(1);
  await dispose();
});

test("a throwing progress callback during registration leaves the job listening", async () => {
  class ReplayingTransport extends RecordingEventTransport {
    override async listen(event: string, listener: (payload: unknown) => void): Promise<Unlisten> {
      const unlisten = await super.listen(event, listener);
      if (event === BRIDGE_PROGRESS_EVENT) {
        listener({ requestId: "request:scan", jobId: "job:scan", progress: 20 });
      }
      return unlisten;
    }
  }

  const transport = new ReplayingTransport();
  const failures: Array<ConnectionFailure> = [];
  const terminal: Array<unknown> = [];
  const dispose = await listenTauriBridgeJob(
    "request:scan",
    "job:scan",
    {
      progressCodec: jsonCodec,
      successCodec: jsonCodec,
      detailCodec: jsonCodec,
      progress() {
        throw new Error("consumer failed during registration");
      },
      terminal: (event) => terminal.push(event),
      onFailure: (failure) => failures.push(failure),
    },
    transport,
  );

  transport.emit(BRIDGE_TERMINAL_EVENT, {
    requestId: "request:scan",
    jobId: "job:scan",
    outcome: { succeeded: { value: 3 } },
  });
  expect(terminal).toHaveLength(1);
  expect(failures).toEqual([]);
  await dispose();
  expect(transport.listeners.size).toBe(0);
});
