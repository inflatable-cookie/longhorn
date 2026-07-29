import { render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import {
  ReactiveClientState,
  type ReactiveConnection,
  type StartableClientState,
} from "../src/index.ts";
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
        connect: () =>
          connection(() => undefined, Promise.reject(failure)),
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
    await first.unmount();
    await Promise.resolve();

    const second = render(LifecycleHarness, { props: { state } });
    await Promise.resolve();
    await second.unmount();
    await Promise.resolve();

    expect(starts).toBe(2);
    expect(stops).toBe(2);
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
    await mounted.unmount();
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
    await mounted.unmount();
    ready.resolve();
    await state.stop();

    expect(disposals).toBe(1);
    expect(state.status.kind).toBe("idle");
  });
});
