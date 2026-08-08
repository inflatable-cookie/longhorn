import { OperationClient, SerializedOperationPort, createDirectOperationPort, type OperationMutationCommand, type OperationMutationResult, type OperationSnapshotResponse } from "@inflatable-cookie/longhorn-operation";

import { equal, operationTrace } from "../../common.ts";
import fixtureValue from "./fixture.json";

const fixture = fixtureValue as {
  snapshotResponse: OperationSnapshotResponse;
  commands: OperationMutationCommand[];
  results: OperationMutationResult[];
  expectedTrace: unknown;
};

const resultByRequest = new Map(fixture.results.map((result) => [result.requestId, result]));
const base = {
  snapshot: async (query: { requestId: string }) => ({ ...fixture.snapshotResponse, requestId: query.requestId }),
  mutate: async (command: OperationMutationCommand) => resultByRequest.get(command.requestId),
  cancel: async () => { throw new Error("minimal operation has no cancellation"); },
  nextRequestId: () => fixture.snapshotResponse.requestId,
};

const traces = [];
for (const port of [createDirectOperationPort(base), new SerializedOperationPort(base)]) {
  const client = new OperationClient(port);
  await client.snapshot();
  const results = [];
  for (const command of fixture.commands) results.push(await client.mutate(command));
  traces.push(operationTrace(results));
}
if (!traces.every((trace) => equal(trace, fixture.expectedTrace))) throw new Error("minimal operation trace diverged");
console.log(JSON.stringify({ shape: "minimal-operation", publicTrace: traces[0], transports: ["direct", "serialized"] }));
