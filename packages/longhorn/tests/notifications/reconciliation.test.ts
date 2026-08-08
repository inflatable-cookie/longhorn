import { describe, expect, test } from "bun:test";

import { isNewerNotificationSnapshot, NotificationClient, notificationEventAction } from "../../src/notifications/client.ts";
import { cloned, fixture } from "./support.ts";

describe("notification snapshot reconciliation", () => {
  test("registers the listener before loading the first snapshot", async () => {
    const order: string[] = [];
    const client = new NotificationClient({
      listen: async () => { order.push("listen"); return () => {}; },
      snapshot: async (query) => { order.push("snapshot"); return { ...fixture.snapshotResponse, requestId: query.requestId }; },
      mutate: async () => fixture.mutationResults[0],
      nextRequestId: () => fixture.snapshotQuery.requestId,
    });
    const subscription = client.subscribe(() => {});
    await subscription.ready;
    expect(order).toEqual(["listen", "snapshot"]);
    await subscription.dispose();
  });

  test("ignores stale events but refreshes gaps and new epochs", () => {
    const current = cloned(fixture.mutationResults[1]!.snapshot);
    const duplicate = cloned(fixture.changedEvents[1]!);
    expect(notificationEventAction(duplicate, current)).toEqual({ kind: "ignore" });
    const gap = cloned(fixture.changedEvents[2]!);
    gap.committedLedgerRevision = current.ledgerRevision + 2;
    expect(notificationEventAction(gap, current)).toEqual({ kind: "refresh" });
    const epoch = cloned(gap);
    epoch.authority.authorityEpoch += 1;
    epoch.committedLedgerRevision = 1;
    expect(notificationEventAction(epoch, current)).toEqual({ kind: "refresh" });
  });

  test("installs only monotonic snapshots from the same authority", () => {
    const current = cloned(fixture.mutationResults[1]!.snapshot);
    const stale = cloned(current);
    stale.ledgerRevision -= 1;
    expect(isNewerNotificationSnapshot(stale, current)).toBe(false);
    const nextEpoch = cloned(current);
    nextEpoch.authority.authorityEpoch += 1;
    nextEpoch.ledgerRevision = 0;
    expect(isNewerNotificationSnapshot(nextEpoch, current)).toBe(true);
    const foreign = cloned(current);
    foreign.authority.authorityId = "authority:foreign";
    expect(isNewerNotificationSnapshot(foreign, current)).toBe(false);
  });
});
