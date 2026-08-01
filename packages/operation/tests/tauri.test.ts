import { describe, expect, test } from "bun:test";

import {
  createTauriOperationPort,
  OPERATION_CANCEL_COMMAND,
  OPERATION_CHANGED_EVENT,
  OPERATION_MUTATE_COMMAND,
  OPERATION_SNAPSHOT_COMMAND,
} from "../src/tauri.ts";
import { fixture } from "./support.ts";

describe("Tauri operation composition", () => {
  test("uses three narrow commands and one invalidation event", async () => {
    const calls: Array<[string, unknown]> = [];
    let eventName = "";
    const transport = {
      async invoke(command: string, args?: Record<string, unknown>) {
        calls.push([command, args]);
        if (command === OPERATION_SNAPSHOT_COMMAND) return fixture.snapshotResponse;
        if (command === OPERATION_MUTATE_COMMAND) return fixture.mutationResults[0];
        return fixture.cancellationResult;
      },
      async listen(name: string) {
        eventName = name;
        return () => {};
      },
    };
    const port = createTauriOperationPort({ transport, nextRequestId: () => "request:test" });
    await port.snapshot(fixture.snapshotQuery);
    await port.mutate(fixture.mutationCommands[0]!);
    await port.cancel(fixture.cancellationCommand);
    await port.listen?.(() => {});
    expect(calls.map(([command]) => command)).toEqual([
      OPERATION_SNAPSHOT_COMMAND,
      OPERATION_MUTATE_COMMAND,
      OPERATION_CANCEL_COMMAND,
    ]);
    expect(eventName).toBe(OPERATION_CHANGED_EVENT);
  });
});
