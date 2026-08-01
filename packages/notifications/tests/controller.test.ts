import { describe, expect, test } from "bun:test";

import { NotificationController, type NotificationActionExecutor } from "../src/controller.ts";
import type { NotificationChangedEvent, NotificationMutationCommand, NotificationMutationResult, NotificationSnapshot, NotificationSnapshotQuery } from "../src/generated/protocol.ts";
import type { NotificationPort, NotificationUnlisten } from "../src/ports.ts";
import { cloned, fixture } from "./support.ts";

class LivePort implements NotificationPort {
  readonly listeners = new Set<(event: unknown) => void>();
  readonly mutations: NotificationMutationCommand[] = [];
  snapshotValue = cloned(fixture.mutationResults[0]!.snapshot);
  unlistenCount = 0;
  request = 0;

  async snapshot(query: NotificationSnapshotQuery): Promise<unknown> { return { requestId: query.requestId, snapshot: cloned(this.snapshotValue) }; }

  async mutate(command: NotificationMutationCommand): Promise<unknown> {
    this.mutations.push(command);
    const template = command.kind === "markSeen" ? fixture.mutationResults[1]! : fixture.mutationResults[2]!;
    const result = { ...cloned(template), requestId: command.requestId } as NotificationMutationResult;
    this.publish(result.snapshot, command.requestId, command.kind === "markSeen" ? "seen" : "dismissed");
    return result;
  }

  listen(listener: (event: unknown) => void): NotificationUnlisten {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); this.unlistenCount += 1; };
  }

  nextRequestId(): string { this.request += 1; return `request:renderer-${this.request}`; }

  publish(snapshot: NotificationSnapshot, requestId = "request:external", kind: NotificationChangedEvent["kind"] = "added"): void {
    const previous = this.snapshotValue.ledgerRevision;
    this.snapshotValue = cloned(snapshot);
    const event: NotificationChangedEvent = {
      protocolVersion: 1,
      requestId,
      authority: snapshot.authority,
      previousLedgerRevision: previous,
      committedLedgerRevision: snapshot.ledgerRevision,
      affectedNotificationIds: snapshot.page.records.map((record) => record.notificationId),
      kind,
    };
    for (const listener of this.listeners) listener(event);
  }
}

describe("notification controller", () => {
  test("two renderer instances reconcile seen mutations without shared state", async () => {
    const port = new LivePort();
    const first = new NotificationController({ port });
    const second = new NotificationController({ port });
    await Promise.all([first.start(), second.start()]);
    expect(first.records[0]?.readState).toBe("unseen");
    expect(second.records[0]?.readState).toBe("unseen");

    await first.markSeen("notification:scan");
    await eventually(() => second.records[0]?.readState === "seen");
    expect(first.records[0]?.readState).toBe("seen");
    expect(second.records[0]?.readState).toBe("seen");
    expect(first).not.toBe(second);

    await first.dismiss("notification:scan");
    await eventually(() => second.records.length === 0);
    expect(first.records).toEqual([]);
    expect(second.records).toEqual([]);
    await Promise.all([first.stop(), second.stop()]);
  });

  test("toast dismissal is local while retained truth survives remount", async () => {
    const port = new LivePort();
    const first = new NotificationController({ port, toast: { select: () => true } });
    await first.start();
    expect(first.toasts).toEqual([]);
    port.publish(withNewRecord(port.snapshotValue));
    await eventually(() => first.toasts.length === 1);
    const toastId = first.toasts[0]!.id;
    first.dismissToast(toastId);
    expect(first.toasts).toEqual([]);
    expect(first.records.map((record) => record.notificationId)).toContain("notification:new");
    expect(port.mutations).toEqual([]);
    await first.stop();

    const remounted = new NotificationController({ port, toast: { select: () => true } });
    await remounted.start();
    expect(remounted.records.map((record) => record.notificationId)).toContain("notification:new");
    expect(remounted.toasts).toEqual([]);
    await remounted.stop();
  });

  test("semantic action admission is evaluated at invocation time", async () => {
    const port = new LivePort();
    let admitted = true;
    const decisions: boolean[] = [];
    const actions: NotificationActionExecutor = {
      admitAndExecute: async () => {
        decisions.push(admitted);
        if (!admitted) throw new Error("action no longer admitted");
      },
    };
    const controller = new NotificationController({ port, toast: { select: () => true }, actions });
    await controller.start();
    port.publish(withNewRecord(port.snapshotValue));
    await eventually(() => controller.toasts.length === 1);
    admitted = false;
    await expect(controller.invokeAction("notification:new", "action:open-result")).rejects.toThrow("action no longer admitted");
    expect(decisions).toEqual([false]);
    expect(controller.records.map((record) => record.notificationId)).toContain("notification:new");
    await controller.stop();
  });

  test("teardown unregisters exactly once and reloads authority", async () => {
    const port = new LivePort();
    const controller = new NotificationController({ port });
    await controller.start();
    expect(port.listeners.size).toBe(1);
    await controller.stop();
    expect(port.listeners.size).toBe(0);
    expect(port.unlistenCount).toBe(1);
    await controller.start();
    expect(controller.records).toHaveLength(1);
    await controller.stop();
    expect(port.unlistenCount).toBe(2);
  });
});

function withNewRecord(snapshot: NotificationSnapshot): NotificationSnapshot {
  const next = cloned(snapshot);
  const source = cloned(next.page.records[0]!);
  next.ledgerRevision += 1;
  source.notificationId = "notification:new";
  source.sequence += 1;
  source.lastChangedLedgerRevision = next.ledgerRevision;
  source.draft.title = "New notification";
  source.readState = "unseen";
  next.page.records.unshift(source);
  next.page.totalCount += 1;
  next.retainedCount += 1;
  next.unseenCount += 1;
  return next;
}

async function eventually(predicate: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 30; attempt += 1) {
    if (predicate()) return;
    await Promise.resolve();
  }
  throw new Error("condition did not become true");
}
