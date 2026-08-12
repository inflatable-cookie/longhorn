import { describe, expect, it } from "bun:test";
import { assertValidHistoryNavigationCommand, assertValidHistoryPageSnapshot } from "../../src/history/validation.ts";
import { HISTORY_VARIANT_FIELDS } from "../../src/history/generated/variant-fields.ts";

// Card 191. The origin is the one state no entry names, so the target that
// reaches it and the page fact that describes it are both new surface.

const command = (target: unknown) => ({
  protocolVersion: 1, authorityEpoch: 7, historyId: "history:x",
  planId: "plan:1", expectedRevision: 4, target,
});

describe("the origin is a position the client can name", () => {
  it("accepts checkoutRoot, which carries nothing but its kind", () => {
    expect(() => assertValidHistoryNavigationCommand(command({ kind: "checkoutRoot" }))).not.toThrow();
  });

  it("rejects checkoutRoot carrying an entry, which would mean something else", () => {
    expect(() => assertValidHistoryNavigationCommand(command({ kind: "checkoutRoot", entryId: "entry:1" }))).toThrow();
  });

  it("derives both key lists from the enum, not from a second copy", () => {
    expect(HISTORY_VARIANT_FIELDS.HistoryNavigationTargetProjection.checkoutRoot).toEqual(["kind"]);
    expect(HISTORY_VARIANT_FIELDS.HistoryPageFloorProjection.baseline).toEqual(["kind", "prunedEntryCount"]);
  });

  // The distinction a renderer must not lose: after a prune the position below
  // the oldest entry is where the surviving history begins, not where the
  // document did.
  it("rejects a baseline floor that omits how much was pruned", () => {
    const page = (floor: unknown) => ({
      protocolVersion: 1, authorityEpoch: 7, historyId: "history:x", revision: 4,
      offset: 0, totalEntries: 0, entries: [], truncatedBefore: false, truncatedAfter: false,
      retainedBaseline: { prunedEntryCount: 1, prunedEncodedWeight: 8, lastPrunedEntryId: "entry:0", lastPrunedSequence: 1 },
      floor,
    });
    expect(() => assertValidHistoryPageSnapshot(page({ kind: "baseline", prunedEntryCount: 1 }))).not.toThrow();
    expect(() => assertValidHistoryPageSnapshot(page({ kind: "baseline" }))).toThrow();
    expect(() => assertValidHistoryPageSnapshot(page({ kind: "origin" }))).not.toThrow();
  });
});
