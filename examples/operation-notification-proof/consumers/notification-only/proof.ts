import { NotificationClient, SerializedNotificationPort, createDirectNotificationPort, type NotificationMutationCommand, type NotificationMutationResult, type NotificationSnapshotResponse } from "@longhorn/notifications";
import { createTauriNotificationPort, NOTIFICATION_MUTATE_COMMAND, NOTIFICATION_SNAPSHOT_COMMAND } from "@longhorn/notifications/tauri";

import { equal, notificationTrace } from "../../common.ts";
import fixtureValue from "./fixture.json";

const fixture = fixtureValue as { snapshotResponse: NotificationSnapshotResponse; commands: NotificationMutationCommand[]; results: NotificationMutationResult[]; expectedTrace: unknown };
const resultByRequest = new Map(fixture.results.map((result) => [result.requestId, result]));
const base = {
  snapshot: async (query: { requestId: string }) => ({ ...fixture.snapshotResponse, requestId: query.requestId }),
  mutate: async (command: NotificationMutationCommand) => resultByRequest.get(command.requestId),
  nextRequestId: () => fixture.snapshotResponse.requestId,
};
const tauri = createTauriNotificationPort({
  transport: { invoke: async (name, args) => name === NOTIFICATION_SNAPSHOT_COMMAND ? base.snapshot(args.query as never) : name === NOTIFICATION_MUTATE_COMMAND ? base.mutate(args.command as never) : undefined },
  nextRequestId: base.nextRequestId,
});
const traces = [];
for (const port of [createDirectNotificationPort(base), new SerializedNotificationPort(base), tauri]) {
  const client = new NotificationClient(port);
  await client.snapshot();
  const results = [];
  for (const command of fixture.commands) results.push(await client.mutate(command));
  traces.push(notificationTrace(results));
}
if (!traces.every((trace) => equal(trace, fixture.expectedTrace))) throw new Error("notification-only trace diverged");
console.log(JSON.stringify({ shape: "notification-only", publicTrace: traces[0], transports: ["direct", "serialized", "tauri"] }));
