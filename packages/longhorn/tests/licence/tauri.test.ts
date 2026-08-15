import { describe, expect, test } from "bun:test";

import { createTauriLicencePort } from "../../../longhorn-tauri/src/licence.ts";
import { fixture } from "./support.ts";

describe("Tauri licence composition", () => {
  test("uses six narrow commands and one invalidation event", async () => {
    const {
      activateCommands,
      deactivateCommand,
      refreshCommand,
      releaseSeatCommand,
      renameSeatCommand,
    } = fixture();
    const activateCommand = activateCommands[0];
    if (activateCommand === undefined) throw new Error("fixture has no activate command");
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
    const port = createTauriLicencePort({ transport });
    await port.snapshot();
    await port.activate(activateCommand);
    await port.deactivate(deactivateCommand);
    await port.refresh(refreshCommand);
    await port.releaseSeat(releaseSeatCommand);
    await port.renameSeat(renameSeatCommand);
    await port.listen?.(() => {});
    expect(calls).toEqual([
      ["longhorn_licence_snapshot", {}],
      ["longhorn_licence_activate", { command: activateCommand }],
      ["longhorn_licence_deactivate", { command: deactivateCommand }],
      ["longhorn_licence_refresh", { command: refreshCommand }],
      ["longhorn_licence_release_seat", { command: releaseSeatCommand }],
      ["longhorn_licence_rename_seat", { command: renameSeatCommand }],
    ]);
    expect(listened).toEqual(["longhorn://licence/changed"]);
  });

  test("an invoke-only transport leaves the port silent", async () => {
    const transport = {
      async invoke() {
        return null;
      },
    };
    const port = createTauriLicencePort({ transport });
    expect(port.listen).toBeUndefined();
  });
});
