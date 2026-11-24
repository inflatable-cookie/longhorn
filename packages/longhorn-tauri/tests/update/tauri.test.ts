import { describe, expect, test } from "bun:test";
import { ok } from "node:assert/strict";
import { UpdateController } from "@inflatable-cookie/longhorn/update";

import { TauriTransport } from "../../src/transport/index.ts";
import { createTauriUpdatePort } from "../../src/update.ts";
import { fixture } from "../../../longhorn/tests/update/support.ts";

describe("Tauri update composition", () => {
  test("uses seven narrow commands and two events", async () => {
    const {
      checkCommand,
      selectChannelCommand,
      deferCommand,
      prepareCommand,
      applyCommand,
      cancelCommand,
    } = fixture();
    const calls: Array<[string, unknown]> = [];
    const listened: Array<string> = [];
    const transport = {
      async invoke(command: string, args?: Record<string, unknown>) {
        calls.push([command, args]);
        return null;
      },
      async listen(event: string) {
        listened.push(event);
        return () => {};
      },
    };
    const port = createTauriUpdatePort({ transport });
    await port.snapshot();
    await port.check(checkCommand);
    await port.selectChannel(selectChannelCommand);
    await port.defer(deferCommand);
    await port.prepare(prepareCommand);
    await port.apply(applyCommand);
    await port.cancel(cancelCommand);
    await port.listen?.(() => {});
    await port.listenProgress?.(() => {});
    expect(calls).toEqual([
      ["longhorn_update_snapshot", {}],
      ["longhorn_update_check", { command: checkCommand }],
      ["longhorn_update_select_channel", { command: selectChannelCommand }],
      ["longhorn_update_defer", { command: deferCommand }],
      ["longhorn_update_prepare", { command: prepareCommand }],
      ["longhorn_update_apply", { command: applyCommand }],
      ["longhorn_update_cancel", { command: cancelCommand }],
    ]);
    expect(listened).toEqual(["longhorn://update/changed", "longhorn://update/progress"]);
  });

  test("an invoke-only transport leaves the port silent", async () => {
    const transport = {
      async invoke() {
        return null;
      },
    };
    const port = createTauriUpdatePort({ transport });
    expect(port.listen).toBeUndefined();
    expect(port.listenProgress).toBeUndefined();
    expect(port.prepareWithProgress).toBeUndefined();
  });

  test("a host that returns a plain snapshot retains event-only progress", async () => {
    const { notCheckedSnapshot, progressEvent, prepareCommand, stagedSnapshot } = fixture();
    let report: ((event: unknown) => void) | undefined;
    let finish: ((outcome: unknown) => void) | undefined;
    class OlderHost extends TauriTransport {
      override invoke(command: string, args: Record<string, unknown>): Promise<unknown> {
        if (command === "longhorn_update_snapshot") {
          expect(args).toEqual({ includeProgressCapabilities: true });
          return Promise.resolve(notCheckedSnapshot);
        }
        expect(command).toBe("longhorn_update_prepare");
        expect(args).toEqual({ command: prepareCommand });
        return new Promise((resolve) => {
          finish = resolve;
        });
      }

      listen(event: string, listener: (event: unknown) => void): () => void {
        if (event === "longhorn://update/progress") {
          report = listener;
        }
        return () => {};
      }
    }
    const controller = new UpdateController({
      port: createTauriUpdatePort({ transport: new OlderHost() }),
    });
    await controller.start();
    const pending = controller.prepare(prepareCommand.version);
    ok(report, "the renderer listens to the legacy progress event");
    ok(finish, "the renderer invokes legacy prepare");
    report(progressEvent);
    expect(controller.progress).toEqual(progressEvent.progress);

    finish({ status: "committed", snapshot: stagedSnapshot });
    await pending;
    report(progressEvent);
    expect(controller.progress).toEqual(stagedSnapshot.progress);
    await controller.stop();
  });

  test("native progress channels are isolated and close with their invocation", async () => {
    const { prepareCommand, progressEvent, snapshot, stagedSnapshot } = fixture();
    const callbacks = new Map<number, (message: unknown) => void>();
    const requests: Array<{
      readonly args: Record<string, unknown>;
      readonly resolve: (outcome: unknown) => void;
    }> = [];
    class PendingTransport extends TauriTransport {
      override invoke(command: string, args: Record<string, unknown>): Promise<unknown> {
        if (command === "longhorn_update_snapshot") {
          expect(args).toEqual({ includeProgressCapabilities: true });
          return Promise.resolve({ snapshot, scopedPrepareProgress: true });
        }
        expect(command).toBe("longhorn_update_prepare");
        return new Promise((resolve) => {
          requests.push({ args, resolve });
        });
      }
    }
    let nextId = 0;
    const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
    Object.defineProperty(globalThis, "window", {
      configurable: true,
      value: {
        __TAURI_INTERNALS__: {
          transformCallback(callback: (message: unknown) => void) {
            callbacks.set(++nextId, callback);
            return nextId;
          },
          unregisterCallback(id: number) {
            callbacks.delete(id);
          },
        },
      },
    });
    try {
      const port = createTauriUpdatePort({ transport: new PendingTransport() });
      expect(await port.snapshot()).toEqual(snapshot);
      const firstReports: Array<unknown> = [];
      const secondReports: Array<unknown> = [];
      const first = port.prepareWithProgress?.(prepareCommand, (value) => firstReports.push(value));
      const second = port.prepareWithProgress?.(prepareCommand, (value) =>
        secondReports.push(value),
      );
      const argumentsOnTheWire = requests.map(({ args }) => JSON.stringify(args));
      expect(argumentsOnTheWire).toEqual(
        ["__CHANNEL__:1", "__CHANNEL__:2"].map((onProgress) =>
          JSON.stringify({ command: prepareCommand, onProgress }),
        ),
      );
      callbacks.get(2)?.({ index: 0, message: progressEvent });
      expect(firstReports).toEqual([]);
      expect(secondReports).toEqual([progressEvent]);
      for (const request of requests) {
        request.resolve({ status: "committed", snapshot: stagedSnapshot });
      }
      await Promise.all([first, second]);
      expect(callbacks.size).toBe(0);
    } finally {
      if (originalWindow === undefined) {
        Reflect.deleteProperty(globalThis, "window");
      } else {
        Object.defineProperty(globalThis, "window", originalWindow);
      }
    }
  });
});
