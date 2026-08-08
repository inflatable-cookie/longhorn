import { OperationClient, createDirectOperationPort, type OperationCancellationCommand, type OperationCancellationResult, type OperationMutationCommand, type OperationMutationResult, type OperationSnapshotResponse } from "@inflatable-cookie/longhorn/operation";
import { createTauriOperationPort, OPERATION_CANCEL_COMMAND, OPERATION_MUTATE_COMMAND, OPERATION_SNAPSHOT_COMMAND } from "@inflatable-cookie/longhorn-tauri/operation";

import { equal, operationTrace } from "../../common.ts";
import fixtureValue from "./fixture.json";

const fixture = fixtureValue as {
  snapshotResponse: OperationSnapshotResponse;
  steps: Array<{ kind: "mutate"; command: OperationMutationCommand; result: OperationMutationResult } | { kind: "cancel"; command: OperationCancellationCommand; result: OperationCancellationResult }>;
  expectedTrace: unknown;
};
const mutation = new Map(fixture.steps.filter((step) => step.kind === "mutate").map((step) => [step.command.requestId, step.result]));
const cancellation = new Map(fixture.steps.filter((step) => step.kind === "cancel").map((step) => [step.command.requestId, step.result]));
const direct = {
  snapshot: async (query: { requestId: string }) => ({ ...fixture.snapshotResponse, requestId: query.requestId }),
  mutate: async (command: OperationMutationCommand) => mutation.get(command.requestId),
  cancel: async (command: OperationCancellationCommand) => cancellation.get(command.requestId),
  nextRequestId: () => fixture.snapshotResponse.requestId,
};
const tauri = createTauriOperationPort({
  transport: { invoke: async (name, args) => name === OPERATION_SNAPSHOT_COMMAND ? direct.snapshot(args.query as never) : name === OPERATION_MUTATE_COMMAND ? direct.mutate(args.command as never) : name === OPERATION_CANCEL_COMMAND ? direct.cancel(args.command as never) : undefined },
  nextRequestId: direct.nextRequestId,
});

const traces = [];
for (const port of [createDirectOperationPort(direct), tauri]) {
  const client = new OperationClient(port);
  await client.snapshot();
  const results = [];
  for (const step of fixture.steps) results.push(step.kind === "mutate" ? await client.mutate(step.command) : await client.cancel(step.command));
  traces.push(operationTrace(results));
}
if (!traces.every((trace) => equal(trace, fixture.expectedTrace))) throw new Error("Soundcheck trace diverged");
console.log(JSON.stringify({ shape: "soundcheck", publicTrace: traces[0], transports: ["direct", "tauri"], reconnectable: true }));
