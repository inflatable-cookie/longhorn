import { describe, expect, test } from "bun:test";

import { createDirectNotificationPort, NotificationClient, NotificationResponseCorrelationError, SerializedNotificationPort } from "../src/index.ts";
import { createTauriNotificationPort, NOTIFICATION_MUTATE_COMMAND, NOTIFICATION_SNAPSHOT_COMMAND } from "../src/tauri.ts";
import { fixture, fixturePort } from "./support.ts";

describe("notification transport conformance", () => {
  test("direct, serialized, and Tauri traces converge", async () => {
    const base = fixturePort();
    const tauri = createTauriNotificationPort({
      transport: { invoke: async (command, args) => command === NOTIFICATION_SNAPSHOT_COMMAND ? base.snapshot(args.query as never) : command === NOTIFICATION_MUTATE_COMMAND ? base.mutate(args.command as never) : undefined },
      nextRequestId: base.nextRequestId,
    });
    for (const port of [createDirectNotificationPort(fixturePort()), new SerializedNotificationPort(fixturePort()), tauri]) {
      const client = new NotificationClient(port);
      expect(await client.snapshot(0, 4)).toEqual(fixture.snapshotResponse);
      for (const [index, command] of fixture.mutationCommands.entries()) expect(await client.mutate(command)).toEqual(fixture.mutationResults[index]);
    }
  });

  test("rejects a reply carrying another request id", async () => {
    const port = fixturePort();
    const client = new NotificationClient({ ...port, snapshot: async () => ({ ...fixture.snapshotResponse, requestId: "request:foreign" }) });
    expect(client.snapshot()).rejects.toBeInstanceOf(NotificationResponseCorrelationError);
  });
});
