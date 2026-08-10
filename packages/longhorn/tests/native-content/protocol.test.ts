import { describe, expect, test } from "bun:test";
import fixture from "../../../../fixtures/native-content/protocol-v1.json";

import {
  NativeContentProtocolValidationError,
  assertValidNativeContentChangedEvent,
  assertValidNativeContentConnectResult,
  assertValidNativeContentDecisionResult,
  assertValidNativeContentDesiredUpdateResult,
  assertValidNativeContentSnapshotResult,
  assertProductPayloadFree,
} from "../../src/native-content/index.ts";
import { clone } from "./support.ts";

describe("Rust fixture parity", () => {
  test("accepts snapshots, observations, proposals, receipts, and rejections", () => {
    assertValidNativeContentConnectResult(fixture.connect);
    assertValidNativeContentSnapshotResult(fixture.snapshot);
    assertValidNativeContentDesiredUpdateResult(fixture.desiredUpdate);
    assertValidNativeContentChangedEvent(fixture.applyEvent);
    assertValidNativeContentChangedEvent(fixture.observationEvent);
    assertValidNativeContentChangedEvent(fixture.proposalEvent);
    assertValidNativeContentDecisionResult(fixture.decision);
    assertValidNativeContentDesiredUpdateResult(fixture.staleRevision);
    assertValidNativeContentConnectResult(fixture.remount);
    assertValidNativeContentSnapshotResult(fixture.staleSession);
    // The destroy receipt reaches its validator only inside a `host_destroyed`
    // event. The receipt is Rust-produced; the three envelope keys are checked
    // against a literal list in the validator itself, so assembling them here
    // does not weaken what the fixture proves.
    assertValidNativeContentChangedEvent({
      ...clone(fixture.applyEvent),
      change: {
        kind: "host_destroyed",
        request_id: "request:destroy",
        receipt: fixture.hostDestroy,
      },
    });
    assertValidNativeContentConnectResult(fixture.incompatible);
  });

  test("client epoch is distinct from attach generation", () => {
    assertValidNativeContentConnectResult(fixture.remount);
    if (fixture.remount.status !== "connected") throw new Error("fixture rejected");
    expect(fixture.remount.snapshot.cursor.client_epoch).toBe(2);
    expect(fixture.remount.snapshot.cursor.attach_generation).toBe(1);
  });

  test("unknown variants and product payload fields fail closed", () => {
    const future = clone(fixture.connect) as Record<string, unknown>;
    const snapshot = future.snapshot as Record<string, unknown>;
    snapshot.protocol_version = 2;
    expect(() => assertValidNativeContentConnectResult(future)).toThrow(
      NativeContentProtocolValidationError,
    );

    const product = clone(fixture.connect) as Record<string, unknown>;
    (product.snapshot as Record<string, unknown>).plugin = { id: "synth" };
    expect(() => assertProductPayloadFree(product)).toThrow(
      NativeContentProtocolValidationError,
    );

    const unknown = clone(fixture.proposalEvent) as Record<string, unknown>;
    (unknown.change as Record<string, unknown>).kind = "product_payload";
    expect(() => assertValidNativeContentChangedEvent(unknown)).toThrow(
      NativeContentProtocolValidationError,
    );
  });
});
