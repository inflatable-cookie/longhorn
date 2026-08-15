import { describe, expect, test } from "bun:test";

import { createTauriForkHistoryPort } from "../../../longhorn-tauri/src/history-tree.ts";

describe("Tauri fork-history composition", () => {
  test("uses seven narrow commands and one invalidation event", async () => {
    // The raw port carries commands opaquely; the stubs stand in for the
    // generated command types, which `ForkHistoryClient` is what narrows.
    const pathCommand = { seam: "path" } as never;
    const branchesCommand = { seam: "branches" } as never;
    const continuationsCommand = { seam: "continuations" } as never;
    const deleteCommand = { seam: "delete" } as never;
    const pruneCommand = { seam: "prune" } as never;
    const navigateCommand = { seam: "navigate" } as never;
    const calls: Array<[string, unknown]> = [];
    const listened: string[] = [];
    const transport = {
      async invoke(command: string, args?: Record<string, unknown>) {
        calls.push([command, args]);
        return null;
      },
      async listen(event: string) {
        listened.push(event);
        return () => {};
      },
    };
    const port = createTauriForkHistoryPort({
      transport,
      nextPlanId: () => "plan:test",
    });
    await port.snapshot();
    await port.path(pathCommand);
    await port.branches(branchesCommand);
    await port.continuations(continuationsCommand);
    await port.deleteContinuation(deleteCommand);
    await port.prune(pruneCommand);
    await port.navigate(navigateCommand);
    await port.listen?.(() => {});
    expect(calls).toEqual([
      ["longhorn_history_tree_snapshot", {}],
      ["longhorn_history_tree_path", { command: pathCommand }],
      ["longhorn_history_tree_branches", { command: branchesCommand }],
      ["longhorn_history_tree_continuations", { command: continuationsCommand }],
      ["longhorn_history_tree_delete_continuation", { command: deleteCommand }],
      ["longhorn_history_tree_prune", { command: pruneCommand }],
      ["longhorn_history_tree_navigate", { command: navigateCommand }],
    ]);
    expect(listened).toEqual(["longhorn://history-tree/changed"]);
    expect(port.nextPlanId()).toBe("plan:test");
  });

  test("an invoke-only transport leaves the port silent", async () => {
    const transport = {
      async invoke() {
        return null;
      },
    };
    const port = createTauriForkHistoryPort({
      transport,
      nextPlanId: () => "plan:test",
    });
    expect(port.listen).toBeUndefined();
  });
});
