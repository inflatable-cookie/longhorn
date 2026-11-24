import { describe, expect, test } from "bun:test";

import { NotificationController } from "../../src/notifications/controller.ts";
import type {
  NotificationChangedEvent,
  NotificationMutationCommand,
  NotificationRecordProjection,
  NotificationSnapshot,
  NotificationSnapshotQuery,
} from "../../src/notifications/generated/protocol.ts";
import type { NotificationPort, NotificationUnlisten } from "../../src/notifications/ports.ts";
import { cloned, fixture } from "./support.ts";

interface HeldPage {
  readonly requestId: string;
  readonly resolve: (value: unknown) => void;
  readonly reject: (error: unknown) => void;
}

/**
 * A port whose ledger the test controls. Page requests past offset zero can be
 * held, so a test decides when (and against which revision) each page answers.
 */
class PagedPort implements NotificationPort {
  readonly listeners = new Set<(event: unknown) => void>();
  readonly held: Array<HeldPage> = [];
  holdPages = false;
  records: Array<NotificationRecordProjection>;
  ledgerRevision: number;
  request = 0;

  constructor(records: Array<NotificationRecordProjection>, ledgerRevision: number) {
    this.records = records;
    this.ledgerRevision = ledgerRevision;
  }

  snapshot(query: NotificationSnapshotQuery): Promise<unknown> {
    if (query.offset === 0 || !this.holdPages) {
      return Promise.resolve({
        requestId: query.requestId,
        snapshot: snapshotAt(this.ledgerRevision, this.records, query.offset, query.limit),
      });
    }
    return new Promise((resolve, reject) => {
      this.held.push({ requestId: query.requestId, resolve, reject });
    });
  }

  mutate(_command: NotificationMutationCommand): Promise<unknown> {
    return Promise.reject(new Error("pagination tests do not mutate"));
  }

