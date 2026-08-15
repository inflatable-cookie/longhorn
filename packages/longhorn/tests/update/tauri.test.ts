import { describe, expect, test } from "bun:test";

import { createTauriUpdatePort } from "../../../longhorn-tauri/src/update.ts";
import { fixture } from "./support.ts";

describe("Tauri update composition", () => {
  test("uses five narrow commands and one invalidation event", async () => {
    const { checkCommand, selectChannelCommand, deferCommand, installCommand } =
      fixture();
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
    const port = createTauriUpdatePort({ transport });
    await port.snapshot();
    await port.check(checkCommand);
    await port.selectChannel(selectChannelCommand);
    await port.defer(deferCommand);
    await port.install(installCommand);
    await port.listen?.(() => {});
    expect(calls).toEqual([
      ["longhorn_update_snapshot", {}],
      ["longhorn_update_check", { command: checkCommand }],
      ["longhorn_update_select_channel", { command: selectChannelCommand }],
      ["longhorn_update_defer", { command: deferCommand }],
      ["longhorn_update_install", { command: installCommand }],
    ]);
    expect(listened).toEqual(["longhorn://update/changed"]);
  });

  test("an invoke-only transport leaves the port silent", async () => {
    const transport = {
      async invoke() {
        return null;
      },
    };
    const port = createTauriUpdatePort({ transport });
    expect(port.listen).toBeUndefined();
  });
});
