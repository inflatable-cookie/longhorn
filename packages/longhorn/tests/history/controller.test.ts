import { describe, expect, test } from "bun:test";

import {
  HistoryController,
  HistoryLateResultError,
} from "../../src/history/controller.ts";
import type {
  HistoryChangedEvent,
  HistoryNavigationCommand,
  HistoryNavigationResult,
  HistoryPageCommand,
  HistoryPageSnapshot,
  HistorySnapshot,
} from "../../src/history/generated/protocol.ts";
import type { HistoryPort, HistoryUnlisten } from "../../src/history/ports.ts";
import { clone, fixture, tick } from "./support.ts";

class MutablePort implements HistoryPort {
  snapshotValue: HistorySnapshot;
  pageValue: HistoryPageSnapshot;
  result: HistoryNavigationResult;
  calls: string[] = [];
  listener: ((event: unknown) => void) | undefined;
  onListen: ((listener: (event: unknown) => void) => void) | undefined;
  navigateOverride:
    | ((command: HistoryNavigationCommand) => Promise<unknown>)
    | undefined;

  constructor() {
    const value = fixture();
    this.snapshotValue = clone(value.snapshot);
    this.pageValue = clone(value.page);
    this.result = clone(value.navigationResults[0]);
  }

  async snapshot(): Promise<unknown> {
    this.calls.push("snapshot");
    return clone(this.snapshotValue);
  }

  async page(_command: HistoryPageCommand): Promise<unknown> {
    this.calls.push("page");
    return clone(this.pageValue);
  }

  async navigate(command: HistoryNavigationCommand): Promise<unknown> {
    this.calls.push("navigate");
    if (this.navigateOverride !== undefined) {
      return this.navigateOverride(command);
    }
    return clone(this.result);
  }

  async listen(listener: (event: unknown) => void): Promise<HistoryUnlisten> {
    this.calls.push("listen");
    this.listener = listener;
    this.onListen?.(listener);
    return () => {
      this.calls.push("unlisten");
    };
  }

  nextPlanId(): string {
    return "plan:controller";
  }

  emit(event: HistoryChangedEvent): void {
    this.listener?.(clone(event));
  }
}

describe("history controller lifecycle", () => {
  test("attaches the listener before the initial snapshot", async () => {
    const port = new MutablePort();
    const controller = new HistoryController({ port, pageSize: 2 });
    await controller.start();
    expect(port.calls.slice(0, 3)).toEqual(["listen", "snapshot", "page"]);
    expect(controller.status).toEqual({ kind: "ready" });
    await controller.stop();
  });

  test("cannot miss a commit between listener registration and snapshot", async () => {
    const value = fixture();
    const port = new MutablePort();
    port.onListen = (listener) => {
      port.snapshotValue = {
        ...clone(value.navigationResults[0].snapshot),
        summary: {
          ...clone(value.navigationResults[0].snapshot.summary),
          revision: 12,
        },
      };
      port.pageValue = {
        ...clone(value.page),
        revision: 12,
      };
      listener({
        ...clone(value.changedEvent),
        previousRevision: 7,
        committedRevision: 12,
      });
    };
    const controller = new HistoryController({ port, pageSize: 2 });

    await controller.start();

    expect(port.calls[0]).toBe("listen");
    expect(controller.snapshot?.summary.revision).toBe(12);
    expect(controller.pageSnapshot?.revision).toBe(12);
    await controller.stop();
  });

  test("resyncs gaps and authority replacement from fresh snapshots", async () => {
    const value = fixture();
    const port = new MutablePort();
    const controller = new HistoryController({ port, pageSize: 2 });
    await controller.start();

    port.snapshotValue = clone(value.navigationResults[0].snapshot);
    port.pageValue = {
      ...clone(value.page),
      revision: 12,
    };
    port.emit({
      ...clone(value.changedEvent),
      previousRevision: 7,
    });
    await tick();
    expect(controller.snapshot?.summary.revision).toBe(12);

    port.snapshotValue = {
      ...clone(port.snapshotValue),
      authorityEpoch: 8,
    };
    port.pageValue = {
      ...clone(port.pageValue),
      authorityEpoch: 8,
    };
    port.emit({
      ...clone(value.changedEvent),
      authorityEpoch: 8,
      previousRevision: null,
      kind: "imported",
    });
    await tick();
    expect(controller.snapshot?.authorityEpoch).toBe(8);
    await controller.stop();
  });

  test("installs the authoritative snapshot from stale navigation rejection", async () => {
    const value = fixture();
    const port = new MutablePort();
    port.result = clone(value.navigationResults[1]);
    const controller = new HistoryController({ port, pageSize: 2 });
    await controller.start();
    port.pageValue = {
      ...clone(value.page),
      revision: 12,
    };
    const result = await controller.undo();
    expect(result.status).toBe("rejected");
    expect(controller.snapshot?.summary.revision).toBe(12);
    expect(controller.rejection?.code).toBe("staleRevision");
    await controller.stop();
  });

  test("disposes listener registration that completes after teardown", async () => {
    const port = new MutablePort();
    let resolveListener:
      | ((unlisten: HistoryUnlisten) => void)
      | undefined;
    let disposed = 0;
    port.listen = async () =>
      new Promise<HistoryUnlisten>((resolve) => {
        resolveListener = resolve;
      });
    const controller = new HistoryController({ port });
    const starting = controller.start();
    await tick();
    const stopping = controller.stop();
    resolveListener?.(() => {
      disposed += 1;
    });
    await Promise.all([starting, stopping]);
    expect(disposed).toBe(1);
    expect(controller.status).toEqual({ kind: "idle" });
  });

  test("rejects a navigation result arriving after teardown", async () => {
    const port = new MutablePort();
    let resolveNavigation:
      | ((result: HistoryNavigationResult) => void)
      | undefined;
    port.navigateOverride = async () =>
      new Promise<HistoryNavigationResult>((resolve) => {
        resolveNavigation = resolve;
      });
    const controller = new HistoryController({ port });
    await controller.start();
    const navigation = controller.undo();
    await tick();
    await controller.stop();
    resolveNavigation?.(clone(fixture().navigationResults[0]));
    await expect(navigation).rejects.toBeInstanceOf(HistoryLateResultError);
    expect(controller.snapshot).toBeUndefined();
  });

  test("filters the authoritative page without changing its identity", async () => {
    const port = new MutablePort();
    const controller = new HistoryController({ port, pageSize: 2 });
    await controller.start();
    controller.setFilter("rename");
    expect(controller.entries.map(({ entryId }) => entryId)).toEqual([
      "entry:future",
    ]);
    expect(controller.pageSnapshot?.entries).toHaveLength(2);
    await controller.stop();
  });
});
