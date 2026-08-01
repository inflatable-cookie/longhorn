import fixtureValue from "../../../fixtures/notifications/protocol-v1.json";
import type {
  NotificationChangedEvent,
  NotificationMutationCommand,
  NotificationMutationResult,
  NotificationSnapshotQuery,
  NotificationSnapshotResponse,
} from "../src/generated/protocol.ts";
import type { NotificationPort } from "../src/ports.ts";

export interface NotificationFixture {
  protocolVersion: number;
  snapshotQuery: NotificationSnapshotQuery;
  snapshotResponse: NotificationSnapshotResponse;
  mutationCommands: NotificationMutationCommand[];
  mutationResults: NotificationMutationResult[];
  changedEvents: Array<NotificationChangedEvent | null>;
  incompatibility: Record<string, unknown>;
}

export const fixture = fixtureValue as NotificationFixture;

export function fixturePort(): NotificationPort {
  const results = new Map(fixture.mutationResults.map((result) => [result.requestId, result]));
  return {
    snapshot: async (query) => ({ ...fixture.snapshotResponse, requestId: query.requestId }),
    mutate: async (command) => results.get(command.requestId),
    nextRequestId: () => fixture.snapshotQuery.requestId,
  };
}

export function cloned<Value>(value: Value): Value { return JSON.parse(JSON.stringify(value)) as Value; }
