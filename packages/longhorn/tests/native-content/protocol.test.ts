import { describe, expect, test } from "bun:test";
import fixture from "../../../../fixtures/native-content/protocol-v1.json";

import {
  NativeContentProtocolCompatibilityError,
  assertCompatibleNativeContentChangedEvent,
  assertCompatibleNativeContentConnectResult,
  assertCompatibleNativeContentDecisionResult,
  assertCompatibleNativeContentDesiredUpdateResult,
  assertCompatibleNativeContentSnapshotResult,
  assertProductPayloadFree,
} from "../../src/native-content/index.ts";
import { clone } from "./support.ts";

describe("Rust fixture parity", () => {
  test("accepts snapshots, observations, proposals, receipts, and rejections", () => {
    assertCompatibleNativeContentConnectResult(fixture.connect);
    assertCompatibleNativeContentSnapshotResult(fixture.snapshot);
    assertCompatibleNativeContentDesiredUpdateResult(fixture.desiredUpdate);
    assertCompatibleNativeContentChangedEvent(fixture.applyEvent);
    assertCompatibleNativeContentChangedEvent(fixture.observationEvent);
    assertCompatibleNativeContentChangedEvent(fixture.proposalEvent);
    assertCompatibleNativeContentDecisionResult(fixture.decision);
    assertCompatibleNativeContentDesiredUpdateResult(fixture.staleRevision);
    assertCompatibleNativeContentConnectResult(fixture.remount);
    assertCompatibleNativeContentSnapshotResult(fixture.staleSession);
    assertCompatibleNativeContentConnectResult(fixture.incompatible);
  });

  test("client epoch is distinct from attach generation", () => {
    assertCompatibleNativeContentConnectResult(fixture.remount);
    if (fixture.remount.status !== "connected") throw new Error("fixture rejected");
    expect(fixture.remount.snapshot.cursor.client_epoch).toBe(2);
    expect(fixture.remount.snapshot.cursor.attach_generation).toBe(1);
  });

  test("unknown variants and product payload fields fail closed", () => {
    const future = clone(fixture.connect) as Record<string, unknown>;
    const snapshot = future.snapshot as Record<string, unknown>;
    snapshot.protocol_version = 2;
    expect(() => assertCompatibleNativeContentConnectResult(future)).toThrow(
      NativeContentProtocolCompatibilityError,
    );

    const product = clone(fixture.connect) as Record<string, unknown>;
    (product.snapshot as Record<string, unknown>).plugin = { id: "synth" };
    expect(() => assertProductPayloadFree(product)).toThrow(
      NativeContentProtocolCompatibilityError,
    );

    const unknown = clone(fixture.proposalEvent) as Record<string, unknown>;
    (unknown.change as Record<string, unknown>).kind = "product_payload";
    expect(() => assertCompatibleNativeContentChangedEvent(unknown)).toThrow(
      NativeContentProtocolCompatibilityError,
    );
  });
});