  listen(listener: (event: unknown) => void): NotificationUnlisten {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  nextRequestId(): string {
    this.request += 1;
    return `request:pagination-${this.request}`;
  }

  /** Moves the ledger and announces it, as the subscription would. */
  publish(records: Array<NotificationRecordProjection>, ledgerRevision: number): void {
    const previous = this.ledgerRevision;
    this.records = records;
    this.ledgerRevision = ledgerRevision;
    const event: NotificationChangedEvent = {
      protocolVersion: 1,
      requestId: "request:external",
      authority: fixture.mutationResults[0]!.snapshot.authority,
      previousLedgerRevision: previous,
      committedLedgerRevision: ledgerRevision,
      affectedNotificationIds: records.map((record) => record.notificationId),
      kind: "added",
    };
    for (const listener of this.listeners) {
      listener(event);
    }
  }

  /** Answers the oldest held page request with the given snapshot. */
  respondHeld(snapshot: NotificationSnapshot): void {
    const held = this.held.shift()!;
    held.resolve({ requestId: held.requestId, snapshot });
  }

  failHeld(error: unknown): void {
    this.held.shift()!.reject(error);
  }
}

describe("notification pagination", () => {
  test("a page answered against an older revision cannot overwrite a newer update", async () => {
    const records = ledgerOf(3);
    const port = new PagedPort(records, 1);
    const controller = new NotificationController({ port, pageSize: 2 });
    await controller.start();
    expect(ids(controller)).toEqual(["notification:0", "notification:1"]);

    port.holdPages = true;
    const more = controller.loadMore();
    port.publish([recordAt("notification:new", 10), ...records], 2);
    await flush();
    expect(ids(controller)).toEqual(["notification:new", "notification:0"]);

    port.respondHeld(snapshotAt(1, records, 2, 2));
    await more;
    expect(ids(controller)).toEqual(["notification:new", "notification:0"]);
    expect(controller.snapshot?.ledgerRevision).toBe(2);
    expect(controller.pagination).toEqual({ kind: "idle" });
    await controller.stop();
  });

  test("a page that resolves after stop writes no state", async () => {
    const records = ledgerOf(3);
    const port = new PagedPort(records, 1);
    const controller = new NotificationController({ port, pageSize: 2 });
    await controller.start();

    port.holdPages = true;
    const more = controller.loadMore();
    await controller.stop();
    port.respondHeld(snapshotAt(1, records, 2, 2));
    await more;

    expect(controller.snapshot).toBeUndefined();
    expect(controller.records).toEqual([]);
    expect(controller.pagination).toEqual({ kind: "idle" });
  });

  test("a page that resolves after a restart does not append to the new lifecycle", async () => {
    const records = ledgerOf(3);
    const port = new PagedPort(records, 1);
    const controller = new NotificationController({ port, pageSize: 2 });
    await controller.start();

    port.holdPages = true;
    const more = controller.loadMore();
    await controller.stop();
    await controller.start();
    expect(ids(controller)).toEqual(["notification:0", "notification:1"]);

    port.respondHeld(snapshotAt(1, records, 2, 2));
    await more;
    expect(ids(controller)).toEqual(["notification:0", "notification:1"]);
    expect(controller.hasMore).toBe(true);
    await controller.stop();
  });

  test("a page from a moved ledger resyncs page zero instead of installing a partial page", async () => {
    const records = ledgerOf(3);
    const port = new PagedPort(records, 1);
    const controller = new NotificationController({ port, pageSize: 2 });
    await controller.start();

    port.holdPages = true;
    const more = controller.loadMore();
    // The ledger moves without an announcement reaching this controller.
    port.ledgerRevision = 2;
    port.records = [recordAt("notification:new", 10), ...records];
    port.respondHeld(snapshotAt(2, port.records, 2, 2));
    await more;

    expect(ids(controller)).toEqual(["notification:new", "notification:0"]);
    expect(controller.snapshot?.ledgerRevision).toBe(2);
    expect(controller.hasMore).toBe(true);
    await controller.stop();
  });

  test("pagination exposes pending and failed states and recovers on retry", async () => {
    const records = ledgerOf(3);
    const port = new PagedPort(records, 1);
    const controller = new NotificationController({ port, pageSize: 2 });
    await controller.start();

    port.holdPages = true;
    const more = controller.loadMore();
    expect(controller.pagination).toEqual({ kind: "loading" });

    const boom = new Error("page refused");
    port.failHeld(boom);
    await expect(more).rejects.toBe(boom);
    expect(controller.pagination).toEqual({ kind: "failed", error: boom });
    expect(ids(controller)).toEqual(["notification:0", "notification:1"]);

    port.holdPages = false;
    await controller.loadMore();
    expect(ids(controller)).toEqual(["notification:0", "notification:1", "notification:2"]);
    expect(controller.hasMore).toBe(false);
    expect(controller.pagination).toEqual({ kind: "idle" });
    await controller.stop();
  });
});

function ledgerOf(count: number): Array<NotificationRecordProjection> {
  return Array.from({ length: count }, (_, index) => recordAt(`notification:${index}`, index + 1));
}

function recordAt(notificationId: string, sequence: number): NotificationRecordProjection {
  const template = cloned(fixture.mutationResults[0]!.snapshot.page.records[0]!);
  return { ...template, notificationId, sequence, readState: "unseen" };
}

function snapshotAt(
  ledgerRevision: number,
  records: ReadonlyArray<NotificationRecordProjection>,
  offset: number,
  limit: number,
): NotificationSnapshot {
  const snapshot = cloned(fixture.mutationResults[0]!.snapshot);
  snapshot.ledgerRevision = ledgerRevision;
  snapshot.retainedCount = records.length;
  snapshot.unseenCount = records.filter((record) => record.readState === "unseen").length;
  snapshot.page = {
    offset,
    totalCount: records.length,
    hasMore: offset + limit < records.length,
    records: cloned(records.slice(offset, offset + limit)),
  };
  return snapshot;
}

function ids(controller: NotificationController): Array<string> {
  return controller.records.map((record) => record.notificationId);
}

function flush(): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
}
