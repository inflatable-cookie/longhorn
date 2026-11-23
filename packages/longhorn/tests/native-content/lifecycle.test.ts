import { describe, expect, test } from "bun:test";

import {
  NativeContentClient,
  NativeContentConnectionDisposedError,
  NativeContentPendingRequestLimitError,
  createDirectNativeContentPort,
} from "../../src/native-content/index.ts";
import type {
  ContentSizeDecision,
  ContentSizeProposal,
  DesiredUpdate,
  NativeContentPort,
} from "../../src/native-content/index.ts";
import { MemoryNativeContentHost, flush, nextUpdate } from "./support.ts";

// Refresh runs through the shared pump, which takes a few more microtasks than
// a fixed flush, so tests wait for the state they assert on rather than a count.
async function settleUntil(condition: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 100 && !condition(); attempt += 1) {
    await flush();
  }
}

describe("listener-first and stale-result lifetime", () => {
  test("an event between connect snapshot and response is reconciled", async () => {
    const host = new MemoryNativeContentHost();
    host.beforeConnectReturn = () => host.admitObservation();
    const connection = new NativeContentClient(host.port(), "island:fixture").connect();
    const ready = await connection.ready;
    expect(host.calls.slice(0, 2)).toEqual(["listen", "connect"]);
    expect(ready.cursor.observed_revision).toBe(1);
    expect(connection.current()?.cursor.observed_revision).toBe(1);
    await connection.dispose();
  });

  test("a late mutation result cannot replace newer attach state", async () => {
    const host = new MemoryNativeContentHost();
    const direct = host.port();
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const delayed: NativeContentPort = {
      ...direct,
      updateDesired: async (request) => {
        const result = await direct.updateDesired(request);
        await gate;
        return result;
      },
    };
    const connection = new NativeContentClient(delayed, "island:fixture").connect();
    const initial = await connection.ready;
    const pending = connection.updateDesired(nextUpdate(initial, 48));
    await flush();
    host.replaceDesired(96, 2);
    await settleUntil(() => connection.current()?.cursor.attach_generation === 2);
    expect(connection.current()?.cursor.attach_generation).toBe(2);
    expect(connection.current()?.cursor.desired_revision).toBe(2);
    release();
    expect((await pending).status).toBe("committed");
    expect(connection.current()?.cursor.attach_generation).toBe(2);
    expect(connection.current()?.cursor.desired_revision).toBe(2);
    await connection.dispose();
  });

  test("remount issues a new client epoch without changing attach generation", async () => {
    const host = new MemoryNativeContentHost();
    const client = new NativeContentClient(host.port(), "island:fixture");
    const first = client.connect();
    const firstSnapshot = await first.ready;
    const second = client.connect();
    const secondSnapshot = await second.ready;
    expect(firstSnapshot.cursor.client_epoch).toBe(1);
    expect(secondSnapshot.cursor.client_epoch).toBe(2);
    expect(secondSnapshot.cursor.attach_generation).toBe(firstSnapshot.cursor.attach_generation);
    await first.dispose();
    await second.dispose();
    expect(host.unlistenCount).toBe(2);
  });

  test("late listener registration is disposed exactly once", async () => {
    let finishListen!: (unlisten: () => void) => void;
    const listen = new Promise<() => void>((resolve) => {
      finishListen = resolve;
    });
    let unlistenCount = 0;
    let connectCount = 0;
    const port = createDirectNativeContentPort({
      connect: async () => {
        connectCount += 1;
        throw new Error("connect must not run after disposal");
      },
      snapshot: async () => ({}),
      updateDesired: async () => ({}),
      decideContentSize: async () => ({}),
      listen: () => listen,
      nextRequestId: () => "request:late-listener",
    });
    const connection = new NativeContentClient(port, "island:fixture").connect();
    const disposed = connection.dispose();
    finishListen(() => {
      unlistenCount += 1;
    });
    await disposed;
    await expect(connection.ready).rejects.toBeInstanceOf(NativeContentConnectionDisposedError);
    expect(connectCount).toBe(0);
    expect(unlistenCount).toBe(1);
  });

  test("pending request correlation is bounded", async () => {
    const host = new MemoryNativeContentHost();
    const direct = host.port();
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const port: NativeContentPort = {
      ...direct,
      updateDesired: async (request) => {
        const result = await direct.updateDesired(request);
        await gate;
        return result;
      },
    };
    const connection = new NativeContentClient(port, "island:fixture", {
      maximumPendingRequests: 1,
    }).connect();
    const initial = await connection.ready;
    const first = connection.updateDesired(nextUpdate(initial, 20));
    await flush();
    await expect(connection.updateDesired(nextUpdate(initial, 30))).rejects.toBeInstanceOf(
      NativeContentPendingRequestLimitError,
    );
    release();
    await first;
    await connection.dispose();
  });

  test("invalid caller input releases its pending slot", async () => {
    const host = new MemoryNativeContentHost();
    const connection = new NativeContentClient(host.port(), "island:fixture", {
      maximumPendingRequests: 1,
    }).connect();
    const initial = await connection.ready;
    // Each invalid request must fail on validation and free its slot; with a
    // limit of one, a leaked slot would turn the second call into a limit error.
    for (let attempt = 0; attempt < 2; attempt += 1) {
      const rejected = connection.updateDesired({} as DesiredUpdate);
      await expect(rejected).rejects.not.toBeInstanceOf(NativeContentPendingRequestLimitError);
    }
    const decision = connection.decideContentSize(
      {} as ContentSizeProposal,
      {} as ContentSizeDecision,
    );
    await expect(decision).rejects.not.toBeInstanceOf(NativeContentPendingRequestLimitError);
    const committed = await connection.updateDesired(nextUpdate(initial, 48));
    expect(committed.status).toBe("committed");
    await connection.dispose();
  });

  test("a failing unlisten is reported and dispose still settles", async () => {
    const host = new MemoryNativeContentHost();
    const direct = host.port();
    const port: NativeContentPort = {
      ...direct,
      listen: () => () => {
        throw new Error("unlisten failed");
      },
    };
    const connection = new NativeContentClient(port, "island:fixture").connect();
    await connection.ready;
    await expect(connection.dispose()).rejects.toThrow("unlisten failed");
    expect(connection.failures().map((failure) => failure.phase)).toEqual(["unlisten"]);
  });

  test("disposing after a failed start resolves without reporting the start failure again", async () => {
    const host = new MemoryNativeContentHost();
    const direct = host.port();
    const port: NativeContentPort = {
      ...direct,
      connect: async () => {
        throw new Error("connect refused");
      },
    };
    const connection = new NativeContentClient(port, "island:fixture").connect();
    await expect(connection.ready).rejects.toThrow("connect refused");
    await expect(connection.dispose()).resolves.toBeUndefined();
    expect(connection.failures().map((failure) => failure.phase)).toEqual(["snapshot"]);
  });
});
