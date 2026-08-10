import { describe, expect, test } from "bun:test";

import {
  assertValidNotificationChangedEvent,
  assertValidNotificationMutationCommand,
  assertValidNotificationMutationResult,
  assertValidNotificationSnapshotQuery,
  assertValidNotificationSnapshotResponse,
  NotificationProtocolValidationError,
} from "../../src/notifications/index.ts";
import { cloned, fixture } from "./support.ts";

describe("generated notification protocol", () => {
  test("accepts every Rust fixture value exactly", () => {
    assertValidNotificationSnapshotQuery(fixture.snapshotQuery);
    assertValidNotificationSnapshotResponse(fixture.snapshotResponse);
    fixture.mutationCommands.forEach(assertValidNotificationMutationCommand);
    fixture.mutationResults.forEach(assertValidNotificationMutationResult);
    fixture.changedEvents.filter((event) => event !== null).forEach(assertValidNotificationChangedEvent);
  });

  test("fails closed on future protocol, variants, and extra fields", () => {
    const future = cloned(fixture.snapshotQuery);
    future.protocolVersion = fixture.incompatibility.futureProtocolVersion as number;
    expect(() => assertValidNotificationSnapshotQuery(future)).toThrow(NotificationProtocolValidationError);

    const command = cloned(fixture.mutationCommands[0]!) as unknown as Record<string, unknown>;
    command.kind = fixture.incompatibility.unknownMutationKind;
    expect(() => assertValidNotificationMutationCommand(command)).toThrow(NotificationProtocolValidationError);

    const response = cloned(fixture.snapshotResponse) as unknown as Record<string, unknown>;
    response.productPayload = { secret: true };
    expect(() => assertValidNotificationSnapshotResponse(response)).toThrow(NotificationProtocolValidationError);
  });
});
