import { describe, expect, test } from "bun:test";

import {
  HISTORY_CHANGED_EVENT,
  HISTORY_NAVIGATE_COMMAND,
  HISTORY_PAGE_COMMAND,
  HISTORY_SNAPSHOT_COMMAND,
  createTauriHistoryPort,
} from "../../../longhorn-tauri/src/history.ts";
import { HistoryClient } from "../../src/history/client.ts";
import { createDirectHistoryPort } from "../../src/history/direct.ts";
import { SerializedHistoryPort } from "../../src/history/serialized.ts";
import type { EventTransport } from "@inflatable-cookie/longhorn/core";
import { clone, fixture } from "./support.ts";

describe("direct and serialized-loopback conformance", () => {
  test("produce the same semantic trace", async () => {
    const value = fixture();
    const createPort = () =>
      createDirectHistoryPort({
        snapshot: async () => clone(value.snapshot),
        page: async () => clone(value.page),
        navigate: async () => clone(value.navigationResults[0]),
        nextPlanId: () => "plan:fixture",
      });
    const direct = await trace(new HistoryClient(createPort()), value);
    const serialized = await trace(
      new HistoryClient(new SerializedHistoryPort(createPort())),
      value,
    );
    expect(serialized).toEqual(direct);
  });
});

describe("Tauri transport adapter", () => {
  test("uses exact commands and the optional event edge", async () => {
    const value = fixture();
    const invocations: Array<readonly [string, Record<string, unknown>]> = [];
    const listeners: Array<readonly [string, (payload: unknown) => void]> = [];
    const transport: EventTransport = {
      async invoke(command, arguments_) {
        invocations.push([command, arguments_]);
        if (command === HISTORY_SNAPSHOT_COMMAND) return clone(value.snapshot);
        if (command === HISTORY_PAGE_COMMAND) return clone(value.page);
        return clone(value.navigationResults[0]);
      },
      async listen(event, listener) {
        listeners.push([event, listener]);
        return () => {};
      },
    };
    const client = new HistoryClient(
      createTauriHistoryPort({
        transport,
        nextPlanId: () => "plan:tauri",
      }),
    );
    const unlisten = await client.listen(() => {});
    await trace(client, value);
    await unlisten();

    expect(listeners[0]?.[0]).toBe(HISTORY_CHANGED_EVENT);
    expect(invocations.map(([command]) => command)).toEqual([
      HISTORY_SNAPSHOT_COMMAND,
      HISTORY_PAGE_COMMAND,
      HISTORY_NAVIGATE_COMMAND,
    ]);
    expect(invocations[1]?.[1]).toEqual({ command: value.pageRequest });
  });
});

async function trace(client: HistoryClient, value: ReturnType<typeof fixture>) {
  return {
    snapshot: await client.snapshot(),
    page: await client.page(value.pageRequest),
    result: await client.navigate(value.navigationCommand),
    nextPlanId: client.nextPlanId(),
  };
}
