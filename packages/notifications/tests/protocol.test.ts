import { describe, expect, test } from "bun:test";

import {
  assertCompatibleNotificationChangedEvent,
  assertCompatibleNotificationMutationCommand,
  assertCompatibleNotificationMutationResult,
  assertCompatibleNotificationSnapshotQuery,
  assertCompatibleNotificationSnapshotResponse,
  NotificationProtocolCompatibilityError,
} from "../src/index.ts";
import { cloned, fixture } from "./support.ts";

describe("generated notification protocol", () => {
  test("accepts every Rust fixture value exactly", () => {
    assertCompatibleNotificationSnapshotQuery(fixture.snapshotQuery);
    assertCompatibleNotificationSnapshotResponse(fixture.snapshotResponse);
    fixture.mutationCommands.forEach(assertCompatibleNotificationMutationCommand);
    fixture.mutationResults.forEach(assertCompatibleNotificationMutationResult);
    fixture.changedEvents.filter((event) => event !== null).forEach(assertCompatibleNotificationChangedEvent);
  });

  test("fails closed on future protocol, variants, and extra fields", () => {
    const future = cloned(fixture.snapshotQuery);
    future.protocolVersion = fixture.incompatibility.futureProtocolVersion as number;
    expect(() => assertCompatibleNotificationSnapshotQuery(future)).toThrow(NotificationProtocolCompatibilityError);

    const command = cloned(fixture.mutationCommands[0]!) as unknown as Record<string, unknown>;
    command.kind = fixture.incompatibility.unknownMutationKind;
    expect(() => assertCompatibleNotificationMutationCommand(command)).toThrow(NotificationProtocolCompatibilityError);

    const response = cloned(fixture.snapshotResponse) as unknown as Record<string, unknown>;
    response.productPayload = { secret: true };
    expect(() => assertCompatibleNotificationSnapshotResponse(response)).toThrow(NotificationProtocolCompatibilityError);
  });
});
