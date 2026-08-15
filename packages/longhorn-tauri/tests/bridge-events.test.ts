import { expect, test } from "bun:test";

import type {
  ConnectionFailure,
  EventTransport,
  Unlisten,
} from "@inflatable-cookie/longhorn/core";
import {
  connectBridgeStream,
} from "@inflatable-cookie/longhorn/bridge/stream";
import {
  BRIDGE_DOMAIN_EVENT,
  BRIDGE_PROGRESS_EVENT,
  BRIDGE_TERMINAL_EVENT,
  listenTauriBridgeJob,
  TauriBridgeStreamSource,
} from "@inflatable-cookie/longhorn-tauri/bridge-events";

import {
  fixture,
  jsonCodec,
} from "../../longhorn/tests/bridge/support.ts";

class RecordingEventTransport implements EventTransport {
  readonly order: string[] = [];
  readonly listeners = new Map<string, (payload: unknown) => void>();

  invoke(command: string): Promise<unknown> {
    this.order.push(command);
    return Promise.resolve(fixture.snapshot);
  }

  listen(
    event: string,
    listener: (payload: unknown) => void,
  ): Promise<Unlisten> {
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
  const source = new TauriBridgeStreamSource(
    "session:fixture",
    "example.workspace",
    transport,
  );
  const connection = connectBridgeStream({
    sessionId: "session:fixture",
    domainId: "example.workspace",
    source,
    snapshot: jsonCodec,
    event: jsonCodec,
    apply: (current) => current,
  });

  await connection.ready;
  expect(transport.order).toEqual([
    BRIDGE_DOMAIN_EVENT,
    "longhorn_bridge_resync",
  ]);
  await connection.dispose();
});

test("job listeners admit only matching correlation and one terminal", async () => {
  const transport = new RecordingEventTransport();
  const progress: unknown[] = [];
  const terminal: unknown[] = [];
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
  return new Promise((resolve) => setTimeout(resolve, 0));
}

test("a malformed terminal event reaches onFailure and ends the job", async () => {
  const transport = new RecordingEventTransport();
  const failures: ConnectionFailure[] = [];
  const terminal: unknown[] = [];
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
  const failures: ConnectionFailure[] = [];
  const progress: unknown[] = [];
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

test("dispose runs both unlistens when terminal disposal rejects", async () => {
  const disposed: string[] = [];
  const transport: EventTransport = {
    invoke: () => Promise.resolve(undefined),
    listen: (event) =>
      Promise.resolve(() => {
        disposed.push(event);
        if (event === BRIDGE_TERMINAL_EVENT) {
          return Promise.reject(new Error("terminal unlisten failed"));
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

  await expect(dispose()).rejects.toThrow("terminal unlisten failed");
  expect(disposed).toEqual([BRIDGE_TERMINAL_EVENT, BRIDGE_PROGRESS_EVENT]);
});
