import { BridgeDomainClient, BridgeSession, type BridgeOperationDescriptor } from "@inflatable-cookie/longhorn/bridge";
import { NotificationClient, createDirectNotificationPort, type NotificationMutationCommand, type NotificationMutationResult, type NotificationSnapshotResponse } from "@inflatable-cookie/longhorn/notifications";
import { createTauriNotificationPort, NOTIFICATION_MUTATE_COMMAND, NOTIFICATION_SNAPSHOT_COMMAND } from "@inflatable-cookie/longhorn-tauri/notifications";
import { OperationClient, createDirectOperationPort, type OperationMutationCommand, type OperationMutationResult, type OperationSnapshotResponse } from "@inflatable-cookie/longhorn/operation";
import { createBridgeOperationPort } from "@inflatable-cookie/longhorn/operation/bridge";
import { createTauriOperationPort, OPERATION_MUTATE_COMMAND, OPERATION_SNAPSHOT_COMMAND } from "@inflatable-cookie/longhorn-tauri/operation";

import { equal, notificationTrace, operationTrace } from "../../common.ts";
import fixtureValue from "./fixture.json";

const fixture = fixtureValue as {
  operation: { snapshotResponse: OperationSnapshotResponse; commands: OperationMutationCommand[]; results: OperationMutationResult[]; expectedTrace: unknown };
  notifications: { snapshotResponse: NotificationSnapshotResponse; commands: NotificationMutationCommand[]; results: NotificationMutationResult[]; expectedTrace: unknown };
};
const operationResults = new Map(fixture.operation.results.map((result) => [result.requestId, result]));
const operationBase = {
  snapshot: async (query: { requestId: string }) => ({ ...fixture.operation.snapshotResponse, requestId: query.requestId }),
  mutate: async (command: OperationMutationCommand) => operationResults.get(command.requestId),
  cancel: async () => { throw new Error("Loophole fixture does not cancel"); },
  nextRequestId: () => fixture.operation.snapshotResponse.requestId,
};
const operationTauri = createTauriOperationPort({
  transport: { invoke: async (name, args) => name === OPERATION_SNAPSHOT_COMMAND ? operationBase.snapshot(args.query as never) : name === OPERATION_MUTATE_COMMAND ? operationBase.mutate(args.command as never) : undefined },
  nextRequestId: operationBase.nextRequestId,
});
const session = new BridgeSession({
  protocolVersion: 1,
  host: { hostInstanceId: "host:loophole", form: "localService" },
  sessionId: "session:loophole",
  connection: { state: "ready", reason: "negotiationAccepted" },
  authentication: "authenticated",
  transportFeatures: [],
  domainCapabilities: [{ domainId: "longhorn.operation", capabilities: ["longhorn.operation.read", "longhorn.operation.manage"] }],
  domainAuthorities: [{ domainId: "longhorn.operation", scopeId: "scope:render", availability: "available", readAuthority: "authoritative", writeAuthority: "authoritative", executionAuthority: "none", authorityEpoch: 4, authoritativeRevision: 0 }],
  diagnostics: [],
});
let bridgeRequest = 0;
const bridge = createBridgeOperationPort({
  client: new BridgeDomainClient(session, { invoke: async (descriptor: BridgeOperationDescriptor<unknown, unknown>, request: unknown) => {
    const envelope = request as { context: { requestId: string }; payload: { requestId: string } };
    if (descriptor.route.endsWith("snapshot")) return { requestId: envelope.context.requestId, outcome: { success: fixture.operation.snapshotResponse } };
    const result = operationResults.get(envelope.payload.requestId);
    return { requestId: envelope.context.requestId, authoritativeRevision: result?.snapshot.catalogueRevision ?? null, outcome: { applied: result } };
  } }),
  domainId: "longhorn.operation",
  sessionId: "session:loophole",
  authorityEpoch: 4,
  nextBridgeRequestId: () => `bridge-request:${++bridgeRequest}`,
  nextOperationRequestId: operationBase.nextRequestId,
});

const operationTraces = [];
for (const port of [createDirectOperationPort(operationBase), operationTauri, bridge]) {
  const client = new OperationClient(port);
  await client.snapshot();
  const results = [];
  for (const command of fixture.operation.commands) results.push(await client.mutate(command));
  operationTraces.push(operationTrace(results));
}
if (!operationTraces.every((trace) => equal(trace, fixture.operation.expectedTrace))) throw new Error("Loophole operation trace diverged");

const notificationResults = new Map(fixture.notifications.results.map((result) => [result.requestId, result]));
const notificationBase = {
  snapshot: async (query: { requestId: string }) => ({ ...fixture.notifications.snapshotResponse, requestId: query.requestId }),
  mutate: async (command: NotificationMutationCommand) => notificationResults.get(command.requestId),
  nextRequestId: () => fixture.notifications.snapshotResponse.requestId,
};
const notificationTauri = createTauriNotificationPort({
  transport: { invoke: async (name, args) => name === NOTIFICATION_SNAPSHOT_COMMAND ? notificationBase.snapshot(args.query as never) : name === NOTIFICATION_MUTATE_COMMAND ? notificationBase.mutate(args.command as never) : undefined },
  nextRequestId: notificationBase.nextRequestId,
});
const notificationTraces = [];
for (const port of [createDirectNotificationPort(notificationBase), notificationTauri]) {
  const client = new NotificationClient(port);
  await client.snapshot();
  const results = [];
  for (const command of fixture.notifications.commands) results.push(await client.mutate(command));
  notificationTraces.push(notificationTrace(results));
}
if (!notificationTraces.every((trace) => equal(trace, fixture.notifications.expectedTrace))) throw new Error("Loophole notification trace diverged");

console.log(JSON.stringify({ shape: "loophole", publicTrace: { operation: operationTraces[0], notifications: notificationTraces[0] }, transports: ["direct", "tauri", "bridge"] }));
