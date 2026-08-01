import fixtureValue from "../../../fixtures/operation/protocol-v1.json";
import type {
  OperationCancellationCommand,
  OperationCancellationResult,
  OperationChangedEvent,
  OperationMutationCommand,
  OperationMutationResult,
  OperationSnapshotQuery,
  OperationSnapshotResponse,
} from "../src/generated/protocol.ts";
import type { OperationPort } from "../src/ports.ts";

export interface OperationFixture {
  protocolVersion: number;
  snapshotQuery: OperationSnapshotQuery;
  snapshotResponse: OperationSnapshotResponse;
  mutationCommands: OperationMutationCommand[];
  mutationResults: OperationMutationResult[];
  cancellationCommand: OperationCancellationCommand;
  cancellationResult: OperationCancellationResult;
  changedEvents: OperationChangedEvent[];
  incompatibility: Record<string, unknown>;
}

export const fixture = fixtureValue as OperationFixture;

export function fixturePort(): OperationPort {
  const mutationResults = new Map(
    fixture.mutationResults.map((result) => [result.requestId, result]),
  );
  return {
    snapshot: async (query) => ({ ...fixture.snapshotResponse, requestId: query.requestId }),
    mutate: async (command) => mutationResults.get(command.requestId),
    cancel: async () => fixture.cancellationResult,
    nextRequestId: () => fixture.snapshotQuery.requestId,
  };
}

export function cloned<Value>(value: Value): Value {
  return JSON.parse(JSON.stringify(value)) as Value;
}
