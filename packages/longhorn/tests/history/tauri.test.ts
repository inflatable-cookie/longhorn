import { describe, expect, test } from "bun:test";

import { createTauriHistoryPort } from "../../../longhorn-tauri/src/history.ts";
import { fixture } from "./support.ts";

describe("Tauri history composition", () => {
  test("uses three narrow commands and one invalidation event", async () => {
    const { pageRequest, navigationCommand } = fixture();
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
    const port = createTauriHistoryPort({
      transport,
      nextPlanId: () => "plan:test",
    });
    await port.snapshot();
    await port.page(pageRequest);
    await port.navigate(navigationCommand);
    await port.listen?.(() => {});
    expect(calls).toEqual([
      ["longhorn_history_snapshot", {}],
      ["longhorn_history_page", { command: pageRequest }],
      ["longhorn_history_navigate", { command: navigationCommand }],
    ]);
    expect(listened).toEqual(["longhorn://history/changed"]);
    expect(port.nextPlanId()).toBe("plan:test");
  });

  test("an invoke-only transport leaves the port silent", async () => {
    const transport = {
      async invoke() {
        return null;
      },
    };
    const port = createTauriHistoryPort({
      transport,
      nextPlanId: () => "plan:test",
    });
    expect(port.listen).toBeUndefined();
  });
});
