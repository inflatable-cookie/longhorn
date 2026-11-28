import { render, waitFor } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

import { ReactiveClientState } from "../../src/bindings/index.ts";
import type { ReactiveConnection, StartableClientState } from "../../src/bindings/index.ts";
import LifecycleHarness from "./LifecycleHarness.svelte";
import StatusHarness from "./StatusHarness.svelte";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((resolve_, reject_) => {
    resolve = resolve_;
    reject = reject_;
  });
  return { promise, resolve, reject };
}

function connection(
  current: () => string | undefined,
  ready: Promise<unknown> = Promise.resolve(),
  dispose: () => Promise<void> = async () => undefined,
): ReactiveConnection<string> {
  return { current, ready, dispose };
}

describe("ReactiveClientState", () => {
  it("retries a failed start after disposing its connection", async () => {
    const failure = new Error("snapshot unavailable");
    const dispose = vi.fn(async () => undefined);
    const connect = vi
      .fn(() => connection(() => "recovered"))
      .mockImplementationOnce(() => connection(() => undefined, Promise.reject(failure), dispose));
    const state = new ReactiveClientState({ capability: { kind: "supported", connect } });

    await expect(state.start()).rejects.toBe(failure);
    expect(dispose).toHaveBeenCalledTimes(1);
    await state.start();
    expect(state.status.kind).toBe("ready");
    expect(state.snapshot).toBe("recovered");
    await state.destroy();
  });

  it("retains the snapshot during reconnect and ignores the old listener", async () => {
    const disposal = deferred<void>();
    const release = vi.fn(() => disposal.promise);
    const replacementRelease = vi.fn(async () => undefined);
    let oldListener: ((value: string) => void) | undefined;
    const state = new ReactiveClientState<string>({
      capability: {
        kind: "supported",
        connect(listener) {
          if (oldListener === undefined) {
            oldListener = listener;
            return connection(() => "initial", Promise.resolve(), release);
          }
          return connection(() => "replacement", Promise.resolve(), replacementRelease);
        },
      },
    });

    await state.start();
    const reconnect = state.reconnect();
    expect(state.status.kind).toBe("reconnecting");
    expect(state.snapshot).toBe("initial");
    oldListener?.("stale");
    expect(state.snapshot).toBe("initial");
    disposal.resolve();
    await reconnect;
    expect(release).toHaveBeenCalledTimes(1);
    expect(state.status.kind).toBe("ready");
    expect(state.snapshot).toBe("replacement");
    await state.stop();
    expect(replacementRelease).toHaveBeenCalledTimes(1);
  });

  it.each(["stop", "destroy"] as const)("lets %s supersede a queued start", async (end) => {
    const disposal = deferred<void>();
    let connections = 0;
    let disposals = 0;
    const state = new ReactiveClientState({
      capability: {
        kind: "supported",
        connect: () => {
          connections += 1;
          return connection(
            () => "snapshot",
            Promise.resolve(),
            () => {
              disposals += 1;
              return disposal.promise;
            },
          );
        },
      },
    });
    await state.start();
    const firstStop = state.stop();
    const restart = state.start();
    const finalStop = state[end]();
    disposal.resolve();
    await Promise.all([firstStop, restart, finalStop]);

    expect(connections).toBe(1);
    expect(disposals).toBe(1);
    expect(state.status.kind).toBe("idle");
    expect(state.snapshot).toBeUndefined();
    if (end === "destroy") {
      await expect(state.start()).rejects.toThrow(/destroyed/);
    }
  });

  it("does not let a superseded ready rejection dispose the replacement", async () => {
    const firstReady = deferred<void>();
    let connections = 0;
    const disposals: Array<number> = [];
    const state = new ReactiveClientState({
      capability: {
        kind: "supported",
        connect: () => {
          const id = ++connections;
          return connection(
            () => `snapshot:${id}`,
            id === 1 ? firstReady.promise : Promise.resolve(),
            async () => {
              disposals.push(id);
            },
          );
        },
      },
    });
    const start = state.start();
    const rejected = expect(start).rejects.toThrow("old ready failed");
    const stop = state.stop();
    const restart = state.start();
    await Promise.all([stop, restart]);
    firstReady.reject(new Error("old ready failed"));
    await rejected;
    expect(state.snapshot).toBe("snapshot:2");
    expect(disposals).toEqual([1]);
    await state.stop();
    expect(disposals).toEqual([1, 2]);
  });

  it("keeps two window instances isolated", async () => {
    const first = new ReactiveClientState({
      capability: {
        kind: "supported",
        connect: () => connection(() => "window:first"),
      },
    });
    const second = new ReactiveClientState({
      capability: {
        kind: "supported",
        connect: () => connection(() => "window:second"),
      },
    });

    await Promise.all([first.start(), second.start()]);
    expect(first.snapshot).toBe("window:first");
    expect(second.snapshot).toBe("window:second");

    await first.stop();
    expect(first.status.kind).toBe("idle");
    expect(first.snapshot).toBeUndefined();
    expect(second.status.kind).toBe("ready");
    expect(second.snapshot).toBe("window:second");
    await second.destroy();
  });

  it("distinguishes unsupported capability from transport failure", async () => {
    const unsupported = new ReactiveClientState<string>({
      capability: { kind: "unsupported", reason: "no transfer host" },
    });
    await unsupported.start();
    expect(unsupported.status).toEqual({
      kind: "unsupported",
      reason: "no transfer host",
    });

    const failure = new Error("transport offline");
    const failed = new ReactiveClientState<string>({
      capability: {
        kind: "supported",
        connect: () => connection(() => undefined, Promise.reject(failure)),
      },
    });
    await expect(failed.start()).rejects.toBe(failure);
    expect(failed.status).toEqual({ kind: "failed", error: failure });
  });

  it("mounts, unmounts, and remounts with idempotent cleanup", async () => {
    let starts = 0;
    let stops = 0;
    const state: StartableClientState = {
      async start() {
        starts += 1;
      },
      async stop() {
        stops += 1;
      },
    };

    const first = render(LifecycleHarness, { props: { state } });
    await Promise.resolve();
    await Promise.resolve(first.unmount());
    await Promise.resolve();

    const second = render(LifecycleHarness, { props: { state } });
    await Promise.resolve();
    await Promise.resolve(second.unmount());
    await Promise.resolve();

    expect(starts).toBe(2);
    expect(stops).toBe(2);
  });

  it("reports cleanup failures even without a caller-supplied reporter", async () => {
    const failure = new Error("could not detach listener");
    const report = vi.spyOn(console, "error").mockImplementation(() => undefined);
    try {
      const mounted = render(LifecycleHarness, {
        props: {
          state: {
            start: () => Promise.resolve(),
            stop: () => Promise.reject(failure),
          },
        },
      });
      mounted.unmount();
      await waitFor(() => expect(report).toHaveBeenCalledExactlyOnceWith(failure));
    } finally {
      report.mockRestore();
    }
  });

  it("publishes rune-backed status changes to mounted components", async () => {
    const ready = deferred<void>();
    const state = new ReactiveClientState({
      capability: {
        kind: "supported",
        connect: () => connection(() => "current", ready.promise),
      },
    });
    const mounted = render(StatusHarness, { props: { state } });

    const start = state.start();
    await waitFor(() => {
      expect(mounted.getByTestId("status").textContent).toBe("loading");
    });
    ready.resolve();
    await start;
    await waitFor(() => {
      expect(mounted.getByTestId("status").textContent).toBe("ready");
    });
    await Promise.resolve(mounted.unmount());
    await state.destroy();
  });

  it("disposes a connection that becomes ready after unmount", async () => {
    const ready = deferred<void>();
    let disposals = 0;
    const state = new ReactiveClientState({
      capability: {
        kind: "supported",
        connect: () =>
          connection(
            () => "late",
            ready.promise,
            async () => {
              disposals += 1;
            },
          ),
      },
    });

    const mounted = render(LifecycleHarness, { props: { state } });
    await Promise.resolve();
    await Promise.resolve(mounted.unmount());
    ready.resolve();
    await state.stop();

    expect(disposals).toBe(1);
    expect(state.status.kind).toBe("idle");
  });
});
