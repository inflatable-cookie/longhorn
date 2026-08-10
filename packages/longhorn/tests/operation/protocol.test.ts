import { describe, expect, test } from "bun:test";

import {
  assertValidOperationCancellationCommand,
  assertValidOperationCancellationResult,
  assertValidOperationChangedEvent,
  assertValidOperationMutationCommand,
  assertValidOperationMutationResult,
  assertValidOperationSnapshotQuery,
  assertValidOperationSnapshotResponse,
  OperationProtocolValidationError,
} from "../../src/operation/index.ts";
import { cloned, fixture } from "./support.ts";

describe("generated operation protocol", () => {
  test("accepts every Rust-produced golden trace value", () => {
    assertValidOperationSnapshotQuery(fixture.snapshotQuery);
    assertValidOperationSnapshotResponse(fixture.snapshotResponse);
    fixture.mutationCommands.forEach(assertValidOperationMutationCommand);
    fixture.mutationResults.forEach(assertValidOperationMutationResult);
    assertValidOperationCancellationCommand(fixture.cancellationCommand);
    assertValidOperationCancellationResult(fixture.cancellationResult);
    fixture.changedEvents.forEach(assertValidOperationChangedEvent);
  });

  test("rejects future versions, unknown variants, unsafe integers, and product payloads", () => {
    const future = cloned(fixture.snapshotResponse);
    future.snapshot.protocolVersion = fixture.incompatibility.futureProtocolVersion as number;
    expect(() => assertValidOperationSnapshotResponse(future)).toThrow(OperationProtocolValidationError);

    const unknown = cloned(fixture.mutationCommands[0]! as unknown as Record<string, unknown>);
    unknown.kind = fixture.incompatibility.unknownMutationKind;
    expect(() => assertValidOperationMutationCommand(unknown)).toThrow(OperationProtocolValidationError);

    const unsafe = cloned(fixture.snapshotResponse);
    unsafe.snapshot.catalogueRevision = Number.MAX_SAFE_INTEGER + 1;
    expect(() => assertValidOperationSnapshotResponse(unsafe)).toThrow(OperationProtocolValidationError);

    const payload = cloned(fixture.snapshotResponse) as unknown as Record<string, unknown>;
    (payload.snapshot as Record<string, unknown>).payload = { secret: true };
    expect(() => assertValidOperationSnapshotResponse(payload)).toThrow(/product payload field is forbidden/);

    const malformedReceipt = cloned(fixture.mutationResults[0]!) as unknown as Record<string, unknown>;
    (malformedReceipt.receipt as Record<string, unknown>).unexpected = true;
    expect(() => assertValidOperationMutationResult(malformedReceipt)).toThrow(/expected keys/);
  });
});
