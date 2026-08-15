import { describe, expect, test } from "bun:test";

import { createTauriNotificationPort } from "../../../longhorn-tauri/src/notifications.ts";
import { fixture } from "./support.ts";

describe("Tauri notification composition", () => {
  test("uses two narrow commands and one invalidation event", async () => {
    const mutationCommand = fixture.mutationCommands[0];
    if (mutationCommand === undefined) throw new Error("fixture has no mutation command");
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
    const port = createTauriNotificationPort({
      transport,
      nextRequestId: () => "request:test",
    });
    await port.snapshot(fixture.snapshotQuery);
    await port.mutate(mutationCommand);
    await port.listen?.(() => {});
    expect(calls).toEqual([
      ["longhorn_notifications_snapshot", { query: fixture.snapshotQuery }],
      ["longhorn_notifications_mutate", { command: mutationCommand }],
    ]);
    expect(listened).toEqual(["longhorn://notifications/changed"]);
    expect(port.nextRequestId()).toBe("request:test");
  });

  test("an invoke-only transport leaves the port silent", async () => {
    const transport = {
      async invoke() {
        return null;
      },
    };
    const port = createTauriNotificationPort({
      transport,
      nextRequestId: () => "request:test",
    });
    expect(port.listen).toBeUndefined();
  });
});
