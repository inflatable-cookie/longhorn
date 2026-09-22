import { describe, expect, test } from "bun:test";

import { createTauriUpdatePort } from "../../../longhorn-tauri/src/update.ts";
import { fixture } from "./support.ts";

describe("Tauri update composition", () => {
  test("uses seven narrow commands and two events", async () => {
    const {
      checkCommand,
      selectChannelCommand,
      deferCommand,
      prepareCommand,
      applyCommand,
      cancelCommand,
    } = fixture();
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
    await port.prepare(prepareCommand);
    await port.apply(applyCommand);
    await port.cancel(cancelCommand);
    await port.listen?.(() => {});
    await port.listenProgress?.(() => {});
    expect(calls).toEqual([
      ["longhorn_update_snapshot", {}],
      ["longhorn_update_check", { command: checkCommand }],
      ["longhorn_update_select_channel", { command: selectChannelCommand }],
      ["longhorn_update_defer", { command: deferCommand }],
      ["longhorn_update_prepare", { command: prepareCommand }],
      ["longhorn_update_apply", { command: applyCommand }],
      ["longhorn_update_cancel", { command: cancelCommand }],
    ]);
    expect(listened).toEqual(["longhorn://update/changed", "longhorn://update/progress"]);
  });

  test("an invoke-only transport leaves the port silent", async () => {
    const transport = {
      async invoke() {
        return null;
      },
    };
    const port = createTauriUpdatePort({ transport });
    expect(port.listen).toBeUndefined();
    expect(port.listenProgress).toBeUndefined();
  });
});
