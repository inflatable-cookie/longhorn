import { describe, expect, it } from "bun:test";
import { HISTORY_TREE_FIELDS } from "../../src/history-tree/index.ts";

// The shape a consumer that cannot depend on Longhorn has to mirror by hand.
// Longhorn cannot see their copy, so this only proves the map is reachable and
// names what it should; the pinning test itself belongs where both are visible.
describe("the generated field map is reachable as public API", () => {
  it("names every field of the records a fork renderer mirrors", () => {
    expect(HISTORY_TREE_FIELDS.ForkContinuationRecord).toEqual([
      "entryId", "label", "recordedAt", "preferred", "entryCount", "branchId", "branchName",
    ]);
    expect(HISTORY_TREE_FIELDS.ForkEntryRecord).toContain("continuationCount");
    expect(HISTORY_TREE_FIELDS.ForkBranchRecord).toContain("divergenceBranchId");
    expect(HISTORY_TREE_FIELDS.ForkPathPageSnapshot).toContain("rootContinuationCount");
  });
});
