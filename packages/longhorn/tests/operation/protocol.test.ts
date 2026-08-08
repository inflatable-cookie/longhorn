import { describe, expect, test } from "bun:test";

import {
  assertCompatibleOperationCancellationCommand,
  assertCompatibleOperationCancellationResult,
  assertCompatibleOperationChangedEvent,
  assertCompatibleOperationMutationCommand,
  assertCompatibleOperationMutationResult,
  assertCompatibleOperationSnapshotQuery,
  assertCompatibleOperationSnapshotResponse,
  OperationProtocolCompatibilityError,
} from "../../src/operation/index.ts";
import { cloned, fixture } from "./support.ts";

describe("generated operation protocol", () => {
  test("accepts every Rust-produced golden trace value", () => {
    assertCompatibleOperationSnapshotQuery(fixture.snapshotQuery);
    assertCompatibleOperationSnapshotResponse(fixture.snapshotResponse);
    fixture.mutationCommands.forEach(assertCompatibleOperationMutationCommand);
    fixture.mutationResults.forEach(assertCompatibleOperationMutationResult);
    assertCompatibleOperationCancellationCommand(fixture.cancellationCommand);
    assertCompatibleOperationCancellationResult(fixture.cancellationResult);
    fixture.changedEvents.forEach(assertCompatibleOperationChangedEvent);
  });

  test("rejects future versions, unknown variants, unsafe integers, and product payloads", () => {
    const future = cloned(fixture.snapshotResponse);
    future.snapshot.protocolVersion = fixture.incompatibility.futureProtocolVersion as number;
    expect(() => assertCompatibleOperationSnapshotResponse(future)).toThrow(OperationProtocolCompatibilityError);

    const unknown = cloned(fixture.mutationCommands[0]! as unknown as Record<string, unknown>);
    unknown.kind = fixture.incompatibility.unknownMutationKind;
    expect(() => assertCompatibleOperationMutationCommand(unknown)).toThrow(OperationProtocolCompatibilityError);

    const unsafe = cloned(fixture.snapshotResponse);
    unsafe.snapshot.catalogueRevision = Number.MAX_SAFE_INTEGER + 1;
    expect(() => assertCompatibleOperationSnapshotResponse(unsafe)).toThrow(OperationProtocolCompatibilityError);

    const payload = cloned(fixture.snapshotResponse) as unknown as Record<string, unknown>;
    (payload.snapshot as Record<string, unknown>).payload = { secret: true };
    expect(() => assertCompatibleOperationSnapshotResponse(payload)).toThrow(/product payload field is forbidden/);

    const malformedReceipt = cloned(fixture.mutationResults[0]!) as unknown as Record<string, unknown>;
    (malformedReceipt.receipt as Record<string, unknown>).unexpected = true;
    expect(() => assertCompatibleOperationMutationResult(malformedReceipt)).toThrow(/expected keys/);
  });
});
