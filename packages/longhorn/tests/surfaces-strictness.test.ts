import { describe, expect, test } from "bun:test";

import {
  assertCompatibleSurfaceSnapshot,
  SurfaceProtocolIncompatibilityError,
} from "../src/surfaces/validation.ts";
import { SURFACE_PROTOCOL_VERSION } from "../src/surfaces/generated/protocol.ts";

/**
 * Contract 010's Boundary Validation Target, on the first package migrated to
 * it. Surfaces previously checked "is this an object" and stopped, so a
 * renamed or dropped field passed straight through.
 */
const valid = {
  protocol_version: SURFACE_PROTOCOL_VERSION,
  epoch: 1,
  revision: 1,
  document: { revision: 1, surfaces: [], windows: [] },
};

describe("surfaces boundary strictness", () => {
  test("accepts exactly the declared fields", () => {
    expect(() => assertCompatibleSurfaceSnapshot(valid)).not.toThrow();
  });

  test("rejects an unknown field", () => {
    expect(() =>
      assertCompatibleSurfaceSnapshot({ ...valid, extra: 1 }),
    ).toThrow(SurfaceProtocolIncompatibilityError);
  });

  test("rejects a missing field", () => {
    const { epoch: _dropped, ...missing } = valid;
    expect(() => assertCompatibleSurfaceSnapshot(missing)).toThrow(
      SurfaceProtocolIncompatibilityError,
    );
  });

  test("reports a non-object as invalid_object", () => {
    // It used to report `unknown_response_status`, because the union had no
    // code for a non-object. Wrong for every caller but one.
    try {
      assertCompatibleSurfaceSnapshot([]);
      throw new Error("expected a rejection");
    } catch (error) {
      expect((error as SurfaceProtocolIncompatibilityError).code).toBe(
        "invalid_object",
      );
    }
  });
});
