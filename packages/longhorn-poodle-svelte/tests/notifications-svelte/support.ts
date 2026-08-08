import { NotificationSession } from "../../src/notifications/svelte.ts";
import type { NotificationChangedEvent, NotificationMutationCommand, NotificationSnapshot, NotificationSnapshotQuery } from "../../../longhorn/src/notifications/generated/protocol.ts";
import type { NotificationPort, NotificationUnlisten } from "../../../longhorn/src/notifications/ports.ts";
import { cloned, fixture } from "../../../longhorn/tests/notifications/support.ts";

export class MountedNotificationPort implements NotificationPort {
  readonly listeners = new Set<(event: unknown) => void>();
  snapshotValue = cloned(fixture.mutationResults[0]!.snapshot);
  request = 0;
  unlistenCount = 0;

  async snapshot(query: NotificationSnapshotQuery): Promise<unknown> { return { requestId: query.requestId, snapshot: cloned(this.snapshotValue) }; }
  async mutate(_command: NotificationMutationCommand): Promise<unknown> { throw new Error("mutation not expected in presentation fixture"); }
  listen(listener: (event: unknown) => void): NotificationUnlisten { this.listeners.add(listener); return () => { this.listeners.delete(listener); this.unlistenCount += 1; }; }
  nextRequestId(): string { this.request += 1; return `request:svelte-${this.request}`; }

  publish(snapshot: NotificationSnapshot): void {
    const previousLedgerRevision = this.snapshotValue.ledgerRevision;
    this.snapshotValue = cloned(snapshot);
    const event: NotificationChangedEvent = {
      protocolVersion: 1,
      requestId: `request:event-${snapshot.ledgerRevision}`,
      authority: snapshot.authority,
      previousLedgerRevision,
      committedLedgerRevision: snapshot.ledgerRevision,
      affectedNotificationIds: snapshot.page.records.map((record) => record.notificationId),
      kind: "added",
    };
    for (const listener of this.listeners) listener(event);
  }
}

export function createMountedSession(port = new MountedNotificationPort()) {
  return { port, session: new NotificationSession({ port, toast: { select: () => true } }) };
}

export function withNewRecord(snapshot: NotificationSnapshot): NotificationSnapshot {
  const next = cloned(snapshot);
  const source = cloned(next.page.records[0]!);
  next.ledgerRevision += 1;
  source.notificationId = "notification:new";
  source.sequence += 1;
  source.lastChangedLedgerRevision = next.ledgerRevision;
  source.draft.title = "New notification";
  next.page.records.unshift(source);
  next.page.totalCount += 1;
  next.retainedCount += 1;
  next.unseenCount += 1;
  return next;
}
