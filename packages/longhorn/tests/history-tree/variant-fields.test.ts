import { describe, expect, it } from "bun:test";
import { assertForkNavigationCommand, assertForkPathCommand } from "../../src/history-tree/validation.ts";
import { HISTORY_TREE_VARIANT_FIELDS } from "../../src/history-tree/generated/variant-fields.ts";

const base = { protocolVersion: 1, authorityEpoch: 7, historyId: "history:tree", expectedRevision: 4 } as const;
const navigation = (target: unknown) => ({ ...base, planId: "plan:test", target });
const path = (target: unknown) => ({ ...base, target, offset: 0, limit: 2 });

describe("tagged unions validate from the generated map", () => {
  // Card 181 shipped this target; the hand-written key list allowed only
  // ["kind"] for every target but checkout, so the first consumer to send a
  // branchId would have been rejected at the boundary. Nothing caught it,
  // because no generated fact contradicted the list.
  it("accepts checkoutBranchRoot with the branchId it carries", () => {
    expect(() => assertForkNavigationCommand(navigation({ kind: "checkoutBranchRoot", branchId: "branch:main" }))).not.toThrow();
  });

  it("rejects checkoutBranchRoot with a key the variant does not declare", () => {
    expect(() => assertForkNavigationCommand(navigation({ kind: "checkoutBranchRoot", branchId: "branch:main", entryId: "entry:a" }))).toThrow();
  });

  it("rejects checkoutBranchRoot without its branchId", () => {
    expect(() => assertForkNavigationCommand(navigation({ kind: "checkoutBranchRoot" }))).toThrow();
  });

  it("keeps every other navigation target exact", () => {
    expect(() => assertForkNavigationCommand(navigation({ kind: "undo" }))).not.toThrow();
    expect(() => assertForkNavigationCommand(navigation({ kind: "undo", branchId: "branch:main" }))).toThrow();
    expect(() => assertForkNavigationCommand(navigation({ kind: "checkoutContinuation", entryId: "entry:d" }))).not.toThrow();
  });

  it("validates path targets from the same map", () => {
    expect(() => assertForkPathCommand(path({ kind: "default" }))).not.toThrow();
    expect(() => assertForkPathCommand(path({ kind: "continuation", fromEntryId: "entry:d" }))).not.toThrow();
    expect(() => assertForkPathCommand(path({ kind: "continuation", branchId: "branch:main" }))).toThrow();
  });

  // The map is the authority for these lists; a hand-written copy is the thing
  // the card removed.
  it("derives the keys from the Rust enum", () => {
    expect(HISTORY_TREE_VARIANT_FIELDS.ForkNavigationTargetProjection.checkoutBranchRoot).toEqual(["kind", "branchId"]);
    expect(HISTORY_TREE_VARIANT_FIELDS.ForkNavigationTargetProjection.undo).toEqual(["kind"]);
  });
});
