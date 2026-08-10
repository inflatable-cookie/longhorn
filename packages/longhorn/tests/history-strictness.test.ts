import { describe, expect, test } from "bun:test";

import { assertValidHistoryChangedEvent } from "../src/history/validation.ts";
import { HISTORY_FIELDS } from "../src/history/generated/fields.ts";
import { assertForkChangedEvent } from "../src/history-tree/validation.ts";
import { HISTORY_TREE_FIELDS } from "../src/history-tree/generated/fields.ts";

/**
 * `history` and `history-tree` were already strict — both rejected an unknown
 * and a missing key at every object, from hand-written literal lists. What
 * changed is where the lists come from: the generated field map, so a Rust
 * field added or renamed moves the TypeScript with it instead of drifting.
 *
 * These tests hold the strictness that was already there and add the part that
 * was not: that the lists still match the Rust structs.
 */
describe("history boundary strictness", () => {
  const valid = {
    protocolVersion: 1,
    authorityEpoch: 1,
    historyId: "history:one",
    previousRevision: 1,
    committedRevision: 2,
    kind: "committed",
  };

  test("the field map matches the fixture's keys", () => {
    expect([...HISTORY_FIELDS.HistoryChangedEvent!].sort()).toEqual(
      Object.keys(valid).sort(),
    );
  });

  test("rejects an unknown field", () => {
    expect(() =>
      assertValidHistoryChangedEvent({ ...valid, extra: 1 }),
    ).toThrow(/unknown field/);
  });

  test("rejects a missing field", () => {
    const { historyId: _dropped, ...missing } = valid;
    expect(() => assertValidHistoryChangedEvent(missing)).toThrow(
      /missing field/,
    );
  });

  test("omits tagged unions rather than guessing their keys", () => {
    expect(HISTORY_FIELDS.HistoryNavigationResult).toBeUndefined();
  });
});

describe("history-tree boundary strictness", () => {
  const valid = {
    protocolVersion: 1,
    authorityEpoch: 1,
    historyId: "history:one",
    previousRevision: 1,
    committedRevision: 2,
    kind: "committed",
  };

  test("the field map matches the fixture's keys", () => {
    expect([...HISTORY_TREE_FIELDS.ForkChangedEvent!].sort()).toEqual(
      Object.keys(valid).sort(),
    );
  });

  test("rejects an unknown field", () => {
    expect(() => assertForkChangedEvent({ ...valid, extra: 1 })).toThrow(
      /unexpected keys/,
    );
  });

  /**
   * The two page commands differ by one field, and a single helper validates
   * both. Before the field map that helper took the difference as a spread
   * argument; the migration has to keep them distinct rather than collapsing
   * them onto whichever list it saw first.
   */
  test("keeps the two page commands distinct", () => {
    expect(HISTORY_TREE_FIELDS.ForkPathPageCommand).toContain("target");
    expect(HISTORY_TREE_FIELDS.ForkBranchPageCommand).not.toContain("target");
  });
});
