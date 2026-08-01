import { describe, expect, test } from "bun:test";

import {
  isNewerOperationSnapshot,
  OperationClient,
  operationEventAction,
} from "../src/client.ts";
import { cloned, fixture } from "./support.ts";

describe("operation snapshot reconciliation", () => {
  test("registers the listener before loading the first snapshot", async () => {
    const order: string[] = [];
    const client = new OperationClient({
      listen: async () => { order.push("listen"); return () => {}; },
      snapshot: async (query) => { order.push("snapshot"); return { ...fixture.snapshotResponse, requestId: query.requestId }; },
      mutate: async () => fixture.mutationResults[0],
      cancel: async () => fixture.cancellationResult,
      nextRequestId: () => fixture.snapshotQuery.requestId,
    });
    const subscription = client.subscribe(() => {});
    await subscription.ready;
    expect(order).toEqual(["listen", "snapshot"]);
    await subscription.dispose();
  });

  test("ignores stale and duplicate events, but refreshes gaps and new epochs", () => {
    const current = cloned(fixture.snapshotResponse.snapshot);
    current.catalogueRevision = 2;
    const duplicate = cloned(fixture.changedEvents[1]!);
    expect(operationEventAction(duplicate, current)).toEqual({ kind: "ignore" });

    const gap = cloned(fixture.changedEvents[3]!);
    expect(operationEventAction(gap, current)).toEqual({ kind: "refresh" });

    const newEpoch = cloned(gap);
    newEpoch.authority.authorityEpoch += 1;
    newEpoch.committedCatalogueRevision = 1;
    expect(operationEventAction(newEpoch, current)).toEqual({ kind: "refresh" });
  });

  test("only installs monotonic snapshots from the same authority", () => {
    const current = cloned(fixture.snapshotResponse.snapshot);
    const stale = cloned(current);
    stale.catalogueRevision -= 1;
    expect(isNewerOperationSnapshot(stale, current)).toBe(false);

    const nextEpoch = cloned(current);
    nextEpoch.authority.authorityEpoch += 1;
    nextEpoch.catalogueRevision = 0;
    expect(isNewerOperationSnapshot(nextEpoch, current)).toBe(true);

    const foreign = cloned(current);
    foreign.authority.authorityId = "authority:foreign";
    expect(isNewerOperationSnapshot(foreign, current)).toBe(false);
  });
});
